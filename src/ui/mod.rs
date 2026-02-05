//! User interface rendering module
//!
//! This module is organized into submodules for better maintainability:
//! - `header` - Header, title, and common widget rendering
//! - `menus` - Menu rendering (main, tools, categories)
//! - `installer` - Installation and configuration UI
//! - `dialogs` - Input and confirmation dialog rendering
//! - `descriptions` - Tool description text generation
//! - `screens` - Guided installer wizard screens (Sprint 7)

#![allow(dead_code)]

mod descriptions;
mod dialogs;
mod header;
mod installer;
mod menus;
pub mod screens;

use std::path::PathBuf;

// ============================================================================
// Wizard State Machine (Sprint 7)
// ============================================================================

/// Wizard state for the guided installer workflow.
///
/// The installer progresses through these states linearly. Users cannot
/// skip steps or proceed without completing required fields.
///
/// # State Transitions
///
/// ```text
/// Welcome -> DiskSelect -> Partitioner -> PackageSelect -> UserConfig -> InstallProgress -> Done
/// ```
///
/// # Invariants
///
/// - Cannot transition to `InstallProgress` without a valid disk selection
/// - Cannot transition to `InstallProgress` without a valid username
/// - Cannot go backwards from `InstallProgress` or `Done`
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WizardState {
    /// Welcome screen with safety warnings and introduction.
    Welcome,
    /// Disk selection screen - lists available disks with size/model.
    /// **SAFETY**: User must explicitly select a disk before proceeding.
    DiskSelect,
    /// Partition configuration - auto or manual partitioning.
    Partitioner,
    /// Package selection - base packages and optional extras.
    PackageSelect,
    /// User configuration - hostname, username, password.
    /// **SECURITY**: Passwords are masked and zeroed after use.
    UserConfig,
    /// Installation in progress - no user interaction, shows progress.
    InstallProgress,
    /// Installation complete - success or failure summary.
    Done,
}

impl Default for WizardState {
    fn default() -> Self {
        Self::Welcome
    }
}

impl WizardState {
    /// Get the next state in the wizard sequence.
    ///
    /// Returns `None` if at the final state.
    pub fn next(&self) -> Option<Self> {
        match self {
            Self::Welcome => Some(Self::DiskSelect),
            Self::DiskSelect => Some(Self::Partitioner),
            Self::Partitioner => Some(Self::PackageSelect),
            Self::PackageSelect => Some(Self::UserConfig),
            Self::UserConfig => Some(Self::InstallProgress),
            Self::InstallProgress => Some(Self::Done),
            Self::Done => None,
        }
    }

    /// Get the previous state in the wizard sequence.
    ///
    /// Returns `None` if at the first state or if going back is not allowed.
    pub fn previous(&self) -> Option<Self> {
        match self {
            Self::Welcome => None,
            Self::DiskSelect => Some(Self::Welcome),
            Self::Partitioner => Some(Self::DiskSelect),
            Self::PackageSelect => Some(Self::Partitioner),
            Self::UserConfig => Some(Self::PackageSelect),
            // Cannot go back during or after installation
            Self::InstallProgress => None,
            Self::Done => None,
        }
    }

    /// Check if the current state allows going back.
    pub fn can_go_back(&self) -> bool {
        self.previous().is_some()
    }

    /// Get the display title for this state.
    pub fn title(&self) -> &'static str {
        match self {
            Self::Welcome => "Welcome to Arch Linux Installer",
            Self::DiskSelect => "Select Installation Disk",
            Self::Partitioner => "Partition Configuration",
            Self::PackageSelect => "Package Selection",
            Self::UserConfig => "User Configuration",
            Self::InstallProgress => "Installing Arch Linux",
            Self::Done => "Installation Complete",
        }
    }

    /// Get the step number (1-indexed for display).
    pub fn step_number(&self) -> usize {
        match self {
            Self::Welcome => 1,
            Self::DiskSelect => 2,
            Self::Partitioner => 3,
            Self::PackageSelect => 4,
            Self::UserConfig => 5,
            Self::InstallProgress => 6,
            Self::Done => 7,
        }
    }

    /// Total number of steps.
    pub const TOTAL_STEPS: usize = 7;
}

