//! Application module
//!
//! Contains the main application logic, state management, and event handling.
//!
//! # Module Structure
//! - `state` - Application state types (AppState, AppMode, ToolDialogState, etc.)
//! - Main module - App struct and event loop

mod state;

// Re-export state types for external use
pub use state::{
    AppMode, AppState, ConfigEditState, ManualPartitionMap, PackageResult, ToolDialogState,
    ToolParam, ToolParameter,
};

use crate::components::confirm_dialog::{
    start_install_confirm, ConfirmDialogState,
    ConfirmSeverity,
};
use crate::components::floating_window::FloatingOutputState;
use crate::components::keybindings::KeybindingContext;
use crate::components::pty_terminal::{PtyTerminal, PtyTerminalState};
use crate::config::Configuration;
use crate::error;
use crate::hardware::HardwareInfo;
use crate::input::InputHandler;
use crate::installer::Installer;
use crate::process_guard::{ChildRegistry, CommandProcessGroup, ProcessGuard};
use crate::script_manifest::ManifestRegistry;
use crate::script_traits::{is_dry_run, shell_safe, ScriptArgs};
use crate::scripts::config::{GenFstabArgs, UserAddArgs};
use crate::scripts::disk::{
    AddPartitionArgs, CheckDiskHealthArgs, CreateTableArgs, DeletePartitionArgs,
    FormatPartitionArgs, ManualPartitionArgs, WipeDiskArgs, WipeMethod,
};
use crate::scripts::encryption::{LuksCloseArgs, LuksFormatArgs, LuksCipher, LuksOpenArgs, SecretFile};
use crate::scripts::network::{MirrorSortMethod, NetworkDiagnosticsArgs, TestNetworkArgs, UpdateMirrorsArgs};
use crate::scripts::profiles::{EnableServicesArgs, InstallDotfilesArgs};
use crate::scripts::system::{BootloaderArgs, ChrootArgs, ServicesArgs, SystemInfoArgs};
use crate::scripts::user::{GroupsArgs, ResetPasswordArgs, SecurityAuditArgs, SshArgs};
use crate::scripts::user_ops::{InstallAurHelperArgs, UserRunArgs};
use crate::types::{AurHelper, DesktopEnvironment};
use crate::ui::UiRenderer;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use tracing::{debug, info};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Messages sent from tool execution threads to the main UI thread
#[derive(Debug)]
pub enum ToolMessage {
    /// A line of stdout output
    Stdout(String),
    /// A line of stderr output
    Stderr(String),
    /// Tool execution completed successfully
    Complete { success: bool, exit_code: Option<i32> },
    /// Tool execution failed to start
    Error(String),
}

/// Main application struct
pub struct App {
    state: Arc<Mutex<AppState>>,
    installer: Option<Installer>,
    ui_renderer: UiRenderer,
    input_handler: InputHandler,
    save_config_path: Option<std::path::PathBuf>,
    /// PTY terminal for embedded interactive tools
    pty_terminal: Option<PtyTerminal>,
    /// Keybinding context for navigation hints
    keybinding_context: KeybindingContext,
    /// Channel sender for tool execution output (cloned to threads)
    tool_tx: Sender<ToolMessage>,
    /// Channel receiver for tool execution output (polled in main loop)
    tool_rx: Receiver<ToolMessage>,
    /// Process guard for child process lifecycle management
    /// Ensures all spawned bash scripts are terminated when App is dropped
    _process_guard: ProcessGuard,
    /// Detected hardware info (firmware mode, network state)
    /// Set once at startup via HardwareInfo::detect()
    hardware_info: HardwareInfo,
    /// Script manifest registry for validating tool executions
    manifest_registry: ManifestRegistry,
    /// Active LUKS SecretFile kept alive during encryption operations
    _active_secret_file: Option<SecretFile>,
}

// =============================================================================
// SECTION: Application State Management
// =============================================================================
//
// AppState Machine Overview:
//
//   MainMenu ──┬── GuidedInstaller ───── Installation ───── Complete
//              │
//              ├── AutomatedInstall ───── FileBrowser ───── Installation
//              │
//              └── ToolsMenu ──┬── DiskTools ──┬── ToolDialog ── FloatingOutput
//                              │               │
//                              ├── SystemTools │── EmbeddedTerminal
//                              │               │
//                              ├── UserTools   └── ConfirmDialog
//                              │
//                              └── NetworkTools
//
// State Transitions:
// - Navigation (Up/Down/Enter/Esc) drives menu traversal
// - Tool execution creates FloatingOutput or EmbeddedTerminal
// - Confirmation dialogs gate destructive operations
// - Installation mode runs in background with streaming output
//
// =============================================================================

