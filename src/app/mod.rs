//! Application module
//!
//! Contains the main application logic, state management, and event handling.
//!
//! # Module Structure
//! - `state` - Application state types (AppState, AppMode, ToolDialogState, etc.)
//! - Main module - App struct and event loop

mod state;

// Re-export state types for external use
pub use state::{AppMode, AppState, ToolDialogState, ToolParam, ToolParameter};

use crate::components::confirm_dialog::{
    format_partition_confirm, start_install_confirm, wipe_disk_confirm,
};
use crate::components::floating_window::FloatingOutputState;
use crate::components::keybindings::KeybindingContext;
use crate::components::pty_terminal::{PtyTerminal, PtyTerminalState};
use crate::config::Configuration;
use crate::error;
use crate::input::InputHandler;
use crate::installer::Installer;
use crate::process_guard::{ChildRegistry, CommandProcessGroup, ProcessGuard};
use crate::ui::UiRenderer;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use log::{debug, info};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{BufRead, BufReader};
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
}

impl App {
    /// Helper function to safely lock the state mutex
    fn lock_state(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, AppState>, Box<dyn std::error::Error>> {
        self.state
            .lock()
            .map_err(|e| error::general_error(format!("Mutex poisoned: {}", e)).into())
    }

    /// Helper function to safely lock the state mutex mutably
    fn lock_state_mut(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, AppState>, Box<dyn std::error::Error>> {
        self.state
            .lock()
            .map_err(|e| error::general_error(format!("Mutex poisoned: {}", e)).into())
    }

    /// Create a new application instance
    pub fn new(save_config_path: Option<std::path::PathBuf>) -> Self {
        info!("Creating new App instance");
        let (tool_tx, tool_rx) = mpsc::channel();

        // ProcessGuard ensures all child processes are killed when App is dropped
        // This prevents orphaned bash scripts continuing after TUI crash
        let process_guard = ProcessGuard::new();
        debug!("ProcessGuard initialized for child process tracking");

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
        }
    }

    /// Get reference to keybinding context
    #[allow(dead_code)] // API method available for future use
    pub fn keybinding_context(&self) -> &KeybindingContext {
        &self.keybinding_context
    }

    /// Toggle help overlay visibility
    pub fn toggle_help(&self) {
        if let Ok(mut state) = self.lock_state_mut() {
            state.help_visible = !state.help_visible;
        }
    }

    /// Load a configuration file and start installation
    fn load_config_file(&mut self, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
        use crate::config_file::InstallationConfig;

        // Clear file browser state first
        {
            let mut state = self.lock_state_mut()?;
            state.file_browser = None;
        }

        // Load and validate the config file
        match InstallationConfig::load_from_file(path) {
            Ok(config) => {
                match config.validate() {
                    Ok(_) => {
                        // Config is valid - start installation
                        let mut state = self.lock_state_mut()?;
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
                        state.mode = AppMode::FloatingOutput;
                    }
                    Err(e) => {
                        let mut state = self.lock_state_mut()?;
                        state.mode = AppMode::AutomatedInstall;
                        state.status_message = format!("Config validation failed: {}", e);
                    }
                }
            }
            Err(e) => {
                let mut state = self.lock_state_mut()?;
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

                let mut state = self.lock_state_mut()?;
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
                log::warn!("PTY fallback: {}", reason);
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

        // Run the command
        let status = Command::new(cmd).args(args).status();

        // Return to alternate screen
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::EnterAlternateScreen
        )?;

        // Check status and return to appropriate mode
        match status {
            Ok(exit_status) => {
                let mut state = self.lock_state_mut()?;
                if exit_status.success() {
                    state.status_message = format!("{} completed successfully", cmd);
                } else {
                    state.status_message = format!("{} exited with error", cmd);
                }
                state.mode = return_mode;
            }
            Err(e) => {
                let mut state = self.lock_state_mut()?;
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

        // Return to previous mode
        let mut state = self.lock_state_mut()?;
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
    fn poll_tool_messages(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Process all pending messages without blocking
        while let Ok(msg) = self.tool_rx.try_recv() {
            let mut state = self.lock_state_mut()?;

            match msg {
                ToolMessage::Stdout(line) => {
                    if let Some(ref mut floating) = state.floating_output {
                        floating.append_line(line);
                        // Auto-scroll to bottom if enabled
                        if floating.auto_scroll {
                            floating.scroll_offset = floating.content.len().saturating_sub(1);
                        }
                    }
                }
                ToolMessage::Stderr(line) => {
                    if let Some(ref mut floating) = state.floating_output {
                        floating.append_line(format!("⚠ {}", line));
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
                        format!("Tool failed with exit code: {}", exit_code.unwrap_or(-1))
                    };
                    state.status_message = status_msg.clone();
                    state.current_tool = None;

                    // Now update floating output
                    if let Some(ref mut floating) = state.floating_output {
                        floating.append_line(String::new());
                        if success {
                            floating.append_line("✅ Tool completed successfully".to_string());
                        } else {
                            floating.append_line(format!(
                                "❌ Tool failed with exit code: {}",
                                exit_code.unwrap_or(-1)
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

                    if let Some(ref mut floating) = state.floating_output {
                        floating.append_line(format!("❌ Error: {}", err));
                        floating.append_line(String::new());
                        floating.append_line("Press Esc or Enter to close".to_string());
                        floating.mark_complete();
                    }
                }
            }
        }
        Ok(())
    }

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

            // Check if installation is complete
            {
                let state = self
                    .state
                    .lock()
                    .map_err(|e| error::general_error(format!("Mutex poisoned: {}", e)))?;
                if state.mode == AppMode::Complete {
                    break;
                }
            }

            // Render UI
            terminal.draw(|f| {
                let mut state = match self.state.lock() {
                    Ok(state) => state,
                    Err(_) => {
                        // If mutex is poisoned, we can't continue safely
                        eprintln!("Fatal error: Mutex poisoned, cannot continue");
                        std::process::exit(1);
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
                self.ui_renderer
                    .render_with_context(f, &state, &mut self.input_handler, &self.keybinding_context, self.pty_terminal.as_mut());
            })?;
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
            if let Ok(state) = self.lock_state() {
                (state.mode.clone(), state.help_visible)
            } else {
                return Ok(false);
            }
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
                        "format_partition" => {
                            // Show confirmation dialog before formatting
                            let mut state = self.lock_state_mut()?;
                            state.pre_dialog_mode = Some(AppMode::DiskTools);
                            state.confirm_dialog =
                                Some(format_partition_confirm(&value, "ext4"));
                            state.mode = AppMode::ConfirmDialog;
                            return Ok(false);
                        }
                        "wipe_disk" => {
                            // Show confirmation dialog before wiping
                            let mut state = self.lock_state_mut()?;
                            state.pre_dialog_mode = Some(AppMode::DiskTools);
                            state.confirm_dialog = Some(wipe_disk_confirm(&value));
                            state.mode = AppMode::ConfirmDialog;
                            return Ok(false);
                        }
                        _ => {}
                    }
                }

                // User confirmed input, update configuration
                self.update_configuration_value(value)?;
            }
            return Ok(false);
        }

        // Handle floating output mode
        if current_mode == AppMode::FloatingOutput {
            match key_event.code {
                KeyCode::Esc | KeyCode::Enter | KeyCode::Char('b') | KeyCode::Char('B') => {
                    // Dismiss floating output and return to previous mode
                    let mut state = self.lock_state_mut()?;
                    if let Some(_output) = state.floating_output.take() {
                        // Return to tools menu or previous mode
                        state.mode = AppMode::ToolsMenu;
                    }
                }
                KeyCode::Up => {
                    let mut state = self.lock_state_mut()?;
                    if let Some(ref mut output) = state.floating_output {
                        if output.scroll_offset > 0 {
                            output.scroll_offset -= 1;
                        }
                    }
                }
                KeyCode::Down => {
                    let mut state = self.lock_state_mut()?;
                    if let Some(ref mut output) = state.floating_output {
                        if output.scroll_offset < output.content.len().saturating_sub(1) {
                            output.scroll_offset += 1;
                        }
                    }
                }
                _ => {}
            }
            return Ok(false);
        }

        // Handle file browser mode
        if current_mode == AppMode::FileBrowser {
            let mut state = self.lock_state_mut()?;
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
            let mut state = self.lock_state_mut()?;
            if let Some(ref mut dialog) = state.confirm_dialog {
                match key_event.code {
                    KeyCode::Left | KeyCode::Right | KeyCode::Tab => {
                        // Toggle between No (0) and Yes (1)
                        let old_selected = dialog.selected;
                        dialog.selected = if dialog.selected == 0 { 1 } else { 0 };
                        log::debug!("ConfirmDialog toggle: {} -> {} (0=No/left, 1=Yes/right)",
                            old_selected, dialog.selected);
                    }
                    KeyCode::Enter => {
                        // SECURITY FIX: Use is_confirmed() method to get correct selection
                        // selected = 0 means No/Cancel (left), selected = 1 means Yes/Confirm (right)
                        let confirmed = dialog.is_confirmed(); // Returns true if selected == 1 (Yes on right)
                        let action = dialog.confirm_action.clone();
                        let data = dialog.action_data.clone();

                        log::info!("ConfirmDialog Enter: selected={}, is_confirmed={}, action={}",
                            dialog.selected, confirmed, action);

                        // Clear dialog and restore previous mode
                        state.confirm_dialog = None;
                        if let Some(prev_mode) = state.pre_dialog_mode.take() {
                            state.mode = prev_mode;
                        }

                        if confirmed {
                            log::info!("Executing confirmed action: {}", action);
                            // Drop the lock before executing action
                            drop(state);
                            self.execute_confirmed_action(&action, data)?;
                        } else {
                            log::info!("Action cancelled, returning to previous mode");
                        }
                    }
                    KeyCode::Esc => {
                        // Cancel - restore previous mode
                        state.confirm_dialog = None;
                        if let Some(prev_mode) = state.pre_dialog_mode.take() {
                            state.mode = prev_mode;
                        }
                    }
                    _ => {}
                }
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
                self.handle_enter()?;
            }
            _ => {}
        }

        Ok(false)
    }

    /// Navigate to previous option
    fn navigate_up(&self) {
        if let Ok(mut state) = self.lock_state_mut() {
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
                _ => {}
            }
        }
    }

    /// Navigate to next option
    fn navigate_down(&self) {
        if let Ok(mut state) = self.lock_state_mut() {
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
                    if state.tools_menu_selection < 5 {
                        // 6 items total (0-5)
                        state.tools_menu_selection += 1;
                    }
                }
                AppMode::SystemTools | AppMode::UserTools => {
                    if state.tools_menu_selection < 5 {
                        // 6 items total (0-5)
                        state.tools_menu_selection += 1;
                    }
                }
                AppMode::NetworkTools => {
                    if state.tools_menu_selection < 4 {
                        // 5 items total (0-4)
                        state.tools_menu_selection += 1;
                    }
                }
                AppMode::ToolDialog => {
                    if let Some(ref mut dialog) = state.tool_dialog {
                        if dialog.current_param < dialog.parameters.len() - 1 {
                            dialog.current_param += 1;
                        }
                    }
                }
                AppMode::GuidedInstaller => {
                    state.config_scroll.move_down();
                }
                _ => {}
            }
        }
    }

    /// Page up in configuration list
    fn page_up(&self) {
        if let Ok(mut state) = self.lock_state_mut() {
            if state.mode == AppMode::GuidedInstaller {
                state.config_scroll.page_up();
            }
        }
    }

    /// Page down in configuration list
    fn page_down(&self) {
        if let Ok(mut state) = self.lock_state_mut() {
            if state.mode == AppMode::GuidedInstaller {
                state.config_scroll.page_down();
            }
        }
    }

    /// Move to first configuration option
    fn move_to_first(&self) {
        if let Ok(mut state) = self.lock_state_mut() {
            if state.mode == AppMode::GuidedInstaller {
                state.config_scroll.move_to_first();
            }
        }
    }

    /// Move to last configuration option
    fn move_to_last(&self) {
        if let Ok(mut state) = self.lock_state_mut() {
            if state.mode == AppMode::GuidedInstaller {
                state.config_scroll.move_to_last();
            }
        }
    }

    /// Handle Enter key press
    fn handle_enter(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let current_mode = {
            let state = self.lock_state()?;
            state.mode.clone()
        };

        match current_mode {
            AppMode::MainMenu => {
                self.handle_main_menu_selection()?;
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
            AppMode::ToolExecution => {
                // Tool execution in progress, no action needed
            }
            AppMode::Installation => {
                // Installation is running, no action needed
            }
            AppMode::Complete => {
                // Installation complete, no action needed
            }
            AppMode::EmbeddedTerminal => {
                // Embedded terminal handles its own input
            }
            AppMode::FloatingOutput => {
                // Dismiss floating output on Enter
                let mut state = self.lock_state_mut()?;
                if let Some(_output) = state.floating_output.take() {
                    state.mode = AppMode::ToolsMenu;
                }
            }
            AppMode::FileBrowser => {
                // File browser handles its own Enter key
            }
            AppMode::ConfirmDialog => {
                // Handle confirmation dialog selection
                self.handle_confirm_dialog_enter()?;
            }
        }

        Ok(())
    }

    /// Handle confirmation dialog Enter key
    fn handle_confirm_dialog_enter(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (confirmed, action, action_data, pre_mode) = {
            let state = self.lock_state()?;
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
            let mut state = self.lock_state_mut()?;
            state.confirm_dialog = None;
            // Return to previous mode
            if let Some(mode) = pre_mode {
                state.mode = mode;
            }
            state.pre_dialog_mode = None;
        }

        if confirmed {
            // Execute the confirmed action
            match action.as_str() {
                "wipe_disk" => {
                    if let Some(disk) = action_data {
                        log::info!("Confirmed: wiping disk {}", disk);
                        // Execute wipe disk operation
                        self.execute_wipe_disk(&disk)?;
                    }
                }
                "format_partition" => {
                    if let Some(partition) = action_data {
                        log::info!("Confirmed: formatting partition {}", partition);
                        // Execute format operation
                    }
                }
                "start_installation" => {
                    log::info!("Confirmed: starting installation");
                    self.start_installation()?;
                }
                _ => {
                    log::warn!("Unknown confirm action: {}", action);
                }
            }
        }

        Ok(())
    }

    /// Execute wipe disk operation
    fn execute_wipe_disk(&mut self, disk: &str) -> Result<(), Box<dyn std::error::Error>> {
        // Show floating output for the operation
        let mut state = self.lock_state_mut()?;
        state.floating_output = Some(FloatingOutputState::new(&format!("Wiping {}", disk)));
        state.floating_output.as_mut().unwrap().append_line(format!("Starting secure wipe of {}...", disk));
        state.floating_output.as_mut().unwrap().append_line("This may take a while depending on disk size.".to_string());
        state.mode = AppMode::FloatingOutput;
        Ok(())
    }

    /// Execute action after confirmation dialog
    fn execute_confirmed_action(
        &mut self,
        action: &str,
        data: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        match action {
            "wipe_disk" => {
                if let Some(disk) = data {
                    self.execute_wipe_disk(&disk)?;
                }
            }
            "format_partition" => {
                if let Some(partition) = data {
                    self.execute_tool_with_device("format_partition.sh", &partition, &[])?;
                }
            }
            "install_bootloader" => {
                if let Some(params) = data {
                    // params format: "bootloader:disk"
                    let parts: Vec<&str> = params.split(':').collect();
                    if parts.len() == 2 {
                        self.execute_tool_with_device(
                            "install_bootloader.sh",
                            parts[1],
                            &["--bootloader", parts[0]],
                        )?;
                    }
                }
            }
            "start_installation" => {
                // Start the installation process
                self.start_installation()?;
            }
            _ => {
                // Unknown action
                let mut state = self.lock_state_mut()?;
                state.status_message = format!("Unknown action: {}", action);
            }
        }
        Ok(())
    }

    /// Handle main menu selection
    fn handle_main_menu_selection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let selection = {
            let state = self.lock_state()?;
            state.main_menu_selection
        };

        debug!("Main menu selection: {}", selection);

        let mut state = self.lock_state_mut()?;
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
                return Ok(());
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle tools menu selection
    fn handle_tools_menu_selection(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let selection = {
            let state = self.lock_state()?;
            state.tools_menu_selection
        };

        let mut state = self.lock_state_mut()?;
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
            let state = self.lock_state()?;
            (state.mode.clone(), state.tools_menu_selection)
        };

        // Check if user selected "Back" option (last item in each menu)
        let is_back_option = match current_mode {
            AppMode::DiskTools => selection == 5, // 6 items (0-5), back is at index 5
            AppMode::SystemTools | AppMode::UserTools => selection == 5, // 6 items (0-5), back is at index 5
            AppMode::NetworkTools => selection == 4, // 5 items (0-4), back is at index 4
            _ => false,
        };

        if is_back_option {
            // Go back to tools menu
            let mut state = self.lock_state_mut()?;
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
                        // Partition Disk (cfdisk) - Launch in embedded terminal
                        let _ = self.launch_embedded_tool("cfdisk", &[], "cfdisk", AppMode::DiskTools);
                    }
                    1 => {
                        // Format Partition - Use disk selection dialog
                        self.input_handler.start_disk_selection("".to_string());
                        let mut state = self.lock_state_mut()?;
                        state.current_tool = Some("format_partition".to_string());
                        state.status_message =
                            "Select partition to format (Enter to select, Esc to cancel)"
                                .to_string();
                    }
                    2 => {
                        // Wipe Disk - Use disk selection dialog
                        self.input_handler.start_disk_selection("".to_string());
                        let mut state = self.lock_state_mut()?;
                        state.current_tool = Some("wipe_disk".to_string());
                        state.status_message =
                            "Select disk to wipe (Enter to select, Esc to cancel)".to_string();
                    }
                    3 => {
                        // Check Disk Health - Use disk selection dialog
                        self.input_handler.start_disk_selection("".to_string());
                        let mut state = self.lock_state_mut()?;
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
                        // Back to Tools Menu
                        let mut state = self.lock_state_mut()?;
                        state.mode = AppMode::ToolsMenu;
                        state.tools_menu_selection = 0;
                        state.status_message =
                            "Arch Linux Tools - System repair and administration".to_string();
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
                        // Enable/Disable Services
                        let mut state = self.lock_state_mut()?;
                        state.current_tool = Some("manage_services".to_string());
                        state.status_message = "Service management tool...".to_string();
                    }
                    4 => {
                        // System Information - Simple tool with no parameters
                        {
                            let mut state = self.lock_state_mut()?;
                            state.current_tool = Some("system_info".to_string());
                            state.status_message = "Gathering system information...".to_string();
                        }

                        // Execute system info tool directly
                        if let Err(e) = self.execute_simple_tool("system_info.sh", &["--detailed"])
                        {
                            eprintln!("Failed to execute system info tool: {}", e);
                            let mut state = self.lock_state_mut()?;
                            state.status_message = "System info tool failed".to_string();
                        }
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
                        // Manage User Groups
                        let mut state = self.lock_state_mut()?;
                        state.current_tool = Some("manage_groups".to_string());
                        state.status_message = "User group management tool...".to_string();
                    }
                    3 => {
                        // Configure SSH
                        let mut state = self.lock_state_mut()?;
                        state.current_tool = Some("configure_ssh".to_string());
                        state.status_message = "SSH configuration tool...".to_string();
                    }
                    4 => {
                        // Security Audit
                        let mut state = self.lock_state_mut()?;
                        state.current_tool = Some("security_audit".to_string());
                        state.status_message = "Security audit tool...".to_string();
                    }
                    _ => {}
                }
            }
            AppMode::NetworkTools => {
                match selection {
                    0 => {
                        // Configure Network Interface - Create dialog
                        self.create_tool_dialog("configure")?;
                    }
                    1 => {
                        // Test Network Connectivity - Simple tool
                        {
                            let mut state = self.lock_state_mut()?;
                            state.current_tool = Some("test_network".to_string());
                            state.status_message = "Testing network connectivity...".to_string();
                        }

                        // Execute network test tool directly
                        if let Err(e) =
                            self.execute_simple_tool("test_network.sh", &["--action", "full"])
                        {
                            eprintln!("Failed to execute network test tool: {}", e);
                            let mut state = self.lock_state_mut()?;
                            state.status_message = "Network test tool failed".to_string();
                        }
                    }
                    2 => {
                        // Configure Firewall
                        let mut state = self.lock_state_mut()?;
                        state.current_tool = Some("configure_firewall".to_string());
                        state.status_message = "Firewall configuration tool...".to_string();
                    }
                    3 => {
                        // Network Diagnostics
                        let mut state = self.lock_state_mut()?;
                        state.current_tool = Some("network_diagnostics".to_string());
                        state.status_message = "Network diagnostics tool...".to_string();
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
        let (should_open_input, should_start_installation) = {
            let state = self.lock_state()?;
            // Check if we're on the green button (one step past the last config option)
            if state.config_scroll.selected_index == state.config.options.len() {
                (false, true) // Start installation
            } else {
                (true, false) // Open input dialog
            }
        };

        // Open input dialog if needed
        if should_open_input {
            self.open_input_dialog()?;
        }

        // Start installation if needed - show confirmation dialog first
        if should_start_installation {
            if self.validate_configuration_for_installation() {
                // Show confirmation dialog before starting
                let mut state = self.lock_state_mut()?;
                state.pre_dialog_mode = Some(AppMode::GuidedInstaller);
                state.confirm_dialog = Some(start_install_confirm());
                state.mode = AppMode::ConfirmDialog;
            } else {
                // Validation failed - status message already set in validate_configuration_for_installation
                // User will see the error message
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

        let mut state = self.lock_state_mut()?;
        state.file_browser = Some(file_browser);
        state.mode = AppMode::FileBrowser;
        state.status_message = "Select a configuration file (.toml or .json)".to_string();
        Ok(())
    }

    /// Handle back key navigation
    fn handle_back_key(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let current_mode = {
            let state = self.lock_state()?;
            state.mode.clone()
        };

        let mut state = self.lock_state_mut()?;
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
                if let Some(ref tool_name) = state.current_tool {
                    match tool_name.as_str() {
                        "format_partition" | "wipe_disk" | "check_disk_health"
                        | "mount_partitions" | "manual_partition" => {
                            state.mode = AppMode::DiskTools;
                            state.tools_menu_selection = 0;
                            state.status_message = "Disk & Filesystem Tools".to_string();
                        }
                        "install_bootloader" | "generate_fstab" | "chroot_system"
                        | "manage_services" | "system_info" => {
                            state.mode = AppMode::SystemTools;
                            state.tools_menu_selection = 0;
                            state.status_message = "System & Boot Tools".to_string();
                        }
                        "add_user" | "reset_password" | "manage_groups" | "configure_ssh"
                        | "security_audit" => {
                            state.mode = AppMode::UserTools;
                            state.tools_menu_selection = 0;
                            state.status_message = "User & Security Tools".to_string();
                        }
                        "configure_network"
                        | "test_network"
                        | "configure_firewall"
                        | "network_diagnostics" => {
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
            }
            AppMode::ToolExecution => {
                // Go back to the appropriate tool menu (same logic as ToolDialog)
                if let Some(ref tool_name) = state.current_tool {
                    match tool_name.as_str() {
                        "format_partition" | "wipe_disk" | "check_disk_health"
                        | "mount_partitions" | "manual_partition" => {
                            state.mode = AppMode::DiskTools;
                            state.tools_menu_selection = 0;
                            state.status_message = "Disk & Filesystem Tools".to_string();
                        }
                        "install_bootloader" | "generate_fstab" | "chroot_system"
                        | "manage_services" | "system_info" => {
                            state.mode = AppMode::SystemTools;
                            state.tools_menu_selection = 0;
                            state.status_message = "System & Boot Tools".to_string();
                        }
                        "add_user" | "reset_password" | "manage_groups" | "configure_ssh"
                        | "security_audit" => {
                            state.mode = AppMode::UserTools;
                            state.tools_menu_selection = 0;
                            state.status_message = "User & Security Tools".to_string();
                        }
                        "configure_network"
                        | "test_network"
                        | "configure_firewall"
                        | "network_diagnostics" => {
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
                // Clear tool execution state
                state.tool_output.clear();
                state.current_tool = None;
            }
            AppMode::Installation => {
                // During installation, go back to guided installer
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
                // Dismiss floating output and return to tools menu
                if let Some(_output) = state.floating_output.take() {
                    state.mode = AppMode::ToolsMenu;
                    state.tools_menu_selection = 0;
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
        }
        Ok(())
    }

    /// Validate that all required configuration options are set and valid
    fn validate_configuration(&self, config: &Configuration) -> bool {
        // First check basic option validation
        if !config.options.iter().all(|option| option.is_valid()) {
            return false;
        }

        // Then check secure boot requirements
        self.validate_secure_boot_requirements(config)
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
            let state = match self.lock_state() {
                Ok(state) => state,
                Err(_) => return false, // If we can't lock the state, validation fails
            };
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
            let mut state = match self.lock_state_mut() {
                Ok(state) => state,
                Err(_) => return false, // If we can't lock the state, validation fails
            };
            let errors = self.get_validation_errors(&config);

            if errors.len() == 1 {
                state.status_message = format!("❌ Cannot start installation: {}", errors[0]);
            } else {
                state.status_message = format!(
                    "❌ Cannot start installation: {} (and {} more errors)",
                    errors[0],
                    errors.len() - 1
                );
            }
            false
        }
    }

    /// Start the installation process
    fn start_installation(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting installation process");

        // Check if we need to save the config before starting
        if let Some(save_path) = &self.save_config_path {
            info!("Saving configuration to: {:?}", save_path);
            let state = self.lock_state()?;
            let file_config = crate::config_file::InstallationConfig::from(&state.config);
            file_config.save_to_file(save_path)?;

            let mut state_mut = self.lock_state_mut()?;
            state_mut.status_message = format!("✓ Config saved to {}", save_path.display());
            drop(state_mut);

            // Give user a moment to see the save message
            std::thread::sleep(std::time::Duration::from_millis(1000));
        }

        // Update state to installation mode
        {
            let mut state = self.lock_state_mut()?;
            state.mode = AppMode::Installation;
            state.status_message = "Starting installation...".to_string();
        }

        // Create installer with current configuration
        let config = {
            let state = self.lock_state()?;
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
            let state = self.lock_state()?;
            let current_step = state.config_scroll.selected_index;
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
                    let state = match self.lock_state() {
                        Ok(state) => state,
                        Err(_) => return Ok(()), // If we can't lock the state, skip this option
                    };
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Partitioning Strategy")
                        .map(|opt| opt.value.clone())
                        .unwrap_or_default()
                };

                if partitioning_strategy == "manual" {
                    let options = vec!["Yes".to_string(), "No".to_string()];
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else {
                    // Show message that encryption is auto-set for non-manual strategies
                    if let Ok(mut state) = self.lock_state_mut() {
                        state.status_message = "Encryption is auto-set based on partitioning strategy. Use manual partitioning to control encryption.".to_string();
                    }
                }
            }
            "Swap Size" => {
                // Only allow swap size configuration if swap is enabled
                let swap_enabled = {
                    let state = match self.lock_state() {
                        Ok(state) => state,
                        Err(_) => return Ok(()),
                    };
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Swap")
                        .map(|opt| opt.value.to_lowercase() == "yes")
                        .unwrap_or(false)
                };

                if swap_enabled {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else if let Ok(mut state) = self.lock_state_mut() {
                    state.status_message =
                        "Swap size can only be configured when swap is enabled.".to_string();
                }
            }
            "Btrfs Frequency" | "Btrfs Keep Count" | "Btrfs Assistant" => {
                // Only allow btrfs configuration if snapshots are enabled
                let snapshots_enabled = {
                    let state = match self.lock_state() {
                        Ok(state) => state,
                        Err(_) => return Ok(()),
                    };
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Btrfs Snapshots")
                        .map(|opt| opt.value.to_lowercase() == "yes")
                        .unwrap_or(false)
                };

                if snapshots_enabled {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else if let Ok(mut state) = self.lock_state_mut() {
                    state.status_message = format!(
                        "{} can only be configured when Btrfs snapshots are enabled.",
                        option.name
                    );
                }
            }
            "GRUB Theme Selection" => {
                // Only allow theme selection if GRUB themes are enabled
                let themes_enabled = {
                    let state = match self.lock_state() {
                        Ok(state) => state,
                        Err(_) => return Ok(()),
                    };
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "GRUB Theme")
                        .map(|opt| opt.value.to_lowercase() == "yes")
                        .unwrap_or(false)
                };

                if themes_enabled {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else if let Ok(mut state) = self.lock_state_mut() {
                    state.status_message =
                        "GRUB theme selection is only available when GRUB themes are enabled."
                            .to_string();
                }
            }
            "Git Repository URL" => {
                // Only allow URL input if git repository is enabled
                let git_enabled = {
                    let state = match self.lock_state() {
                        Ok(state) => state,
                        Err(_) => return Ok(()),
                    };
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Git Repository")
                        .map(|opt| opt.value.to_lowercase() == "yes")
                        .unwrap_or(false)
                };

                if git_enabled {
                    self.input_handler.start_text_input(
                        option.name.clone(),
                        option.value,
                        "Enter git repository URL".to_string(),
                    );
                } else if let Ok(mut state) = self.lock_state_mut() {
                    state.status_message =
                        "Git repository URL can only be configured when git repository is enabled."
                            .to_string();
                }
            }
            "Disk" => {
                // Check if we need multi-disk selection
                let state = match self.lock_state() {
                    Ok(state) => state,
                    Err(_) => return Ok(()),
                };
                let partitioning_strategy = state
                    .config
                    .options
                    .iter()
                    .find(|opt| opt.name == "Partitioning Strategy")
                    .map(|opt| opt.value.clone())
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
                    .start_package_selection(option.name.clone(), option.value);
            }
            "Timezone Region" => {
                let options = InputHandler::get_predefined_options(&option.name);
                self.input_handler
                    .start_selection(option.name.clone(), options, option.value);
            }
            "Timezone" => {
                // Get timezone options based on selected region
                let timezone_region = {
                    let state = match self.lock_state() {
                        Ok(state) => state,
                        Err(_) => return Ok(()),
                    };
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Timezone Region")
                        .map(|opt| opt.value.clone())
                        .unwrap_or_default()
                };

                if !timezone_region.is_empty() {
                    let options = InputHandler::get_timezones_for_region(&timezone_region);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else if let Ok(mut state) = self.lock_state_mut() {
                    state.status_message = "Please select a timezone region first.".to_string();
                }
            }
            "Display Manager" => {
                // Check if desktop environment is set to something other than "none"
                let desktop_env = {
                    let state = match self.lock_state() {
                        Ok(state) => state,
                        Err(_) => return Ok(()),
                    };
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Desktop Environment")
                        .map(|opt| opt.value.clone())
                        .unwrap_or_default()
                };

                if desktop_env != "none" && !desktop_env.is_empty() {
                    // Desktop environment is selected, display manager should be auto-set
                    if let Ok(mut state) = self.lock_state_mut() {
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
                let state = match self.lock_state() {
                    Ok(state) => state,
                    Err(_) => return Ok(()),
                };
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
                    if let Ok(mut state) = self.lock_state_mut() {
                        state.status_message = format!("Partitioning failed: {}", e);
                        return Ok(());
                    }
                }

                // Validate partitioning after user finishes
                let boot_mode = {
                    let state = match self.lock_state() {
                        Ok(state) => state,
                        Err(_) => return Ok(()),
                    };
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
                        if let Ok(mut state) = self.lock_state_mut() {
                            state.status_message = format!(
                                "Manual partitioning validated successfully! Found {} partitions with {} table",
                                layout.partitions.len(),
                                layout.table_type
                            );
                        }
                    }
                    Err(e) => {
                        if let Ok(mut state) = self.lock_state_mut() {
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

        // Auto-set display manager based on desktop environment
        if option_name == "Desktop Environment" {
            self.auto_set_display_manager(&value)?;
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
            if let Ok(mut state) = self.lock_state_mut() {
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
                if let Ok(mut state) = self.lock_state_mut() {
                    // Find encryption option (index 6)
                    if state.config.options.len() > 6 {
                        state.config.options[6].value = encryption_value.to_string();
                        state.status_message = format!(
                            "Auto-set Encryption to: {} (based on partitioning strategy)",
                            encryption_value
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Auto-set display manager based on desktop environment
    fn auto_set_display_manager(
        &mut self,
        desktop_env: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let display_manager = match desktop_env {
            "kde" => "sddm",
            "gnome" => "gdm",
            "hyprland" => "sddm",
            "none" => "", // Don't auto-set when "none" - let user choose
            _ => "",
        };

        if !display_manager.is_empty() {
            {
                if let Ok(mut state) = self.lock_state_mut() {
                    // Find display manager option by name
                    if let Some(display_manager_option) = state
                        .config
                        .options
                        .iter_mut()
                        .find(|opt| opt.name == "Display Manager")
                    {
                        display_manager_option.value = display_manager.to_string();
                        state.status_message = format!(
                            "Auto-set Display Manager to: {} (based on desktop environment)",
                            display_manager
                        );
                    }
                }
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
        if let Ok(mut state) = self.lock_state_mut() {
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
        if let Ok(mut state) = self.lock_state_mut() {
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

    /// Get tool parameter definitions for a specific tool
    fn get_tool_parameters(tool_name: &str) -> Vec<ToolParam> {
        match tool_name {
            "install_bootloader" => vec![
                ToolParam {
                    name: "type".to_string(),
                    description: "Bootloader type (grub or systemd-boot)".to_string(),
                    param_type: ToolParameter::Selection(
                        vec!["grub".to_string(), "systemd-boot".to_string()],
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "disk".to_string(),
                    description: "Target disk device (e.g., /dev/sda)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: true,
                },
                ToolParam {
                    name: "efi_path".to_string(),
                    description: "EFI partition path (optional)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: false,
                },
                ToolParam {
                    name: "mode".to_string(),
                    description: "Boot mode (uefi or bios, auto-detected if empty)".to_string(),
                    param_type: ToolParameter::Selection(
                        vec!["".to_string(), "uefi".to_string(), "bios".to_string()],
                        0,
                    ),
                    required: false,
                },
                ToolParam {
                    name: "repair".to_string(),
                    description: "Repair existing bootloader installation".to_string(),
                    param_type: ToolParameter::Boolean(false),
                    required: false,
                },
            ],
            "generate_fstab" => vec![ToolParam {
                name: "root".to_string(),
                description: "Root partition path (e.g., /mnt)".to_string(),
                param_type: ToolParameter::Text("".to_string()),
                required: true,
            }],
            "format_partition" => vec![
                ToolParam {
                    name: "device".to_string(),
                    description: "Partition device (e.g., /dev/sda1)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: true,
                },
                ToolParam {
                    name: "filesystem".to_string(),
                    description: "Filesystem type".to_string(),
                    param_type: ToolParameter::Selection(
                        vec![
                            "ext4".to_string(),
                            "xfs".to_string(),
                            "btrfs".to_string(),
                            "f2fs".to_string(),
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
                    param_type: ToolParameter::Text("".to_string()),
                    required: true,
                },
                ToolParam {
                    name: "method".to_string(),
                    description: "Wipe method".to_string(),
                    param_type: ToolParameter::Selection(
                        vec![
                            "zero".to_string(),
                            "random".to_string(),
                            "secure".to_string(),
                        ],
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "confirm".to_string(),
                    description: "Confirm destructive operation".to_string(),
                    param_type: ToolParameter::Boolean(false),
                    required: true,
                },
            ],
            "health" => vec![ToolParam {
                name: "output_level".to_string(),
                description: "Output detail level".to_string(),
                param_type: ToolParameter::Selection(
                    vec!["basic".to_string(), "detailed".to_string()],
                    0,
                ),
                required: false,
            }],
            "mount" => vec![
                ToolParam {
                    name: "action".to_string(),
                    description: "Action to perform".to_string(),
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
                    description: "Device path (e.g., /dev/sda1) or mountpoint path".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: true,
                },
                ToolParam {
                    name: "destination".to_string(),
                    description: "Destination directory (for mount action)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: false,
                },
                ToolParam {
                    name: "readonly".to_string(),
                    description: "Mount as read-only".to_string(),
                    param_type: ToolParameter::Boolean(false),
                    required: false,
                },
                ToolParam {
                    name: "force".to_string(),
                    description: "Force operation (unmount if busy)".to_string(),
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
                    name: "no_mount".to_string(),
                    description: "Skip mounting /proc, /sys, /dev".to_string(),
                    param_type: ToolParameter::Boolean(false),
                    required: false,
                },
            ],
            "info" => vec![
                ToolParam {
                    name: "detailed".to_string(),
                    description: "Show detailed system information".to_string(),
                    param_type: ToolParameter::Boolean(false),
                    required: false,
                },
                ToolParam {
                    name: "json".to_string(),
                    description: "Output in JSON format".to_string(),
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
                    param_type: ToolParameter::Text("/bin/bash".to_string()),
                    required: false,
                },
                ToolParam {
                    name: "system_user".to_string(),
                    description: "Create as system user".to_string(),
                    param_type: ToolParameter::Boolean(false),
                    required: false,
                },
            ],
            "reset_password" => vec![ToolParam {
                name: "username".to_string(),
                description: "Username to reset password for".to_string(),
                param_type: ToolParameter::Text("".to_string()),
                required: true,
            }],
            "configure_network" => vec![
                ToolParam {
                    name: "interface".to_string(),
                    description: "Network interface name (e.g., eth0, enp0s3)".to_string(),
                    param_type: ToolParameter::Text("".to_string()),
                    required: true,
                },
                ToolParam {
                    name: "action".to_string(),
                    description: "Action to perform".to_string(),
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
                    description: "Configuration type (for configure action)".to_string(),
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
                let mut state = self.lock_state_mut()?;
                if let Some(ref mut dialog) = state.tool_dialog {
                    if dialog.current_param > 0 {
                        dialog.current_param -= 1;
                    }
                }
            }
            KeyCode::Down => {
                // Move to next parameter (if not at last)
                let mut state = self.lock_state_mut()?;
                if let Some(ref mut dialog) = state.tool_dialog {
                    if dialog.current_param < dialog.parameters.len() - 1 {
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
                let mut state = self.lock_state_mut()?;
                let current_tool = state.current_tool.clone();
                state.tool_dialog = None;
                state.current_tool = None;
                // Go back to appropriate tool menu
                if let Some(ref tool_name) = current_tool {
                    match tool_name.as_str() {
                        "format_partition" | "wipe_disk" | "health" | "mount"
                        | "manual_partition" => {
                            state.mode = AppMode::DiskTools;
                            state.tools_menu_selection = 0;
                            state.status_message = "Disk & Filesystem Tools".to_string();
                        }
                        "install_bootloader" | "generate_fstab" | "chroot" | "info" => {
                            state.mode = AppMode::SystemTools;
                            state.tools_menu_selection = 0;
                            state.status_message = "System & Boot Tools".to_string();
                        }
                        "reset_password" => {
                            state.mode = AppMode::UserTools;
                            state.tools_menu_selection = 0;
                            state.status_message = "User & Security Tools".to_string();
                        }
                        "configure" => {
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
            KeyCode::Char(c) => {
                // Handle text input for current parameter
                let mut state = self.lock_state_mut()?;
                if let Some(ref mut dialog) = state.tool_dialog {
                    if dialog.current_param < dialog.param_values.len() {
                        dialog.param_values[dialog.current_param].push(c);
                    }
                }
            }
            KeyCode::Backspace => {
                // Handle backspace for current parameter
                let mut state = self.lock_state_mut()?;
                if let Some(ref mut dialog) = state.tool_dialog {
                    if dialog.current_param < dialog.param_values.len() {
                        dialog.param_values[dialog.current_param].pop();
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// Handle tool dialog enter key
    fn handle_tool_dialog_enter(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (tool_name, current_param, param_values) = {
            let state = self.lock_state()?;
            if let Some(ref dialog) = state.tool_dialog {
                (
                    dialog.tool_name.clone(),
                    dialog.current_param,
                    dialog.param_values.clone(),
                )
            } else {
                return Ok(());
            }
        };

        {
            let mut state = self.lock_state_mut()?;
            if let Some(ref mut dialog) = state.tool_dialog {
                if current_param < dialog.parameters.len() {
                    // Move to next parameter or execute tool
                    if current_param == dialog.parameters.len() - 1 {
                        // All parameters collected, execute tool
                        state.mode = AppMode::ToolExecution;
                    } else {
                        // Move to next parameter
                        dialog.current_param += 1;
                    }
                }
            }
        }

        // Execute tool outside of the state lock
        if current_param
            == self
                .lock_state()?
                .tool_dialog
                .as_ref()
                .map(|d| d.parameters.len() - 1)
                .unwrap_or(0)
        {
            self.execute_tool_with_params(&tool_name, param_values)?;
        }

        Ok(())
    }

    /// Create a tool dialog for parameter collection
    fn create_tool_dialog(&mut self, tool_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let parameters = Self::get_tool_parameters(tool_name);
        let param_values = vec![String::new(); parameters.len()];

        let mut state = self.lock_state_mut()?;
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
        self.execute_tool_with_device("check_disk_health.sh", &selected_disk, &["--detailed"])
    }

    /// Generic function to execute tools that need a device parameter (async/non-blocking)
    fn execute_tool_with_device(
        &mut self,
        script_name: &str,
        device: &str,
        extra_args: &[&str],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut args = Vec::new();
        args.push("--device".to_string());
        args.push(device.to_string());

        for arg in extra_args {
            args.push(arg.to_string());
        }

        let script_path = format!("scripts/tools/{}", script_name);
        let tool_display = script_name.replace(".sh", "").replace('_', " ");

        // Set up floating output window
        {
            let mut state = self.lock_state_mut()?;
            state.floating_output = Some(FloatingOutputState {
                title: format!("Running: {} on {}", tool_display, device),
                content: vec![
                    format!("Executing: {} {}", script_path, args.join(" ")),
                    String::new(),
                ],
                scroll_offset: 0,
                auto_scroll: true,
                complete: false,
                progress: None,
                status: "Running...".to_string(),
            });
            state.mode = AppMode::FloatingOutput;
            state.current_tool = Some(tool_display);
        }

        // Spawn the tool in a background thread
        self.spawn_tool_script(&script_path, args)?;

        Ok(())
    }

    /// Generic function to execute simple tools with no parameters (async/non-blocking)
    fn execute_simple_tool(
        &mut self,
        script_name: &str,
        extra_args: &[&str],
    ) -> Result<(), Box<dyn std::error::Error>> {
        let args: Vec<String> = extra_args.iter().map(|s| s.to_string()).collect();
        let script_path = format!("scripts/tools/{}", script_name);
        let tool_display = script_name.replace(".sh", "").replace('_', " ");

        // Set up floating output window
        {
            let mut state = self.lock_state_mut()?;
            state.floating_output = Some(FloatingOutputState {
                title: format!("Running: {}", tool_display),
                content: vec![
                    format!("Executing: {} {}", script_path, args.join(" ")),
                    String::new(),
                ],
                scroll_offset: 0,
                auto_scroll: true,
                complete: false,
                progress: None,
                status: "Running...".to_string(),
            });
            state.mode = AppMode::FloatingOutput;
            state.current_tool = Some(tool_display.clone());
        }

        // Spawn the tool in a background thread
        self.spawn_tool_script(&script_path, args)?;

        Ok(())
    }

    /// Spawn a tool script in a background thread with real-time output streaming
    ///
    /// # Process Lifecycle Management
    /// - Child runs in its own process group (allows clean termination of entire tree)
    /// - Child PID is registered with global ChildRegistry
    /// - On App drop or signal, all registered children receive SIGTERM
    fn spawn_tool_script(
        &self,
        script_path: &str,
        args: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.tool_tx.clone();
        let script_path = script_path.to_string();

        thread::spawn(move || {
            // Spawn the child process in its own process group
            // This allows us to kill the entire process tree if needed
            let child_result = Command::new("bash")
                .arg(&script_path)
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null())
                .in_new_process_group()
                .spawn();

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
            let stdout_handle = if let Some(stdout) = child.stdout.take() {
                Some(thread::spawn(move || {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines().map_while(Result::ok) {
                        if stdout_tx.send(ToolMessage::Stdout(line)).is_err() {
                            break; // Receiver dropped
                        }
                    }
                }))
            } else {
                None
            };

            // Stream stderr in a separate thread
            let stderr_tx = tx.clone();
            let stderr_handle = if let Some(stderr) = child.stderr.take() {
                Some(thread::spawn(move || {
                    let reader = BufReader::new(stderr);
                    for line in reader.lines().map_while(Result::ok) {
                        if stderr_tx.send(ToolMessage::Stderr(line)).is_err() {
                            break; // Receiver dropped
                        }
                    }
                }))
            } else {
                None
            };

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

    /// Spawn a tool script with optional password passed via environment variable
    /// Password is passed via USER_PASSWORD env var (lint rules forbid stdin reading)
    ///
    /// # Process Lifecycle Management
    /// - Child runs in its own process group (allows clean termination of entire tree)
    /// - Child PID is registered with global ChildRegistry
    /// - On App drop or signal, all registered children receive SIGTERM
    fn spawn_tool_script_with_password(
        &self,
        script_path: &str,
        args: Vec<String>,
        password: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let tx = self.tool_tx.clone();
        let script_path = script_path.to_string();

        thread::spawn(move || {
            // Build command with optional password in environment
            let mut cmd = Command::new("bash");
            cmd.arg(&script_path)
                .args(&args)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::null()); // Non-interactive per lint rules

            // Pass password via environment variable if provided
            if let Some(ref pw) = password {
                cmd.env("USER_PASSWORD", pw);
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
            let stdout_handle = if let Some(stdout) = child.stdout.take() {
                Some(thread::spawn(move || {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines().map_while(Result::ok) {
                        if stdout_tx.send(ToolMessage::Stdout(line)).is_err() {
                            break;
                        }
                    }
                }))
            } else {
                None
            };

            // Stream stderr in a separate thread
            let stderr_tx = tx.clone();
            let stderr_handle = if let Some(stderr) = child.stderr.take() {
                Some(thread::spawn(move || {
                    let reader = BufReader::new(stderr);
                    for line in reader.lines().map_while(Result::ok) {
                        if stderr_tx.send(ToolMessage::Stderr(line)).is_err() {
                            break;
                        }
                    }
                }))
            } else {
                None
            };

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

    /// Execute a tool with the collected parameters
    pub fn execute_tool_with_params(
        &mut self,
        tool_name: &str,
        params: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut args = Vec::new();

        // Map tool names to their script names and build arguments
        match tool_name {
            "Format Partition" => {
                if params.len() >= 2 {
                    args.push("--device".to_string());
                    args.push(params[0].clone());
                    args.push("--filesystem".to_string());
                    args.push(params[1].clone());
                    if params.len() >= 3 && !params[2].is_empty() {
                        args.push("--label".to_string());
                        args.push(params[2].clone());
                    }
                }
            }
            "Wipe Disk" => {
                if params.len() >= 2 {
                    args.push("--device".to_string());
                    args.push(params[0].clone());
                    args.push("--method".to_string());
                    args.push(params[1].clone());
                    if params.len() >= 3 && params[2] == "true" {
                        args.push("--confirm".to_string());
                    }
                }
            }
            "Add New User" => {
                if params.len() >= 2 {
                    args.push("--username".to_string());
                    args.push(params[0].clone());
                    args.push("--password".to_string());
                    args.push(params[1].clone());
                    if params.len() >= 3 && params[2] == "true" {
                        args.push("--sudo".to_string());
                    }
                }
            }
            "health" => {
                // Device is handled through disk selection dialog
                if !params.is_empty() && params[0] == "detailed" {
                    args.push("--detailed".to_string());
                }
            }
            "mount" => {
                if params.len() >= 2 {
                    args.push("--action".to_string());
                    args.push(params[0].clone());

                    // Determine if target is device or mountpoint based on action
                    let action = &params[0];
                    let target = &params[1];

                    if action == "mount" {
                        // For mount: target is device, destination is mountpoint
                        args.push("--device".to_string());
                        args.push(target.clone());
                        if params.len() >= 3 && !params[2].is_empty() {
                            args.push("--mountpoint".to_string());
                            args.push(params[2].clone());
                        }
                    } else if action == "umount" {
                        // For umount: determine if target is device or mountpoint
                        if target.starts_with("/dev/") {
                            args.push("--device".to_string());
                            args.push(target.clone());
                        } else {
                            args.push("--mountpoint".to_string());
                            args.push(target.clone());
                        }
                    } else {
                        // For info/list: target is device
                        args.push("--device".to_string());
                        args.push(target.clone());
                    }

                    // Add flags
                    if params.len() >= 4 && params[3] == "true" {
                        args.push("--readonly".to_string());
                    }
                    if params.len() >= 5 && params[4] == "true" {
                        args.push("--force".to_string());
                    }
                }
            }
            "chroot" => {
                if !params.is_empty() {
                    args.push("--root".to_string());
                    args.push(params[0].clone());
                    if params.len() >= 2 && params[1] == "true" {
                        args.push("--no-mount".to_string());
                    }
                }
            }
            "info" => {
                if !params.is_empty() && params[0] == "true" {
                    args.push("--detailed".to_string());
                }
                if params.len() >= 2 && params[1] == "true" {
                    args.push("--json".to_string());
                }
            }
            "install_bootloader" => {
                if !params.is_empty() {
                    args.push("--type".to_string());
                    args.push(params[0].clone());
                }
                if params.len() >= 2 {
                    args.push("--disk".to_string());
                    args.push(params[1].clone());
                }
                if params.len() >= 3 && !params[2].is_empty() {
                    args.push("--efi-path".to_string());
                    args.push(params[2].clone());
                }
                if params.len() >= 4 && !params[3].is_empty() {
                    args.push("--mode".to_string());
                    args.push(params[3].clone());
                }
                if params.len() >= 5 && params[4] == "true" {
                    args.push("--repair".to_string());
                }
            }
            "add_user" => {
                // Parameter order: username, password, full_name, groups, shell, system_user
                // NOTE: Password (params[1]) is NOT passed as command-line arg for security
                // It will be passed via stdin to prevent exposure in `ps aux`
                if !params.is_empty() {
                    args.push("--username".to_string());
                    args.push(params[0].clone());
                }
                // params[1] is password - handled separately via stdin (security)
                if params.len() >= 3 && !params[2].is_empty() {
                    args.push("--full-name".to_string());
                    args.push(params[2].clone());
                }
                if params.len() >= 4 && !params[3].is_empty() {
                    args.push("--groups".to_string());
                    args.push(params[3].clone());
                }
                if params.len() >= 5 && !params[4].is_empty() {
                    args.push("--shell".to_string());
                    args.push(params[4].clone());
                }
                if params.len() >= 6 && params[5] == "true" {
                    args.push("--system".to_string());
                }
            }
            "reset_password" => {
                if !params.is_empty() {
                    args.push("--username".to_string());
                    args.push(params[0].clone());
                }
            }
            "configure_network" => {
                if !params.is_empty() {
                    args.push("--interface".to_string());
                    args.push(params[0].clone());
                }
                if params.len() >= 2 {
                    args.push("--action".to_string());
                    args.push(params[1].clone());
                }
                if params.len() >= 3 && !params[2].is_empty() {
                    args.push("--config_type".to_string());
                    args.push(params[2].clone());
                }
                if params.len() >= 4 && !params[3].is_empty() {
                    args.push("--ip".to_string());
                    args.push(params[3].clone());
                }
                if params.len() >= 5 && !params[4].is_empty() {
                    args.push("--netmask".to_string());
                    args.push(params[4].clone());
                }
                if params.len() >= 6 && !params[5].is_empty() {
                    args.push("--gateway".to_string());
                    args.push(params[5].clone());
                }
            }
            _ => {
                // Generic parameter handling for other tools
                for param in &params {
                    if !param.is_empty() {
                        args.push(param.clone());
                    }
                }
            }
        }

        let script_name = match tool_name {
            "format_partition" => "format_partition.sh",
            "wipe_disk" => "wipe_disk.sh",
            "install_bootloader" => "install_bootloader.sh",
            "generate_fstab" => "generate_fstab.sh",
            "add_user" => "add_user.sh",
            "health" => "check_disk_health.sh",
            "mount" => "mount_partitions.sh",
            "chroot" => "chroot_system.sh",
            "info" => "system_info.sh",
            "reset_password" => "reset_password.sh",
            "configure_network" => "configure_network.sh",
            "manual_partition" => "manual_partition.sh",
            _ => {
                return Err(format!("Unknown tool: {}", tool_name).into());
            }
        };

        // Interactive tools should use embedded terminal
        let interactive_tools = ["chroot", "manual_partition"];
        if interactive_tools.contains(&tool_name) {
            let script_path = format!("scripts/tools/{}", script_name);

            // Determine return mode based on tool
            let return_mode = match tool_name {
                "chroot" => AppMode::SystemTools,
                "manual_partition" => AppMode::DiskTools,
                _ => AppMode::ToolsMenu,
            };

            // Clear tool dialog state before launching
            if let Ok(mut state) = self.lock_state_mut() {
                state.tool_dialog = None;
                state.current_tool = None;
            }

            // Build argument list for bash: ["-c", "script_path arg1 arg2 ..."]
            let full_cmd = if args.is_empty() {
                script_path.clone()
            } else {
                format!("{} {}", script_path, args.iter().map(|a| format!("'{}'", a)).collect::<Vec<_>>().join(" "))
            };
            let _ = self.launch_embedded_tool("bash", &["-c", &full_cmd], tool_name, return_mode);
            return Ok(());
        }

        let script_path = format!("scripts/tools/{}", script_name);
        let tool_display = tool_name.replace('_', " ");

        // Non-interactive tools use floating output window with async execution
        {
            let mut state = self.lock_state_mut()?;
            state.tool_dialog = None;
            state.floating_output = Some(FloatingOutputState {
                title: format!("Running: {}", tool_display),
                content: vec![
                    format!("Executing: {} {}", script_path, args.join(" ")),
                    String::new(),
                ],
                scroll_offset: 0,
                auto_scroll: true,
                complete: false,
                progress: None,
                status: "Running...".to_string(),
            });
            state.mode = AppMode::FloatingOutput;
            state.current_tool = Some(tool_display);
        }

        // Spawn the tool in a background thread
        // For add_user, pass password via USER_PASSWORD environment variable
        if tool_name == "add_user" {
            // Extract password from params[1] if present and non-empty
            let password = if params.len() >= 2 && !params[1].is_empty() {
                Some(params[1].clone())
            } else {
                None
            };
            self.spawn_tool_script_with_password(&script_path, args, password)?;
        } else {
            self.spawn_tool_script(&script_path, args)?;
        }

        Ok(())
    }
}