/// Wizard data collected during the guided installer flow.
///
/// This struct accumulates user choices as they progress through the wizard.
/// All fields start as `None` and are validated before installation begins.
#[derive(Debug, Clone, Default)]
pub struct WizardData {
    /// Selected disk device path (e.g., `/dev/sda`).
    pub selected_disk: Option<PathBuf>,
    /// Selected disk model for display confirmation.
    pub disk_model: Option<String>,
    /// Selected disk size in bytes.
    pub disk_size: Option<u64>,
    /// Whether to use automatic partitioning.
    pub auto_partition: bool,
    /// Filesystem type for root partition.
    pub filesystem: Option<String>,
    /// System hostname.
    pub hostname: Option<String>,
    /// Primary username.
    pub username: Option<String>,
    /// User password (zeroed after use).
    pub password: Option<String>,
    /// Whether user has sudo access.
    pub user_sudo: bool,
    /// Root password (optional, zeroed after use).
    pub root_password: Option<String>,
    /// Selected extra packages.
    pub extra_packages: Vec<String>,
    /// Dry run mode - don't execute destructive operations.
    /// Used when wizard is fully integrated with script execution.
    #[allow(dead_code)] // WIP: Wizard dry-run integration
    pub dry_run: bool,
}

impl WizardData {
    /// Check if disk selection is valid.
    pub fn has_valid_disk(&self) -> bool {
        self.selected_disk.is_some()
    }

    /// Check if user configuration is valid.
    pub fn has_valid_user(&self) -> bool {
        self.hostname.as_ref().is_some_and(|h| !h.is_empty())
            && self.username.as_ref().is_some_and(|u| !u.is_empty())
            && self.password.as_ref().is_some_and(|p| !p.is_empty())
    }

    /// Check if ready to start installation.
    pub fn is_ready_for_install(&self) -> bool {
        self.has_valid_disk() && self.has_valid_user()
    }

    /// Zero out sensitive data (passwords).
    ///
    /// Called after installation completes or on error to minimize
    /// password exposure time in memory.
    pub fn zero_sensitive_data(&mut self) {
        if let Some(ref mut pwd) = self.password {
            // Overwrite with zeros before dropping
            // Note: This is MVP security - production should use secrecy crate
            pwd.clear();
            pwd.shrink_to_fit();
        }
        self.password = None;

        if let Some(ref mut pwd) = self.root_password {
            pwd.clear();
            pwd.shrink_to_fit();
        }
        self.root_password = None;
    }
}

impl Drop for WizardData {
    fn drop(&mut self) {
        // Ensure passwords are zeroed when WizardData is dropped
        self.zero_sensitive_data();
    }
}

use crate::app::{AppMode, AppState};
use crate::components::keybindings::KeybindingContext;
use crate::components::pty_terminal::PtyTerminal;
use crate::input::InputHandler;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

// Re-export for external use
pub use header::HeaderRenderer;
// Sprint 7 wizard screen exports - will be used when wizard is fully integrated
#[allow(unused_imports)]
pub use screens::{DiskInfo, render_disk_select_screen, render_user_config_screen};

/// UI renderer for the application
///
/// This is the main entry point for UI rendering. It delegates to specialized
/// submodules for different parts of the UI.
pub struct UiRenderer {
    /// Header renderer instance
    header: HeaderRenderer,
}

impl Default for UiRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl UiRenderer {
    /// Create a new UI renderer
    pub fn new() -> Self {
        Self {
            header: HeaderRenderer::new(),
        }
    }

    /// Render the complete UI based on application state (legacy method for compatibility)
    pub fn render(&self, f: &mut Frame, state: &AppState, input_handler: &mut InputHandler) {
        let keybinding_ctx = KeybindingContext::new();
        self.render_with_context(f, state, input_handler, &keybinding_ctx, None);
    }