impl App {
    /// Helper function to safely lock the state mutex.
    /// MutexGuard provides both &T and &mut T — use `let mut state` at
    /// the call site when mutation is needed.
    ///
    /// Recovers from mutex poisoning per ROE §2.1: "Recover from mutex
    /// poisoning, never panic." If the mutex is poisoned (a thread panicked
    /// while holding the lock), the guard is recovered via `into_inner()`.
    fn lock_state(&self) -> std::sync::MutexGuard<'_, AppState> {
        self.state
            .lock()
            .unwrap_or_else(|e| {
                tracing::warn!("Mutex was poisoned, recovering state");
                e.into_inner()
            })
    }

    /// Create a new application instance
    ///
    /// Accepts `HardwareInfo` from `HardwareInfo::detect()` so the TUI knows
    /// the firmware mode and network state before presenting options.
    pub fn new(save_config_path: Option<std::path::PathBuf>, hardware_info: HardwareInfo) -> Self {
        info!(firmware = %hardware_info.firmware, network = %hardware_info.network, "Creating new App instance");
        let (tool_tx, tool_rx) = mpsc::channel();

        // ProcessGuard ensures all child processes are killed when App is dropped
        // This prevents orphaned bash scripts continuing after TUI crash
        let process_guard = ProcessGuard::new();
        debug!("ProcessGuard initialized for child process tracking");

        // Load script manifests for runtime validation
        let mut manifest_registry = ManifestRegistry::with_core_manifests();
        if let Err(e) = manifest_registry.load_from_directory("scripts/manifests") {
            tracing::warn!("Failed to load manifests from scripts/manifests: {}", e);
        } else {
            info!("Script manifests loaded successfully");
        }

        Self {
            state: Arc::new(Mutex::new(AppState::default())),
            installer: None,
            ui_renderer: UiRenderer::new(),
            input_handler: InputHandler::new(),
            save_config_path,
            pty_terminal: None,
            keybinding_context: KeybindingContext::new(),
            tool_tx,
            tool_rx,
            _process_guard: process_guard,
            hardware_info,
            manifest_registry,
            _active_secret_file: None,
        }
    }

    /// Get reference to keybinding context
    #[allow(dead_code)] // API method available for future use
    pub fn keybinding_context(&self) -> &KeybindingContext {
        &self.keybinding_context
    }

    /// Get reference to detected hardware info
    #[allow(dead_code)] // API method — consumed when InstallerContext is created
    pub fn hardware_info(&self) -> &HardwareInfo {
        &self.hardware_info
    }

    /// Toggle help overlay visibility
    pub fn toggle_help(&self) {
        { let mut state = self.lock_state();
            state.help_visible = !state.help_visible;
        }
    }

    /// Load a configuration file and start installation
    fn load_config_file(&mut self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        use crate::config_file::InstallationConfig;

        // Clear file browser state first
        {
            let mut state = self.lock_state();
            state.file_browser = None;
        }

        // Load and validate the config file
        match InstallationConfig::load_from_file(path) {
            Ok(config) => {
                match config.validate() {
                    Ok(_) => {
                        // Config is valid - start installation
                        let mut state = self.lock_state();
                        state.status_message = format!(
                            "Configuration loaded from: {}",
                            path.display()
                        );

                        // Set up floating output to show config details
                        let mut content = vec![
                            format!("Configuration file: {}", path.display()),
                            String::new(),
                            "Configuration loaded successfully!".to_string(),
                            String::new(),
                            format!("Disk: {}", config.install_disk),
                            format!("Hostname: {}", config.hostname),
                            format!("Username: {}", config.username),
                            format!("Bootloader: {}", config.bootloader),
                            String::new(),
                        ];

                        if config.desktop_environment != crate::types::DesktopEnvironment::None {
                            content.push(format!("Desktop: {}", config.desktop_environment));
                        }

                        content.push(String::new());
                        content.push("Press Enter to start installation or Esc to cancel".to_string());

                        state.floating_output = Some(crate::components::floating_window::FloatingOutputState {
                            title: "Configuration Loaded".to_string(),
                            content,
                            scroll_offset: 0,
                            auto_scroll: false,
                            complete: true,
                            progress: None,
                            status: "Ready to install".to_string(),
                        });
                        state.pre_dialog_mode = Some(AppMode::AutomatedInstall);
                        state.mode = AppMode::FloatingOutput;
                    }
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.mode = AppMode::AutomatedInstall;
                        state.status_message = format!("Config validation failed: {}", e);
                    }
                }
            }
            Err(e) => {
                let mut state = self.lock_state();
                state.mode = AppMode::AutomatedInstall;
                state.status_message = format!("Failed to load config: {}", e);
            }
        }

        Ok(())
    }

    /// Launch an embedded terminal for interactive tools
    pub fn launch_embedded_tool(
        &mut self,
        cmd: &str,
        args: &[&str],
        tool_name: &str,
        return_mode: AppMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use crate::components::pty_terminal::{spawn_or_fallback, PtySpawnResult};

        // Get terminal size
        let (cols, rows) = crossterm::terminal::size()?;
        let pty_rows = rows.saturating_sub(2); // Reserve space for nav bar

        match spawn_or_fallback(cmd, args, cols, pty_rows) {
            PtySpawnResult::Success(pty) => {
                self.pty_terminal = Some(*pty);

                let mut state = self.lock_state();
                let return_menu_selection = state.tools_menu_selection;
                state.embedded_terminal = Some(PtyTerminalState {
                    tool_name: tool_name.to_string(),
                    return_mode,
                    return_menu_selection,
                });
                state.mode = AppMode::EmbeddedTerminal;
                Ok(())
            }
            PtySpawnResult::Fallback(reason) => {
                // Log the fallback reason and use passthrough mode
                tracing::warn!("PTY fallback: {}", reason);
                self.launch_passthrough_tool(cmd, args, return_mode)
            }
        }
    }

    /// Launch a tool in passthrough mode (fallback when PTY fails)
    fn launch_passthrough_tool(
        &mut self,
        cmd: &str,
        args: &[&str],
        return_mode: AppMode,
    ) -> Result<(), Box<dyn std::error::Error>> {
        use std::process::Command;

        // Temporarily leave alternate screen
        crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen
        )?;
        crossterm::terminal::disable_raw_mode()?;

        // Run the command (Death Pact: process group isolation prevents orphaned children)
        let status = Command::new(cmd).args(args).in_new_process_group().status();

        // Return to alternate screen
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::EnterAlternateScreen
        )?;

        // Check status and return to appropriate mode
        match status {
            Ok(exit_status) => {
                let mut state = self.lock_state();
                if exit_status.success() {
                    state.status_message = format!("{} completed successfully", cmd);
                } else {
                    state.status_message = format!("{} exited with error", cmd);
                }
                state.mode = return_mode;
            }
            Err(e) => {
                let mut state = self.lock_state();
                state.status_message = format!("Failed to run {}: {}", cmd, e);
                state.mode = return_mode;
            }
        }

        Ok(())
    }

    /// Exit embedded terminal and return to previous mode
    fn exit_embedded_terminal(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Kill the PTY if running
        if let Some(ref mut pty) = self.pty_terminal {
            pty.kill();
        }
        self.pty_terminal = None;

        // Check if cfdisk exit should loop back to disk layout
        let pending_device = {
            let state = self.lock_state();
            let tool_name = state.embedded_terminal.as_ref().map(|t| t.tool_name.clone());
            if tool_name.as_deref() == Some("manual_partition") {
                state.pending_tool_device.clone()
            } else {
                None
            }
        };

        if let Some(device) = pending_device {
            // cfdisk completed → show updated disk layout
            let mut state = self.lock_state();
            state.embedded_terminal = None;
            state.current_tool = Some("manual_partition".to_string());
            drop(state);
            self.show_disk_layout(&device);
            return Ok(());
        }

        // Return to previous mode
        let mut state = self.lock_state();
        if let Some(terminal_state) = state.embedded_terminal.take() {
            state.mode = terminal_state.return_mode;
            state.tools_menu_selection = terminal_state.return_menu_selection;
            state.status_message = format!("{} closed", terminal_state.tool_name);
        } else {
            state.mode = AppMode::MainMenu;
        }

        Ok(())
    }

    /// Poll PTY output if in embedded terminal mode
    fn poll_pty(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(ref mut pty) = self.pty_terminal {
            // Check if PTY is still running
            if !pty.is_running() {
                // PTY exited, return to previous mode
                self.exit_embedded_terminal()?;
            }
        }
        Ok(())
    }

    /// Poll for tool execution messages from background threads
    /// Strip ANSI escape sequences from a string to prevent terminal corruption
    fn strip_ansi(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut chars = s.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '\x1b' {
                // Skip ESC [ ... (letter) sequences (CSI)
                if chars.peek() == Some(&'[') {
                    chars.next(); // consume '['
                    // Consume until we hit a letter (the terminator)
                    while let Some(&next) = chars.peek() {
                        chars.next();
                        if next.is_ascii_alphabetic() {
                            break;
                        }
                    }
                }
                // Skip other ESC sequences (ESC followed by one char)
                else if chars.peek().is_some() {
                    chars.next();
                }
            } else {
                result.push(c);
            }
        }
        result
    }

    fn poll_tool_messages(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Track whether tool finished so we can clean up SecretFile after releasing state lock
        let mut should_clear_secret = false;

        // Process all pending messages without blocking
        while let Ok(msg) = self.tool_rx.try_recv() {
            let mut state = self.lock_state();

            match msg {
                ToolMessage::Stdout(line) => {
                    if let Some(ref mut floating) = state.floating_output {
                        floating.append_line(Self::strip_ansi(&line));
                        // Auto-scroll to bottom if enabled
                        if floating.auto_scroll {
                            floating.scroll_offset = floating.content.len().saturating_sub(1);
                        }
                    }
                }
                ToolMessage::Stderr(line) => {
                    if let Some(ref mut floating) = state.floating_output {
                        floating.append_line(format!("⚠ {}", Self::strip_ansi(&line)));
                        if floating.auto_scroll {
                            floating.scroll_offset = floating.content.len().saturating_sub(1);
                        }
                    }
                }
                ToolMessage::Complete { success, exit_code } => {
                    // Update status message first (before borrowing floating_output)
                    let status_msg = if success {
                        "Tool completed successfully".to_string()
                    } else {
                        format!("Tool failed with exit code: {}", exit_code.map(|c| c.to_string()).unwrap_or_else(|| "killed by signal".to_string()))
                    };
                    state.status_message = status_msg.clone();
                    state.current_tool = None;
                    should_clear_secret = true;

                    // Now update floating output
                    if let Some(ref mut floating) = state.floating_output {
                        floating.append_line(String::new());
                        if success {
                            floating.append_line("✅ Tool completed successfully".to_string());
                        } else {
                            floating.append_line(format!(
                                "❌ Tool failed with exit code: {}",
                                exit_code.map(|c| c.to_string()).unwrap_or_else(|| "killed by signal".to_string())
                            ));
                        }
                        floating.append_line(String::new());
                        floating.append_line("Press Esc or Enter to close".to_string());
                        floating.mark_complete();
                    }
                }
                ToolMessage::Error(err) => {
                    state.status_message = format!("Tool error: {}", err);
                    state.current_tool = None;
                    should_clear_secret = true;

                    if let Some(ref mut floating) = state.floating_output {
                        floating.append_line(format!("❌ Error: {}", err));
                        floating.append_line(String::new());
                        floating.append_line("Press Esc or Enter to close".to_string());
                        floating.mark_complete();
                    }
                }
            }
        }

        // Clean up any active secret file (LUKS keyfile) after tool completion
        if should_clear_secret {
            self._active_secret_file = None;
        }

        Ok(())
    }

    // =========================================================================
    // SECTION: Main Event Loop
    // =========================================================================
    //
    // The main loop runs at ~20Hz (50ms poll timeout) and:
    // 1. Polls PTY for embedded terminal output
    // 2. Polls tool_rx channel for script execution output
    // 3. Handles keyboard/resize events
    // 4. Checks for completion
    // 5. Renders UI
    //
    // Exit conditions:
    // - User selects Quit from main menu
    // - Mode reaches AppMode::Complete after installation
    // - Ctrl+C / SIGTERM (handled by ProcessGuard)
    //
    // =========================================================================

    /// Run the main application loop
    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting main application loop");

        loop {
            // Poll PTY if in embedded terminal mode
            self.poll_pty()?;

            // Poll for tool execution output messages
            self.poll_tool_messages()?;

            // Handle input events
            if crossterm::event::poll(Duration::from_millis(50))? {
                match crossterm::event::read()? {
                    Event::Key(key_event) => {
                        if self.handle_key_event(key_event)? {
                            break; // Exit requested
                        }
                    }
                    Event::Resize(width, height) => {
                        // Handle window resize - update scroll state
                        self.handle_resize(width, height)?;
                        // Also resize PTY if active
                        if let Some(ref mut pty) = self.pty_terminal {
                            let _ = pty.resize(width, height.saturating_sub(2));
                        }
                    }
                    _ => {}
                }
            }

            // Render UI
            let mut render_poisoned = false;
            terminal.draw(|f| {
                let mut state = match self.state.lock() {
                    Ok(state) => state,
                    Err(poisoned) => {
                        render_poisoned = true;
                        poisoned.into_inner()
                    }
                };
                // Update scroll state with actual available space for config options
                if state.mode == AppMode::GuidedInstaller {
                    // Calculate the config area height (total height minus reserved space)
                    let config_area_height = f.area().height.saturating_sub(17); // 17 lines reserved (includes nav bar)
                    let visible_items = config_area_height.saturating_sub(2); // Account for borders
                    state
                        .config_scroll
                        .update_visible_items(visible_items as usize);
                }
                // Sync actual installer output viewport height for scroll calculations
                if state.mode == AppMode::Installation {
                    // Layout: nav_bar(1) + header(7) + title(3) + progress(3) + status(1) + output(rest)
                    // Output block has borders (2 lines), so inner height = total - 17 - 2
                    let output_inner = f.area().height.saturating_sub(19) as usize;
                    if output_inner > 0 {
                        state.installer_visible_height = output_inner;
                    }
                }
                self.ui_renderer
                    .render_with_context(f, &state, &mut self.input_handler, &self.keybinding_context, self.pty_terminal.as_mut());
            })?;
            if render_poisoned {
                return Err("Fatal error: Mutex poisoned, cannot continue".into());
            }
        }

        Ok(())
    }

    /// Handle keyboard input events
    fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Get current mode and help visibility
        let (current_mode, help_visible) = {
            let state = self.lock_state();
            (state.mode.clone(), state.help_visible)
        };

        // Handle embedded terminal mode - forward all keys except Ctrl+Q
        if current_mode == AppMode::EmbeddedTerminal {
            // Ctrl+Q exits the embedded terminal
            if key_event.modifiers.contains(KeyModifiers::CONTROL)
                && key_event.code == KeyCode::Char('q')
            {
                self.exit_embedded_terminal()?;
                return Ok(false);
            }

            // Forward all other keys to PTY
            if let Some(ref mut pty) = self.pty_terminal {
                let _ = pty.send_key(key_event);
            }
            return Ok(false);
        }

        // Handle help overlay - ? or Esc dismisses it
        if help_visible {
            match key_event.code {
                KeyCode::Char('?') | KeyCode::Esc => {
                    self.toggle_help();
                }
                _ => {}
            }
            return Ok(false);
        }

        // Global help toggle with '?' (except in dialogs and embedded terminal)
        if key_event.code == KeyCode::Char('?') && !self.input_handler.is_dialog_active() {
            self.toggle_help();
            return Ok(false);
        }

        // Check if we're in a tool dialog
        let is_tool_dialog = current_mode == AppMode::ToolDialog;

        if is_tool_dialog {
            self.handle_tool_dialog_input(key_event)?;
            return Ok(false);
        }

        // Check if we're in an input dialog
        if self.input_handler.is_dialog_active() {
            if let Some(value) = self.input_handler.handle_input(key_event) {
                // Check if we're in disk selection mode for a tool
                let current_tool = {
                    let state = self.state.lock().map_err(|_| "Failed to lock state")?;
                    state.current_tool.clone()
                };

                if let Some(tool) = current_tool {
                    match tool.as_str() {
                        "health" => {
                            // Handle disk selection for health tool
                            self.execute_health_tool_with_disk(value)?;
                            return Ok(false);
                        }
                        "manual_partition" | "format_partition" | "wipe_disk" => {
                            // All three disk tools: show layout first, then proceed to action
                            let device = value.split_whitespace().next().unwrap_or(&value).to_string();
                            {
                                let mut state = self.lock_state();
                                state.pending_tool_device = Some(device.clone());
                            }
                            self.show_disk_layout(&device);
                            return Ok(false);
                        }
                        _ => {}
                    }
                }

                // Check if we're in the manual partition assignment flow
                if self.input_handler.manual_assign_state.is_some() {
                    if let Some(map) = self.input_handler.advance_manual_assignment(value) {
                        let mut state = self.lock_state();
                        state.manual_partition_map = Some(map);
                        state.status_message =
                            "Partition assignments complete. Ready to install."
                                .to_string();
                    } else {
                        // More steps remain — update status hint
                        if let Some(ref assign) = self.input_handler.manual_assign_state {
                            let hint = match assign.step {
                                crate::input::ManualAssignStep::Root => {
                                    "Select ROOT partition"
                                }
                                crate::input::ManualAssignStep::RootFs => {
                                    "Select ROOT filesystem"
                                }
                                crate::input::ManualAssignStep::Boot => {
                                    "Select BOOT partition"
                                }
                                crate::input::ManualAssignStep::Efi => {
                                    "Select EFI partition"
                                }
                                crate::input::ManualAssignStep::Home => {
                                    "Select HOME partition (or skip)"
                                }
                                crate::input::ManualAssignStep::HomeFs => {
                                    "Select HOME filesystem"
                                }
                                crate::input::ManualAssignStep::Swap => {
                                    "Select SWAP partition (or skip)"
                                }
                            };
                            let mut state = self.lock_state();
                            state.status_message =
                                format!("Assign partitions: {}", hint);
                        }
                    }
                    return Ok(false);
                }

                // User confirmed input, update configuration
                self.update_configuration_value(value)?;
            }
            return Ok(false);
        }

        // Handle floating output mode
        if current_mode == AppMode::FloatingOutput {
            match key_event.code {
                KeyCode::Enter => {
                    let mut state = self.lock_state();
                    if let Some(output) = state.floating_output.take() {
                        if !output.complete {
                            if let Ok(mut registry) = ChildRegistry::global().lock() {
                                registry.terminate_current(Duration::from_secs(3));
                            }
                        }
                        // Check if we're in the disk layout → action loop
                        let pending_device = state.pending_tool_device.clone();
                        let tool = state.current_tool.clone();
                        if let (Some(device), Some(ref tool_name)) = (pending_device, &tool) {
                            match tool_name.as_str() {
                                "manual_partition" => {
                                    // Enter = correct disk → show action menu
                                    drop(state);
                                    self.show_partition_action_menu(&device);
                                    return Ok(false);
                                }
                                "format_partition" => {
                                    // Enter = correct disk → open format ToolDialog
                                    drop(state);
                                    self.open_format_dialog_for_device(&device);
                                    return Ok(false);
                                }
                                "wipe_disk" => {
                                    // Enter = correct disk → open wipe ToolDialog
                                    drop(state);
                                    self.open_wipe_dialog_for_device(&device);
                                    return Ok(false);
                                }
                                "partition_create_table" | "partition_add" | "partition_delete" => {
                                    // Action completed → loop back: show updated layout
                                    state.current_tool = Some("manual_partition".to_string());
                                    drop(state);
                                    self.show_disk_layout(&device);
                                    return Ok(false);
                                }
                                _ => {}
                            }
                        }
                        // Default: return to previous mode
                        state.mode = state.pre_dialog_mode.take().unwrap_or(AppMode::ToolsMenu);
                        state.current_tool = None;
                    }
                }
                KeyCode::Esc | KeyCode::Char('b') | KeyCode::Char('B') => {
                    // Dismiss floating output and return to DiskTools (or previous mode)
                    let mut state = self.lock_state();
                    if let Some(output) = state.floating_output.take() {
                        if !output.complete {
                            if let Ok(mut registry) = ChildRegistry::global().lock() {
                                registry.terminate_current(Duration::from_secs(3));
                            }
                        }
                        state.pending_tool_device = None;
                        state.mode = state.pre_dialog_mode.take().unwrap_or(AppMode::ToolsMenu);
                        state.current_tool = None;
                    }
                }
                KeyCode::Up => {
                    let mut state = self.lock_state();
                    if let Some(ref mut output) = state.floating_output {
                        if output.scroll_offset > 0 {
                            output.scroll_offset -= 1;
                            output.auto_scroll = false;
                        }
                    }
                }
                KeyCode::Down => {
                    let mut state = self.lock_state();
                    if let Some(ref mut output) = state.floating_output {
                        let max = output.content.len().saturating_sub(1);
                        if output.scroll_offset < max {
                            output.scroll_offset += 1;
                        }
                        if output.scroll_offset >= max {
                            output.auto_scroll = true;
                        }
                    }
                }
                _ => {}
            }
            return Ok(false);
        }

        // Handle file browser mode
        if current_mode == AppMode::FileBrowser {
            let mut state = self.lock_state();
            if let Some(ref mut browser) = state.file_browser {
                match key_event.code {
                    KeyCode::Esc => {
                        browser.cancel();
                    }
                    KeyCode::Enter => {
                        browser.handle_enter();
                    }
                    KeyCode::Up => {
                        browser.move_up();
                    }
                    KeyCode::Down => {
                        browser.move_down();
                    }
                    KeyCode::Char('~') => {
                        browser.go_home();
                    }
                    KeyCode::Char('/') => {
                        browser.go_root();
                    }
                    _ => {}
                }

                // Check if file browser is complete
                if browser.complete {
                    if let Some(selected_path) = browser.selected_file.clone() {
                        // Load the config file
                        drop(state); // Release the lock before loading
                        self.load_config_file(&selected_path)?;
                    } else {
                        // Cancelled - return to automated install screen
                        state.file_browser = None;
                        state.mode = AppMode::AutomatedInstall;
                        state.status_message = "File selection cancelled".to_string();
                    }
                }
            }
            return Ok(false);
        }

        // Handle confirm dialog mode
        if current_mode == AppMode::ConfirmDialog {
            let mut state = self.lock_state();
            if let Some(ref mut dialog) = state.confirm_dialog {
                match key_event.code {
                    KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                        // Toggle between No (0) and Yes (1)
                        let old_selected = dialog.selected;
                        dialog.selected = if dialog.selected == 0 { 1 } else { 0 };
                        tracing::debug!("ConfirmDialog toggle: {} -> {} (0=No/left, 1=Yes/right)",
                            old_selected, dialog.selected);
                    }
                    KeyCode::Enter => {
                        // SECURITY FIX: Use is_confirmed() method to get correct selection
                        // selected = 0 means No/Cancel (left), selected = 1 means Yes/Confirm (right)
                        let confirmed = dialog.is_confirmed(); // Returns true if selected == 1 (Yes on right)
                        let action = dialog.confirm_action.clone();
                        let data = dialog.action_data.clone();

                        tracing::info!("ConfirmDialog Enter: selected={}, is_confirmed={}, action={}",
                            dialog.selected, confirmed, action);

                        // Clear dialog and restore previous mode
                        state.confirm_dialog = None;
                        if let Some(prev_mode) = state.pre_dialog_mode.take() {
                            state.mode = prev_mode;
                        } else {
                            state.mode = AppMode::ToolsMenu;
                        }

                        if confirmed {
                            tracing::info!("Executing confirmed action: {}", action);
                            // Drop the lock before executing action
                            drop(state);
                            self.execute_confirmed_action(&action, data)?;
                        } else {
                            tracing::info!("Action cancelled, returning to previous mode");
                        }
                    }
                    KeyCode::Esc => {
                        // Cancel - restore previous mode
                        state.confirm_dialog = None;
                        if let Some(prev_mode) = state.pre_dialog_mode.take() {
                            state.mode = prev_mode;
                        } else {
                            state.mode = AppMode::ToolsMenu;
                        }
                    }
                    _ => {}
                }
            }
            return Ok(false);
        }

        // Handle Left/Right/Tab for button toggle in GuidedInstaller mode
        if current_mode == AppMode::GuidedInstaller
            && matches!(key_event.code, KeyCode::Left | KeyCode::Right | KeyCode::Tab)
        {
            let mut state = self.lock_state();
            if state.config_scroll.selected_index == state.config.options.len() {
                match key_event.code {
                    KeyCode::Left => {
                        state.installer_button_selection =
                            state.installer_button_selection.saturating_sub(1);
                    }
                    KeyCode::Right | KeyCode::Tab => {
                        state.installer_button_selection =
                            (state.installer_button_selection + 1).min(2);
                    }
                    _ => {}
                }
            }
            return Ok(false);
        }

        // Guard: Q during active installation requires confirmation via B/Escape, not bare Q
        if current_mode == AppMode::Installation {
            match key_event.code {
                KeyCode::Up => { self.navigate_up(); }
                KeyCode::Down => { self.navigate_down(); }
                KeyCode::Char('b') | KeyCode::Char('B') | KeyCode::Esc => {
                    self.handle_back_key()?;
                }
                _ => {}
            }
            return Ok(false);
        }

        // Handle main application navigation
        match key_event.code {
            KeyCode::Char('q') => {
                // Exit application
                return Ok(true);
            }
            KeyCode::Char('b') | KeyCode::Char('B') => {
                // Go back in menu system
                self.handle_back_key()?;
            }
            KeyCode::Up => {
                self.navigate_up();
            }
            KeyCode::Down => {
                self.navigate_down();
            }
            KeyCode::PageUp => {
                self.page_up();
            }
            KeyCode::PageDown => {
                self.page_down();
            }
            KeyCode::Home => {
                self.move_to_first();
            }
            KeyCode::End => {
                self.move_to_last();
            }
            KeyCode::Enter => {
                if self.handle_enter()? {
                    return Ok(true);
                }
            }
            _ => {}
        }

        Ok(false)
    }

    // =========================================================================
    // SECTION: Navigation Handlers
    // =========================================================================
    //
    // These functions handle Up/Down/PageUp/PageDown/Home/End key navigation
    // within menus and dialogs. Each mode has its own navigation bounds.
    //
    // =========================================================================

    /// Navigate to previous option
    fn navigate_up(&self) {
        { let mut state = self.lock_state();
            match state.mode {
                AppMode::MainMenu => {
                    if state.main_menu_selection > 0 {
                        state.main_menu_selection -= 1;
                    }
                }
                AppMode::ToolsMenu
                | AppMode::DiskTools
                | AppMode::SystemTools
                | AppMode::UserTools
                | AppMode::NetworkTools => {
                    if state.tools_menu_selection > 0 {
                        state.tools_menu_selection -= 1;
                    }
                }
                AppMode::ToolDialog => {
                    if let Some(ref mut dialog) = state.tool_dialog {
                        if dialog.current_param > 0 {
                            dialog.current_param -= 1;
                        }
                    }
                }
                AppMode::GuidedInstaller => {
                    state.config_scroll.move_up();
                }
                AppMode::Installation => {
                    // Snapshot current position when switching from auto to manual
                    if state.installer_auto_scroll {
                        state.installer_scroll_offset =
                            state.installer_output.len().saturating_sub(state.installer_visible_height);
                    }
                    state.installer_auto_scroll = false;
                    state.installer_scroll_offset =
                        state.installer_scroll_offset.saturating_sub(1);
                }
                AppMode::DryRunSummary => {
                    state.dry_run_scroll_offset = state.dry_run_scroll_offset.saturating_sub(1);
                }
                _ => {}
            }
        }
    }

    /// Navigate to next option
    fn navigate_down(&self) {
        { let mut state = self.lock_state();
            match state.mode {
                AppMode::MainMenu => {
                    if state.main_menu_selection < 3 {
                        // 4 items total (0-3)
                        state.main_menu_selection += 1;
                    }
                }
                AppMode::ToolsMenu => {
                    if state.tools_menu_selection < 4 {
                        // 5 items total (0-4)
                        state.tools_menu_selection += 1;
                    }
                }
                AppMode::DiskTools => {
                    if state.tools_menu_selection < 6 {
                        // 7 items total (0-6)
                        state.tools_menu_selection += 1;
                    }
                }
                AppMode::SystemTools => {
                    if state.tools_menu_selection < 9 {
                        // 10 items total (0-9)
                        state.tools_menu_selection += 1;
                    }
                }
                AppMode::UserTools => {
                    if state.tools_menu_selection < 7 {
                        // 8 items total (0-7)
                        state.tools_menu_selection += 1;
                    }
                }
                AppMode::NetworkTools => {
                    if state.tools_menu_selection < 5 {
                        // 6 items total (0-5)
                        state.tools_menu_selection += 1;
                    }
                }
                AppMode::ToolDialog => {
                    if let Some(ref mut dialog) = state.tool_dialog {
                        if dialog.current_param < dialog.parameters.len().saturating_sub(1) {
                            dialog.current_param += 1;
                        }
                    }
                }
                AppMode::GuidedInstaller => {
                    state.config_scroll.move_down();
                }
                AppMode::Installation => {
                    let max_offset = state.installer_output.len().saturating_sub(1);
                    if state.installer_scroll_offset < max_offset {
                        state.installer_scroll_offset += 1;
                    }
                    // Re-enable auto-scroll when scrolled near bottom
                    if state.installer_scroll_offset
                        >= state.installer_output.len().saturating_sub(state.installer_visible_height)
                    {
                        state.installer_auto_scroll = true;
                    }
                }
                AppMode::DryRunSummary => {
                    if let Some(ref summary) = state.dry_run_summary {
                        let max = summary.len().saturating_sub(1);
                        if state.dry_run_scroll_offset < max {
                            state.dry_run_scroll_offset += 1;
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Page up in configuration list
    fn page_up(&self) {
        { let mut state = self.lock_state();
            if state.mode == AppMode::GuidedInstaller {
                state.config_scroll.page_up();
            }
        }
    }

    /// Page down in configuration list
    fn page_down(&self) {
        { let mut state = self.lock_state();
            if state.mode == AppMode::GuidedInstaller {
                state.config_scroll.page_down();
            }
        }
    }

    /// Move to first configuration option
    fn move_to_first(&self) {
        { let mut state = self.lock_state();
            if state.mode == AppMode::GuidedInstaller {
                state.config_scroll.move_to_first();
            }
        }
    }

    /// Move to last configuration option
    fn move_to_last(&self) {
        { let mut state = self.lock_state();
            if state.mode == AppMode::GuidedInstaller {
                state.config_scroll.move_to_last();
            }
        }
    }

    // =========================================================================
    // SECTION: Enter Key / Selection Handlers
    // =========================================================================
    //
    // These functions handle Enter key presses and menu selections.
    // Each AppMode has specific behavior when Enter is pressed:
    //
    // - MainMenu: Navigate to Guided/Automated/Tools or Quit
    // - ToolsMenu: Navigate to tool categories (Disk/System/User/Network)
    // - DiskTools/SystemTools/UserTools/NetworkTools: Execute selected tool
    // - GuidedInstaller: Open input dialog for current option
    // - ToolDialog: Submit parameters and execute tool
    // - ConfirmDialog: Execute or cancel confirmed action
    // - FloatingOutput: Dismiss output window
    //
    // =========================================================================

    /// Handle Enter key press. Returns true if the app should quit.
    fn handle_enter(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let current_mode = {
            let state = self.lock_state();
            state.mode.clone()
        };

        match current_mode {
            AppMode::MainMenu => {
                if self.handle_main_menu_selection()? {
                    return Ok(true);
                }
            }
            AppMode::ToolsMenu => {
                self.handle_tools_menu_selection()?;
            }
            AppMode::DiskTools
            | AppMode::SystemTools
            | AppMode::UserTools
            | AppMode::NetworkTools => {
                self.handle_tool_selection()?;
            }
            AppMode::GuidedInstaller => {
                self.handle_guided_installer_enter()?;
            }
            AppMode::AutomatedInstall => {
                self.handle_automated_install_enter()?;
            }
            AppMode::ToolDialog => {
                self.handle_tool_dialog_enter()?;
            }
            AppMode::Installation => {
                // Installation is running, no action needed
            }
            AppMode::Complete => {
                let mut state = self.lock_state();
                state.mode = AppMode::MainMenu;
                state.main_menu_selection = 0;
                state.status_message = "Welcome to Arch Linux Toolkit".to_string();
            }
            AppMode::EmbeddedTerminal => {
                // Embedded terminal handles its own input
            }
            AppMode::FloatingOutput => {
                // Dismiss floating output on Enter
                let mut state = self.lock_state();
                if let Some(_output) = state.floating_output.take() {
                    state.mode = state.pre_dialog_mode.take().unwrap_or(AppMode::ToolsMenu);
                }
            }
            AppMode::FileBrowser => {
                // File browser handles its own Enter key
            }
            AppMode::ConfirmDialog => {
                // Handle confirmation dialog selection
                self.handle_confirm_dialog_enter()?;
            }
            AppMode::DryRunSummary => {
                // Dismiss dry-run summary and return to guided installer
                let mut state = self.lock_state();
                state.mode = AppMode::GuidedInstaller;
                state.dry_run_summary = None;
            }
        }

        Ok(false)
    }

    // =========================================================================
    // SECTION: Confirmation Dialog Handlers
    // =========================================================================
    //
    // Confirmation dialogs gate destructive operations (wipe disk, format, etc.)
    // The dialog stores:
    // - confirm_action: Name of the action to execute if confirmed
    // - action_data: Optional data (e.g., device path)
    // - pre_dialog_mode: Mode to return to after dialog closes
    //
    // =========================================================================

    /// Handle confirmation dialog Enter key
    fn handle_confirm_dialog_enter(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (confirmed, action, action_data, pre_mode) = {
            let state = self.lock_state();
            if let Some(ref dialog) = state.confirm_dialog {
                (
                    dialog.is_confirmed(),
                    dialog.confirm_action.clone(),
                    dialog.action_data.clone(),
                    state.pre_dialog_mode.clone(),
                )
            } else {
                return Ok(());
            }
        };

        // Clear the dialog
        {
            let mut state = self.lock_state();
            state.confirm_dialog = None;
            // Return to previous mode
            if let Some(mode) = pre_mode {
                state.mode = mode;
            }
            state.pre_dialog_mode = None;
        }

        if confirmed {
            // Delegate to execute_confirmed_action which handles all action types
            self.execute_confirmed_action(&action, action_data)?;
        }

        Ok(())
    }

    /// Execute action after confirmation dialog
    fn execute_confirmed_action(
        &mut self,
        action: &str,
        data: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match action {
            "install_bootloader" => {
                if let Some(params) = data {
                    // params format: "bootloader:disk"
                    let parts: Vec<&str> = params.split(':').collect();
                    if parts.len() == 2 {
                        let sa = BootloaderArgs {
                            bootloader_type: parts[0].to_string(),
                            disk: PathBuf::from(parts[1]),
                            mode: String::new(),
                            efi_path: None,
                        };
                        self.execute_via_script_args(
                            sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                            "install bootloader", sa.is_destructive(), true,
                        )?;
                    }
                }
            }
            "start_installation" => {
                // Start the installation process
                self.start_installation()?;
            }
            "execute_tool" => {
                if let Some(data) = data {
                    match serde_json::from_str::<serde_json::Value>(&data) {
                        Ok(pending) => {
                            let script_name = pending["script_name"].as_str().unwrap_or("").to_string();
                            if script_name.is_empty() {
                                let mut state = self.lock_state();
                                state.status_message = "Error: missing script name in confirmed action".to_string();
                                return Ok(());
                            }
                            let cli_args: Vec<String> = pending["cli_args"]
                                .as_array()
                                .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                                .unwrap_or_default();
                            let env_vars: Vec<(String, String)> = pending["env_vars"]
                                .as_array()
                                .map(|a| {
                                    a.iter()
                                        .filter_map(|v| {
                                            let arr = v.as_array()?;
                                            Some((arr.first()?.as_str()?.to_string(), arr.get(1)?.as_str()?.to_string()))
                                        })
                                        .collect()
                                })
                                .unwrap_or_default();
                            let display_name = pending["display_name"].as_str().unwrap_or("tool").to_string();

                            let script_path = crate::script_runner::scripts_base_dir()
                                .join("tools")
                                .join(&script_name)
                                .to_string_lossy()
                                .to_string();
                            // Set up floating output and spawn directly (already confirmed)
                            {
                                let mut state = self.lock_state();
                                state.floating_output = Some(FloatingOutputState {
                                    title: format!("Running: {}", display_name),
                                    content: vec![
                                        format!("Executing: {} {}", script_path, cli_args.join(" ")),
                                        String::new(),
                                    ],
                                    scroll_offset: 0,
                                    auto_scroll: true,
                                    complete: false,
                                    progress: None,
                                    status: "Running...".to_string(),
                                });
                                if state.pre_dialog_mode.is_none() {
                                    state.pre_dialog_mode = Some(state.mode.clone());
                                }
                                state.mode = AppMode::FloatingOutput;
                                state.current_tool = Some(display_name);
                            }
                            self.spawn_tool_script_with_env(&script_path, cli_args, env_vars)?;
                        }
                        Err(e) => {
                            let mut state = self.lock_state();
                            state.status_message = format!("Internal error: failed to parse tool data: {}", e);
                        }
                    }
                } else {
                    let mut state = self.lock_state();
                    state.status_message = "Error: no action data for confirmed tool execution".to_string();
                }
            }
            _ => {
                // Unknown action
                let mut state = self.lock_state();
                state.status_message = format!("Unknown action: {}", action);
            }
        }
        Ok(())
    }

    // =========================================================================
    // SECTION: Menu Selection Handlers
    // =========================================================================
    //
    // These functions map menu indices to actions:
    //
    // MainMenu:
    //   0 → GuidedInstaller, 1 → AutomatedInstall, 2 → ToolsMenu, 3 → Quit
    //
    // ToolsMenu:
    //   0 → DiskTools, 1 → SystemTools, 2 → UserTools, 3 → NetworkTools, 4 → Back
    //
    // DiskTools/SystemTools/UserTools/NetworkTools:
    //   Index maps to specific tool, last index is always "Back"
    //
    // =========================================================================

    /// Handle main menu selection
    fn handle_main_menu_selection(&mut self) -> Result<bool, Box<dyn std::error::Error>> {
        let selection = {
            let state = self.lock_state();
            state.main_menu_selection
        };

        debug!("Main menu selection: {}", selection);

        let mut state = self.lock_state();
        match selection {
            0 => {
                // Guided Installer
                state.mode = AppMode::GuidedInstaller;
                state.status_message = "Starting guided installation...".to_string();
            }
            1 => {
                // Automated Install
                state.mode = AppMode::AutomatedInstall;
                state.status_message =
                    "Select configuration file for automated installation...".to_string();
            }
            2 => {
                // Arch Linux Tools
                state.mode = AppMode::ToolsMenu;
                state.tools_menu_selection = 0;
                state.status_message =
                    "Arch Linux Tools - System repair and administration".to_string();
            }
            3 => {
                // Quit
                return Ok(true);
            }
            _ => {}
        }
        Ok(false)
    }

    /// Handle tools menu selection
    fn handle_tools_menu_selection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let selection = {
            let state = self.lock_state();
            state.tools_menu_selection
        };

        let mut state = self.lock_state();
        match selection {
            0 => {
                // Disk & Filesystem Tools
                state.mode = AppMode::DiskTools;
                state.tools_menu_selection = 0;
                state.status_message = "Disk & Filesystem Tools".to_string();
            }
            1 => {
                // System & Boot Tools
                state.mode = AppMode::SystemTools;
                state.tools_menu_selection = 0;
                state.status_message = "System & Boot Tools".to_string();
            }
            2 => {
                // User & Security Tools
                state.mode = AppMode::UserTools;
                state.tools_menu_selection = 0;
                state.status_message = "User & Security Tools".to_string();
            }
            3 => {
                // Network Tools
                state.mode = AppMode::NetworkTools;
                state.tools_menu_selection = 0;
                state.status_message = "Network Tools".to_string();
            }
            4 => {
                // Back to Main Menu
                state.mode = AppMode::MainMenu;
                state.main_menu_selection = 0;
                state.status_message = "Welcome to Arch Linux Toolkit".to_string();
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle tool selection within a category
    fn handle_tool_selection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (current_mode, selection) = {
            let state = self.lock_state();
            (state.mode.clone(), state.tools_menu_selection)
        };

        // Check if user selected "Back" option (last item in each menu)
        let is_back_option = match current_mode {
            AppMode::DiskTools => selection == 6, // 7 items (0-6), back is at index 6
            AppMode::SystemTools => selection == 9, // 10 items (0-9), back is at index 9
            AppMode::UserTools => selection == 7, // 8 items (0-7), back is at index 7
            AppMode::NetworkTools => selection == 5, // 6 items (0-5), back is at index 5
            _ => false,
        };

        if is_back_option {
            // Go back to tools menu
            let mut state = self.lock_state();
            state.mode = AppMode::ToolsMenu;
            state.tools_menu_selection = 0;
            state.status_message =
                "Arch Linux Tools - System repair and administration".to_string();
        } else {
            // Execute the selected tool
            self.execute_tool(&current_mode, selection)?;
        }
        Ok(())
    }

    /// Execute a specific tool
    fn execute_tool(
        &mut self,
        mode: &AppMode,
        selection: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match mode {
            AppMode::DiskTools => {
                match selection {
                    0 => {
                        // Partition Disk - Use disk selection dialog (then show layout)
                        self.input_handler.start_disk_selection("".to_string());
                        let mut state = self.lock_state();
                        state.current_tool = Some("manual_partition".to_string());
                        state.status_message =
                            "Select disk to partition (Enter to select, Esc to cancel)"
                                .to_string();
                    }
                    1 => {
                        // Format Partition - Use disk selection dialog
                        self.input_handler.start_disk_selection("".to_string());
                        let mut state = self.lock_state();
                        state.current_tool = Some("format_partition".to_string());
                        state.status_message =
                            "Select partition to format (Enter to select, Esc to cancel)"
                                .to_string();
                    }
                    2 => {
                        // Wipe Disk - Use disk selection dialog
                        self.input_handler.start_disk_selection("".to_string());
                        let mut state = self.lock_state();
                        state.current_tool = Some("wipe_disk".to_string());
                        state.status_message =
                            "Select disk to wipe (Enter to select, Esc to cancel)".to_string();
                    }
                    3 => {
                        // Check Disk Health - Use disk selection dialog
                        self.input_handler.start_disk_selection("".to_string());
                        let mut state = self.lock_state();
                        state.current_tool = Some("health".to_string());
                        state.status_message =
                            "Select disk to check health (Enter to select, Esc to cancel)"
                                .to_string();
                    }
                    4 => {
                        // Mount/Unmount Partitions - Create dialog
                        self.create_tool_dialog("mount")?;
                    }
                    5 => {
                        // LUKS Encryption - Create dialog
                        self.create_tool_dialog("encrypt_device")?;
                    }
                    _ => {}
                }
            }
            AppMode::SystemTools => {
                match selection {
                    0 => {
                        // Install/Repair Bootloader - Create dialog
                        self.create_tool_dialog("install_bootloader")?;
                    }
                    1 => {
                        // Generate fstab - Create dialog
                        self.create_tool_dialog("generate_fstab")?;
                    }
                    2 => {
                        // Chroot into System - Create dialog
                        self.create_tool_dialog("chroot")?;
                    }
                    3 => {
                        // Enable/Disable Services - Create dialog
                        self.create_tool_dialog("manage_services")?;
                    }
                    4 => {
                        // System Information - no parameters needed
                        let sa = SystemInfoArgs { detailed: true };
                        self.execute_via_script_args(
                            sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                            "system info", sa.is_destructive(), true,
                        )?;
                    }
                    5 => {
                        // Enable Services - Create dialog
                        self.create_tool_dialog("enable_services")?;
                    }
                    6 => {
                        // Install AUR Helper - Create dialog
                        self.create_tool_dialog("install_aur_helper")?;
                    }
                    7 => {
                        // Rebuild Initramfs - Create dialog for root path
                        self.create_tool_dialog("rebuild_initramfs")?;
                    }
                    8 => {
                        // View Install Logs - run directly with --latest
                        self.execute_via_script_args(
                            "view_install_logs.sh",
                            vec!["--latest".to_string()],
                            vec![],
                            "view install logs",
                            false,
                            true,
                        )?;
                    }
                    _ => {}
                }
            }
            AppMode::UserTools => {
                match selection {
                    0 => {
                        // Add New User - Create dialog
                        self.create_tool_dialog("add_user")?;
                    }
                    1 => {
                        // Reset Password - Create dialog
                        self.create_tool_dialog("reset_password")?;
                    }
                    2 => {
                        // Manage User Groups - Create dialog
                        self.create_tool_dialog("manage_groups")?;
                    }
                    3 => {
                        // Configure SSH - Create dialog
                        self.create_tool_dialog("configure_ssh")?;
                    }
                    4 => {
                        // Security Audit - Create dialog for audit level
                        self.create_tool_dialog("security_audit")?;
                    }
                    5 => {
                        // Install Dotfiles - Create dialog
                        self.create_tool_dialog("install_dotfiles")?;
                    }
                    6 => {
                        // Run As User - Create dialog
                        self.create_tool_dialog("run_as_user")?;
                    }
                    _ => {}
                }
            }
            AppMode::NetworkTools => {
                match selection {
                    0 => {
                        // Configure Network Interface - Create dialog
                        self.create_tool_dialog("configure_network")?;
                    }
                    1 => {
                        // Test Network Connectivity - Create dialog for test type
                        self.create_tool_dialog("test_network")?;
                    }
                    2 => {
                        // Configure Firewall - Create dialog
                        self.create_tool_dialog("configure_firewall")?;
                    }
                    3 => {
                        // Network Diagnostics - Create dialog for diagnostic type
                        self.create_tool_dialog("network_diagnostics")?;
                    }
                    4 => {
                        // Update Mirrors - Create dialog
                        self.create_tool_dialog("update_mirrors")?;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle guided installer enter (original logic)
    fn handle_guided_installer_enter(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (should_open_input, should_start_installation, should_test_config, should_export_config) = {
            let state = self.lock_state();
            if state.config_scroll.selected_index == state.config.options.len() {
                match state.installer_button_selection {
                    0 => (false, false, true, false),  // Test Config
                    1 => (false, false, false, true),   // Export Config
                    _ => (false, true, false, false),   // Start Install
                }
            } else {
                (true, false, false, false) // Open input dialog
            }
        };

        if should_open_input {
            self.open_input_dialog()?;
        }

        if should_test_config {
            self.generate_test_config_summary()?;
        }

        if should_export_config {
            self.export_config()?;
        }

        // Start installation if needed - show confirmation dialog first
        if should_start_installation {
            if self.validate_configuration_for_installation() {
                // Show confirmation dialog before starting
                let mut state = self.lock_state();
                state.pre_dialog_mode = Some(AppMode::GuidedInstaller);
                state.confirm_dialog = Some(start_install_confirm());
                state.mode = AppMode::ConfirmDialog;
            } else {
                // Validation failed - status message already set in validate_configuration_for_installation
            }
        }

        Ok(())
    }

    /// Handle automated install enter
    fn handle_automated_install_enter(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Launch file browser for config file selection
        let start_dir = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("/"));
        let file_browser = crate::components::file_browser::FileBrowserState::new(
            &start_dir,
            vec!["toml".to_string(), "json".to_string()],
        );

        let mut state = self.lock_state();
        state.file_browser = Some(file_browser);
        state.mode = AppMode::FileBrowser;
        state.status_message = "Select a configuration file (.toml or .json)".to_string();
        Ok(())
    }

    /// Handle back key navigation
    fn handle_back_key(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let current_mode = {
            let state = self.lock_state();
            state.mode.clone()
        };

        let mut state = self.lock_state();
        match current_mode {
            AppMode::MainMenu => {
                // Already at top level - could show exit confirmation
                state.status_message =
                    "Press 'Q' to quit or use arrow keys to navigate".to_string();
            }
            AppMode::GuidedInstaller => {
                // Go back to main menu from guided installer
                state.mode = AppMode::MainMenu;
                state.main_menu_selection = 0;
                state.status_message = "Welcome to Arch Linux Toolkit".to_string();
            }
            AppMode::AutomatedInstall => {
                // Go back to main menu
                state.mode = AppMode::MainMenu;
                state.main_menu_selection = 0;
                state.status_message = "Welcome to Arch Linux Toolkit".to_string();
            }
            AppMode::ToolsMenu => {
                // Go back to main menu
                state.mode = AppMode::MainMenu;
                state.main_menu_selection = 0;
                state.status_message = "Welcome to Arch Linux Toolkit".to_string();
            }
            AppMode::DiskTools
            | AppMode::SystemTools
            | AppMode::UserTools
            | AppMode::NetworkTools => {
                // Go back to tools menu
                state.mode = AppMode::ToolsMenu;
                state.tools_menu_selection = 0;
                state.status_message =
                    "Arch Linux Tools - System repair and administration".to_string();
            }
            AppMode::ToolDialog => {
                // Go back to the appropriate tool menu based on current tool
                // NOTE: tool names must match what create_tool_dialog() and execute_tool() set
                if let Some(ref tool_name) = state.current_tool {
                    match tool_name.as_str() {
                        "format_partition" | "wipe_disk" | "health" | "mount"
                        | "manual_partition" | "encrypt_device"
                        | "partition_create_table" | "partition_add"
                        | "partition_delete" | "partition_action_menu" => {
                            state.pending_tool_device = None;
                            state.mode = AppMode::DiskTools;
                            state.tools_menu_selection = 0;
                            state.status_message = "Disk & Filesystem Tools".to_string();
                        }
                        "install_bootloader" | "generate_fstab" | "chroot" | "info"
                        | "manage_services" | "system_info" | "enable_services"
                        | "install_aur_helper" | "rebuild_initramfs" => {
                            state.mode = AppMode::SystemTools;
                            state.tools_menu_selection = 0;
                            state.status_message = "System & Boot Tools".to_string();
                        }
                        "add_user" | "reset_password" | "manage_groups"
                        | "configure_ssh" | "security_audit" | "install_dotfiles"
                        | "run_as_user" => {
                            state.mode = AppMode::UserTools;
                            state.tools_menu_selection = 0;
                            state.status_message = "User & Security Tools".to_string();
                        }
                        "configure_network" | "configure_firewall"
                        | "test_network" | "network_diagnostics"
                        | "update_mirrors" => {
                            state.mode = AppMode::NetworkTools;
                            state.tools_menu_selection = 0;
                            state.status_message = "Network Tools".to_string();
                        }
                        _ => {
                            // Fallback to tools menu
                            state.mode = AppMode::ToolsMenu;
                            state.tools_menu_selection = 0;
                            state.status_message =
                                "Arch Linux Tools - System repair and administration".to_string();
                        }
                    }
                } else {
                    // Fallback to tools menu
                    state.mode = AppMode::ToolsMenu;
                    state.tools_menu_selection = 0;
                    state.status_message =
                        "Arch Linux Tools - System repair and administration".to_string();
                }
                // Clear tool dialog state
                state.tool_dialog = None;
                state.current_tool = None;
                state.pre_dialog_mode = None;
            }
            // ToolExecution removed — tool execution uses FloatingOutput mode
            AppMode::Installation => {
                // Kill the running installer process group before switching modes
                if let Some(pid) = state.installer_pid.take() {
                    if let Ok(mut registry) = ChildRegistry::global().lock() {
                        registry.terminate_current(std::time::Duration::from_secs(3));
                        registry.unregister(pid);
                    }
                }
                state.mode = AppMode::GuidedInstaller;
                state.status_message =
                    "Installation cancelled - configure your settings".to_string();
            }
            AppMode::Complete => {
                // From completion screen, go back to main menu
                state.mode = AppMode::MainMenu;
                state.main_menu_selection = 0;
                state.status_message = "Welcome to Arch Linux Toolkit".to_string();
            }
            AppMode::EmbeddedTerminal => {
                // Embedded terminal uses Ctrl+Q to exit, but we can also handle 'b'
                // Return to previous mode - will be handled by exit_embedded_terminal
                drop(state);
                self.exit_embedded_terminal()?;
                return Ok(());
            }
            AppMode::FloatingOutput => {
                // Dismiss floating output and return to previous mode
                if let Some(_output) = state.floating_output.take() {
                    state.pending_tool_device = None;
                    state.mode = state.pre_dialog_mode.take().unwrap_or(AppMode::ToolsMenu);
                    state.tools_menu_selection = 0;
                    state.current_tool = None;
                    state.status_message =
                        "Arch Linux Tools - System repair and administration".to_string();
                }
            }
            AppMode::FileBrowser => {
                // Cancel file browser and return to automated install
                state.file_browser = None;
                state.mode = AppMode::AutomatedInstall;
                state.status_message = "File selection cancelled".to_string();
            }
            AppMode::ConfirmDialog => {
                // Cancel confirmation dialog and return to previous mode
                state.confirm_dialog = None;
                if let Some(mode) = state.pre_dialog_mode.take() {
                    state.mode = mode;
                } else {
                    state.mode = AppMode::ToolsMenu;
                }
                state.status_message = "Operation cancelled".to_string();
            }
            AppMode::DryRunSummary => {
                // Return to guided installer from dry-run summary
                state.mode = AppMode::GuidedInstaller;
                state.dry_run_summary = None;
                state.status_message = "Dry-run complete - review your configuration".to_string();
            }
        }
        Ok(())
    }

    /// Validate that all required configuration options are set and valid
    fn validate_configuration(&self, config: &Configuration) -> bool {
        // First check basic option validation
        if !config.options.iter().all(|option| option.is_valid()) {
            return false;
        }

        // Check AUR helper is selected when DE requires AUR packages
        if self.de_requires_aur_helper(config) {
            return false;
        }

        // Then check secure boot requirements
        self.validate_secure_boot_requirements(config)
    }

    /// Check if the selected DE requires an AUR helper but none is configured
    fn de_requires_aur_helper(&self, config: &Configuration) -> bool {
        let de_value = config
            .options
            .iter()
            .find(|opt| opt.name == "Desktop Environment")
            .map(|opt| opt.get_value().to_string())
            .unwrap_or_default();
        let de: DesktopEnvironment = de_value.parse().unwrap_or_default();
        if de.requires_aur() {
            let aur_value = config
                .options
                .iter()
                .find(|opt| opt.name == "AUR Helper")
                .map(|opt| opt.get_value().to_string())
                .unwrap_or_default();
            let aur: AurHelper = aur_value.parse().unwrap_or_default();
            return aur == AurHelper::None;
        }
        false
    }

    /// Validate secure boot requirements
    fn validate_secure_boot_requirements(&self, config: &Configuration) -> bool {
        // Find the Secure Boot option
        if let Some(secure_boot_option) =
            config.options.iter().find(|opt| opt.name == "Secure Boot")
        {
            if secure_boot_option.value.to_lowercase() == "yes" {
                // Check if Boot Mode is UEFI
                if let Some(boot_mode_option) =
                    config.options.iter().find(|opt| opt.name == "Boot Mode")
                {
                    let boot_mode = boot_mode_option.value.to_lowercase();
                    if boot_mode != "uefi" && boot_mode != "auto" {
                        // Secure boot validation failed - this will be handled in the main validation
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Check if secure boot warning should be shown after setting value
    fn check_secure_boot_warning(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Secure boot warning is now always shown in the dialog itself
        Ok(())
    }

    /// Check if UEFI is supported on this system
    fn is_uefi_supported(&self) -> bool {
        // Check for UEFI support by looking at /sys/firmware/efi
        std::path::Path::new("/sys/firmware/efi").exists()
    }

    /// Show secure boot warning dialog
    fn show_secure_boot_warning(&mut self) {
        let warning_message = vec![
            "SECURE BOOT REQUIREMENTS NOT MET".to_string(),
            "".to_string(),
            "Secure Boot requires UEFI firmware configuration:".to_string(),
            "".to_string(),
            "1. Boot into UEFI firmware settings".to_string(),
            "2. Enable UEFI mode (disable Legacy/CSM)".to_string(),
            "3. Enable Secure Boot in firmware".to_string(),
            "4. Set Secure Boot to 'Windows UEFI' mode".to_string(),
            "5. Save and exit firmware".to_string(),
            "".to_string(),
            "⚠️  WARNING: Secure Boot can prevent booting".to_string(),
            "if not configured properly!".to_string(),
            "".to_string(),
            "See: https://wiki.archlinux.org/title/Unified_Extensible_Firmware_Interface#UEFI_variables".to_string(),
        ];

        self.input_handler
            .start_warning("Secure Boot Warning".to_string(), warning_message);
    }

    /// Get detailed validation errors
    fn get_validation_errors(&self, config: &Configuration) -> Vec<String> {
        let mut errors: Vec<String> = config
            .options
            .iter()
            .filter_map(|option| option.validation_error())
            .collect();

        // Add AUR helper validation errors
        if self.de_requires_aur_helper(config) {
            let de_value = config
                .options
                .iter()
                .find(|opt| opt.name == "Desktop Environment")
                .map(|opt| opt.get_value().to_string())
                .unwrap_or_default();
            errors.push(format!(
                "{} requires an AUR helper (packages like wlogout are AUR-only)",
                de_value
            ));
        }

        // Add secure boot validation errors
        if let Some(secure_boot_option) =
            config.options.iter().find(|opt| opt.name == "Secure Boot")
        {
            if secure_boot_option.value.to_lowercase() == "yes" {
                if let Some(boot_mode_option) =
                    config.options.iter().find(|opt| opt.name == "Boot Mode")
                {
                    let boot_mode = boot_mode_option.value.to_lowercase();
                    if boot_mode != "uefi" && boot_mode != "auto" {
                        errors.push("Secure Boot requires UEFI boot mode. Please configure UEFI firmware first.".to_string());
                    }
                }
            }
        }

        errors
    }

    /// Validate configuration for installation (with user feedback)
    fn validate_configuration_for_installation(&mut self) -> bool {
        let config = {
            let state = self.lock_state();
            state.config.clone()
        };

        // Check for secure boot issues first (show warning dialog)
        if let Some(secure_boot_option) =
            config.options.iter().find(|opt| opt.name == "Secure Boot")
        {
            if secure_boot_option.value.to_lowercase() == "yes" {
                if let Some(boot_mode_option) =
                    config.options.iter().find(|opt| opt.name == "Boot Mode")
                {
                    let boot_mode = boot_mode_option.value.to_lowercase();
                    if boot_mode != "uefi" && boot_mode != "auto" {
                        self.show_secure_boot_warning();
                        return false;
                    }
                }
            }
        }

        if self.validate_configuration(&config) {
            // All validation passed - installation can proceed
            true
        } else {
            let mut state = self.lock_state();
            let errors = self.get_validation_errors(&config);

            if errors.is_empty() {
                state.status_message =
                    "Cannot start installation: configuration is invalid".to_string();
            } else if errors.len() == 1 {
                state.status_message = format!("Cannot start installation: {}", errors[0]);
            } else {
                state.status_message = format!(
                    "Cannot start installation: {} (and {} more errors)",
                    errors[0],
                    errors.len() - 1
                );
            }
            false
        }
    }

    /// Generate a dry-run summary from the current configuration and transition to DryRunSummary mode.
    /// No scripts are executed — this is purely in-memory config inspection.
    fn generate_test_config_summary(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (config, is_valid, errors) = {
            let state = self.lock_state();
            let cfg = state.config.clone();
            let valid = self.validate_configuration(&cfg);
            let errs = self.get_validation_errors(&cfg);
            (cfg, valid, errs)
        };

        let env_vars = config.to_env_vars();
        let mut summary = Vec::new();

        // Header
        summary.push("=== Configuration Test Summary ===".to_string());
        summary.push(String::new());

        // Validation status
        if is_valid {
            summary.push("[OK] Configuration is valid - ready to install".to_string());
        } else {
            summary.push("[FAIL] Configuration has errors:".to_string());
            for err in &errors {
                summary.push(format!("  -> {}", err));
            }
        }
        summary.push(String::new());

        // Config fields
        summary.push("=== Configuration Values ===".to_string());
        for option in &config.options {
            let display_value = if option.name.contains("Password") {
                if option.value.is_empty() { "(not set)".to_string() } else { "********".to_string() }
            } else {
                let val = option.get_value();
                if val.is_empty() { "(not set)".to_string() } else { val }
            };
            let status = if option.required && option.value.trim().is_empty() {
                "[REQUIRED - MISSING]"
            } else if option.required {
                "[OK]"
            } else {
                "[optional]"
            };
            summary.push(format!("  {} {}: {}", status, option.name, display_value));
        }
        summary.push(String::new());

        // Environment variables that would be passed
        summary.push("=== Environment Variables (passed to install scripts) ===".to_string());
        let mut sorted_vars: Vec<_> = env_vars.iter().collect();
        sorted_vars.sort_by_key(|(k, _)| k.as_str());
        for (key, value) in &sorted_vars {
            let display = if key.contains("PASSWORD") {
                "********"
            } else {
                value.as_str()
            };
            summary.push(format!("  {}={}", key, display));
        }

        // Show manual partition assignments if applicable
        {
            let state = self.lock_state();
            let strategy = state
                .config
                .options
                .iter()
                .find(|opt| opt.name == "Partitioning Strategy")
                .map(|opt| opt.get_value())
                .unwrap_or_default();
            if strategy == "manual" {
                summary.push(String::new());
                summary.push("=== Manual Partition Assignments ===".to_string());
                if let Some(ref map) = state.manual_partition_map {
                    summary.push(format!("  Root:  {} ({})", map.root, map.root_fs));
                    summary.push(format!("  Boot:  {} (ext4)", map.boot));
                    if !map.efi.is_empty() {
                        summary.push(format!("  EFI:   {} (vfat)", map.efi));
                    }
                    if !map.home.is_empty() {
                        summary.push(format!("  Home:  {} ({})", map.home, map.home_fs));
                    }
                    if !map.swap.is_empty() {
                        summary.push(format!("  Swap:  {}", map.swap));
                    }
                } else {
                    summary.push("  [NOT ASSIGNED] — select Disk to run cfdisk and assign partitions".to_string());
                }
            }
        }

        let mut state = self.lock_state();
        state.dry_run_summary = Some(summary);
        state.dry_run_scroll_offset = 0;
        state.mode = AppMode::DryRunSummary;
        state.status_message = "Test Config - review your settings (B to go back)".to_string();
        Ok(())
    }

    /// Export current configuration to a JSON file in the working directory
    fn export_config(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let file_config = {
            let state = self.lock_state();
            crate::config_file::InstallationConfig::from(&state.config)
        };

        let export_path = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("archtui-config.json");

        let mut state = self.lock_state();
        match file_config.save_to_file(&export_path) {
            Ok(()) => {
                info!("Exported config to: {}", export_path.display());
                state.status_message =
                    format!("Config exported to {}", export_path.display());
            }
            Err(e) => {
                tracing::error!("Failed to export config: {}", e);
                state.status_message = format!("Export failed: {}", e);
            }
        }
        Ok(())
    }

    // =========================================================================
    // SECTION: Installation
    // =========================================================================
    //
    // Installation flow:
    //
    // 1. User completes guided installer config
    // 2. Confirmation dialog shown → start_install_confirm()
    // 3. User confirms → start_installation()
    // 4. Config optionally saved to file
    // 5. Mode changed to AppMode::Installation
    // 6. Installer.start() spawns scripts/install_wrapper.sh
    // 7. Output streams to UI via ToolMessage channel
    // 8. On completion → AppMode::Complete
    //
    // =========================================================================

    /// Start the installation process
    fn start_installation(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting installation process");

        // Block manual strategy without partition assignments
        {
            let state = self.lock_state();
            let strategy = state
                .config
                .options
                .iter()
                .find(|opt| opt.name == "Partitioning Strategy")
                .map(|opt| opt.get_value())
                .unwrap_or_default();
            if strategy == "manual" && state.manual_partition_map.is_none() {
                drop(state);
                let mut state = self.lock_state();
                state.status_message =
                    "Manual strategy requires partition assignments. Select Disk first to run cfdisk and assign partitions."
                        .to_string();
                return Ok(());
            }
        }

        // Check if we need to save the config before starting
        if let Some(save_path) = &self.save_config_path {
            info!("Saving configuration to: {:?}", save_path);
            let file_config = {
                let state = self.lock_state();
                crate::config_file::InstallationConfig::from(&state.config)
            };
            file_config.save_to_file(save_path)?;

            let mut state = self.lock_state();
            state.status_message = format!("Config saved to {}", save_path.display());
        }

        // Update state to installation mode
        {
            let mut state = self.lock_state();
            state.mode = AppMode::Installation;
            state.status_message = "Starting installation...".to_string();
        }

        // Create installer with current configuration
        let config = {
            let state = self.lock_state();
            state.config.clone()
        };

        self.installer = Some(Installer::new(config, Arc::clone(&self.state)));

        // Start installation in background
        if let Some(ref mut installer) = self.installer {
            installer.start()?;
        }

        Ok(())
    }

    /// Open input dialog for the current configuration option
    fn open_input_dialog(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let option = {
            let state = self.lock_state();
            let current_step = state.config_scroll.selected_index;
            if current_step >= state.config.options.len() {
                return Ok(()); // On button row, not a config option
            }
            state.config.options[current_step].clone()
        };

        match option.name.as_str() {
            "Boot Mode" => {
                // Show normal selection dialog
                let options = InputHandler::get_predefined_options(&option.name);
                self.input_handler
                    .start_selection(option.name.clone(), options, option.value);
            }
            "Secure Boot" => {
                // Always show selection dialog with static warning about requirements
                let mut options = InputHandler::get_predefined_options(&option.name);

                // Check if UEFI is supported
                let uefi_supported = self.is_uefi_supported();

                // Insert static warning at the top of the options
                options.insert(
                    0,
                    "⚠️  WARNING: Secure Boot requires UEFI firmware!".to_string(),
                );
                options.insert(
                    1,
                    "Make sure your motherboard supports UEFI and".to_string(),
                );
                options.insert(2, "Secure Boot is properly configured in BIOS.".to_string());
                options.insert(3, "See: https://wiki.archlinux.org/title/UEFI".to_string());
                options.insert(4, "".to_string());

                // If UEFI is not supported, only show "No" option
                if !uefi_supported {
                    options = vec!["No".to_string()];
                    options.insert(
                        0,
                        "⚠️  WARNING: Secure Boot requires UEFI firmware!".to_string(),
                    );
                    options.insert(1, "UEFI is not supported on this system.".to_string());
                    options.insert(2, "Secure Boot is not available.".to_string());
                    options.insert(3, "".to_string());
                }

                self.input_handler
                    .start_selection(option.name.clone(), options, option.value);
            }
            "Encryption" => {
                // Only allow encryption Yes/No for manual partitioning
                let partitioning_strategy = {
                    let state = self.lock_state();
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Partitioning Strategy")
                        .map(|opt| opt.get_value())
                        .unwrap_or_default()
                };

                if partitioning_strategy == "manual" {
                    let options = vec!["Yes".to_string(), "No".to_string()];
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else {
                    // Show message that encryption is auto-set for non-manual strategies
                    { let mut state = self.lock_state();
                        state.status_message = "Encryption is auto-set based on partitioning strategy. Use manual partitioning to control encryption.".to_string();
                    }
                }
            }
            "Swap Size" => {
                // Only allow swap size configuration if swap is enabled
                let swap_enabled = {
                    let state = self.lock_state();
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Swap")
                        .map(|opt| opt.get_value().to_lowercase() == "yes")
                        .unwrap_or(false)
                };

                if swap_enabled {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else { let mut state = self.lock_state();
                    state.status_message =
                        "Swap size can only be configured when swap is enabled.".to_string();
                }
            }
            "Root Size" => {
                // Allow when Separate Home = Yes OR strategy contains "lvm"
                let (home_enabled, strategy) = {
                    let state = self.lock_state();
                    let home = state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Separate Home Partition")
                        .map(|opt| opt.get_value().to_lowercase() == "yes")
                        .unwrap_or(false);
                    let strat = state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Partitioning Strategy")
                        .map(|opt| opt.get_value())
                        .unwrap_or_default();
                    (home, strat)
                };

                if home_enabled || strategy.contains("lvm") {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else { let mut state = self.lock_state();
                    state.status_message =
                        "Root size is configurable when using a separate home partition or LVM strategy.".to_string();
                }
            }
            "Home Size" => {
                // Only allow when Separate Home = Yes
                let home_enabled = {
                    let state = self.lock_state();
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Separate Home Partition")
                        .map(|opt| opt.get_value().to_lowercase() == "yes")
                        .unwrap_or(false)
                };

                if home_enabled {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else { let mut state = self.lock_state();
                    state.status_message =
                        "Home size is only configurable when separate home partition is enabled.".to_string();
                }
            }
            "Btrfs Frequency" | "Btrfs Keep Count" | "Btrfs Assistant" => {
                // Only allow btrfs configuration if root filesystem is btrfs AND snapshots are enabled
                let (is_btrfs, snapshots_enabled) = {
                    let state = self.lock_state();
                    let btrfs = state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Root Filesystem")
                        .map(|opt| opt.get_value().to_lowercase() == "btrfs")
                        .unwrap_or(false);
                    let snapshots = state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Btrfs Snapshots")
                        .map(|opt| opt.get_value().to_lowercase() == "yes")
                        .unwrap_or(false);
                    (btrfs, snapshots)
                };

                if !is_btrfs {
                    { let mut state = self.lock_state();
                        state.status_message =
                            "Btrfs options are only available when Root Filesystem is btrfs."
                                .to_string();
                    }
                } else if snapshots_enabled {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else { let mut state = self.lock_state();
                    state.status_message = format!(
                        "{} can only be configured when Btrfs snapshots are enabled.",
                        option.name
                    );
                }
            }
            "GRUB Theme" | "GRUB Theme Selection" => {
                // Only allow GRUB theme options when bootloader is GRUB
                let (is_grub, themes_enabled) = {
                    let state = self.lock_state();
                    let grub = state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Bootloader")
                        .map(|opt| opt.get_value().to_lowercase() == "grub")
                        .unwrap_or(false);
                    let themes = state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "GRUB Theme")
                        .map(|opt| opt.get_value().to_lowercase() == "yes")
                        .unwrap_or(false);
                    (grub, themes)
                };

                if !is_grub {
                    { let mut state = self.lock_state();
                        state.status_message =
                            "GRUB theme options are only available with the GRUB bootloader."
                                .to_string();
                    }
                } else if option.name == "GRUB Theme Selection" && !themes_enabled {
                    { let mut state = self.lock_state();
                        state.status_message =
                            "GRUB theme selection is only available when GRUB themes are enabled."
                                .to_string();
                    }
                } else {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                }
            }
            "Git Repository URL" => {
                // Only allow URL input if git repository is enabled
                let git_enabled = {
                    let state = self.lock_state();
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Git Repository")
                        .map(|opt| opt.get_value().to_lowercase() == "yes")
                        .unwrap_or(false)
                };

                if git_enabled {
                    self.input_handler.start_text_input(
                        option.name.clone(),
                        option.value,
                        "Enter git repository URL".to_string(),
                    );
                } else { let mut state = self.lock_state();
                    state.status_message =
                        "Git repository URL can only be configured when git repository is enabled."
                            .to_string();
                }
            }
            "Disk" => {
                // Check if we need multi-disk selection
                let state = self.lock_state();
                let partitioning_strategy = state
                    .config
                    .options
                    .iter()
                    .find(|opt| opt.name == "Partitioning Strategy")
                    .map(|opt| opt.get_value())
                    .unwrap_or_default();
                drop(state); // Release the lock

                match partitioning_strategy.as_str() {
                    "auto_raid" | "auto_raid_luks" | "auto_raid_lvm" | "auto_raid_lvm_luks"
                    | "manual" => {
                        self.input_handler
                            .start_multi_disk_selection(&partitioning_strategy);
                    }
                    _ => {
                        self.input_handler.start_disk_selection(option.value);
                    }
                }
            }
            "Username" | "Hostname" => {
                let placeholder = match option.name.as_str() {
                    "Username" => "Enter username",
                    "Hostname" => "Enter hostname",
                    _ => "Enter value",
                }
                .to_string();

                self.input_handler
                    .start_text_input(option.name.clone(), option.value, placeholder);
            }
            "Encryption Password" => {
                // Only allow encryption password when encryption is enabled
                let encryption_enabled = {
                    let state = self.lock_state();
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Encryption")
                        .map(|opt| opt.get_value().to_lowercase() != "no")
                        .unwrap_or(false)
                };

                if encryption_enabled {
                    self.input_handler.start_password_input(
                        option.name.clone(),
                        option.value,
                        "Enter encryption passphrase".to_string(),
                    );
                } else { let mut state = self.lock_state();
                    state.status_message =
                        "Encryption password is only needed when encryption is enabled."
                            .to_string();
                }
            }
            "User Password" | "Root Password" => {
                let placeholder = match option.name.as_str() {
                    "User Password" => "Enter user password",
                    "Root Password" => "Enter root password",
                    _ => "Enter password",
                }
                .to_string();

                self.input_handler.start_password_input(
                    option.name.clone(),
                    option.value,
                    placeholder,
                );
            }
            "Additional Pacman Packages" | "Additional AUR Packages" => {
                self.input_handler
                    .start_package_selection(option.name.clone(), option.get_value());
            }
            "Timezone Region" => {
                let options = InputHandler::get_predefined_options(&option.name);
                self.input_handler
                    .start_selection(option.name.clone(), options, option.value);
            }
            "Timezone" => {
                // Get timezone options based on selected region
                let timezone_region = {
                    let state = self.lock_state();
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Timezone Region")
                        .map(|opt| opt.get_value())
                        .unwrap_or_default()
                };

                if !timezone_region.is_empty() {
                    let options = InputHandler::get_timezones_for_region(&timezone_region);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else { let mut state = self.lock_state();
                    state.status_message = "Please select a timezone region first.".to_string();
                }
            }
            "Display Manager" => {
                // Check if desktop environment is set to something other than "none"
                let desktop_env = {
                    let state = self.lock_state();
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Desktop Environment")
                        .map(|opt| opt.get_value())
                        .unwrap_or_default()
                };

                if desktop_env != "none" && !desktop_env.is_empty() {
                    // Desktop environment is selected, display manager should be auto-set
                    { let mut state = self.lock_state();
                        state.status_message =
                            "Display Manager is auto-set based on Desktop Environment selection."
                                .to_string();
                    }
                } else {
                    // No desktop environment or "none" selected, allow manual selection
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                }
            }
            "Plymouth Theme" => {
                // Only allow theme selection when Plymouth is enabled
                let plymouth_enabled = {
                    let state = self.lock_state();
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Plymouth")
                        .map(|opt| opt.get_value().to_lowercase() == "yes")
                        .unwrap_or(false)
                };

                if plymouth_enabled {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else { let mut state = self.lock_state();
                    state.status_message =
                        "Plymouth theme can only be selected when Plymouth is enabled."
                            .to_string();
                }
            }
            "OS Prober" => {
                // Only relevant for GRUB bootloader
                let is_grub = {
                    let state = self.lock_state();
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Bootloader")
                        .map(|opt| opt.get_value().to_lowercase() == "grub")
                        .unwrap_or(false)
                };

                if is_grub {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else { let mut state = self.lock_state();
                    state.status_message =
                        "OS Prober is only available with the GRUB bootloader.".to_string();
                }
            }
            "Home Filesystem" => {
                // Only relevant when Separate Home Partition is enabled
                let home_enabled = {
                    let state = self.lock_state();
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Separate Home Partition")
                        .map(|opt| opt.get_value().to_lowercase() == "yes")
                        .unwrap_or(false)
                };

                if home_enabled {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else { let mut state = self.lock_state();
                    state.status_message =
                        "Home filesystem can only be selected when Separate Home Partition is enabled."
                            .to_string();
                }
            }
            "Separate Home Partition" => {
                // Plain RAID strategies can't support separate home partitions
                let is_plain_raid = {
                    let state = self.lock_state();
                    let strat = state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Partitioning Strategy")
                        .map(|opt| opt.get_value())
                        .unwrap_or_default();
                    strat == "auto_raid" || strat == "auto_raid_luks"
                };

                if is_plain_raid {
                    { let mut state = self.lock_state();
                        state.status_message =
                            "Separate home not available for RAID without LVM."
                                .to_string();
                    }
                } else {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                }
            }
            "Btrfs Snapshots" => {
                // Only allow toggling btrfs snapshots when root filesystem is btrfs
                let is_btrfs = {
                    let state = self.lock_state();
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Root Filesystem")
                        .map(|opt| opt.get_value().to_lowercase() == "btrfs")
                        .unwrap_or(false)
                };

                if is_btrfs {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else { let mut state = self.lock_state();
                    state.status_message =
                        "Btrfs Snapshots requires Root Filesystem to be btrfs.".to_string();
                }
            }
            "RAID Level" => {
                // Only allow RAID level selection when a RAID strategy is selected
                let is_raid = {
                    let state = self.lock_state();
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Partitioning Strategy")
                        .map(|opt| opt.get_value().contains("raid"))
                        .unwrap_or(false)
                };

                if is_raid {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else { let mut state = self.lock_state();
                    state.status_message =
                        "RAID Level is only available for RAID partitioning strategies.".to_string();
                }
            }
            "AUR Helper" => {
                // When DE requires AUR, lock the field — user can't set to None
                let de_requires_aur = {
                    let state = self.lock_state();
                    let de_value = state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Desktop Environment")
                        .map(|opt| opt.get_value().to_string())
                        .unwrap_or_default();
                    let de: DesktopEnvironment = de_value.parse().unwrap_or_default();
                    de.requires_aur()
                };

                if de_requires_aur {
                    // Block the dialog entirely, like encryption does for LUKS strategies
                    { let mut state = self.lock_state();
                        state.status_message =
                            "AUR Helper is required by the selected desktop environment and cannot be set to None."
                                .to_string();
                    }
                } else {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                }
            }
            _ => {
                // Use predefined options for selection fields
                let options = InputHandler::get_predefined_options(&option.name);
                self.input_handler
                    .start_selection(option.name.clone(), options, option.value);
            }
        }

        Ok(())
    }

    /// Update configuration value after input dialog
    fn update_configuration_value(
        &mut self,
        value: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (current_step, option_name) = {
            let state = self
                .state
                .lock()
                .map_err(|e| error::general_error(format!("Mutex poisoned: {}", e)))?;
            if state.config_scroll.selected_index >= state.config.options.len() {
                return Err(error::general_error("Invalid configuration option index").into());
            }
            (
                state.config_scroll.selected_index,
                state.config.options[state.config_scroll.selected_index]
                    .name
                    .clone(),
            )
        };

        // Update the configuration value
        {
            let mut state = self
                .state
                .lock()
                .map_err(|e| error::general_error(format!("Mutex poisoned: {}", e)))?;
            if current_step < state.config.options.len() {
                // Parse disk selection to extract only device path
                let parsed_value = if option_name == "Disk" {
                    // Check if this is multi-disk selection by counting /dev/ occurrences
                    // (can't just check for commas since disk info may contain commas like "VBOX HARDDISK, 512B")
                    let dev_count = value.matches("/dev/").count();
                    if dev_count > 1 {
                        // Multi-disk selection - extract all disk paths
                        // Split by /dev/ and reconstruct paths
                        let disk_paths: Vec<String> = value
                            .split("/dev/")
                            .skip(1) // Skip empty first element
                            .map(|d| format!("/dev/{}", d.split_whitespace().next().unwrap_or("")))
                            .filter(|d| d.len() > 5) // Filter out just "/dev/"
                            .collect();

                        // Check if this is manual partitioning
                        let partitioning_strategy = state
                            .config
                            .options
                            .iter()
                            .find(|opt| opt.name == "Partitioning Strategy")
                            .map(|opt| opt.value.as_str())
                            .unwrap_or("");

                        if partitioning_strategy == "manual" {
                            // For manual partitioning, show confirmation dialog
                            drop(state); // Release the lock
                            self.input_handler
                                .start_manual_partitioning_confirmation(&disk_paths);
                            return Ok(());
                        } else {
                            // For RAID strategies, join with commas
                            disk_paths.join(",")
                        }
                    } else {
                        // Single disk selection - extract just the device path from "/dev/sda (128G)" -> "/dev/sda"
                        value
                            .split_whitespace()
                            .next()
                            .unwrap_or(&value)
                            .to_string()
                    }
                } else {
                    value.clone()
                };

                state.config.options[current_step].value = parsed_value.clone();
                state.status_message = format!(
                    "Set {} to: {}",
                    state.config.options[current_step].name, parsed_value
                );
            }
        }

        // Handle manual partitioning confirmation
        if option_name == "manual_partitioning_confirm" {
            if value == "Yes, start partitioning" {
                // User confirmed manual partitioning
                let state = self.lock_state();
                let disk_value = state
                    .config
                    .options
                    .iter()
                    .find(|opt| opt.name == "Disk")
                    .map(|opt| opt.value.clone())
                    .unwrap_or_default();
                drop(state);

                // Extract disk paths
                let disk_paths: Vec<String> = disk_value
                    .split(',')
                    .map(|d| d.split_whitespace().next().unwrap_or("").to_string())
                    .filter(|d| !d.is_empty())
                    .collect();

                // Launch partitioning tool
                if let Err(e) = self.input_handler.launch_partitioning_tool(&disk_paths) {
                    { let mut state = self.lock_state();
                        state.status_message = format!("Partitioning failed: {}", e);
                        return Ok(());
                    }
                }

                // Validate partitioning after user finishes
                let boot_mode = {
                    let state = self.lock_state();
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Boot Mode")
                        .map(|opt| opt.value.as_str())
                        .unwrap_or("BIOS")
                        .to_string()
                };

                match self
                    .input_handler
                    .validate_manual_partitioning(&disk_paths, &boot_mode)
                {
                    Ok(layout) => {
                        // Launch partition assignment dialog sequence
                        let partitions: Vec<(String, String)> = layout
                            .partitions
                            .iter()
                            .map(|p| (p.name.clone(), p.size.clone()))
                            .collect();
                        let is_uefi = boot_mode.to_lowercase() == "uefi"
                            || self.is_uefi_supported();

                        self.input_handler
                            .start_manual_assignment(partitions, is_uefi);

                        let mut state = self.lock_state();
                        state.status_message =
                            "Assign partitions: select ROOT partition".to_string();
                    }
                    Err(e) => {
                        { let mut state = self.lock_state();
                            state.status_message = format!("Partitioning validation failed: {}", e);
                        }
                    }
                }
            }
            // If user chose "No, go back", just return (dialog will close)
            return Ok(());
        }

        // Auto-set encryption based on partitioning strategy
        if option_name == "Partitioning Strategy" {
            self.auto_set_encryption(&value)?;
        }

        // Auto-set display manager and AUR helper based on desktop environment
        if option_name == "Desktop Environment" {
            self.auto_set_de_dependencies(&value)?;
        }

        // Handle warning dialog acknowledgment
        if value == "acknowledged" {
            // Warning was acknowledged, proceed to show normal selection dialog
        }

        // Check for secure boot warning after setting Secure Boot
        if option_name == "Secure Boot" && value.to_lowercase() == "yes" {
            self.check_secure_boot_warning()?;
        }

        // Handle dependent option updates
        self.handle_dependent_options(&option_name, &value)?;

        // Move to next step
        {
            { let mut state = self.lock_state();
                if state.config_scroll.selected_index < state.config.options.len() - 1 {
                    let next_index = state.config_scroll.selected_index + 1;
                    state.config_scroll.set_selected(next_index);
                }
            }
        }

        Ok(())
    } // Close the update_configuration_value function

    /// Auto-set encryption based on partitioning strategy
    fn auto_set_encryption(
        &mut self,
        partitioning_strategy: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Only auto-set if not manual partitioning
        if partitioning_strategy != "manual" {
            let encryption_value = if partitioning_strategy.contains("luks") {
                "Yes"
            } else {
                "No"
            };

            {
                { let mut state = self.lock_state();
                    if let Some(enc_opt) = state.config.options.iter_mut().find(|opt| opt.name == "Encryption") {
                        enc_opt.value = encryption_value.to_string();
                        state.status_message = format!(
                            "Auto-set Encryption to: {} (based on partitioning strategy)",
                            encryption_value
                        );
                    }
                    // Mark encryption password as N/A when encryption is disabled, clear when enabled
                    if encryption_value == "No" {
                        if let Some(pass_opt) = state.config.options.iter_mut().find(|opt| opt.name == "Encryption Password") {
                            pass_opt.value = "N/A".to_string();
                        }
                    } else if let Some(pass_opt) = state.config.options.iter_mut().find(|opt| opt.name == "Encryption Password") {
                        if pass_opt.value == "N/A" {
                            pass_opt.value = String::new();
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Auto-set display manager and AUR helper based on desktop environment
    fn auto_set_de_dependencies(
        &mut self,
        desktop_env: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let display_manager = match desktop_env {
            "kde" | "sway" => "sddm",
            "gnome" | "budgie" => "gdm",
            "hyprland" => "sddm",
            "i3" | "xfce" | "cinnamon" | "mate" => "lightdm",
            "none" => "none",
            _ => "",
        };

        let de: DesktopEnvironment = desktop_env.parse().unwrap_or_default();

        { let mut state = self.lock_state();
            // Auto-set display manager
            if !display_manager.is_empty() {
                if let Some(dm_opt) = state
                    .config
                    .options
                    .iter_mut()
                    .find(|opt| opt.name == "Display Manager")
                {
                    dm_opt.value = display_manager.to_string();
                }
            }

            // Auto-set AUR helper when DE requires AUR packages
            if de.requires_aur() {
                if let Some(aur_opt) = state
                    .config
                    .options
                    .iter_mut()
                    .find(|opt| opt.name == "AUR Helper")
                {
                    if aur_opt.get_value().to_lowercase() == "none" {
                        aur_opt.value = "paru".to_string();
                    }
                }
                state.status_message = format!(
                    "{} requires AUR packages — AUR Helper auto-set",
                    desktop_env
                );
            } else if !display_manager.is_empty() {
                state.status_message = format!(
                    "Auto-set Display Manager to: {} (based on desktop environment)",
                    display_manager
                );
            }
        }

        Ok(())
    }

    /// Handle dependent option updates based on user selections
    fn handle_dependent_options(
        &mut self,
        option_name: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        { let mut state = self.lock_state();
            match option_name {
                "Swap" => {
                    if value.to_lowercase() == "no" {
                        // Disable swap size when swap is disabled
                        if let Some(swap_size_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Swap Size")
                        {
                            swap_size_option.value = "N/A".to_string();
                        }
                    } else if value.to_lowercase() == "yes" {
                        // Reset swap size to default when swap is enabled
                        if let Some(swap_size_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Swap Size")
                        {
                            swap_size_option.value = "2GB".to_string();
                        }
                    }
                }
                "Separate Home Partition" => {
                    if value.to_lowercase() == "no" {
                        // No home: root takes remaining, home size/filesystem not applicable
                        if let Some(root_opt) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Root Size")
                        {
                            root_opt.value = "Remaining".to_string();
                        }
                        if let Some(home_opt) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Home Size")
                        {
                            home_opt.value = "N/A".to_string();
                        }
                        if let Some(home_fs_opt) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Home Filesystem")
                        {
                            home_fs_opt.value = "N/A".to_string();
                        }
                    } else if value.to_lowercase() == "yes" {
                        // With home: root gets fixed default, home takes remaining
                        if let Some(root_opt) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Root Size")
                        {
                            root_opt.value = "50GB".to_string();
                        }
                        if let Some(home_opt) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Home Size")
                        {
                            home_opt.value = "Remaining".to_string();
                        }
                        // Restore home filesystem to match root filesystem
                        let root_fs = state
                            .config
                            .options
                            .iter()
                            .find(|opt| opt.name == "Root Filesystem")
                            .map(|opt| opt.value.clone())
                            .unwrap_or_else(|| "ext4".to_string());
                        if let Some(home_fs_opt) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Home Filesystem")
                        {
                            home_fs_opt.value = root_fs;
                        }
                    }
                }
                "Partitioning Strategy" => {
                    // Clear stale manual partition assignments on strategy change
                    state.manual_partition_map = None;

                    let is_plain_raid = (value == "auto_raid" || value == "auto_raid_luks")
                        && !value.contains("lvm");

                    if is_plain_raid {
                        // Plain RAID without LVM can't support separate home partitions
                        // (data partition takes entire disk, no room for home partition)
                        if let Some(home_opt) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Separate Home Partition")
                        {
                            home_opt.value = "No".to_string();
                        }
                        // Cascade: root takes remaining, home not applicable
                        if let Some(root_opt) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Root Size")
                        {
                            root_opt.value = "Remaining".to_string();
                        }
                        for name in &["Home Size", "Home Filesystem"] {
                            if let Some(opt) = state
                                .config
                                .options
                                .iter_mut()
                                .find(|o| o.name == *name)
                            {
                                opt.value = "N/A".to_string();
                            }
                        }
                    }
                    // Set RAID Level based on whether strategy is RAID
                    if value.contains("raid") {
                        if let Some(raid_opt) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "RAID Level")
                        {
                            if raid_opt.value == "N/A" {
                                raid_opt.value = "raid1".to_string();
                            }
                        }
                    } else if let Some(raid_opt) = state
                        .config
                        .options
                        .iter_mut()
                        .find(|opt| opt.name == "RAID Level")
                    {
                        raid_opt.value = "N/A".to_string();
                    }

                    if value.contains("lvm") && !is_plain_raid {
                        // When switching to LVM without home, ensure root size is usable
                        let home_enabled = state
                            .config
                            .options
                            .iter()
                            .find(|opt| opt.name == "Separate Home Partition")
                            .map(|opt| opt.get_value().to_lowercase() == "yes")
                            .unwrap_or(false);

                        if !home_enabled {
                            // LVM without home: root should take remaining space
                            if let Some(root_opt) = state
                                .config
                                .options
                                .iter_mut()
                                .find(|opt| opt.name == "Root Size")
                            {
                                if root_opt.value == "N/A" {
                                    root_opt.value = "Remaining".to_string();
                                }
                            }
                        }
                    }
                }
                "Root Filesystem" => {
                    if value.to_lowercase() != "btrfs" {
                        // Disable all btrfs options when not using btrfs
                        for name in &[
                            "Btrfs Snapshots",
                            "Btrfs Frequency",
                            "Btrfs Keep Count",
                            "Btrfs Assistant",
                        ] {
                            if let Some(opt) = state
                                .config
                                .options
                                .iter_mut()
                                .find(|o| o.name == *name)
                            {
                                opt.value = if *name == "Btrfs Snapshots" || *name == "Btrfs Assistant" {
                                    "No".to_string()
                                } else {
                                    "N/A".to_string()
                                };
                            }
                        }
                    }
                }
                "Btrfs Snapshots" => {
                    if value.to_lowercase() == "no" {
                        // Disable btrfs frequency, keep count, and assistant when snapshots are disabled
                        if let Some(freq_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Btrfs Frequency")
                        {
                            freq_option.value = "N/A".to_string();
                        }
                        if let Some(keep_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Btrfs Keep Count")
                        {
                            keep_option.value = "N/A".to_string();
                        }
                        if let Some(assistant_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Btrfs Assistant")
                        {
                            assistant_option.value = "No".to_string();
                        }
                    } else if value.to_lowercase() == "yes" {
                        // Reset btrfs options to defaults when snapshots are enabled
                        if let Some(freq_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Btrfs Frequency")
                        {
                            freq_option.value = "weekly".to_string();
                        }
                        if let Some(keep_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Btrfs Keep Count")
                        {
                            keep_option.value = "3".to_string();
                        }
                    }
                }
                "Encryption" => {
                    if value.to_lowercase() == "no" {
                        // Mark encryption password as N/A when encryption is disabled
                        if let Some(pass_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Encryption Password")
                        {
                            pass_option.value = "N/A".to_string();
                        }
                    } else if let Some(pass_option) = state
                        .config
                        .options
                        .iter_mut()
                        .find(|opt| opt.name == "Encryption Password")
                    {
                        // Restore password field when encryption is re-enabled
                        if pass_option.value == "N/A" {
                            pass_option.value = String::new();
                        }
                    }
                }
                "Plymouth" => {
                    if value.to_lowercase() == "no" {
                        // Set plymouth theme to none when plymouth is disabled
                        if let Some(theme_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Plymouth Theme")
                        {
                            theme_option.value = "none".to_string();
                        }
                    }
                }
                "Disk" => {
                    // Clear stale manual partition assignments on disk change
                    state.manual_partition_map = None;
                }
                "Git Repository" => {
                    if value.to_lowercase() == "no" {
                        // Clear git repository URL when git is disabled
                        if let Some(url_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Git Repository URL")
                        {
                            url_option.value = String::new();
                        }
                    }
                }
                "GRUB Theme" => {
                    if value.to_lowercase() == "no" {
                        // Set GRUB theme selection to none when themes are disabled
                        if let Some(theme_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "GRUB Theme Selection")
                        {
                            theme_option.value = "none".to_string();
                        }
                    }
                }
                "Bootloader" => {
                    if value.to_lowercase() != "grub" {
                        // Reset GRUB-specific options when switching away from GRUB
                        if let Some(theme_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "GRUB Theme")
                        {
                            theme_option.value = "No".to_string();
                        }
                        if let Some(selection_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "GRUB Theme Selection")
                        {
                            selection_option.value = "none".to_string();
                        }
                        if let Some(prober_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "OS Prober")
                        {
                            prober_option.value = "No".to_string();
                        }
                    }
                }
                "Timezone Region" => {
                    // Reset timezone when region changes
                    if let Some(timezone_option) = state
                        .config
                        .options
                        .iter_mut()
                        .find(|opt| opt.name == "Timezone")
                    {
                        timezone_option.value = "".to_string(); // Reset to empty to force selection
                    }

                    // Auto-select mirror country based on region for quality of life
                    let mirror_country = match value {
                        "US" => "United States",
                        "Europe" => "Germany", // Default to Germany for Europe
                        "Asia" => "Japan",     // Default to Japan for Asia
                        "Australia" => "Australia",
                        "America" => "United States", // For America region, default to US
                        _ => "",                      // Don't auto-select for other regions
                    };

                    if !mirror_country.is_empty() {
                        if let Some(mirror_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Mirror Country")
                        {
                            mirror_option.value = mirror_country.to_string();
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Handle window resize events
    fn handle_resize(
        &mut self,
        _width: u16,
        height: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Update scroll state with new visible height
        { let mut state = self.lock_state();
            if state.mode == AppMode::GuidedInstaller {
                // Calculate available height for config list
                // Header(7) + Title(3) + Instructions(3) + Start Button(3) = 16 lines reserved
                let available_height = (height as usize).saturating_sub(16);
                // Use most of the available space, with a minimum of 5 lines
                let visible_height = available_height.max(5);
                state.config_scroll.update_visible_items(visible_height);
            }
        }
        Ok(())
    }

    // =========================================================================
    // SECTION: Tool Parameter Definitions
    // =========================================================================
    //
    // Each tool that requires user input has parameter definitions here.
    // Parameters can be:
    // - Text: Free-form text input
    // - Selection: Dropdown/list selection
    // - Boolean: Yes/No toggle
    //
    // Parameters marked `required: true` must be filled before execution.
    //
    // =========================================================================

    /// Get tool parameter definitions for a specific tool
    fn get_tool_parameters(tool_name: &str) -> Vec<ToolParam> {
        match tool_name {
            "install_bootloader" => vec![
                ToolParam {
                    name: "bootloader".to_string(),
                    description: "Bootloader to install (GRUB supports BIOS+UEFI, systemd-boot is UEFI only)".to_string(),
                    param_type: ToolParameter::Selection(
                        vec!["grub".to_string(), "systemd-boot".to_string()],
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "disk".to_string(),
                    description: "Target disk device (e.g., /dev/sda)".to_string(),
                    param_type: ToolParameter::Selection(
                        crate::detection::detect_disks(),
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "efi_path".to_string(),
                    description: "EFI system partition mount point (e.g., /boot/efi)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: false,
                },
                ToolParam {
                    name: "boot_mode".to_string(),
                    description: "Boot mode — leave empty for auto-detection".to_string(),
                    param_type: ToolParameter::Selection(
                        vec!["".to_string(), "uefi".to_string(), "bios".to_string()],
                        0,
                    ),
                    required: false,
                },
                ToolParam {
                    name: "repair".to_string(),
                    description: "Repair existing bootloader instead of fresh install".to_string(),
                    param_type: ToolParameter::Boolean(false),
                    required: false,
                },
            ],
            "generate_fstab" => vec![ToolParam {
                name: "root".to_string(),
                description: "Mounted root path to generate fstab from (e.g., /mnt)".to_string(),
                param_type: ToolParameter::Text("".to_string()),
                required: true,
            }],
            "format_partition" => vec![
                ToolParam {
                    name: "device".to_string(),
                    description: "Partition device (e.g., /dev/sda1)".to_string(),
                    param_type: ToolParameter::Selection(
                        crate::detection::detect_partitions(),
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "filesystem".to_string(),
                    description: "Filesystem type to format the partition with".to_string(),
                    param_type: ToolParameter::Selection(
                        vec![
                            "ext4".to_string(),
                            "xfs".to_string(),
                            "btrfs".to_string(),
                            "fat32".to_string(),
                            "ntfs".to_string(),
                        ],
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "label".to_string(),
                    description: "Partition label (optional)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: false,
                },
            ],
            "wipe_disk" => vec![
                ToolParam {
                    name: "device".to_string(),
                    description: "Disk device to wipe (e.g., /dev/sda)".to_string(),
                    param_type: ToolParameter::Selection(
                        crate::detection::detect_disks(),
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "method".to_string(),
                    description: "quick: signatures only | secure: full wipe | auto: detect SSD/HDD".to_string(),
                    param_type: ToolParameter::Selection(
                        vec![
                            "quick".to_string(),
                            "secure".to_string(),
                            "auto".to_string(),
                        ],
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "confirm".to_string(),
                    description: "ALL DATA WILL BE DESTROYED. Type CONFIRM to proceed.".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: true,
                },
            ],
            "mount" => vec![
                ToolParam {
                    name: "action".to_string(),
                    description: "mount: attach device | umount: detach | list: show mounts | info: device details".to_string(),
                    param_type: ToolParameter::Selection(
                        vec![
                            "mount".to_string(),
                            "umount".to_string(),
                            "list".to_string(),
                            "info".to_string(),
                        ],
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "target".to_string(),
                    description: "Device path for mount/info (e.g., /dev/sda1) or mountpoint for umount".to_string(),
                    param_type: ToolParameter::Selection(
                        crate::detection::detect_partitions(),
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "destination".to_string(),
                    description: "Mount destination directory (e.g., /mnt)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: false,
                },
                ToolParam {
                    name: "read_only".to_string(),
                    description: "Mount filesystem as read-only".to_string(),
                    param_type: ToolParameter::Boolean(false),
                    required: false,
                },
                ToolParam {
                    name: "force".to_string(),
                    description: "Force lazy unmount if device is busy".to_string(),
                    param_type: ToolParameter::Boolean(false),
                    required: false,
                },
            ],
            "chroot" => vec![
                ToolParam {
                    name: "root".to_string(),
                    description: "Root directory to chroot into".to_string(),
                    param_type: ToolParameter::Text("/mnt".to_string()),
                    required: true,
                },
                ToolParam {
                    name: "skip_mount".to_string(),
                    description: "Skip mounting /proc, /sys, /dev (if already mounted)".to_string(),
                    param_type: ToolParameter::Boolean(false),
                    required: false,
                },
            ],
            "add_user" => vec![
                ToolParam {
                    name: "username".to_string(),
                    description: "Username to create".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: true,
                },
                ToolParam {
                    name: "password".to_string(),
                    description: "User password".to_string(),
                    param_type: ToolParameter::Password("".to_string()),
                    required: false,
                },
                ToolParam {
                    name: "full_name".to_string(),
                    description: "Full name (optional)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: false,
                },
                ToolParam {
                    name: "groups".to_string(),
                    description: "Additional groups (comma-separated, optional)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: false,
                },
                ToolParam {
                    name: "shell".to_string(),
                    description: "Login shell (default: /bin/bash)".to_string(),
                    param_type: ToolParameter::Selection(
                        crate::detection::detect_shells(),
                        0,
                    ),
                    required: false,
                },
                ToolParam {
                    name: "system_user".to_string(),
                    description: "Create as system user".to_string(),
                    param_type: ToolParameter::Boolean(false),
                    required: false,
                },
            ],
            "reset_password" => vec![
                ToolParam {
                    name: "username".to_string(),
                    description: "Username to reset password for".to_string(),
                    param_type: ToolParameter::Selection(
                        crate::detection::detect_users(),
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "password".to_string(),
                    description: "New password".to_string(),
                    param_type: ToolParameter::Password("".to_string()),
                    required: true,
                },
            ],
            "configure_network" => vec![
                ToolParam {
                    name: "interface".to_string(),
                    description: "Network interface name (e.g., eth0, enp0s3)".to_string(),
                    param_type: ToolParameter::Selection(
                        crate::detection::detect_interfaces(),
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "action".to_string(),
                    description: "configure: set IP | status/info: view state | enable/disable: toggle interface".to_string(),
                    param_type: ToolParameter::Selection(
                        vec![
                            "configure".to_string(),
                            "status".to_string(),
                            "info".to_string(),
                            "enable".to_string(),
                            "disable".to_string(),
                        ],
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "config_type".to_string(),
                    description: "DHCP for automatic or static for manual IP assignment".to_string(),
                    param_type: ToolParameter::Selection(
                        vec!["".to_string(), "dhcp".to_string(), "static".to_string()],
                        0,
                    ),
                    required: false,
                },
                ToolParam {
                    name: "ip".to_string(),
                    description: "IP address (for static configuration)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: false,
                },
                ToolParam {
                    name: "netmask".to_string(),
                    description: "Network mask (e.g., 255.255.255.0 or 24)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: false,
                },
                ToolParam {
                    name: "gateway".to_string(),
                    description: "Default gateway (for static configuration)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: false,
                },
            ],
            "manage_services" => vec![
                ToolParam {
                    name: "action".to_string(),
                    description: "enable/disable: persist across reboots | status: check state | list: show all".to_string(),
                    param_type: ToolParameter::Selection(
                        vec![
                            "enable".to_string(),
                            "disable".to_string(),
                            "status".to_string(),
                            "list".to_string(),
                        ],
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "service".to_string(),
                    description: "Systemd service name (e.g., NetworkManager, sshd, bluetooth)".to_string(),
                    param_type: ToolParameter::Selection(
                        crate::detection::detect_services(),
                        0,
                    ),
                    required: true,
                },
            ],
            "manage_groups" => vec![
                ToolParam {
                    name: "action".to_string(),
                    description: "add: add user to group | remove: remove from group | list: show memberships".to_string(),
                    param_type: ToolParameter::Selection(
                        vec![
                            "add".to_string(),
                            "remove".to_string(),
                            "list".to_string(),
                        ],
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "user".to_string(),
                    description: "Target username to modify group membership for".to_string(),
                    param_type: ToolParameter::Selection(
                        crate::detection::detect_users(),
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "group".to_string(),
                    description: "Group name (e.g., wheel, audio, video, docker)".to_string(),
                    param_type: ToolParameter::Selection(
                        crate::detection::detect_groups(),
                        0,
                    ),
                    required: true,
                },
            ],
            "configure_ssh" => vec![
                ToolParam {
                    name: "action".to_string(),
                    description: "status: check sshd | install: install openssh | enable/disable: toggle service".to_string(),
                    param_type: ToolParameter::Selection(
                        vec![
                            "status".to_string(),
                            "install".to_string(),
                            "enable".to_string(),
                            "disable".to_string(),
                            "configure".to_string(),
                        ],
                        0,
                    ),
                    required: true,
                },
            ],
            "configure_firewall" => vec![
                ToolParam {
                    name: "action".to_string(),
                    description: "status: show rules | enable: apply defaults | disable: permissive | rules: list numbered".to_string(),
                    param_type: ToolParameter::Selection(
                        vec![
                            "status".to_string(),
                            "enable".to_string(),
                            "disable".to_string(),
                            "rules".to_string(),
                        ],
                        0,
                    ),
                    required: true,
                },
            ],
            "security_audit" => vec![ToolParam {
                name: "action".to_string(),
                description: "basic: quick permission/service checks | full: comprehensive audit".to_string(),
                param_type: ToolParameter::Selection(
                    vec!["basic".to_string(), "full".to_string()],
                    0,
                ),
                required: true,
            }],
            "test_network" => vec![
                ToolParam {
                    name: "action".to_string(),
                    description: "ping: ICMP test | dns: name resolution | http: web access | full: all tests".to_string(),
                    param_type: ToolParameter::Selection(
                        vec![
                            "full".to_string(),
                            "ping".to_string(),
                            "dns".to_string(),
                            "http".to_string(),
                        ],
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "timeout".to_string(),
                    description: "Timeout in seconds for each test".to_string(),
                    param_type: ToolParameter::Selection(
                        vec!["5".to_string(), "10".to_string(), "30".to_string()],
                        0,
                    ),
                    required: false,
                },
            ],
            "network_diagnostics" => vec![ToolParam {
                name: "action".to_string(),
                description: "info: interfaces | basic: quick check | detailed: full analysis | troubleshoot: diagnose issues".to_string(),
                param_type: ToolParameter::Selection(
                    vec![
                        "info".to_string(),
                        "basic".to_string(),
                        "detailed".to_string(),
                        "troubleshoot".to_string(),
                    ],
                    0,
                ),
                required: true,
            }],
            "encrypt_device" => vec![
                ToolParam {
                    name: "action".to_string(),
                    description: "format: encrypt device | open: unlock encrypted device | close: lock device".to_string(),
                    param_type: ToolParameter::Selection(
                        vec![
                            "format".to_string(),
                            "open".to_string(),
                            "close".to_string(),
                        ],
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "device".to_string(),
                    description: "Device path (e.g., /dev/sda2)".to_string(),
                    param_type: ToolParameter::Selection(
                        crate::detection::detect_partitions(),
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "password".to_string(),
                    description: "LUKS passphrase (for format/open)".to_string(),
                    param_type: ToolParameter::Password("".to_string()),
                    required: false,
                },
                ToolParam {
                    name: "mapper_name".to_string(),
                    description: "Mapper device name (default: cryptroot)".to_string(),
                    param_type: ToolParameter::Text("cryptroot".to_string()),
                    required: false,
                },
            ],
            "enable_services" => vec![
                ToolParam {
                    name: "services".to_string(),
                    description: "Service names (comma-separated, e.g., sddm,NetworkManager)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: true,
                },
                ToolParam {
                    name: "root".to_string(),
                    description: "Root mount path for chroot".to_string(),
                    param_type: ToolParameter::Text("/mnt".to_string()),
                    required: true,
                },
            ],
            "install_aur_helper" => vec![
                ToolParam {
                    name: "helper".to_string(),
                    description: "AUR helper to install".to_string(),
                    param_type: ToolParameter::Selection(
                        vec!["paru".to_string(), "yay".to_string()],
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "user".to_string(),
                    description: "Target user (must be non-root)".to_string(),
                    param_type: ToolParameter::Selection(
                        crate::detection::detect_users(),
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "root".to_string(),
                    description: "Root mount path for chroot".to_string(),
                    param_type: ToolParameter::Text("/mnt".to_string()),
                    required: true,
                },
            ],
            "install_dotfiles" => vec![
                ToolParam {
                    name: "repo_url".to_string(),
                    description: "Git repository URL".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: true,
                },
                ToolParam {
                    name: "target_user".to_string(),
                    description: "Target user for dotfiles".to_string(),
                    param_type: ToolParameter::Selection(
                        crate::detection::detect_users(),
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "branch".to_string(),
                    description: "Branch to clone (optional, default: main)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: false,
                },
                ToolParam {
                    name: "backup".to_string(),
                    description: "Backup existing files before overwriting".to_string(),
                    param_type: ToolParameter::Boolean(true),
                    required: false,
                },
            ],
            "run_as_user" => vec![
                ToolParam {
                    name: "user".to_string(),
                    description: "Username to run command as".to_string(),
                    param_type: ToolParameter::Selection(
                        crate::detection::detect_users(),
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "command".to_string(),
                    description: "Command to execute".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: true,
                },
                ToolParam {
                    name: "root".to_string(),
                    description: "Chroot root path".to_string(),
                    param_type: ToolParameter::Text("/mnt".to_string()),
                    required: true,
                },
                ToolParam {
                    name: "work_dir".to_string(),
                    description: "Working directory inside chroot (optional)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: false,
                },
            ],
            "manual_partition" => vec![ToolParam {
                name: "device".to_string(),
                description: "Disk device to partition (e.g., /dev/sda)".to_string(),
                param_type: ToolParameter::Selection(
                    crate::detection::detect_disks(),
                    0,
                ),
                required: true,
            }],
            "partition_create_table" => vec![
                ToolParam {
                    name: "device".to_string(),
                    description: "Disk device (pre-filled from selection)".to_string(),
                    param_type: ToolParameter::Selection(
                        crate::detection::detect_disks(),
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "table_type".to_string(),
                    description: "GPT (modern, UEFI) or MBR (legacy BIOS)".to_string(),
                    param_type: ToolParameter::Selection(
                        vec!["gpt".to_string(), "mbr".to_string()],
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "confirm".to_string(),
                    description: "ALL DATA WILL BE DESTROYED. Type CONFIRM to proceed.".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: true,
                },
            ],
            "partition_add" => vec![
                ToolParam {
                    name: "device".to_string(),
                    description: "Disk device (pre-filled from selection)".to_string(),
                    param_type: ToolParameter::Selection(
                        crate::detection::detect_disks(),
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "number".to_string(),
                    description: "Partition number".to_string(),
                    param_type: ToolParameter::Selection(
                        vec![
                            "1".to_string(), "2".to_string(), "3".to_string(),
                            "4".to_string(), "5".to_string(), "6".to_string(),
                        ],
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "size".to_string(),
                    description: "Partition size (e.g., 512M, 50G, remaining)".to_string(),
                    param_type: ToolParameter::Text("remaining".to_string()),
                    required: true,
                },
                ToolParam {
                    name: "type".to_string(),
                    description: "Partition type code".to_string(),
                    param_type: ToolParameter::Selection(
                        vec![
                            "Linux".to_string(),
                            "EFI System".to_string(),
                            "Linux Swap".to_string(),
                            "Linux LVM".to_string(),
                            "Linux LUKS".to_string(),
                            "BIOS Boot".to_string(),
                        ],
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "label".to_string(),
                    description: "Partition label (optional, GPT only)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: false,
                },
                ToolParam {
                    name: "confirm".to_string(),
                    description: "This will modify the partition table. Type CONFIRM to proceed.".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: true,
                },
            ],
            "partition_delete" => vec![
                ToolParam {
                    name: "device".to_string(),
                    description: "Disk device (pre-filled from selection)".to_string(),
                    param_type: ToolParameter::Selection(
                        crate::detection::detect_disks(),
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "number".to_string(),
                    description: "Partition number to delete".to_string(),
                    param_type: ToolParameter::Selection(
                        vec![
                            "1".to_string(), "2".to_string(), "3".to_string(),
                            "4".to_string(), "5".to_string(), "6".to_string(),
                        ],
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "confirm".to_string(),
                    description: "This will delete a partition. Type CONFIRM to proceed.".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: true,
                },
            ],
            "rebuild_initramfs" => vec![ToolParam {
                name: "root".to_string(),
                description: "Root of the installed system (e.g., /mnt)".to_string(),
                param_type: ToolParameter::Text("/mnt".to_string()),
                required: true,
            }],
            "update_mirrors" => vec![
                ToolParam {
                    name: "country".to_string(),
                    description: "Country filter (ISO 3166-1 code, e.g., US, DE)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: false,
                },
                ToolParam {
                    name: "limit".to_string(),
                    description: "Number of mirrors to keep".to_string(),
                    param_type: ToolParameter::Selection(
                        vec!["10".to_string(), "20".to_string(), "50".to_string()],
                        1,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "sort".to_string(),
                    description: "rate: download speed | age: last sync time | score: mirror score".to_string(),
                    param_type: ToolParameter::Selection(
                        vec![
                            "rate".to_string(),
                            "age".to_string(),
                            "score".to_string(),
                        ],
                        0,
                    ),
                    required: true,
                },
            ],
            _ => vec![],
        }
    }

    /// Handle tool dialog input (navigation, parameter input, etc.)
    fn handle_tool_dialog_input(
        &mut self,
        key_event: KeyEvent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match key_event.code {
            KeyCode::Up => {
                // Move to previous parameter (if not at first)
                let mut state = self.lock_state();
                if let Some(ref mut dialog) = state.tool_dialog {
                    if dialog.current_param > 0 {
                        dialog.current_param -= 1;
                    }
                }
            }
            KeyCode::Down => {
                // Move to next parameter (if not at last)
                let mut state = self.lock_state();
                if let Some(ref mut dialog) = state.tool_dialog {
                    if dialog.current_param < dialog.parameters.len().saturating_sub(1) {
                        dialog.current_param += 1;
                    }
                }
            }
            KeyCode::Enter => {
                // Execute tool with collected parameters
                self.handle_tool_dialog_enter()?;
            }
            KeyCode::Esc => {
                // Cancel tool dialog and go back
                let mut state = self.lock_state();
                let current_tool = state.current_tool.clone();
                state.tool_dialog = None;
                state.current_tool = None;
                state.pre_dialog_mode = None;
                // Go back to appropriate tool menu
                if let Some(ref tool_name) = current_tool {
                    match tool_name.as_str() {
                        "format_partition" | "wipe_disk" | "health" | "mount"
                        | "manual_partition" | "encrypt_device"
                        | "partition_create_table" | "partition_add"
                        | "partition_delete" | "partition_action_menu" => {
                            state.pending_tool_device = None;
                            state.mode = AppMode::DiskTools;
                            state.tools_menu_selection = 0;
                            state.status_message = "Disk & Filesystem Tools".to_string();
                        }
                        "install_bootloader" | "generate_fstab" | "chroot" | "info"
                        | "manage_services" | "system_info" | "enable_services"
                        | "install_aur_helper" | "rebuild_initramfs" => {
                            state.mode = AppMode::SystemTools;
                            state.tools_menu_selection = 0;
                            state.status_message = "System & Boot Tools".to_string();
                        }
                        "add_user" | "reset_password" | "manage_groups"
                        | "configure_ssh" | "security_audit" | "install_dotfiles"
                        | "run_as_user" => {
                            state.mode = AppMode::UserTools;
                            state.tools_menu_selection = 0;
                            state.status_message = "User & Security Tools".to_string();
                        }
                        "configure_network" | "configure_firewall"
                        | "test_network" | "network_diagnostics"
                        | "update_mirrors" => {
                            state.mode = AppMode::NetworkTools;
                            state.tools_menu_selection = 0;
                            state.status_message = "Network Tools".to_string();
                        }
                        _ => {
                            state.mode = AppMode::ToolsMenu;
                            state.tools_menu_selection = 0;
                            state.status_message =
                                "Arch Linux Tools - System repair and administration".to_string();
                        }
                    }
                } else {
                    state.mode = AppMode::ToolsMenu;
                    state.tools_menu_selection = 0;
                    state.status_message =
                        "Arch Linux Tools - System repair and administration".to_string();
                }
            }
            KeyCode::Left | KeyCode::Right => {
                // Cycle through Selection parameter options
                let mut state = self.lock_state();
                if let Some(ref mut dialog) = state.tool_dialog {
                    let idx = dialog.current_param;
                    if idx < dialog.parameters.len() && idx < dialog.param_values.len() {
                        if let ToolParameter::Selection(ref options, _) = dialog.parameters[idx].param_type {
                            if !options.is_empty() {
                                // Find current selection index
                                let current_val = &dialog.param_values[idx];
                                let current_idx = options.iter().position(|o| o == current_val).unwrap_or(0);
                                let new_idx = if key_event.code == KeyCode::Right {
                                    (current_idx + 1) % options.len()
                                } else {
                                    current_idx.checked_sub(1).unwrap_or(options.len() - 1)
                                };
                                dialog.param_values[idx] = options[new_idx].clone();
                            }
                        }
                    }
                }
            }
            KeyCode::Char(c) => {
                // Handle text input for current parameter (skip for Selection types)
                let mut state = self.lock_state();
                if let Some(ref mut dialog) = state.tool_dialog {
                    let idx = dialog.current_param;
                    if idx < dialog.parameters.len() && idx < dialog.param_values.len()
                        && !matches!(dialog.parameters[idx].param_type, ToolParameter::Selection(_, _))
                    {
                        dialog.param_values[idx].push(c);
                    }
                }
            }
            KeyCode::Backspace => {
                // Handle backspace for current parameter (skip for Selection types)
                let mut state = self.lock_state();
                if let Some(ref mut dialog) = state.tool_dialog {
                    let idx = dialog.current_param;
                    if idx < dialog.parameters.len() && idx < dialog.param_values.len()
                        && !matches!(dialog.parameters[idx].param_type, ToolParameter::Selection(_, _))
                    {
                        dialog.param_values[idx].pop();
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle tool dialog enter key
    fn handle_tool_dialog_enter(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (tool_name, current_param, total_params, param_values) = {
            let state = self.lock_state();
            if let Some(ref dialog) = state.tool_dialog {
                (
                    dialog.tool_name.clone(),
                    dialog.current_param,
                    dialog.parameters.len(),
                    dialog.param_values.clone(),
                )
            } else {
                return Ok(());
            }
        };

        let last_param = total_params.saturating_sub(1);

        if current_param < total_params && current_param != last_param {
            // Not on last param — advance to next
            let mut state = self.lock_state();
            if let Some(ref mut dialog) = state.tool_dialog {
                dialog.current_param += 1;
            }
        } else if current_param == last_param {
            // On last param — execute tool
            self.execute_tool_with_params(&tool_name, param_values)?;
        }

        Ok(())
    }

    /// Create a tool dialog for parameter collection
    fn create_tool_dialog(&mut self, tool_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let parameters = Self::get_tool_parameters(tool_name);
        // Initialize param_values with defaults — Selection params get their default option
        let param_values: Vec<String> = parameters
            .iter()
            .map(|p| match &p.param_type {
                ToolParameter::Selection(options, default_idx) => {
                    options.get(*default_idx).cloned().unwrap_or_default()
                }
                _ => String::new(),
            })
            .collect();

        let mut state = self.lock_state();
        // Save current mode so the UI can render it behind the dialog
        if state.pre_dialog_mode.is_none() {
            state.pre_dialog_mode = Some(state.mode.clone());
        }
        state.current_tool = Some(tool_name.to_string());
        state.tool_dialog = Some(ToolDialogState {
            tool_name: tool_name.to_string(),
            parameters,
            current_param: 0,
            param_values,
            is_executing: false,
        });
        state.mode = AppMode::ToolDialog;
        state.status_message = format!("Configure parameters for {}", tool_name);

        Ok(())
    }

    /// Execute health tool with selected disk
    fn execute_health_tool_with_disk(
        &mut self,
        selected_disk: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Extract device path from formatted string like "/dev/sda (128G) info..."
        let device = selected_disk
            .split_whitespace()
            .next()
            .unwrap_or(&selected_disk)
            .to_string();
        let sa = CheckDiskHealthArgs {
            device: PathBuf::from(&device),
        };
        let mut cli_args = sa.to_cli_args();
        cli_args.push("--detailed".to_string());
        self.execute_via_script_args(
            sa.script_name(), cli_args, sa.get_env_vars(),
            "check disk health", sa.is_destructive(), true,
        )
    }

    /// Spawn a tool script in a background thread with real-time output streaming
    /// and optional environment variables.
    ///
    /// # Process Lifecycle Management
    /// - Child runs in its own process group (allows clean termination of entire tree)
    /// - Child PID is registered with global ChildRegistry
    /// - On App drop or signal, all registered children receive SIGTERM
    fn spawn_tool_script_with_env(
        &self,
        script_path: &str,
        args: Vec<String>,
        env_vars: Vec<(String, String)>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.tool_tx.clone();
        let script_path = script_path.to_string();

        thread::spawn(move || {
            let mut cmd = Command::new("bash");
            cmd.arg(&script_path)
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null()); // Non-interactive per lint rules

            // Pass environment variables
            for (key, val) in &env_vars {
                cmd.env(key, val);
            }

            // Spawn the child process in its own process group
            let child_result = cmd.in_new_process_group().spawn();

            let mut child = match child_result {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(ToolMessage::Error(format!(
                        "Failed to start script: {}",
                        e
                    )));
                    return;
                }
            };

            // Register child PID for lifecycle management
            let child_pid = child.id();
            if let Ok(mut registry) = ChildRegistry::global().lock() {
                registry.register(child_pid);
            }

            // Stream stdout in a separate thread
            let stdout_tx = tx.clone();
            let stdout_handle = child.stdout.take().map(|stdout| {
                thread::spawn(move || {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines().map_while(Result::ok) {
                        if stdout_tx.send(ToolMessage::Stdout(line)).is_err() {
                            break; // Receiver dropped
                        }
                    }
                })
            });

            // Stream stderr in a separate thread
            let stderr_tx = tx.clone();
            let stderr_handle = child.stderr.take().map(|stderr| {
                thread::spawn(move || {
                    let reader = BufReader::new(stderr);
                    for line in reader.lines().map_while(Result::ok) {
                        if stderr_tx.send(ToolMessage::Stderr(line)).is_err() {
                            break; // Receiver dropped
                        }
                    }
                })
            });

            // Wait for stdout/stderr threads to finish
            if let Some(h) = stdout_handle {
                let _ = h.join();
            }
            if let Some(h) = stderr_handle {
                let _ = h.join();
            }

            // Wait for child process to complete
            let result = child.wait();

            // Unregister child PID (process has exited)
            if let Ok(mut registry) = ChildRegistry::global().lock() {
                registry.unregister(child_pid);
            }

            match result {
                Ok(status) => {
                    let _ = tx.send(ToolMessage::Complete {
                        success: status.success(),
                        exit_code: status.code(),
                    });
                }
                Err(e) => {
                    let _ = tx.send(ToolMessage::Error(format!(
                        "Failed to wait for script: {}",
                        e
                    )));
                }
            }
        });

        Ok(())
    }

    // =========================================================================
    // SECTION: Disk Layout Viewer + Partition Action Helpers
    // =========================================================================

    /// Show disk layout in a FloatingOutput window.
    /// Called after DiskSelection for manual_partition, format_partition, wipe_disk.
    fn show_disk_layout(&mut self, device: &str) {
        // Run lsblk synchronously
        let lsblk_output = Command::new("lsblk")
            .args(["-o", "NAME,SIZE,TYPE,FSTYPE,LABEL,MOUNTPOINTS,PTTYPE", device])
            .output();

        let mut lines = vec![
            format!("  Disk Layout: {}", device),
            String::new(),
        ];

        match lsblk_output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    lines.push(format!("  {}", line));
                }
            }
            Err(e) => {
                lines.push(format!("  Failed to run lsblk: {}", e));
            }
        }

        lines.push(String::new());

        // Also show sgdisk/fdisk for partition type codes
        let sgdisk_output = Command::new("sgdisk")
            .args(["-p", device])
            .stderr(Stdio::null())
            .output();

        match sgdisk_output {
            Ok(output) if output.status.success() => {
                lines.push("  Partition table details:".to_string());
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines() {
                    lines.push(format!("  {}", line));
                }
            }
            _ => {
                // Fallback to fdisk for MBR tables
                let fdisk_output = Command::new("fdisk")
                    .args(["-l", device])
                    .stderr(Stdio::null())
                    .output();
                if let Ok(output) = fdisk_output {
                    lines.push("  Partition table details:".to_string());
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        lines.push(format!("  {}", line));
                    }
                }
            }
        }

        let tool_name = {
            let state = self.lock_state();
            state.current_tool.clone().unwrap_or_default()
        };
        let status = if tool_name == "manual_partition" {
            "Enter: partition actions | Esc: back to disk selection".to_string()
        } else {
            "Enter: continue | Esc: back to disk selection".to_string()
        };

        let mut state = self.lock_state();
        state.floating_output = Some(FloatingOutputState {
            title: format!("Disk Layout: {}", device),
            content: lines,
            scroll_offset: 0,
            auto_scroll: false,
            complete: true,
            progress: None,
            status,
        });
        state.pre_dialog_mode = Some(AppMode::DiskTools);
        state.mode = AppMode::FloatingOutput;
    }

    /// Show the partition action menu (Selection dialog with 5 options).
    /// Called when Enter is pressed on disk layout for manual_partition.
    fn show_partition_action_menu(&mut self, device: &str) {
        let options = vec![
            "Create Partition Table".to_string(),
            "Add Partition".to_string(),
            "Delete Partition".to_string(),
            "Advanced Editor (cfdisk)".to_string(),
            "Back to Disk Selection".to_string(),
        ];

        let parameters = vec![ToolParam {
            name: "action".to_string(),
            description: format!("Select partition action for {}", device),
            param_type: ToolParameter::Selection(options, 0),
            required: true,
        }];
        let param_values = vec!["Create Partition Table".to_string()];

        let mut state = self.lock_state();
        state.pre_dialog_mode = Some(AppMode::DiskTools);
        state.current_tool = Some("partition_action_menu".to_string());
        state.tool_dialog = Some(ToolDialogState {
            tool_name: "partition_action_menu".to_string(),
            parameters,
            current_param: 0,
            param_values,
            is_executing: false,
        });
        state.mode = AppMode::ToolDialog;
        state.status_message = format!("Select partition action for {}", device);
    }

    /// Open the format ToolDialog with device pre-filled.
    fn open_format_dialog_for_device(&mut self, device: &str) {
        let parameters = Self::get_tool_parameters("format_partition");
        let mut param_values: Vec<String> = parameters
            .iter()
            .map(|p| match &p.param_type {
                ToolParameter::Selection(options, default_idx) => {
                    options.get(*default_idx).cloned().unwrap_or_default()
                }
                _ => String::new(),
            })
            .collect();
        param_values[0] = device.to_string();

        let mut state = self.lock_state();
        state.pre_dialog_mode = Some(AppMode::DiskTools);
        state.current_tool = Some("format_partition".to_string());
        state.tool_dialog = Some(ToolDialogState {
            tool_name: "format_partition".to_string(),
            parameters,
            current_param: 1, // Skip device, start at filesystem
            param_values,
            is_executing: false,
        });
        state.mode = AppMode::ToolDialog;
        state.status_message = "Select filesystem type for formatting".to_string();
    }

    /// Open the wipe ToolDialog with device pre-filled.
    fn open_wipe_dialog_for_device(&mut self, device: &str) {
        let parameters = Self::get_tool_parameters("wipe_disk");
        let mut param_values: Vec<String> = parameters
            .iter()
            .map(|p| match &p.param_type {
                ToolParameter::Selection(options, default_idx) => {
                    options.get(*default_idx).cloned().unwrap_or_default()
                }
                ToolParameter::Boolean(v) => v.to_string(),
                _ => String::new(),
            })
            .collect();
        param_values[0] = device.to_string();

        let mut state = self.lock_state();
        state.pre_dialog_mode = Some(AppMode::DiskTools);
        state.current_tool = Some("wipe_disk".to_string());
        state.tool_dialog = Some(ToolDialogState {
            tool_name: "wipe_disk".to_string(),
            parameters,
            current_param: 1, // Skip device, start at method
            param_values,
            is_executing: false,
        });
        state.mode = AppMode::ToolDialog;
        state.status_message = "Select wipe method for disk".to_string();
    }

    /// Open a partition sub-tool ToolDialog with device pre-filled.
    fn open_partition_tool_dialog(&mut self, tool_name: &str, device: &str) {
        let parameters = Self::get_tool_parameters(tool_name);
        let mut param_values: Vec<String> = parameters
            .iter()
            .map(|p| match &p.param_type {
                ToolParameter::Selection(options, default_idx) => {
                    options.get(*default_idx).cloned().unwrap_or_default()
                }
                _ => String::new(),
            })
            .collect();
        param_values[0] = device.to_string();

        let mut state = self.lock_state();
        state.pre_dialog_mode = Some(AppMode::DiskTools);
        state.current_tool = Some(tool_name.to_string());
        state.tool_dialog = Some(ToolDialogState {
            tool_name: tool_name.to_string(),
            parameters,
            current_param: 1, // Skip device, start at next param
            param_values,
            is_executing: false,
        });
        state.mode = AppMode::ToolDialog;
        state.status_message = format!("Configure parameters for {}", tool_name);
    }

    /// Execute a tool via ScriptArgs: set up floating output and spawn script.
    ///
    /// If `is_destructive` is true and `skip_confirm` is false, shows a confirmation
    /// dialog instead of executing immediately.
    fn execute_via_script_args(
        &mut self,
        script_name: &'static str,
        cli_args: Vec<String>,
        env_vars: Vec<(String, String)>,
        display_name: &str,
        is_destructive: bool,
        skip_confirm: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Shell-safety check: reject args with dangerous characters
        for arg in &cli_args {
            if !shell_safe(arg) {
                let mut state = self.lock_state();
                state.status_message = "Error: unsafe characters in argument".to_string();
                return Ok(());
            }
        }

        // Dry-run check: show what WOULD execute without actually running
        if is_dry_run() {
            let script_path = crate::script_runner::scripts_base_dir()
                .join("tools")
                .join(script_name)
                .to_string_lossy()
                .to_string();
            let env_display: Vec<String> = env_vars.iter().map(|(k,v)| format!("{}={}", k, v)).collect();
            let env_prefix = if env_display.is_empty() { String::new() } else { format!("{} ", env_display.join(" ")) };
            let would_execute = format!("{}bash {} {}", env_prefix, script_path, cli_args.join(" "));

            let mut state = self.lock_state();
            state.tool_dialog = None;
            state.floating_output = Some(FloatingOutputState {
                title: format!("[DRY RUN] {}", display_name),
                content: vec![
                    "[DRY RUN] Would execute:".to_string(),
                    would_execute,
                    String::new(),
                    "No changes were made.".to_string(),
                    String::new(),
                    "Press Esc or Enter to close".to_string(),
                ],
                scroll_offset: 0,
                auto_scroll: false,
                complete: true,
                progress: None,
                status: "Dry run complete".to_string(),
            });
            state.pre_dialog_mode = Some(state.mode.clone());
            state.mode = AppMode::FloatingOutput;
            return Ok(());
        }

        // If destructive and not already confirmed, show confirmation dialog
        if is_destructive && !skip_confirm {
            let severity = match script_name {
                "wipe_disk.sh" | "encrypt_device.sh" => ConfirmSeverity::Danger,
                "format_partition.sh" => ConfirmSeverity::Warning,
                _ => ConfirmSeverity::Warning,
            };

            let pending = serde_json::json!({
                "script_name": script_name,
                "cli_args": cli_args,
                "env_vars": env_vars,
                "display_name": display_name,
            });

            let title = format!("Confirm: {}", display_name);
            let message = format!("This will execute {} which modifies system state. Continue?", display_name);
            let detail1 = format!("Script: scripts/tools/{}", script_name);
            let detail2 = format!("Args: {}", cli_args.join(" "));
            let action_data = pending.to_string();

            let mut state = self.lock_state();
            // Use the ToolDialog's parent menu as the return target, not ToolDialog itself
            // (tool_dialog is cleared below, so returning to ToolDialog mode leaves a ghost state)
            let return_mode = state.pre_dialog_mode.take()
                .unwrap_or_else(|| state.mode.clone());
            state.pre_dialog_mode = Some(return_mode);
            state.tool_dialog = None;
            state.current_tool = None;
            state.confirm_dialog = Some(
                ConfirmDialogState::new(&title, &message, severity, "execute_tool")
                    .with_detail(&detail1)
                    .with_detail(&detail2)
                    .with_action_data(&action_data),
            );
            state.mode = AppMode::ConfirmDialog;
            return Ok(());
        }

        let script_path = crate::script_runner::scripts_base_dir()
            .join("tools")
            .join(script_name)
            .to_string_lossy()
            .to_string();

        // Manifest validation — destructive scripts are blocked without a manifest
        let manifest_key = format!("scripts/tools/{}", script_name);
        if let Some(manifest) = self.manifest_registry.get(&manifest_key) {
            tracing::debug!("Manifest found for {}: v{}", script_name, manifest.version);
        } else if is_destructive {
            tracing::error!("BLOCKED: No manifest found for destructive script: {}", script_name);
            let mut state = self.lock_state();
            state.status_message = format!("Error: no manifest for destructive script {}", script_name);
            return Ok(());
        } else {
            tracing::warn!("No manifest found for script: {} (non-destructive, allowing)", script_name);
        }

        // Set up floating output window
        {
            let mut state = self.lock_state();
            state.tool_dialog = None;
            state.floating_output = Some(FloatingOutputState {
                title: format!("Running: {}", display_name),
                content: vec![
                    format!("Executing: {} {}", script_path, cli_args.join(" ")),
                    String::new(),
                ],
                scroll_offset: 0,
                auto_scroll: true,
                complete: false,
                progress: None,
                status: "Running...".to_string(),
            });
            if state.pre_dialog_mode.is_none() {
                state.pre_dialog_mode = Some(state.mode.clone());
            }
            state.mode = AppMode::FloatingOutput;
            state.current_tool = Some(display_name.to_string());
        }

        // Spawn the tool in a background thread
        self.spawn_tool_script_with_env(&script_path, cli_args, env_vars)?;

        Ok(())
    }

    // =========================================================================
    // SECTION: Tool Execution
    // =========================================================================
    //
    // Tool execution flow:
    //
    // 1. User selects tool from menu → execute_tool()
    // 2. If tool needs params → show ToolDialog
    // 3. User fills params → handle_tool_dialog_enter()
    // 4. Params validated → execute_tool_with_params()
    // 5. Params mapped to CLI args → spawn_tool_script()
    // 6. Script runs → output streams to FloatingOutput
    // 7. Completion message → user dismisses with Enter
    //
    // =========================================================================

    /// Validate that a required parameter is present and non-empty.
    fn validate_required_param(params: &[String], index: usize, name: &str) -> Result<String, String> {
        let val = params.get(index).cloned().unwrap_or_default();
        if val.trim().is_empty() {
            return Err(format!("Required parameter '{}' is empty", name));
        }
        Ok(val)
    }

    /// Execute a tool with the collected parameters using ScriptArgs for type safety.
    pub fn execute_tool_with_params(
        &mut self,
        tool_name: &str,
        params: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Interactive tools: launch in embedded terminal
        match tool_name {
            "chroot" => {
                let sa = ChrootArgs {
                    root: PathBuf::from(if !params.is_empty() && !params[0].is_empty() { &params[0] } else { "/mnt" }),
                    no_mount: params.len() >= 2 && params[1] == "true",
                };
                let script_path = crate::script_runner::scripts_base_dir()
                    .join("tools")
                    .join(sa.script_name())
                    .to_string_lossy()
                    .to_string();
                let cli_args = sa.to_cli_args();
                { let mut state = self.lock_state();
                    state.tool_dialog = None;
                    state.current_tool = None;
                }
                let full_cmd = format!("{} {}", script_path, cli_args.iter().map(|a| format!("'{}'", a)).collect::<Vec<_>>().join(" "));
                let _ = self.launch_embedded_tool("bash", &["-c", &full_cmd], tool_name, AppMode::SystemTools);
                return Ok(());
            }
            "manual_partition" => {
                // cfdisk interactive mode (launched from action menu)
                let device = if !params.is_empty() && !params[0].is_empty() {
                    params[0].clone()
                } else {
                    let state = self.lock_state();
                    state.pending_tool_device.clone().unwrap_or_default()
                };
                if device.is_empty() {
                    let mut state = self.lock_state();
                    state.status_message = "No device selected for partitioning".to_string();
                    return Ok(());
                }
                let sa = ManualPartitionArgs {
                    device: PathBuf::from(&device),
                };
                let script_path = crate::script_runner::scripts_base_dir()
                    .join("tools")
                    .join(sa.script_name())
                    .to_string_lossy()
                    .to_string();
                let cli_args = sa.to_cli_args();
                let env_vars = sa.get_env_vars();
                let env_prefix: String = env_vars.iter().map(|(k, v)| format!("{}='{}' ", k, v)).collect();
                { let mut state = self.lock_state();
                    state.tool_dialog = None;
                    state.current_tool = Some("manual_partition".to_string());
                }
                let full_cmd = format!("{}{} {}", env_prefix, script_path, cli_args.iter().map(|a| format!("'{}'", a)).collect::<Vec<_>>().join(" "));
                let _ = self.launch_embedded_tool("bash", &["-c", &full_cmd], "manual_partition", AppMode::DiskTools);
                return Ok(());
            }
            "partition_action_menu" => {
                // Route the selected action to the appropriate sub-tool
                let action = params.first().cloned().unwrap_or_default();
                let device = {
                    let state = self.lock_state();
                    state.pending_tool_device.clone().unwrap_or_default()
                };
                if device.is_empty() {
                    let mut state = self.lock_state();
                    state.tool_dialog = None;
                    state.current_tool = None;
                    state.mode = AppMode::DiskTools;
                    state.status_message = "No device selected".to_string();
                    return Ok(());
                }
                // Clear the action menu dialog first
                { let mut state = self.lock_state();
                    state.tool_dialog = None;
                }
                match action.as_str() {
                    "Create Partition Table" => {
                        self.open_partition_tool_dialog("partition_create_table", &device);
                    }
                    "Add Partition" => {
                        self.open_partition_tool_dialog("partition_add", &device);
                    }
                    "Delete Partition" => {
                        self.open_partition_tool_dialog("partition_delete", &device);
                    }
                    "Advanced Editor (cfdisk)" => {
                        self.execute_tool_with_params("manual_partition", vec![device])?;
                    }
                    "Back to Disk Selection" => {
                        let mut state = self.lock_state();
                        state.pending_tool_device = None;
                        state.current_tool = Some("manual_partition".to_string());
                        drop(state);
                        self.input_handler.start_disk_selection("".to_string());
                        let mut state = self.lock_state();
                        state.status_message = "Select disk to partition (Enter to select, Esc to cancel)".to_string();
                    }
                    _ => {
                        let mut state = self.lock_state();
                        state.mode = AppMode::DiskTools;
                        state.status_message = "Unknown action selected".to_string();
                    }
                }
                return Ok(());
            }
            _ => {}
        }

        // Non-interactive tools: construct ScriptArgs and execute via helper
        match tool_name {
            "format_partition" | "Format Partition" => {
                let device = match Self::validate_required_param(&params, 0, "device") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let fs: crate::types::Filesystem = params.get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(crate::types::Filesystem::Ext4);
                let sa = FormatPartitionArgs {
                    device: PathBuf::from(device),
                    filesystem: fs,
                    label: params.get(2).filter(|s| !s.is_empty()).cloned(),
                    force: false,
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "format partition", sa.is_destructive(), false,
                )
            }
            "wipe_disk" | "Wipe Disk" => {
                let device = match Self::validate_required_param(&params, 0, "device") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let method: WipeMethod = params.get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(WipeMethod::Quick);
                // User must type "CONFIRM" — reject anything else
                let confirm_text = params.get(2).cloned().unwrap_or_default();
                if confirm_text != "CONFIRM" {
                    let mut state = self.lock_state();
                    state.tool_dialog = None;
                    state.current_tool = None;
                    state.mode = state.pre_dialog_mode.take().unwrap_or(AppMode::DiskTools);
                    state.status_message = "Wipe cancelled — you must type CONFIRM exactly".to_string();
                    return Ok(());
                }
                let sa = WipeDiskArgs {
                    device: PathBuf::from(device),
                    method,
                    confirm: true, // Validated by typed CONFIRM above
                };
                // skip_confirm=true: the typed CONFIRM IS the confirmation
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "wipe disk", sa.is_destructive(), true,
                )
            }
            "partition_create_table" => {
                let device = match Self::validate_required_param(&params, 0, "device") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let table_type: crate::scripts::disk::TableType = params.get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(crate::scripts::disk::TableType::Gpt);
                let confirm_text = params.get(2).cloned().unwrap_or_default();
                if confirm_text != "CONFIRM" {
                    let mut state = self.lock_state();
                    state.tool_dialog = None;
                    state.current_tool = Some("manual_partition".to_string());
                    state.mode = AppMode::DiskTools;
                    state.status_message = "Create table cancelled — you must type CONFIRM exactly".to_string();
                    return Ok(());
                }
                let sa = CreateTableArgs {
                    device: PathBuf::from(&device),
                    table_type,
                    confirm: true,
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "create partition table", sa.is_destructive(), true,
                )
            }
            "partition_add" => {
                let device = match Self::validate_required_param(&params, 0, "device") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let number: u8 = params.get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1);
                let size = params.get(2).cloned().unwrap_or_else(|| "remaining".to_string());
                let partition_type: crate::scripts::disk::PartitionType = params.get(3)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(crate::scripts::disk::PartitionType::Linux);
                let label = params.get(4).filter(|s| !s.is_empty()).cloned();
                let confirm_text = params.get(5).cloned().unwrap_or_default();
                if confirm_text != "CONFIRM" {
                    let mut state = self.lock_state();
                    state.tool_dialog = None;
                    state.current_tool = Some("manual_partition".to_string());
                    state.mode = AppMode::DiskTools;
                    state.status_message = "Add partition cancelled — you must type CONFIRM exactly".to_string();
                    return Ok(());
                }
                let sa = AddPartitionArgs {
                    device: PathBuf::from(&device),
                    number,
                    size,
                    partition_type,
                    label,
                    confirm: true,
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "add partition", sa.is_destructive(), true,
                )
            }
            "partition_delete" => {
                let device = match Self::validate_required_param(&params, 0, "device") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let number: u8 = params.get(1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(1);
                let confirm_text = params.get(2).cloned().unwrap_or_default();
                if confirm_text != "CONFIRM" {
                    let mut state = self.lock_state();
                    state.tool_dialog = None;
                    state.current_tool = Some("manual_partition".to_string());
                    state.mode = AppMode::DiskTools;
                    state.status_message = "Delete partition cancelled — you must type CONFIRM exactly".to_string();
                    return Ok(());
                }
                let sa = DeletePartitionArgs {
                    device: PathBuf::from(&device),
                    number,
                    confirm: true,
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "delete partition", sa.is_destructive(), true,
                )
            }
            "health" => {
                // Device is handled through disk selection dialog; params[0] is output_level
                let detailed = !params.is_empty() && params[0] == "detailed";
                let mut cli_args = vec![];
                if detailed {
                    cli_args.push("--detailed".to_string());
                }
                self.execute_via_script_args(
                    "check_disk_health.sh", cli_args, vec![],
                    "check disk health", false, true,
                )
            }
            "mount" => {
                let action = params.first().cloned().unwrap_or_default();
                let target = params.get(1).cloned().unwrap_or_default();
                let mut cli_args = vec!["--action".to_string(), action.clone()];
                if action == "mount" {
                    cli_args.push("--device".to_string());
                    cli_args.push(target);
                    if let Some(mp) = params.get(2).filter(|s| !s.is_empty()) {
                        cli_args.push("--mountpoint".to_string());
                        cli_args.push(mp.clone());
                    }
                } else if action == "umount" {
                    if target.starts_with("/dev/") {
                        cli_args.push("--device".to_string());
                    } else {
                        cli_args.push("--mountpoint".to_string());
                    }
                    cli_args.push(target);
                } else {
                    cli_args.push("--device".to_string());
                    cli_args.push(target);
                }
                if params.len() >= 4 && params[3] == "true" {
                    cli_args.push("--readonly".to_string());
                }
                if params.len() >= 5 && params[4] == "true" {
                    cli_args.push("--force".to_string());
                }
                self.execute_via_script_args(
                    "mount_partitions.sh", cli_args, vec![],
                    "mount partitions", false, true,
                )
            }
            "info" => {
                let sa = SystemInfoArgs {
                    detailed: !params.is_empty() && params[0] == "true",
                };
                let mut cli_args = sa.to_cli_args();
                if params.len() >= 2 && params[1] == "true" {
                    cli_args.push("--json".to_string());
                }
                self.execute_via_script_args(
                    sa.script_name(), cli_args, sa.get_env_vars(),
                    "system info", sa.is_destructive(), true,
                )
            }
            "install_bootloader" => {
                let bootloader_type = match Self::validate_required_param(&params, 0, "bootloader type") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let disk = match Self::validate_required_param(&params, 1, "disk") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let sa = BootloaderArgs {
                    bootloader_type,
                    disk: PathBuf::from(disk),
                    efi_path: params.get(2).filter(|s| !s.is_empty()).map(PathBuf::from),
                    mode: params.get(3).filter(|s| !s.is_empty()).cloned().unwrap_or_default(),
                };
                let mut cli_args = sa.to_cli_args();
                if params.len() >= 5 && params[4] == "true" {
                    cli_args.push("--repair".to_string());
                }
                self.execute_via_script_args(
                    sa.script_name(), cli_args, sa.get_env_vars(),
                    "install bootloader", sa.is_destructive(), false,
                )
            }
            "generate_fstab" => {
                let sa = GenFstabArgs {
                    root: PathBuf::from(params.first().cloned().unwrap_or_default()),
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "generate fstab", sa.is_destructive(), false,
                )
            }
            "add_user" | "Add New User" => {
                // params: username, password, full_name, groups, shell, system_user
                let username = match Self::validate_required_param(&params, 0, "username") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let sa = UserAddArgs {
                    username,
                    password: params.get(1).filter(|s| !s.is_empty()).cloned(),
                    full_name: params.get(2).filter(|s| !s.is_empty()).cloned(),
                    groups: params.get(3).filter(|s| !s.is_empty()).cloned(),
                    shell: params.get(4).filter(|s| !s.is_empty()).cloned(),
                    home_dir: None,
                    create_home: true,
                    sudo: params.len() >= 6 && params[5] == "true",
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "add user", sa.is_destructive(), false,
                )
            }
            "reset_password" => {
                let username = match Self::validate_required_param(&params, 0, "username") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let password = match Self::validate_required_param(&params, 1, "password") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let sa = ResetPasswordArgs { username, password };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "reset password", sa.is_destructive(), false,
                )
            }
            "configure_network" => {
                // params: interface, action, config_type, ip, netmask, gateway
                let interface = match Self::validate_required_param(&params, 0, "interface") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let mut cli_args = vec![];
                cli_args.push("--interface".to_string());
                cli_args.push(interface);
                if params.len() >= 2 {
                    cli_args.push("--action".to_string());
                    cli_args.push(params[1].clone());
                }
                if params.len() >= 3 && !params[2].is_empty() {
                    match params[2].as_str() {
                        "dhcp" => cli_args.push("--dhcp".to_string()),
                        "static" => cli_args.push("--static".to_string()),
                        _ => {}
                    }
                }
                if params.len() >= 4 && !params[3].is_empty() {
                    cli_args.push("--ip".to_string());
                    cli_args.push(params[3].clone());
                }
                if params.len() >= 5 && !params[4].is_empty() {
                    cli_args.push("--netmask".to_string());
                    cli_args.push(params[4].clone());
                }
                if params.len() >= 6 && !params[5].is_empty() {
                    cli_args.push("--gateway".to_string());
                    cli_args.push(params[5].clone());
                }
                self.execute_via_script_args(
                    "configure_network.sh", cli_args, vec![],
                    "configure network", true, false,
                )
            }
            "manage_services" => {
                let sa = ServicesArgs {
                    action: params.first().cloned().unwrap_or_default(),
                    service: params.get(1).filter(|s| !s.is_empty()).cloned(),
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "manage services", sa.is_destructive(), false,
                )
            }
            "manage_groups" => {
                let action = match Self::validate_required_param(&params, 0, "action") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let sa = GroupsArgs {
                    action,
                    user: params.get(1).filter(|s| !s.is_empty()).cloned(),
                    group: params.get(2).filter(|s| !s.is_empty()).cloned(),
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "manage groups", sa.is_destructive(), false,
                )
            }
            "configure_ssh" => {
                let sa = SshArgs {
                    action: params.first().cloned().unwrap_or_default(),
                    port: None,
                    enable_root_login: None,
                    enable_password_auth: None,
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "configure SSH", sa.is_destructive(), false,
                )
            }
            "configure_firewall" => {
                let sa = crate::scripts::network::FirewallArgs {
                    action: params.first().cloned().unwrap_or_default(),
                    firewall_type: "iptables".to_string(),
                    port: None,
                    protocol: "tcp".to_string(),
                    allow: false,
                    deny: false,
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "configure firewall", sa.is_destructive(), false,
                )
            }
            "security_audit" => {
                // params: action
                let sa = SecurityAuditArgs {
                    action: params.first().cloned().unwrap_or_else(|| "basic".to_string()),
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "security audit", sa.is_destructive(), true,
                )
            }
            "test_network" => {
                // params: action, timeout
                let sa = TestNetworkArgs {
                    action: params.first().cloned().unwrap_or_else(|| "full".to_string()),
                    host: None,
                    timeout: params.get(1).and_then(|s| s.parse().ok()).unwrap_or(5),
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "test network", sa.is_destructive(), true,
                )
            }
            "network_diagnostics" => {
                // params: action
                let sa = NetworkDiagnosticsArgs {
                    action: params.first().cloned().unwrap_or_else(|| "info".to_string()),
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "network diagnostics", sa.is_destructive(), true,
                )
            }
            // === New tools (Phase 3) ===
            "encrypt_device" => {
                // params: action, device, password, mapper_name
                let action = params.first().cloned().unwrap_or_default();
                let device = match Self::validate_required_param(&params, 1, "device") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let password = params.get(2).cloned().unwrap_or_default();
                let mapper_name = params.get(3).filter(|s| !s.is_empty()).cloned().unwrap_or_else(|| "cryptroot".to_string());

                match action.as_str() {
                    "format" => {
                        // Create SecretFile for the password
                        let secret = SecretFile::new(&password)
                            .map_err(|e| anyhow::anyhow!("Failed to create temporary keyfile for LUKS format: {}", e))?;
                        let key_file_path = secret.path().to_path_buf();
                        // Store SecretFile on App to keep it alive during execution
                        self._active_secret_file = Some(secret);

                        let sa = LuksFormatArgs {
                            device: PathBuf::from(&device),
                            cipher: LuksCipher::default(),
                            key_file: key_file_path,
                            label: None,
                            confirm: true,
                        };
                        self.execute_via_script_args(
                            sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                            "LUKS format", sa.is_destructive(), false,
                        )
                    }
                    "open" => {
                        let secret = SecretFile::new(&password)
                            .map_err(|e| anyhow::anyhow!("Failed to create temporary keyfile for LUKS open: {}", e))?;
                        let key_file_path = secret.path().to_path_buf();
                        self._active_secret_file = Some(secret);

                        let sa = LuksOpenArgs {
                            device: PathBuf::from(&device),
                            mapper_name,
                            key_file: key_file_path,
                        };
                        self.execute_via_script_args(
                            sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                            "LUKS open", sa.is_destructive(), true,
                        )
                    }
                    "close" => {
                        let sa = LuksCloseArgs { mapper_name };
                        self.execute_via_script_args(
                            sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                            "LUKS close", sa.is_destructive(), true,
                        )
                    }
                    _ => Err(format!("Unknown encrypt action: {}", action).into()),
                }
            }
            "enable_services" => {
                // params: services (index 0), root (index 1) — matches dialog definition order
                let sa = EnableServicesArgs {
                    services: params.first().map(|s| s.split(',').map(|v| v.trim().to_string()).collect()).unwrap_or_default(),
                    root: PathBuf::from(params.get(1).filter(|s| !s.is_empty()).cloned().unwrap_or_else(|| "/mnt".to_string())),
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "enable services", sa.is_destructive(), false,
                )
            }
            "rebuild_initramfs" => {
                // params: root
                let root = params.first()
                    .filter(|s| !s.is_empty())
                    .cloned()
                    .unwrap_or_else(|| "/mnt".to_string());
                self.execute_via_script_args(
                    "rebuild_initramfs.sh",
                    vec!["--root".to_string(), root],
                    vec![],
                    "rebuild initramfs",
                    true,
                    false,
                )
            }
            "install_aur_helper" => {
                // params: helper, user, root
                let helper: AurHelper = params.first()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(AurHelper::None);
                let target_user = match Self::validate_required_param(&params, 1, "target user") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let sa = InstallAurHelperArgs {
                    helper,
                    target_user,
                    chroot_path: PathBuf::from(params.get(2).filter(|s| !s.is_empty()).cloned().unwrap_or_else(|| "/mnt".to_string())),
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "install AUR helper", sa.is_destructive(), false,
                )
            }
            "install_dotfiles" => {
                // params: repo_url, target_user, branch, backup
                let repo_url = match Self::validate_required_param(&params, 0, "repository URL") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let target_user = match Self::validate_required_param(&params, 1, "target user") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let sa = InstallDotfilesArgs {
                    repo_url,
                    target_user,
                    target_dir: None,
                    branch: params.get(2).filter(|s| !s.is_empty()).cloned(),
                    backup: params.get(3).map(|s| s == "true").unwrap_or(true),
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "install dotfiles", sa.is_destructive(), false,
                )
            }
            "run_as_user" => {
                // params: user, command, root, workdir
                let user = match Self::validate_required_param(&params, 0, "user") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let command = match Self::validate_required_param(&params, 1, "command") {
                    Ok(v) => v,
                    Err(e) => {
                        let mut state = self.lock_state();
                        state.status_message = e;
                        return Ok(());
                    }
                };
                let sa = UserRunArgs {
                    user,
                    command,
                    chroot_path: PathBuf::from(params.get(2).filter(|s| !s.is_empty()).cloned().unwrap_or_else(|| "/mnt".to_string())),
                    workdir: params.get(3).filter(|s| !s.is_empty()).map(PathBuf::from),
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "run as user", sa.is_destructive(), false,
                )
            }
            "update_mirrors" => {
                // params: country, limit, sort
                let limit: u32 = params.get(1).and_then(|s| s.parse().ok()).unwrap_or(20);
                let sort = match params.get(2).map(|s| s.as_str()) {
                    Some("age") => MirrorSortMethod::Age,
                    Some("score") => MirrorSortMethod::Score,
                    _ => MirrorSortMethod::Rate,
                };
                let sa = UpdateMirrorsArgs {
                    country: params.first().filter(|s| !s.is_empty()).cloned(),
                    limit,
                    sort,
                    protocol: Some("https".to_string()),
                    save: true,
                };
                self.execute_via_script_args(
                    sa.script_name(), sa.to_cli_args(), sa.get_env_vars(),
                    "update mirrors", sa.is_destructive(), false,
                )
            }
            _ => {
                Err(format!("Unknown tool: {}", tool_name).into())
            }
        }
    }
}
