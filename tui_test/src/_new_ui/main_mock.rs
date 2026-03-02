//! ArchTUI Visual Test Binary
//!
//! Interactive screen viewer for testing the redesigned TUI.
//! Tab cycles between screens, arrow keys navigate menus, q quits.
//! Enter on config options opens interactive editing in the right panel.
//! Includes gating and cascading logic matching the real app.

mod app;
mod components;
mod config;
mod scrolling;
mod theme;
mod ui;

use app::{
    AppMode, AppState, ConfirmDialogState, ConfirmSeverity, ConfigEditState, PackageResult,
    ToolDialogState, ToolParam, ToolParameter,
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;
use ui::UiRenderer;

/// All screens available for Tab cycling
const SCREENS: &[AppMode] = &[
    AppMode::MainMenu,
    AppMode::ToolsMenu,
    AppMode::DiskTools,
    AppMode::SystemTools,
    AppMode::UserTools,
    AppMode::NetworkTools,
    AppMode::GuidedInstaller,
    AppMode::AutomatedInstall,
    AppMode::Installation,
    AppMode::Complete,
    AppMode::ToolDialog,
    AppMode::FloatingOutput,
    AppMode::ConfirmDialog,
    AppMode::FileBrowser,
    AppMode::DryRunSummary,
];

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_app(&mut terminal);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn menu_item_count(mode: &AppMode) -> usize {
    match mode {
        AppMode::MainMenu => 4,
        AppMode::ToolsMenu => 5,
        AppMode::DiskTools => 7,
        AppMode::SystemTools => 10,
        AppMode::UserTools => 8,
        AppMode::NetworkTools => 6,
        _ => 0,
    }
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> io::Result<()> {
    let renderer = UiRenderer::new();
    let mut state = AppState::default();
    let mut screen_index: usize = 0;

    loop {
        // Sync scroll state to current terminal height before each render
        let size = terminal.size()?;
        handle_resize(&mut state, size.height);

        terminal.draw(|f| {
            renderer.render(f, &state);
        })?;

        if event::poll(Duration::from_millis(50))? {
            let ev = event::read()?;

            if let Event::Key(KeyEvent { code, .. }) = ev {
                // If we're in config edit mode, handle editing keys first
                if state.mode == AppMode::GuidedInstaller
                    && !matches!(state.config_edit, ConfigEditState::None)
                {
                    handle_config_edit(&mut state, code);
                    continue;
                }

                // If we're in ToolDialog mode, handle dialog-specific keys
                if state.mode == AppMode::ToolDialog {
                    handle_tool_dialog(&mut state, code);
                    continue;
                }

                match code {
                    KeyCode::Char('q') => {
                        // Don't quit from overlay modes — dismiss them first
                        match state.mode {
                            AppMode::ConfirmDialog
                            | AppMode::FloatingOutput
                            | AppMode::FileBrowser => {
                                dismiss_overlay(&mut state);
                            }
                            _ => break,
                        }
                    }

                    KeyCode::Tab => {
                        screen_index = (screen_index + 1) % SCREENS.len();
                        state = create_screen_state(&SCREENS[screen_index]);
                    }

                    KeyCode::BackTab => {
                        screen_index = if screen_index == 0 {
                            SCREENS.len() - 1
                        } else {
                            screen_index - 1
                        };
                        state = create_screen_state(&SCREENS[screen_index]);
                    }

                    KeyCode::Up => handle_up(&mut state),
                    KeyCode::Down => handle_down(&mut state),

                    KeyCode::Enter => handle_enter(&mut state),

                    KeyCode::Char('?') => {
                        state.help_visible = !state.help_visible;
                    }

                    KeyCode::Left | KeyCode::Right => {
                        if state.mode == AppMode::GuidedInstaller {
                            // On button row, cycle button selection
                            let is_button_row = state.config_scroll.selected_index
                                == state.config.options.len();
                            if is_button_row {
                                if code == KeyCode::Left {
                                    if state.installer_button_selection > 0 {
                                        state.installer_button_selection -= 1;
                                    }
                                } else if state.installer_button_selection < 2 {
                                    state.installer_button_selection += 1;
                                }
                            }
                        } else if let Some(ref mut dialog) = state.confirm_dialog {
                            dialog.selected = if dialog.selected == 0 { 1 } else { 0 };
                        }
                    }

                    KeyCode::Esc | KeyCode::Char('b') => {
                        // Close help first if visible
                        if state.help_visible {
                            state.help_visible = false;
                        } else {
                            handle_back(&mut state);
                        }
                    }

                    _ => {}
                }
            }
        }
    }

    Ok(())
}

// =============================================================================
// Up/Down navigation
// =============================================================================

fn handle_up(state: &mut AppState) {
    match state.mode {
        AppMode::MainMenu => {
            if state.main_menu_selection > 0 {
                state.main_menu_selection -= 1;
            }
        }
        AppMode::GuidedInstaller => {
            state.config_scroll.move_up();
        }
        AppMode::FloatingOutput => {
            if let Some(ref mut output) = state.floating_output {
                if output.scroll_offset > 0 {
                    output.scroll_offset -= 1;
                }
            }
        }
        AppMode::DryRunSummary => {
            if state.dry_run_scroll_offset > 0 {
                state.dry_run_scroll_offset -= 1;
            }
        }
        AppMode::FileBrowser => {
            if let Some(ref mut browser) = state.file_browser {
                if browser.selected > 0 {
                    browser.selected -= 1;
                    if browser.selected < browser.scroll_offset {
                        browser.scroll_offset = browser.selected;
                    }
                }
            }
        }
        AppMode::Installation => {
            if state.installer_scroll_offset > 0 {
                state.installer_scroll_offset -= 1;
                state.installer_auto_scroll = false;
            }
        }
        // Menu modes (ToolsMenu, DiskTools, etc.)
        _ => {
            let max = menu_item_count(&state.mode);
            if max > 0 && state.tools_menu_selection > 0 {
                state.tools_menu_selection -= 1;
            }
        }
    }
}

fn handle_down(state: &mut AppState) {
    match state.mode {
        AppMode::MainMenu => {
            let max = menu_item_count(&state.mode);
            if state.main_menu_selection < max - 1 {
                state.main_menu_selection += 1;
            }
        }
        AppMode::GuidedInstaller => {
            state.config_scroll.move_down();
        }
        AppMode::FloatingOutput => {
            if let Some(ref mut output) = state.floating_output {
                if output.scroll_offset < output.content.len().saturating_sub(1) {
                    output.scroll_offset += 1;
                }
            }
        }
        AppMode::DryRunSummary => {
            if let Some(ref summary) = state.dry_run_summary {
                if state.dry_run_scroll_offset < summary.len().saturating_sub(1) {
                    state.dry_run_scroll_offset += 1;
                }
            }
        }
        AppMode::FileBrowser => {
            if let Some(ref mut browser) = state.file_browser {
                if browser.selected < browser.entries.len().saturating_sub(1) {
                    browser.selected += 1;
                }
            }
        }
        AppMode::Installation => {
            state.installer_scroll_offset += 1;
            let max = state
                .installer_output
                .len()
                .saturating_sub(state.installer_visible_height);
            if state.installer_scroll_offset >= max {
                state.installer_auto_scroll = true;
            }
        }
        // Menu modes (ToolsMenu, DiskTools, etc.)
        _ => {
            let max = menu_item_count(&state.mode);
            if max > 0 && state.tools_menu_selection < max - 1 {
                state.tools_menu_selection += 1;
            }
        }
    }
}

// =============================================================================
// Enter key
// =============================================================================

fn handle_enter(state: &mut AppState) {
    match state.mode {
        AppMode::MainMenu => match state.main_menu_selection {
            0 => state.mode = AppMode::GuidedInstaller,
            1 => state.mode = AppMode::AutomatedInstall,
            2 => {
                state.mode = AppMode::ToolsMenu;
                state.tools_menu_selection = 0;
            }
            3 => {} // Quit — handled by 'q'
            _ => {}
        },
        AppMode::ToolsMenu => match state.tools_menu_selection {
            0 => {
                state.mode = AppMode::DiskTools;
                state.tools_menu_selection = 0;
            }
            1 => {
                state.mode = AppMode::SystemTools;
                state.tools_menu_selection = 0;
            }
            2 => {
                state.mode = AppMode::UserTools;
                state.tools_menu_selection = 0;
            }
            3 => {
                state.mode = AppMode::NetworkTools;
                state.tools_menu_selection = 0;
            }
            4 => state.mode = AppMode::MainMenu,
            _ => {}
        },
        AppMode::DiskTools => {
            let sel = state.tools_menu_selection;
            if sel == 6 {
                state.mode = AppMode::ToolsMenu;
                state.tools_menu_selection = 0;
            } else {
                open_tool_dialog_for(state, "disk", sel);
            }
        }
        AppMode::SystemTools => {
            let sel = state.tools_menu_selection;
            if sel == 9 {
                state.mode = AppMode::ToolsMenu;
                state.tools_menu_selection = 1;
            } else {
                open_tool_dialog_for(state, "system", sel);
            }
        }
        AppMode::UserTools => {
            let sel = state.tools_menu_selection;
            if sel == 7 {
                state.mode = AppMode::ToolsMenu;
                state.tools_menu_selection = 2;
            } else {
                open_tool_dialog_for(state, "user", sel);
            }
        }
        AppMode::NetworkTools => {
            let sel = state.tools_menu_selection;
            if sel == 5 {
                state.mode = AppMode::ToolsMenu;
                state.tools_menu_selection = 3;
            } else {
                open_tool_dialog_for(state, "network", sel);
            }
        }
        AppMode::GuidedInstaller => {
            // Check if on button row
            let is_button_row =
                state.config_scroll.selected_index == state.config.options.len();
            if is_button_row {
                handle_button_press(state);
            } else {
                open_config_edit(state);
            }
        }
        AppMode::ConfirmDialog => {
            // If "Confirm" button selected (index 0), start installation demo
            if let Some(ref dialog) = state.confirm_dialog {
                if dialog.selected == 0 && dialog.confirm_action == "start_install" {
                    state.confirm_dialog = None;
                    state.pre_dialog_mode = None;
                    let mut new = AppState::demo_installation();
                    new.config = state.config.clone();
                    *state = new;
                    return;
                }
            }
            dismiss_overlay(state);
        }
        AppMode::FloatingOutput => {
            dismiss_overlay(state);
        }
        AppMode::FileBrowser => {
            dismiss_overlay(state);
        }
        AppMode::Complete => {
            state.mode = AppMode::MainMenu;
            state.main_menu_selection = 0;
        }
        AppMode::DryRunSummary => {
            state.mode = AppMode::GuidedInstaller;
        }
        _ => {}
    }
}

// =============================================================================
// Button row handling (Guided Installer)
// =============================================================================

fn handle_button_press(state: &mut AppState) {
    match state.installer_button_selection {
        0 => {
            // TEST CONFIG → Dry Run Summary
            state.dry_run_summary = Some(generate_dry_run(state));
            state.dry_run_scroll_offset = 0;
            state.mode = AppMode::DryRunSummary;
        }
        1 => {
            // EXPORT CONFIG → show floating output
            state.pre_dialog_mode = Some(AppMode::GuidedInstaller);
            state.floating_output = Some(app::FloatingOutputState {
                title: "Export Configuration".to_string(),
                content: vec![
                    "==> Exporting configuration to /tmp/archtui-config.toml".to_string(),
                    "  Writing 46 configuration options...".to_string(),
                    "SUCCESS: Configuration saved".to_string(),
                ],
                scroll_offset: 0,
                auto_scroll: false,
                complete: true,
                progress: None,
                status: "Complete — press Esc to close".to_string(),
            });
            state.mode = AppMode::FloatingOutput;
        }
        2 => {
            // START INSTALL → Confirm dialog
            state.pre_dialog_mode = Some(AppMode::GuidedInstaller);
            state.confirm_dialog = Some(ConfirmDialogState {
                title: "START INSTALLATION".to_string(),
                message: "Begin Arch Linux installation with current settings?".to_string(),
                details: vec![
                    format!(
                        "Target disk: {}",
                        get_option_value(&state.config, "Disk")
                    ),
                    format!(
                        "Strategy: {}",
                        get_option_value(&state.config, "Partitioning Strategy")
                    ),
                    "This will ERASE the target disk".to_string(),
                ],
                severity: ConfirmSeverity::Danger,
                selected: 1, // Default to Cancel for safety
                confirm_action: "start_install".to_string(),
            });
            state.mode = AppMode::ConfirmDialog;
        }
        _ => {}
    }
}

/// Generate a dry-run summary from current config
fn generate_dry_run(state: &AppState) -> Vec<String> {
    let c = &state.config;
    let disk = get_option_value(c, "Disk");
    let strategy = get_option_value(c, "Partitioning Strategy");
    let root_fs = get_option_value(c, "Root Filesystem");
    let bootloader = get_option_value(c, "Bootloader");
    let hostname = get_option_value(c, "Hostname");
    let username = get_option_value(c, "Username");
    let tz_region = get_option_value(c, "Timezone Region");
    let tz_city = get_option_value(c, "Timezone");
    let de = get_option_value(c, "Desktop Environment");
    let aur = get_option_value(c, "AUR Helper");
    let encryption = get_option_value(c, "Encryption");

    let mut lines = Vec::new();

    lines.push(format!("[DESTRUCTIVE] Partition {} ({})", disk, strategy));
    lines.push("  -> Create EFI partition: 512MB FAT32".to_string());
    lines.push(format!(
        "  -> Create root partition: {} {}",
        get_option_value(c, "Root Size"),
        root_fs
    ));

    if get_option_value(c, "Separate Home Partition") == "Yes" {
        lines.push(format!(
            "  -> Create home partition: {} {}",
            get_option_value(c, "Home Size"),
            get_option_value(c, "Home Filesystem")
        ));
    }
    if get_option_value(c, "Swap") == "Yes" {
        lines.push(format!(
            "  -> Create swap partition: {}",
            get_option_value(c, "Swap Size")
        ));
    }
    lines.push(String::new());

    if encryption != "No" {
        lines.push("[DESTRUCTIVE] Setup LUKS encryption".to_string());
        lines.push(String::new());
    }

    lines.push("[DESTRUCTIVE] Format partitions".to_string());
    lines.push("  -> Format EFI as FAT32".to_string());
    lines.push(format!("  -> Format root as {}", root_fs));
    lines.push(String::new());

    lines.push("Install base packages via pacstrap".to_string());
    lines.push(format!(
        "  -> base {} linux-firmware",
        get_option_value(c, "Kernel")
    ));
    let extra = get_option_value(c, "Additional Pacman Packages");
    if !extra.is_empty() {
        lines.push(format!("  -> Additional: {}", extra));
    }
    lines.push(String::new());

    lines.push(format!("Install {} bootloader", bootloader));
    lines.push(format!("  -> Target: {}", disk));
    lines.push(String::new());

    lines.push("Configure system".to_string());
    if !hostname.is_empty() {
        lines.push(format!("  -> Hostname: {}", hostname));
    }
    if !username.is_empty() {
        lines.push(format!("  -> Username: {}", username));
    }
    lines.push(format!("  -> Timezone: {}/{}", tz_region, tz_city));
    lines.push(format!(
        "  -> Locale: {}",
        get_option_value(c, "Locale")
    ));
    lines.push(String::new());

    if de == "none" {
        lines.push("[SKIP] Desktop environment: none".to_string());
    } else {
        lines.push(format!("Install desktop environment: {}", de));
        lines.push(format!(
            "  -> Display manager: {}",
            get_option_value(c, "Display Manager")
        ));
    }
    if aur == "none" {
        lines.push("[SKIP] AUR helper: none".to_string());
    } else {
        lines.push(format!("Install AUR helper: {}", aur));
    }

    lines
}

// =============================================================================
// Esc/Back key
// =============================================================================

fn handle_back(state: &mut AppState) {
    match state.mode {
        AppMode::GuidedInstaller | AppMode::AutomatedInstall => {
            state.config_edit = ConfigEditState::None;
            state.mode = AppMode::MainMenu;
        }
        AppMode::ToolsMenu => {
            state.mode = AppMode::MainMenu;
        }
        AppMode::DiskTools
        | AppMode::SystemTools
        | AppMode::UserTools
        | AppMode::NetworkTools => {
            state.mode = AppMode::ToolsMenu;
            state.tools_menu_selection = 0;
        }
        AppMode::ConfirmDialog | AppMode::FloatingOutput | AppMode::FileBrowser => {
            dismiss_overlay(state);
        }
        AppMode::DryRunSummary => {
            state.mode = AppMode::GuidedInstaller;
        }
        AppMode::Complete | AppMode::Installation => {
            state.mode = AppMode::MainMenu;
            state.main_menu_selection = 0;
        }
        _ => {
            state.config_edit = ConfigEditState::None;
        }
    }
}

/// Dismiss an overlay dialog and return to the background mode
fn dismiss_overlay(state: &mut AppState) {
    if let Some(pre_mode) = state.pre_dialog_mode.take() {
        state.mode = pre_mode;
    } else {
        state.mode = AppMode::MainMenu;
    }
    state.tool_dialog = None;
    state.floating_output = None;
    state.confirm_dialog = None;
    state.file_browser = None;
}

/// Simulate tool execution — show FloatingOutput with mock output
fn simulate_tool_execution(state: &mut AppState, tool: &str, values: &[String]) {
    let title = snake_to_title(&tool.replace('_', " "));

    // Build a summary of what was "executed"
    let mut content = vec![
        format!("==> Running: {}", title),
        format!("    Script: scripts/tools/{}.sh", tool),
    ];

    // Show parameter values
    if let Some(ref dialog) = state.tool_dialog {
        for (i, param) in dialog.parameters.iter().enumerate() {
            let val = values.get(i).map(|s| s.as_str()).unwrap_or("");
            if !val.is_empty() {
                if matches!(param.param_type, ToolParameter::Password(_)) {
                    content.push(format!("    --{} ********", param.name));
                } else {
                    content.push(format!("    --{} {}", param.name, val));
                }
            }
        }
    }

    content.push(String::new());

    // Tool-specific mock output
    match tool {
        "manual_partition" => {
            content.push("  Launching partition editor...".to_string());
            content.push("  Device: /dev/sda".to_string());
            content.push("  Partition table: GPT".to_string());
            content.push(String::new());
            content.push("SUCCESS: Partition editor closed".to_string());
        }
        "format_partition" => {
            let fs = values.get(1).map(|s| s.as_str()).unwrap_or("ext4");
            let dev = values.first().map(|s| s.as_str()).unwrap_or("/dev/sda1");
            content.push(format!("  Formatting {} as {}...", dev, fs));
            content.push("  Creating filesystem...".to_string());
            content.push("  Setting label...".to_string());
            content.push(String::new());
            content.push("SUCCESS: Format complete".to_string());
        }
        "wipe_disk" => {
            let dev = values.first().map(|s| s.as_str()).unwrap_or("/dev/sda");
            let method = values.get(1).map(|s| s.as_str()).unwrap_or("quick");
            content.push(format!("  Wiping {} using {} method...", dev, method));
            content.push("  Clearing partition table...".to_string());
            content.push("  Zeroing first 1MB...".to_string());
            content.push("  Zeroing last 1MB...".to_string());
            content.push(String::new());
            content.push("SUCCESS: Disk wiped".to_string());
        }
        "check_disk_health" => {
            content.push("  Running SMART diagnostics...".to_string());
            content.push("  Model: Samsung SSD 970 EVO Plus".to_string());
            content.push("  Health Status: PASSED".to_string());
            content.push("  Temperature: 34C".to_string());
            content.push("  Power On Hours: 12,847".to_string());
            content.push(String::new());
            content.push("SUCCESS: Health check complete".to_string());
        }
        "mount" => {
            let action = values.first().map(|s| s.as_str()).unwrap_or("list");
            match action {
                "list" => {
                    content.push("  /dev/nvme0n1p2 on / type ext4 (rw,relatime)".to_string());
                    content.push("  /dev/nvme0n1p1 on /boot type vfat (rw,relatime)".to_string());
                    content.push("  tmpfs on /tmp type tmpfs (rw,nosuid,nodev)".to_string());
                }
                _ => {
                    content.push(format!("  Action: {}", action));
                    content.push("SUCCESS: Mount operation complete".to_string());
                }
            }
        }
        "encrypt_device" => {
            let action = values.first().map(|s| s.as_str()).unwrap_or("format");
            content.push(format!("  LUKS action: {}", action));
            content.push("  Encryption setup complete".to_string());
            content.push(String::new());
            content.push("SUCCESS: LUKS operation complete".to_string());
        }
        "install_bootloader" => {
            let bl = values.first().map(|s| s.as_str()).unwrap_or("grub");
            content.push(format!("  Installing {}...", bl));
            content.push("  Detecting boot mode...".to_string());
            content.push("  UEFI mode detected".to_string());
            content.push("  Installing to EFI partition...".to_string());
            content.push("  Generating configuration...".to_string());
            content.push(String::new());
            content.push("SUCCESS: Bootloader installed".to_string());
        }
        "generate_fstab" => {
            content.push("  Generating /etc/fstab from mount points...".to_string());
            content.push("  UUID=xxxx-xxxx / ext4 rw,relatime 0 1".to_string());
            content.push("  UUID=xxxx-xxxx /boot vfat rw,relatime 0 2".to_string());
            content.push(String::new());
            content.push("SUCCESS: fstab generated".to_string());
        }
        "chroot" => {
            content.push("  Mounting /proc, /sys, /dev...".to_string());
            content.push("  Entering chroot at /mnt...".to_string());
            content.push("  [chroot session active]".to_string());
            content.push(String::new());
            content.push("SUCCESS: Chroot session ended".to_string());
        }
        "manage_services" => {
            let action = values.first().map(|s| s.as_str()).unwrap_or("list");
            let svc = values.get(1).map(|s| s.as_str()).unwrap_or("");
            content.push(format!("  Action: {} {}", action, svc));
            content.push("SUCCESS: Service operation complete".to_string());
        }
        "enable_services" => {
            let svcs = values.first().map(|s| s.as_str()).unwrap_or("");
            content.push(format!("  Enabling services: {}", svcs));
            content.push("SUCCESS: Services enabled".to_string());
        }
        "install_aur_helper" => {
            let helper = values.first().map(|s| s.as_str()).unwrap_or("paru");
            content.push(format!("  Installing {}...", helper));
            content.push("  Cloning from AUR...".to_string());
            content.push("  Building package...".to_string());
            content.push("  Installing...".to_string());
            content.push(String::new());
            content.push(format!("SUCCESS: {} installed", helper));
        }
        "rebuild_initramfs" => {
            content.push("  Running mkinitcpio -P...".to_string());
            content.push("  ==> Building image from preset: /etc/mkinitcpio.d/linux.preset".to_string());
            content.push("  ==> Image generation successful".to_string());
            content.push(String::new());
            content.push("SUCCESS: Initramfs rebuilt".to_string());
        }
        "add_user" => {
            let user = values.first().map(|s| s.as_str()).unwrap_or("user");
            content.push(format!("  Creating user: {}", user));
            content.push("  Setting password...".to_string());
            content.push("  Adding to groups...".to_string());
            content.push(String::new());
            content.push(format!("SUCCESS: User {} created", user));
        }
        "reset_password" => {
            let user = values.first().map(|s| s.as_str()).unwrap_or("user");
            content.push(format!("  Resetting password for {}...", user));
            content.push(String::new());
            content.push("SUCCESS: Password updated".to_string());
        }
        "manage_groups" => {
            let action = values.first().map(|s| s.as_str()).unwrap_or("list");
            content.push(format!("  Group action: {}", action));
            content.push("SUCCESS: Group operation complete".to_string());
        }
        "configure_ssh" => {
            let action = values.first().map(|s| s.as_str()).unwrap_or("status");
            content.push(format!("  SSH action: {}", action));
            match action {
                "status" => content.push("  sshd is active (running)".to_string()),
                "install" => content.push("  openssh installed and enabled".to_string()),
                _ => content.push("SUCCESS: SSH configuration updated".to_string()),
            }
        }
        "security_audit" => {
            let level = values.first().map(|s| s.as_str()).unwrap_or("basic");
            content.push(format!("  Running {} security audit...", level));
            content.push("  Checking file permissions...".to_string());
            content.push("  Checking running services...".to_string());
            content.push("  Checking open ports...".to_string());
            content.push(String::new());
            content.push("SUCCESS: Audit complete — 0 issues found".to_string());
        }
        "install_dotfiles" => {
            let url = values.first().map(|s| s.as_str()).unwrap_or("");
            content.push(format!("  Cloning {}...", url));
            content.push("  Installing dotfiles...".to_string());
            content.push(String::new());
            content.push("SUCCESS: Dotfiles installed".to_string());
        }
        "run_as_user" => {
            let user = values.first().map(|s| s.as_str()).unwrap_or("user");
            let cmd = values.get(1).map(|s| s.as_str()).unwrap_or("");
            content.push(format!("  Running as {}: {}", user, cmd));
            content.push(String::new());
            content.push("SUCCESS: Command completed".to_string());
        }
        "configure_network" => {
            let iface = values.first().map(|s| s.as_str()).unwrap_or("eth0");
            content.push(format!("  Interface: {}", iface));
            content.push("SUCCESS: Network configured".to_string());
        }
        "test_network" => {
            let test = values.first().map(|s| s.as_str()).unwrap_or("full");
            content.push(format!("  Running {} connectivity test...", test));
            content.push("  PING 8.8.8.8: 64 bytes, time=12.3ms".to_string());
            content.push("  DNS: archlinux.org resolved to 95.217.163.246".to_string());
            content.push("  HTTP: https://archlinux.org — 200 OK".to_string());
            content.push(String::new());
            content.push("SUCCESS: All tests passed".to_string());
        }
        "configure_firewall" => {
            let action = values.first().map(|s| s.as_str()).unwrap_or("status");
            content.push(format!("  Firewall action: {}", action));
            content.push("SUCCESS: Firewall operation complete".to_string());
        }
        "network_diagnostics" => {
            let level = values.first().map(|s| s.as_str()).unwrap_or("info");
            content.push(format!("  Running {} diagnostics...", level));
            content.push("  eth0: UP, 192.168.1.100/24".to_string());
            content.push("  Gateway: 192.168.1.1 (reachable)".to_string());
            content.push("  DNS: 8.8.8.8 (responding)".to_string());
            content.push(String::new());
            content.push("SUCCESS: Diagnostics complete".to_string());
        }
        "update_mirrors" => {
            let country = values.first().map(|s| s.as_str()).unwrap_or("");
            let limit = values.get(1).map(|s| s.as_str()).unwrap_or("20");
            content.push(format!("  Updating mirrors (country={}, limit={})...", country, limit));
            content.push("  Ranking mirrors by speed...".to_string());
            content.push("  1. mirror.rackspace.com — 15.2 MiB/s".to_string());
            content.push("  2. mirrors.kernel.org — 12.8 MiB/s".to_string());
            content.push("  3. mirror.leaseweb.net — 11.5 MiB/s".to_string());
            content.push(String::new());
            content.push("SUCCESS: Mirror list updated".to_string());
        }
        _ => {
            content.push("  Executing tool...".to_string());
            content.push(String::new());
            content.push("SUCCESS: Operation complete".to_string());
        }
    }

    // Transition: ToolDialog → FloatingOutput (keeping pre_dialog_mode for return path)
    state.tool_dialog = None;
    state.floating_output = Some(app::FloatingOutputState {
        title: format!("Running: {}", title),
        content,
        scroll_offset: 0,
        auto_scroll: false,
        complete: true,
        progress: None,
        status: "Complete — press Esc to close".to_string(),
    });
    state.mode = AppMode::FloatingOutput;
}

/// Convert snake_case to Title Case
fn snake_to_title(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            let mut chars = w.chars();
            match chars.next() {
                None => String::new(),
                Some(c) => c.to_uppercase().to_string() + &chars.as_str().to_lowercase(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// =============================================================================
// ToolDialog key handling
// =============================================================================

fn handle_tool_dialog(state: &mut AppState, code: KeyCode) {
    match code {
        KeyCode::Up => {
            if let Some(ref mut dialog) = state.tool_dialog {
                if dialog.current_param > 0 {
                    dialog.current_param -= 1;
                }
            }
        }
        KeyCode::Down => {
            if let Some(ref mut dialog) = state.tool_dialog {
                if dialog.current_param < dialog.parameters.len().saturating_sub(1) {
                    dialog.current_param += 1;
                }
            }
        }
        KeyCode::Left | KeyCode::Right => {
            let mut refresh_layout = false;
            let mut new_device = String::new();

            if let Some(ref mut dialog) = state.tool_dialog {
                let idx = dialog.current_param;
                if let Some(param) = dialog.parameters.get(idx) {
                    match &param.param_type {
                        ToolParameter::Selection(ref options, _) => {
                            if let Some(val) = dialog.param_values.get(idx) {
                                let current_pos =
                                    options.iter().position(|o| o == val).unwrap_or(0);
                                let new_pos = if code == KeyCode::Left {
                                    if current_pos == 0 {
                                        options.len().saturating_sub(1)
                                    } else {
                                        current_pos - 1
                                    }
                                } else {
                                    (current_pos + 1) % options.len()
                                };
                                if let Some(new_val) = options.get(new_pos) {
                                    dialog.param_values[idx] = new_val.clone();

                                    // Refresh disk layout when device/disk param changes
                                    if matches!(param.name.as_str(), "device" | "disk" | "target")
                                        && new_val.starts_with("/dev/")
                                    {
                                        refresh_layout = true;
                                        new_device = new_val.clone();
                                    }
                                }
                            }
                        }
                        ToolParameter::Boolean(_) => {
                            if idx < dialog.param_values.len() {
                                let current = dialog.param_values[idx] == "true";
                                dialog.param_values[idx] = (!current).to_string();
                            }
                        }
                        _ => {}
                    }
                }
            }

            if refresh_layout {
                state.disk_layout = config::get_disk_layout(&new_device);
            }
        }
        KeyCode::Char(c) => {
            if let Some(ref mut dialog) = state.tool_dialog {
                let idx = dialog.current_param;
                if let Some(param) = dialog.parameters.get(idx) {
                    match param.param_type {
                        ToolParameter::Text(_) | ToolParameter::Password(_) => {
                            if idx < dialog.param_values.len() {
                                dialog.param_values[idx].push(c);
                            }
                        }
                        _ => {
                            if c == 'q' {
                                dismiss_overlay(state);
                            }
                        }
                    }
                }
            }
        }
        KeyCode::Backspace => {
            if let Some(ref mut dialog) = state.tool_dialog {
                let idx = dialog.current_param;
                if let Some(param) = dialog.parameters.get(idx) {
                    // Only backspace for text/password fields (matching real app)
                    if matches!(param.param_type, ToolParameter::Text(_) | ToolParameter::Password(_))
                        && idx < dialog.param_values.len()
                    {
                        dialog.param_values[idx].pop();
                    }
                }
            }
        }
        KeyCode::Enter => {
            // Real app behavior: Enter advances to next param, executes on last
            if let Some(ref mut dialog) = state.tool_dialog {
                let current = dialog.current_param;
                let total = dialog.parameters.len();
                let last = total.saturating_sub(1);

                if current < total && current != last {
                    // Not on last param — advance to next
                    dialog.current_param += 1;
                } else if current == last {
                    // On last param — simulate tool execution via FloatingOutput
                    let tool = dialog.tool_name.clone();
                    let values = dialog.param_values.clone();
                    simulate_tool_execution(state, &tool, &values);
                }
            }
        }
        KeyCode::Esc => {
            dismiss_overlay(state);
        }
        _ => {}
    }
}

// =============================================================================
// Tool menu → ToolDialog dispatch
// =============================================================================

fn open_tool_dialog_for(state: &mut AppState, category: &str, index: usize) {
    // Auto-detect system resources based on category
    let detected_disks = if category == "disk" || category == "system" { config::detect_disks() } else { Vec::new() };
    let detected_parts = if category == "disk" { config::detect_partitions() } else { Vec::new() };
    let detected_users = if matches!(category, "user" | "system") { config::detect_users() } else { Vec::new() };
    let detected_ifaces = if category == "network" { config::detect_interfaces() } else { Vec::new() };
    let detected_shells = if category == "user" { config::detect_shells() } else { Vec::new() };
    let detected_groups = if category == "user" { config::detect_groups() } else { Vec::new() };
    let detected_services = if category == "system" { config::detect_services() } else { Vec::new() };

    let disk_refs: Vec<&str> = detected_disks.iter().map(|s| s.as_str()).collect();
    let part_refs: Vec<&str> = detected_parts.iter().map(|s| s.as_str()).collect();
    let user_refs: Vec<&str> = detected_users.iter().map(|s| s.as_str()).collect();
    let iface_refs: Vec<&str> = detected_ifaces.iter().map(|s| s.as_str()).collect();
    let shell_refs: Vec<&str> = detected_shells.iter().map(|s| s.as_str()).collect();
    let group_refs: Vec<&str> = detected_groups.iter().map(|s| s.as_str()).collect();
    let service_refs: Vec<&str> = detected_services.iter().map(|s| s.as_str()).collect();

    let (tool_name, params) = match category {
        // =================================================================
        // Disk Tools — device params use auto-detected disks/partitions
        // =================================================================
        "disk" => match index {
            // 0: Partition Disk — select whole disk
            0 => (
                "manual_partition",
                vec![mk_param_sel("device", "Select disk to partition", &disk_refs, true)],
            ),
            // 1: Format Partition — select partition
            1 => (
                "format_partition",
                vec![
                    mk_param_sel("device", "Select partition to format", &part_refs, true),
                    mk_param_sel("filesystem", "Filesystem type to format the partition with",
                        &["ext4", "xfs", "btrfs", "fat32", "ntfs"], true),
                    mk_param_text("label", "Partition label (optional)", false),
                ],
            ),
            // 2: Wipe Disk — select whole disk
            2 => (
                "wipe_disk",
                vec![
                    mk_param_sel("device", "Select disk to wipe", &disk_refs, true),
                    mk_param_sel("method", "quick: signatures only | secure: full wipe | auto: detect SSD/HDD",
                        &["quick", "secure", "auto"], true),
                    mk_param_text("confirm", "ALL DATA WILL BE DESTROYED. Type CONFIRM to proceed.", true),
                ],
            ),
            // 3: Check Disk Health — select whole disk
            3 => (
                "check_disk_health",
                vec![mk_param_sel("device", "Select disk to check", &disk_refs, true)],
            ),
            // 4: Mount/Unmount — target is a partition selection
            4 => (
                "mount",
                vec![
                    mk_param_sel("action", "mount: attach device | umount: detach | list: show mounts | info: device details",
                        &["mount", "umount", "list", "info"], true),
                    mk_param_sel("target", "Select device for mount/info or mountpoint for umount", &part_refs, true),
                    mk_param_text("destination", "Mount destination directory (e.g., /mnt)", false),
                    mk_param_bool("read_only", "Mount filesystem as read-only", false),
                    mk_param_bool("force", "Force lazy unmount if device is busy", false),
                ],
            ),
            // 5: LUKS Encryption — partition selection
            5 => (
                "encrypt_device",
                vec![
                    mk_param_sel("action", "format: encrypt device | open: unlock encrypted device | close: lock device",
                        &["format", "open", "close"], true),
                    mk_param_sel("device", "Select device for LUKS encryption", &part_refs, true),
                    mk_param_password("password", "LUKS passphrase (for format/open)", false),
                    mk_param_text_default("mapper_name", "Mapper device name (default: cryptroot)", "cryptroot", false),
                ],
            ),
            _ => return,
        },
        // =================================================================
        // System Tools
        // =================================================================
        "system" => match index {
            // 0: Install Bootloader — disk auto-detected
            0 => (
                "install_bootloader",
                vec![
                    mk_param_sel("bootloader", "Bootloader to install (GRUB supports BIOS+UEFI, systemd-boot is UEFI only)",
                        &["grub", "systemd-boot"], true),
                    mk_param_sel("disk", "Target disk for bootloader installation", &disk_refs, true),
                    mk_param_text("efi_path", "EFI system partition mount point (e.g., /boot/efi)", false),
                    mk_param_sel("boot_mode", "Boot mode — leave empty for auto-detection",
                        &["", "uefi", "bios"], false),
                    mk_param_bool("repair", "Repair existing bootloader instead of fresh install", false),
                ],
            ),
            // 1: Generate fstab
            1 => (
                "generate_fstab",
                vec![mk_param_text("root", "Mounted root path to generate fstab from (e.g., /mnt)", true)],
            ),
            // 2: Chroot into System
            2 => (
                "chroot",
                vec![
                    mk_param_text_default("root", "Root directory to chroot into", "/mnt", true),
                    mk_param_bool("skip_mount", "Skip mounting /proc, /sys, /dev (if already mounted)", false),
                ],
            ),
            // 3: Manage Services — services auto-detected
            3 => (
                "manage_services",
                vec![
                    mk_param_sel("action", "enable/disable: persist across reboots | status: check state | list: show all",
                        &["enable", "disable", "status", "list"], true),
                    mk_param_sel("service", "Select systemd service", &service_refs, true),
                ],
            ),
            // 4: System Info — runs directly (no ToolDialog)
            4 => {
                state.pre_dialog_mode = Some(state.mode.clone());
                state.floating_output = Some(app::FloatingOutputState {
                    title: "System Information".to_string(),
                    content: vec![
                        "==> System Information (detailed)".to_string(),
                        "".to_string(),
                        "  Hostname:      archlinux".to_string(),
                        "  Kernel:        Linux 6.12.4-arch1-1".to_string(),
                        "  Architecture:  x86_64".to_string(),
                        "  Uptime:        2 hours, 14 minutes".to_string(),
                        "".to_string(),
                        "  CPU:           AMD Ryzen 9 5900X (24 cores)".to_string(),
                        "  Memory:        32GB (8.2GB used)".to_string(),
                        "  Swap:          8GB (0B used)".to_string(),
                        "".to_string(),
                        "  Root FS:       ext4 on /dev/nvme0n1p2".to_string(),
                        "  Boot Mode:     UEFI".to_string(),
                        "  Init System:   systemd 256".to_string(),
                        "".to_string(),
                        "SUCCESS: System info collected".to_string(),
                    ],
                    scroll_offset: 0,
                    auto_scroll: false,
                    complete: true,
                    progress: None,
                    status: "System info — press Esc to close".to_string(),
                });
                state.mode = AppMode::FloatingOutput;
                return;
            }
            // 5: Enable Services
            5 => (
                "enable_services",
                vec![
                    mk_param_text("services", "Service names (comma-separated, e.g., sddm,NetworkManager)", true),
                    mk_param_text_default("root", "Root mount path for chroot", "/mnt", true),
                ],
            ),
            // 6: Install AUR Helper — user auto-detected
            6 => (
                "install_aur_helper",
                vec![
                    mk_param_sel("helper", "AUR helper to install", &["paru", "yay"], true),
                    mk_param_sel("user", "Select target user (must be non-root)", &user_refs, true),
                    mk_param_text_default("root", "Root mount path for chroot", "/mnt", true),
                ],
            ),
            // 7: Rebuild Initramfs
            7 => (
                "rebuild_initramfs",
                vec![mk_param_text("root", "Root mount point (e.g., /mnt)", true)],
            ),
            // 8: View Install Logs — opens FloatingOutput directly
            8 => {
                state.pre_dialog_mode = Some(state.mode.clone());
                state.floating_output = Some(app::FloatingOutputState {
                    title: "Installation Logs".to_string(),
                    content: vec![
                        "==> /var/log/archtui/install.log".to_string(),
                        "".to_string(),
                        "[2024-01-15 14:30:01] Starting installation...".to_string(),
                        "[2024-01-15 14:30:02] Phase 1: Partitioning disk /dev/sda".to_string(),
                        "[2024-01-15 14:30:05] Created GPT partition table".to_string(),
                        "[2024-01-15 14:30:06] Created EFI partition (512MB)".to_string(),
                        "[2024-01-15 14:30:07] Created root partition".to_string(),
                        "SUCCESS: Partitioning complete".to_string(),
                        "[2024-01-15 14:30:10] Phase 2: Formatting partitions".to_string(),
                        "[2024-01-15 14:30:12] Formatted /dev/sda1 as FAT32".to_string(),
                        "[2024-01-15 14:30:15] Formatted /dev/sda2 as ext4".to_string(),
                        "SUCCESS: Formatting complete".to_string(),
                    ],
                    scroll_offset: 0,
                    auto_scroll: false,
                    complete: true,
                    progress: None,
                    status: "Log viewer — press Esc to close".to_string(),
                });
                state.mode = AppMode::FloatingOutput;
                return;
            }
            _ => return,
        },
        // =================================================================
        // User Tools
        // =================================================================
        "user" => match index {
            // 0: Add User — shell auto-detected, groups listed
            0 => (
                "add_user",
                vec![
                    mk_param_text("username", "Username to create", true),
                    mk_param_password("password", "User password", false),
                    mk_param_text("full_name", "Full name (optional)", false),
                    mk_param_sel("groups", "Select primary group to add user to", &group_refs, false),
                    mk_param_sel("shell", "Select login shell", &shell_refs, false),
                    mk_param_bool("system_user", "Create as system user", false),
                ],
            ),
            // 1: Reset Password — users auto-detected
            1 => (
                "reset_password",
                vec![
                    mk_param_sel("username", "Select user to reset password for", &user_refs, true),
                    mk_param_password("password", "New password", true),
                ],
            ),
            // 2: Manage Groups — users and groups auto-detected
            2 => (
                "manage_groups",
                vec![
                    mk_param_sel("action", "add: add user to group | remove: remove from group | list: show memberships",
                        &["add", "remove", "list"], true),
                    mk_param_sel("user", "Select target user", &user_refs, true),
                    mk_param_sel("group", "Select group", &group_refs, true),
                ],
            ),
            // 3: Configure SSH
            3 => (
                "configure_ssh",
                vec![mk_param_sel("action", "status: check sshd | install: install openssh | enable/disable: toggle service",
                    &["status", "install", "enable", "disable", "configure"], true)],
            ),
            // 4: Security Audit
            4 => (
                "security_audit",
                vec![mk_param_sel("action", "basic: quick permission/service checks | full: comprehensive audit",
                    &["basic", "full"], true)],
            ),
            // 5: Install Dotfiles — users auto-detected
            5 => (
                "install_dotfiles",
                vec![
                    mk_param_text("repo_url", "Git repository URL", true),
                    mk_param_sel("target_user", "Select target user for dotfiles", &user_refs, true),
                    mk_param_text("branch", "Branch to clone (optional, default: main)", false),
                    mk_param_bool_default("backup", "Backup existing files before overwriting", true, false),
                ],
            ),
            // 6: Run As User — users auto-detected
            6 => (
                "run_as_user",
                vec![
                    mk_param_sel("user", "Select user to run command as", &user_refs, true),
                    mk_param_text("command", "Command to execute", true),
                    mk_param_text_default("root", "Chroot root path", "/mnt", true),
                    mk_param_text("work_dir", "Working directory inside chroot (optional)", false),
                ],
            ),
            _ => return,
        },
        // =================================================================
        // Network Tools
        // =================================================================
        "network" => match index {
            // 0: Configure Network — interfaces auto-detected
            0 => (
                "configure_network",
                vec![
                    mk_param_sel("interface", "Select network interface", &iface_refs, true),
                    mk_param_sel("action", "configure: set IP | status/info: view state | enable/disable: toggle interface",
                        &["configure", "status", "info", "enable", "disable"], true),
                    mk_param_sel("config_type", "DHCP for automatic or static for manual IP assignment",
                        &["", "dhcp", "static"], false),
                    mk_param_text("ip", "IP address (for static configuration)", false),
                    mk_param_text("netmask", "Network mask (e.g., 255.255.255.0 or 24)", false),
                    mk_param_text("gateway", "Default gateway (for static configuration)", false),
                ],
            ),
            // 1: Test Connectivity
            1 => (
                "test_network",
                vec![
                    mk_param_sel("action", "ping: ICMP test | dns: name resolution | http: web access | full: all tests",
                        &["full", "ping", "dns", "http"], true),
                    mk_param_sel("timeout", "Timeout in seconds for each test",
                        &["5", "10", "30"], false),
                ],
            ),
            // 2: Firewall Rules
            2 => (
                "configure_firewall",
                vec![mk_param_sel("action", "status: show rules | enable: apply defaults | disable: permissive | rules: list numbered",
                    &["status", "enable", "disable", "rules"], true)],
            ),
            // 3: Network Info
            3 => (
                "network_diagnostics",
                vec![mk_param_sel("action", "info: interfaces | basic: quick check | detailed: full analysis | troubleshoot: diagnose issues",
                    &["info", "basic", "detailed", "troubleshoot"], true)],
            ),
            // 4: Update Mirrors
            4 => (
                "update_mirrors",
                vec![
                    mk_param_text("country", "Country filter (ISO 3166-1 code, e.g., US, DE)", false),
                    mk_param_sel_idx("limit", "Number of mirrors to keep",
                        &["10", "20", "50"], 1, true),
                    mk_param_sel("sort", "rate: download speed | age: last sync time | score: mirror score",
                        &["rate", "age", "score"], true),
                ],
            ),
            _ => return,
        },
        _ => return,
    };

    let param_values: Vec<String> = params
        .iter()
        .map(|p| match &p.param_type {
            ToolParameter::Text(default) => default.clone(),
            ToolParameter::Selection(opts, idx) => {
                opts.get(*idx).cloned().unwrap_or_default()
            }
            ToolParameter::Password(_) => String::new(),
            ToolParameter::Boolean(b) => b.to_string(),
            ToolParameter::Number(n) => n.to_string(),
        })
        .collect();

    // If first param is a device/disk selection, populate disk layout from its default value
    if category == "disk" || (category == "system" && index == 0) {
        let first_val = param_values.first().map(|s| s.as_str()).unwrap_or("");
        if !first_val.is_empty() && first_val.starts_with("/dev/") {
            state.disk_layout = config::get_disk_layout(first_val);
        } else {
            state.disk_layout.clear();
        }
    }

    state.pre_dialog_mode = Some(state.mode.clone());
    state.tool_dialog = Some(ToolDialogState {
        tool_name: tool_name.to_string(),
        parameters: params,
        current_param: 0,
        param_values,
        is_executing: false,
    });
    state.mode = AppMode::ToolDialog;
}

// Parameter builder helpers
fn mk_param_text(name: &str, desc: &str, required: bool) -> ToolParam {
    ToolParam {
        name: name.to_string(),
        description: desc.to_string(),
        param_type: ToolParameter::Text(String::new()),
        required,
    }
}

fn mk_param_sel(name: &str, desc: &str, opts: &[&str], required: bool) -> ToolParam {
    mk_param_sel_idx(name, desc, opts, 0, required)
}

fn mk_param_sel_idx(name: &str, desc: &str, opts: &[&str], default_idx: usize, required: bool) -> ToolParam {
    let options: Vec<String> = opts.iter().map(|s| s.to_string()).collect();
    ToolParam {
        name: name.to_string(),
        description: desc.to_string(),
        param_type: ToolParameter::Selection(options, default_idx),
        required,
    }
}

fn mk_param_password(name: &str, desc: &str, required: bool) -> ToolParam {
    ToolParam {
        name: name.to_string(),
        description: desc.to_string(),
        param_type: ToolParameter::Password(String::new()),
        required,
    }
}

fn mk_param_bool(name: &str, desc: &str, required: bool) -> ToolParam {
    ToolParam {
        name: name.to_string(),
        description: desc.to_string(),
        param_type: ToolParameter::Boolean(false),
        required,
    }
}

fn mk_param_bool_default(name: &str, desc: &str, default: bool, required: bool) -> ToolParam {
    ToolParam {
        name: name.to_string(),
        description: desc.to_string(),
        param_type: ToolParameter::Boolean(default),
        required,
    }
}

fn mk_param_text_default(name: &str, desc: &str, default: &str, required: bool) -> ToolParam {
    ToolParam {
        name: name.to_string(),
        description: desc.to_string(),
        param_type: ToolParameter::Text(default.to_string()),
        required,
    }
}

// =============================================================================
// Screen state creation (Tab cycling)
// =============================================================================

fn create_screen_state(mode: &AppMode) -> AppState {
    match mode {
        AppMode::Installation => AppState::demo_installation(),
        AppMode::Complete => AppState::demo_complete(),
        AppMode::ToolDialog => AppState::demo_tool_dialog(),
        AppMode::FloatingOutput => AppState::demo_floating_output(),
        AppMode::ConfirmDialog => AppState::demo_confirm_dialog(),
        AppMode::FileBrowser => AppState::demo_file_browser(),
        AppMode::DryRunSummary => AppState::demo_dry_run_summary(),
        mode => AppState {
            mode: mode.clone(),
            tools_menu_selection: 0,
            ..AppState::default()
        },
    }
}

// =============================================================================
// Config option helpers
// =============================================================================

/// Get a config option's current value by name
/// Sync scroll states to match current terminal height
fn handle_resize(state: &mut AppState, height: u16) {
    let h = height as usize;

    // Always keep GuidedInstaller scroll state in sync (even when not active,
    // so it's correct when we switch to it)
    // Breadcrumb(1) + border top(1) + border bottom(1) + button bar(3) + nav bar(1) = 7
    let available = h.saturating_sub(7);
    let visible = available.max(5);
    state.config_scroll.update_visible_items(visible);

    // Installer output area
    // Breadcrumb(1) + progress(3) + phases(1) + status(1) + borders(2) = 8
    state.installer_visible_height = h.saturating_sub(8).max(3);
}

fn get_option_value(config: &config::Configuration, name: &str) -> String {
    config
        .options
        .iter()
        .find(|o| o.name == name)
        .map(|o| o.get_value())
        .unwrap_or_default()
}

/// Set a config option's value by name
fn set_option_value(config: &mut config::Configuration, name: &str, value: &str) {
    if let Some(opt) = config.options.iter_mut().find(|o| o.name == name) {
        opt.value = value.to_string();
    }
}

// =============================================================================
// Package search
// =============================================================================

/// Search packages via pacman -Ss (real) or mock results for AUR
fn search_packages(term: &str, is_pacman: bool) -> Vec<PackageResult> {
    if is_pacman {
        // Try real pacman -Ss
        if let Ok(output) = std::process::Command::new("pacman")
            .args(["-Ss", term])
            .output()
        {
            if output.status.success() {
                return parse_pacman_results(&String::from_utf8_lossy(&output.stdout));
            }
        }
    }

    // Fallback: mock results (for AUR or if pacman unavailable)
    let mock_packages: &[(&str, &str, &str, &str)] = if is_pacman {
        &[
            ("extra", term, "1.0-1", "Package matching search term"),
            ("extra", &format!("{}-utils", term), "2.1-1", "Utility tools"),
            ("community", &format!("lib{}", term), "0.5-3", "Library package"),
        ]
    } else {
        &[
            ("aur", &format!("{}-bin", term), "1.0-1", "Pre-built binary package"),
            ("aur", &format!("{}-git", term), "r100-1", "Development version from git"),
        ]
    };

    mock_packages
        .iter()
        .map(|(repo, name, ver, desc)| PackageResult {
            repo: repo.to_string(),
            name: name.to_string(),
            version: ver.to_string(),
            description: desc.to_string(),
        })
        .collect()
}

/// Parse pacman -Ss output into PackageResult list
fn parse_pacman_results(output: &str) -> Vec<PackageResult> {
    let mut results = Vec::new();
    let mut lines = output.lines().peekable();

    while let Some(header) = lines.next() {
        let header = header.trim();
        if header.is_empty() {
            continue;
        }

        // Header line format: "repo/name version [installed]"
        let description = lines
            .next()
            .map(|l| l.trim().to_string())
            .unwrap_or_default();

        // Parse "repo/name version"
        let parts: Vec<&str> = header.splitn(2, ' ').collect();
        if parts.is_empty() {
            continue;
        }

        let repo_name: Vec<&str> = parts[0].splitn(2, '/').collect();
        if repo_name.len() < 2 {
            continue;
        }

        let version = if parts.len() > 1 {
            parts[1]
                .split_whitespace()
                .next()
                .unwrap_or("?")
                .to_string()
        } else {
            "?".to_string()
        };

        results.push(PackageResult {
            repo: repo_name[0].to_string(),
            name: repo_name[1].to_string(),
            version,
            description,
        });

        // Limit results to prevent huge lists
        if results.len() >= 50 {
            break;
        }
    }

    results
}

// =============================================================================
// Config editing (Guided Installer right panel)
// =============================================================================

/// Check gating rules — returns Some(message) if the option is gated (blocked)
fn check_gating(config: &config::Configuration, option_name: &str) -> Option<String> {
    let val = |name: &str| -> String { get_option_value(config, name) };

    match option_name {
        // Secure Boot: show detailed UEFI warning
        "Secure Boot" => {
            let boot_mode = val("Boot Mode");
            if boot_mode == "BIOS" {
                return Some(
                    "Secure Boot requires UEFI firmware! BIOS mode does not support Secure Boot.".to_string(),
                );
            }
            // Even if allowed, set a warning as status
        }

        // Timezone: requires region to be selected first
        "Timezone" => {
            let region = val("Timezone Region");
            if region.is_empty() {
                return Some(
                    "Please select a timezone region first.".to_string(),
                );
            }
        }

        // Swap Size: only when Swap == "Yes"
        "Swap Size" => {
            if val("Swap") != "Yes" {
                return Some(
                    "Swap size can only be configured when swap is enabled.".to_string(),
                );
            }
        }

        // Root Size: only when Separate Home == "Yes" OR LVM strategy
        "Root Size" => {
            let strategy = val("Partitioning Strategy");
            let home = val("Separate Home Partition");
            if home != "Yes" && !strategy.contains("lvm") {
                return Some(
                    "Root size is configurable when using a separate home partition or LVM strategy."
                        .to_string(),
                );
            }
        }

        // Home Size: only when Separate Home == "Yes"
        "Home Size" => {
            if val("Separate Home Partition") != "Yes" {
                return Some(
                    "Home size is only configurable when separate home partition is enabled."
                        .to_string(),
                );
            }
        }

        // Home Filesystem: only when Separate Home == "Yes"
        "Home Filesystem" => {
            if val("Separate Home Partition") != "Yes" {
                return Some(
                    "Home filesystem can only be selected when Separate Home Partition is enabled."
                        .to_string(),
                );
            }
        }

        // Btrfs Snapshots: only when Root Filesystem == "btrfs"
        "Btrfs Snapshots" => {
            if val("Root Filesystem") != "btrfs" {
                return Some(
                    "Btrfs Snapshots requires Root Filesystem to be btrfs.".to_string(),
                );
            }
        }

        // Btrfs Frequency/Keep Count/Assistant: Root FS == "btrfs" AND Snapshots == "Yes"
        "Btrfs Frequency" | "Btrfs Keep Count" | "Btrfs Assistant" => {
            if val("Root Filesystem") != "btrfs" {
                return Some(
                    "Btrfs options are only available when Root Filesystem is btrfs.".to_string(),
                );
            }
            if val("Btrfs Snapshots") != "Yes" {
                return Some(format!(
                    "{} can only be configured when Btrfs snapshots are enabled.",
                    option_name
                ));
            }
        }

        // GRUB Theme: only when Bootloader == "grub"
        "GRUB Theme" => {
            if val("Bootloader") != "grub" {
                return Some(
                    "GRUB theme options are only available with the GRUB bootloader.".to_string(),
                );
            }
        }

        // GRUB Theme Selection: Bootloader == "grub" AND GRUB Theme == "Yes"
        "GRUB Theme Selection" => {
            if val("Bootloader") != "grub" {
                return Some(
                    "GRUB theme options are only available with the GRUB bootloader.".to_string(),
                );
            }
            if val("GRUB Theme") != "Yes" {
                return Some(
                    "GRUB theme selection is only available when GRUB themes are enabled."
                        .to_string(),
                );
            }
        }

        // OS Prober: only when Bootloader == "grub"
        "OS Prober" => {
            if val("Bootloader") != "grub" {
                return Some(
                    "OS Prober is only available with the GRUB bootloader.".to_string(),
                );
            }
        }

        // Git Repository URL: only when Git Repository == "Yes"
        "Git Repository URL" => {
            if val("Git Repository") != "Yes" {
                return Some(
                    "Git repository URL can only be configured when git repository is enabled."
                        .to_string(),
                );
            }
        }

        // Encryption Password: only when Encryption != "No"
        "Encryption Password" => {
            if val("Encryption") == "No" {
                return Some(
                    "Encryption password is only needed when encryption is enabled.".to_string(),
                );
            }
        }

        // Plymouth Theme: only when Plymouth == "Yes"
        "Plymouth Theme" => {
            if val("Plymouth") != "Yes" {
                return Some(
                    "Plymouth theme can only be selected when Plymouth is enabled.".to_string(),
                );
            }
        }

        // Encryption: auto-set for non-manual strategies
        "Encryption" => {
            let strategy = val("Partitioning Strategy");
            if strategy != "manual" {
                return Some(
                    "Encryption is auto-set based on partitioning strategy. Use manual partitioning to control encryption."
                        .to_string(),
                );
            }
        }

        // Separate Home Partition: blocked for plain RAID (without LVM)
        "Separate Home Partition" => {
            let strategy = val("Partitioning Strategy");
            if (strategy == "auto_raid" || strategy == "auto_raid_luks")
                && !strategy.contains("lvm")
            {
                return Some(
                    "Separate home not available for RAID without LVM.".to_string(),
                );
            }
        }

        // RAID Level: only for RAID strategies
        "RAID Level" => {
            if !val("Partitioning Strategy").contains("raid") {
                return Some(
                    "RAID Level is only available for RAID partitioning strategies.".to_string(),
                );
            }
        }

        // Display Manager: auto-set when DE != "none"
        "Display Manager" => {
            let de = val("Desktop Environment");
            if de != "none" && !de.is_empty() {
                return Some(
                    "Display Manager is auto-set based on Desktop Environment selection."
                        .to_string(),
                );
            }
        }

        _ => {}
    }

    None
}

/// Refresh disk layout when browsing the Disk config option's Selection choices
fn refresh_disk_layout_for_config(state: &mut AppState) {
    let sel_idx = state.config_scroll.selected_index;
    if let Some(opt) = state.config.options.get(sel_idx) {
        if opt.name == "Disk" {
            if let ConfigEditState::Selection { ref choices, selected } = state.config_edit {
                if let Some(choice) = choices.get(selected) {
                    if choice.starts_with("/dev/") {
                        state.disk_layout = config::get_disk_layout(choice);
                    }
                }
            }
        }
    }
}

/// Run cascading updates after a config value changes
fn handle_cascading(config: &mut config::Configuration, changed_name: &str) {
    let val = |cfg: &config::Configuration, name: &str| -> String {
        get_option_value(cfg, name)
    };

    match changed_name {
        "Swap" => {
            if val(config, "Swap") == "Yes" {
                set_option_value(config, "Swap Size", "2GB");
            } else {
                set_option_value(config, "Swap Size", "N/A");
            }
        }

        "Separate Home Partition" => {
            if val(config, "Separate Home Partition") == "Yes" {
                set_option_value(config, "Root Size", "50GB");
                set_option_value(config, "Home Size", "Remaining");
                // Home filesystem matches root
                let root_fs = val(config, "Root Filesystem");
                set_option_value(config, "Home Filesystem", &root_fs);
            } else {
                set_option_value(config, "Root Size", "Remaining");
                set_option_value(config, "Home Size", "N/A");
                set_option_value(config, "Home Filesystem", "N/A");
            }
        }

        "Partitioning Strategy" => {
            let strategy = val(config, "Partitioning Strategy");

            // Auto-set encryption based on strategy
            if strategy != "manual" {
                if strategy.contains("luks") {
                    set_option_value(config, "Encryption", "Yes");
                    // Clear encryption password for user to set
                    set_option_value(config, "Encryption Password", "");
                } else {
                    set_option_value(config, "Encryption", "No");
                    set_option_value(config, "Encryption Password", "N/A");
                }
            }

            // RAID level
            if strategy.contains("raid") {
                let rl = val(config, "RAID Level");
                if rl == "N/A" || rl.is_empty() {
                    set_option_value(config, "RAID Level", "raid1");
                }
            } else {
                set_option_value(config, "RAID Level", "N/A");
            }

            // Plain RAID forces Separate Home = No
            if (strategy == "auto_raid" || strategy == "auto_raid_luks")
                && !strategy.contains("lvm")
            {
                set_option_value(config, "Separate Home Partition", "No");
                set_option_value(config, "Root Size", "Remaining");
                set_option_value(config, "Home Size", "N/A");
                set_option_value(config, "Home Filesystem", "N/A");
            }
        }

        "Root Filesystem" => {
            let fs = val(config, "Root Filesystem");
            if fs != "btrfs" {
                set_option_value(config, "Btrfs Snapshots", "No");
                set_option_value(config, "Btrfs Frequency", "N/A");
                set_option_value(config, "Btrfs Keep Count", "N/A");
                set_option_value(config, "Btrfs Assistant", "No");
            }
            // Update home filesystem if separate home is enabled
            if val(config, "Separate Home Partition") == "Yes" {
                set_option_value(config, "Home Filesystem", &fs);
            }
        }

        "Btrfs Snapshots" => {
            if val(config, "Btrfs Snapshots") == "Yes" {
                set_option_value(config, "Btrfs Frequency", "weekly");
                set_option_value(config, "Btrfs Keep Count", "3");
            } else {
                set_option_value(config, "Btrfs Frequency", "N/A");
                set_option_value(config, "Btrfs Keep Count", "N/A");
                set_option_value(config, "Btrfs Assistant", "No");
            }
        }

        "Encryption" => {
            if val(config, "Encryption") == "No" {
                set_option_value(config, "Encryption Password", "N/A");
            } else {
                let current = val(config, "Encryption Password");
                if current == "N/A" {
                    set_option_value(config, "Encryption Password", "");
                }
            }
        }

        "Plymouth" => {
            if val(config, "Plymouth") != "Yes" {
                set_option_value(config, "Plymouth Theme", "none");
            }
        }

        "GRUB Theme" => {
            if val(config, "GRUB Theme") != "Yes" {
                set_option_value(config, "GRUB Theme Selection", "none");
            }
        }

        "Bootloader" => {
            if val(config, "Bootloader") != "grub" {
                set_option_value(config, "GRUB Theme", "No");
                set_option_value(config, "GRUB Theme Selection", "none");
                set_option_value(config, "OS Prober", "No");
            }
        }

        "Git Repository" => {
            if val(config, "Git Repository") != "Yes" {
                set_option_value(config, "Git Repository URL", "");
            }
        }

        "Desktop Environment" => {
            let de = val(config, "Desktop Environment");
            // Auto-set display manager
            let dm = match de.as_str() {
                "kde" | "sway" | "hyprland" => "sddm",
                "gnome" | "budgie" => "gdm",
                "i3" | "xfce" | "cinnamon" | "mate" => "lightdm",
                "none" | "" => "none",
                _ => "none",
            };
            set_option_value(config, "Display Manager", dm);

            // Auto-set AUR helper if DE requires it
            let requires_aur = matches!(de.as_str(), "hyprland");
            if requires_aur && val(config, "AUR Helper") == "none" {
                set_option_value(config, "AUR Helper", "paru");
            }
        }

        "Timezone Region" => {
            let region = val(config, "Timezone Region");
            // Clear timezone city and update available options
            set_option_value(config, "Timezone", "");
            // Update timezone option list based on selected region
            if let Some(tz_opt) = config.options.iter_mut().find(|o| o.name == "Timezone") {
                tz_opt.options = config::get_timezones_for_region(&region);
            }
            // Auto-set mirror country based on region
            let mirror = match region.as_str() {
                "US" | "America" => "United States",
                "Europe" => "Germany",
                "Asia" => "Japan",
                "Australia" => "Australia",
                _ => "",
            };
            if !mirror.is_empty() {
                set_option_value(config, "Mirror Country", mirror);
            }
        }

        "Boot Mode" => {
            // If switching to BIOS, force Secure Boot off
            if val(config, "Boot Mode") == "BIOS" {
                set_option_value(config, "Secure Boot", "No");
            }
        }

        _ => {}
    }
}

/// Open the right panel editor for the currently selected config option
fn open_config_edit(state: &mut AppState) {
    let sel = state.config_scroll.selected_index;
    let option = match state.config.options.get(sel) {
        Some(opt) => opt.clone(),
        None => return,
    };

    // Check gating — if blocked, show status message instead of opening editor
    if let Some(msg) = check_gating(&state.config, &option.name) {
        state.status_message = msg;
        return;
    }

    // Secure Boot: show warning as status, then allow selection
    if option.name == "Secure Boot" {
        state.status_message =
            "WARNING: Secure Boot requires UEFI firmware setup. Ensure your motherboard supports UEFI and Secure Boot is configured in BIOS/UEFI settings."
                .to_string();
    }

    // Additional Packages: open interactive package selection
    if option.name == "Additional Pacman Packages" || option.name == "Additional AUR Packages" {
        let current_packages: Vec<String> = if option.value.is_empty() {
            Vec::new()
        } else {
            option.value.split_whitespace().map(|s| s.to_string()).collect()
        };
        let is_pacman = option.name.contains("Pacman");
        let mut output = vec![
            format!(
                "==> Interactive {} Package Selection",
                if is_pacman { "Pacman" } else { "AUR" }
            ),
            "Commands: search <term> | add <pkg> | remove <pkg> | list | done".to_string(),
            String::new(),
        ];
        if !current_packages.is_empty() {
            output.push(format!("Currently selected: {}", current_packages.join(" ")));
        } else {
            output.push("No packages selected yet.".to_string());
        }
        state.config_edit = ConfigEditState::PackageInput {
            packages: current_packages,
            current_input: String::new(),
            output_lines: output,
            is_pacman,
            search_results: Vec::new(),
            results_selected: 0,
            show_search_results: false,
        };
        return;
    }

    // Timezone: dynamically load options from selected region
    if option.name == "Timezone" {
        let region = get_option_value(&state.config, "Timezone Region");
        let cities = config::get_timezones_for_region(&region);
        let current_idx = cities.iter().position(|c| *c == option.value).unwrap_or(0);
        state.config_edit = ConfigEditState::Selection {
            choices: cities,
            selected: current_idx,
        };
        return;
    }

    if !option.options.is_empty() {
        // Selection: find current value's index in choices
        let current_idx = option
            .options
            .iter()
            .position(|o| *o == option.value)
            .unwrap_or(0);
        state.config_edit = ConfigEditState::Selection {
            choices: option.options.clone(),
            selected: current_idx,
        };
        // Populate disk layout when opening the Disk option
        if option.name == "Disk" {
            if let Some(choice) = option.options.get(current_idx) {
                if choice.starts_with("/dev/") {
                    state.disk_layout = config::get_disk_layout(choice);
                }
            }
        }
    } else if option.is_password() {
        state.config_edit = ConfigEditState::PasswordInput {
            value: if option.value == "***" || option.value == "N/A" {
                String::new()
            } else {
                option.value.clone()
            },
            cursor: 0,
        };
    } else {
        // Text input
        let val = option.value.clone();
        let cursor = val.len();
        state.config_edit = ConfigEditState::TextInput {
            value: val,
            cursor,
        };
    }
}

/// Handle key events when in config edit mode
fn handle_config_edit(state: &mut AppState, code: KeyCode) {
    match &mut state.config_edit {
        ConfigEditState::Selection {
            ref choices,
            ref mut selected,
        } => match code {
            KeyCode::Up => {
                if *selected > 0 {
                    *selected -= 1;
                }
                // Refresh disk layout when browsing disk options
                refresh_disk_layout_for_config(state);
            }
            KeyCode::Down => {
                if *selected < choices.len().saturating_sub(1) {
                    *selected += 1;
                }
                refresh_disk_layout_for_config(state);
            }
            KeyCode::Enter => {
                // Confirm selection: update config value and run cascading
                let sel_idx = state.config_scroll.selected_index;
                if let ConfigEditState::Selection { choices, selected } = &state.config_edit {
                    if let Some(choice) = choices.get(*selected) {
                        if let Some(opt) = state.config.options.get_mut(sel_idx) {
                            opt.value = choice.clone();
                        }
                    }
                }
                // Run cascading after value commit
                if let Some(opt) = state.config.options.get(sel_idx) {
                    let name = opt.name.clone();
                    handle_cascading(&mut state.config, &name);
                }
                state.config_edit = ConfigEditState::None;
            }
            KeyCode::Esc => {
                state.config_edit = ConfigEditState::None;
            }
            _ => {}
        },

        ConfigEditState::TextInput {
            ref mut value,
            ref mut cursor,
        }
        | ConfigEditState::PasswordInput {
            ref mut value,
            ref mut cursor,
        } => match code {
            KeyCode::Char(c) => {
                value.insert(*cursor, c);
                *cursor += 1;
            }
            KeyCode::Backspace => {
                if *cursor > 0 {
                    *cursor -= 1;
                    value.remove(*cursor);
                }
            }
            KeyCode::Delete => {
                if *cursor < value.len() {
                    value.remove(*cursor);
                }
            }
            KeyCode::Left => {
                if *cursor > 0 {
                    *cursor -= 1;
                }
            }
            KeyCode::Right => {
                if *cursor < value.len() {
                    *cursor += 1;
                }
            }
            KeyCode::Home => {
                *cursor = 0;
            }
            KeyCode::End => {
                *cursor = value.len();
            }
            KeyCode::Enter => {
                // Confirm: update config value and run cascading
                let sel_idx = state.config_scroll.selected_index;
                let final_value = match &state.config_edit {
                    ConfigEditState::TextInput { value, .. }
                    | ConfigEditState::PasswordInput { value, .. } => value.clone(),
                    _ => String::new(),
                };
                if let Some(opt) = state.config.options.get_mut(sel_idx) {
                    opt.value = final_value;
                }
                if let Some(opt) = state.config.options.get(sel_idx) {
                    let name = opt.name.clone();
                    handle_cascading(&mut state.config, &name);
                }
                state.config_edit = ConfigEditState::None;
            }
            KeyCode::Esc => {
                state.config_edit = ConfigEditState::None;
            }
            _ => {}
        },

        ConfigEditState::PackageInput {
            ref mut packages,
            ref mut current_input,
            ref mut output_lines,
            ref is_pacman,
            ref mut search_results,
            ref mut results_selected,
            ref mut show_search_results,
        } => {
            if *show_search_results {
                // Search results browsing mode: arrow keys + Enter to toggle
                match code {
                    KeyCode::Up => {
                        if *results_selected > 0 {
                            *results_selected -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if *results_selected < search_results.len().saturating_sub(1) {
                            *results_selected += 1;
                        }
                    }
                    KeyCode::Enter => {
                        // Toggle package selection
                        if let Some(result) = search_results.get(*results_selected) {
                            let pkg_name = result.name.clone();
                            if let Some(pos) = packages.iter().position(|p| p == &pkg_name) {
                                packages.remove(pos);
                                output_lines.push(format!("  - Removed: {}", pkg_name));
                            } else {
                                packages.push(pkg_name.clone());
                                output_lines.push(format!("  + Added: {}", pkg_name));
                            }
                        }
                    }
                    KeyCode::Esc => {
                        // Return to command mode
                        *show_search_results = false;
                        search_results.clear();
                        *results_selected = 0;
                    }
                    _ => {}
                }
            } else {
                // Command mode
                match code {
                    KeyCode::Char(c) => {
                        current_input.push(c);
                    }
                    KeyCode::Backspace => {
                        current_input.pop();
                    }
                    KeyCode::Enter => {
                        let cmd = current_input.trim().to_string();
                        current_input.clear();

                        if cmd == "done" {
                            let sel_idx = state.config_scroll.selected_index;
                            let pkg_string = packages.join(" ");
                            if let Some(opt) = state.config.options.get_mut(sel_idx) {
                                opt.value = pkg_string;
                            }
                            state.config_edit = ConfigEditState::None;
                            return;
                        }

                        if cmd == "list" {
                            output_lines.push(String::new());
                            if packages.is_empty() {
                                output_lines.push("No packages selected.".to_string());
                            } else {
                                output_lines.push(format!("Selected ({}):", packages.len()));
                                for pkg in packages.iter() {
                                    output_lines.push(format!("  * {}", pkg));
                                }
                            }
                        } else if let Some(pkg) = cmd.strip_prefix("add ") {
                            let pkg = pkg.trim().to_string();
                            if !pkg.is_empty() && !packages.contains(&pkg) {
                                packages.push(pkg.clone());
                                output_lines.push(format!("  + Added: {}", pkg));
                            } else if packages.contains(&pkg) {
                                output_lines.push(format!("Already selected: {}", pkg));
                            }
                        } else if let Some(pkg) = cmd.strip_prefix("remove ") {
                            let pkg = pkg.trim();
                            if let Some(pos) = packages.iter().position(|p| p == pkg) {
                                packages.remove(pos);
                                output_lines.push(format!("  - Removed: {}", pkg));
                            } else {
                                output_lines.push(format!("Not found: {}", pkg));
                            }
                        } else if let Some(term) = cmd.strip_prefix("search ") {
                            let term = term.trim().to_string();
                            output_lines.push(format!(">>> Searching for '{}'...", term));

                            let results = search_packages(&term, *is_pacman);
                            if results.is_empty() {
                                output_lines.push("No results found.".to_string());
                            } else {
                                output_lines.push(format!(
                                    "Found {} results. Use \u{2191}\u{2193} to browse, Enter to toggle, Esc to return.",
                                    results.len()
                                ));
                                *search_results = results;
                                *results_selected = 0;
                                *show_search_results = true;
                            }
                        } else if !cmd.is_empty() {
                            output_lines.push(format!(
                                "Unknown command: {}. Use search/add/remove/list/done.",
                                cmd
                            ));
                        }
                    }
                    KeyCode::Esc => {
                        state.config_edit = ConfigEditState::None;
                    }
                    _ => {}
                }
            }
        }

        ConfigEditState::None => {}
    }
}