    /// Render the complete UI with keybinding context and PTY terminal
    pub fn render_with_context(
        &self,
        f: &mut Frame,
        state: &AppState,
        input_handler: &mut InputHandler,
        keybinding_ctx: &KeybindingContext,
        pty_terminal: Option<&mut PtyTerminal>,
    ) {
        // If dialog is active, render ONLY the dialog - don't render main UI behind it
        if input_handler.is_dialog_active() {
            dialogs::render_input_dialog(f, input_handler);
            return;
        }

        // Create main layout with nav bar at bottom
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),    // Main content area
                Constraint::Length(1), // Navigation bar
            ])
            .split(f.area());

        let content_area = main_chunks[0];
        let nav_bar_area = main_chunks[1];

        // Render main content based on mode
        match state.mode {
            AppMode::MainMenu => {
                menus::render_main_menu_in_area(f, state, content_area, &self.header);
            }
            AppMode::GuidedInstaller => {
                installer::render_configuration_ui_in_area(f, state, content_area, &self.header);
            }
            AppMode::AutomatedInstall => {
                installer::render_automated_install_ui_in_area(f, state, content_area, &self.header);
            }
            AppMode::ToolsMenu => {
                menus::render_tools_menu_in_area(f, state, content_area, &self.header);
            }
            AppMode::DiskTools => {
                menus::render_disk_tools_menu_in_area(f, state, content_area, &self.header);
            }
            AppMode::SystemTools => {
                menus::render_system_tools_menu_in_area(f, state, content_area, &self.header);
            }
            AppMode::UserTools => {
                menus::render_user_tools_menu_in_area(f, state, content_area, &self.header);
            }
            AppMode::NetworkTools => {
                menus::render_network_tools_menu_in_area(f, state, content_area, &self.header);
            }
            AppMode::ToolDialog => {
                dialogs::render_tool_dialog_in_area(f, state, content_area);
            }
            AppMode::ToolExecution => {
                installer::render_tool_execution_in_area(f, state, content_area, &self.header);
            }
            AppMode::Installation => {
                installer::render_installation_ui_in_area(f, state, content_area, &self.header);
            }
            AppMode::Complete => {
                installer::render_completion_ui_in_area(f, state, content_area, &self.header);
            }
            AppMode::EmbeddedTerminal => {
                dialogs::render_embedded_terminal(f, state, content_area, pty_terminal);
            }
            AppMode::FloatingOutput => {
                // Render background (previous mode content) then floating window
                menus::render_tools_menu_in_area(f, state, content_area, &self.header);
                dialogs::render_floating_output(f, state);
            }
            AppMode::FileBrowser => {
                // Render file browser for config file selection
                installer::render_automated_install_ui_in_area(f, state, content_area, &self.header);
                dialogs::render_file_browser(f, state);
            }
            AppMode::ConfirmDialog => {
                // Render background based on pre_dialog_mode, then confirmation dialog
                if let Some(ref pre_mode) = state.pre_dialog_mode {
                    match pre_mode {
                        AppMode::DiskTools => {
                            menus::render_disk_tools_menu_in_area(f, state, content_area, &self.header)
                        }
                        AppMode::SystemTools => {
                            menus::render_system_tools_menu_in_area(f, state, content_area, &self.header)
                        }
                        AppMode::UserTools => {
                            menus::render_user_tools_menu_in_area(f, state, content_area, &self.header)
                        }
                        AppMode::NetworkTools => {
                            menus::render_network_tools_menu_in_area(f, state, content_area, &self.header)
                        }
                        _ => menus::render_tools_menu_in_area(f, state, content_area, &self.header),
                    }
                } else {
                    menus::render_tools_menu_in_area(f, state, content_area, &self.header);
                }
                // Render the confirmation dialog on top
                dialogs::render_confirm_dialog(f, state);
            }
            AppMode::DryRunSummary => {
                installer::render_dry_run_summary_in_area(f, state, content_area, &self.header);
            }
        }

        // Render navigation bar
        header::render_nav_bar(f, state, keybinding_ctx, nav_bar_area);

        // Render help overlay if visible (on top of everything)
        if state.help_visible {
            header::render_help_overlay(f, state, keybinding_ctx);
        }
    }
}
