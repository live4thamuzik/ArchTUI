//! User interface rendering module (redesigned)
//!
//! Simplified renderer — no more separate header/title bars.
//! Identity embedded in borders, breadcrumb navigation.
//!
//! Submodules:
//! - `header` - Progress bar, installer output, utility renderers
//! - `menus` - Split-pane menu rendering with scrollbars
//! - `installer` - Configuration UI, installation, completion screens
//! - `dialogs` - Tool dialog, floating output, file browser, confirm dialog
//! - `descriptions` - Tool description text
//! - `screens` - Guided installer wizard screens (Sprint 7)
//! - `loading` - Loading/progress UI

#![allow(dead_code)]

mod descriptions;
mod dialogs;
mod header;
mod installer;
pub(crate) mod menus;
pub mod screens;
mod loading;

use std::path::PathBuf;

// ============================================================================
// Wizard State Machine (Sprint 7)
// ============================================================================

/// Wizard state for the guided installer workflow.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum WizardState {
    #[default]
    Welcome,
    DiskSelect,
    Partitioner,
    PackageSelect,
    UserConfig,
    InstallProgress,
    Done,
}

impl WizardState {
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

    pub fn previous(&self) -> Option<Self> {
        match self {
            Self::Welcome => None,
            Self::DiskSelect => Some(Self::Welcome),
            Self::Partitioner => Some(Self::DiskSelect),
            Self::PackageSelect => Some(Self::Partitioner),
            Self::UserConfig => Some(Self::PackageSelect),
            Self::InstallProgress => None,
            Self::Done => None,
        }
    }

    pub fn can_go_back(&self) -> bool {
        self.previous().is_some()
    }

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

    pub const TOTAL_STEPS: usize = 7;
}

/// Wizard data collected during the guided installer flow.
#[derive(Debug, Clone, Default)]
pub struct WizardData {
    pub selected_disk: Option<PathBuf>,
    pub disk_model: Option<String>,
    pub disk_size: Option<u64>,
    pub auto_partition: bool,
    pub filesystem: Option<String>,
    pub hostname: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub user_sudo: bool,
    pub root_password: Option<String>,
    pub extra_packages: Vec<String>,
    #[allow(dead_code)]
    pub dry_run: bool,
}

impl WizardData {
    pub fn has_valid_disk(&self) -> bool {
        self.selected_disk.is_some()
    }

    pub fn has_valid_user(&self) -> bool {
        self.hostname.as_ref().is_some_and(|h| !h.is_empty())
            && self.username.as_ref().is_some_and(|u| !u.is_empty())
            && self.password.as_ref().is_some_and(|p| !p.is_empty())
    }

    pub fn is_ready_for_install(&self) -> bool {
        self.has_valid_disk() && self.has_valid_user()
    }

    pub fn zero_sensitive_data(&mut self) {
        if let Some(ref mut pwd) = self.password {
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
        self.zero_sensitive_data();
    }
}

// ============================================================================
// Imports and re-exports
// ============================================================================

use crate::app::{AppMode, AppState};
use crate::components::keybindings::KeybindingContext;
use crate::components::pty_terminal::PtyTerminal;
use crate::input::InputHandler;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

// Re-export HeaderRenderer for compatibility with code that references it
pub use header::HeaderRenderer;
#[allow(unused_imports)]
pub use screens::{DiskInfo, render_disk_select_screen, render_user_config_screen};

use crate::components::help_overlay::HelpOverlay;
use crate::components::nav_bar::NavBar;

/// UI renderer for the application (redesigned)
///
/// Stateless renderer — no header instance needed.
/// The `HeaderRenderer` field is kept for API compatibility with app/mod.rs
/// but is not used by the redesigned rendering logic.
pub struct UiRenderer {
    /// Kept for API compatibility (app/mod.rs calls UiRenderer::new() which sets this)
    #[allow(dead_code)]
    header: HeaderRenderer,
}

impl Default for UiRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl UiRenderer {
    pub fn new() -> Self {
        Self {
            header: HeaderRenderer::new(),
        }
    }

    /// Legacy render method for compatibility
    pub fn render(&self, f: &mut Frame, state: &AppState, input_handler: &mut InputHandler) {
        let keybinding_ctx = KeybindingContext::new();
        self.render_with_context(f, state, input_handler, &keybinding_ctx, None);
    }

    /// Full render method with all context — the main entry point.
    ///
    /// Uses redesigned rendering: breadcrumbs, split-pane layouts, rounded borders.
    /// The `input_handler`, `keybinding_ctx`, and `pty_terminal` params are accepted
    /// for API compatibility with the real app's event loop.
    pub fn render_with_context(
        &self,
        f: &mut Frame,
        state: &AppState,
        input_handler: &mut InputHandler,
        _keybinding_ctx: &KeybindingContext,
        pty_terminal: Option<&mut PtyTerminal>,
    ) {
        // If dialog is active, render ONLY the dialog
        if input_handler.is_dialog_active() {
            dialogs::render_input_dialog(f, input_handler);
            return;
        }

        let keybinding_ctx = KeybindingContext::new();

        // Main layout: content + nav bar
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),    // Content area
                Constraint::Length(1), // Nav bar
            ])
            .split(f.area());

        let content_area = main_chunks[0];
        let nav_bar_area = main_chunks[1];

        // Render main content based on mode (redesigned — no header param)
        match state.mode {
            AppMode::MainMenu => {
                menus::render_main_menu(f, state, content_area);
            }
            AppMode::GuidedInstaller => {
                installer::render_configuration_ui(f, state, content_area);
            }
            AppMode::AutomatedInstall => {
                installer::render_automated_install_ui(f, state, content_area);
            }
            AppMode::ToolsMenu => {
                menus::render_tools_menu(f, state, content_area);
            }
            AppMode::DiskTools => {
                menus::render_disk_tools_menu(f, state, content_area);
            }
            AppMode::SystemTools => {
                menus::render_system_tools_menu(f, state, content_area);
            }
            AppMode::UserTools => {
                menus::render_user_tools_menu(f, state, content_area);
            }
            AppMode::NetworkTools => {
                menus::render_network_tools_menu(f, state, content_area);
            }
            AppMode::ToolDialog => {
                if let Some(ref pre_mode) = state.pre_dialog_mode {
                    self.render_background(f, state, content_area, pre_mode);
                } else {
                    menus::render_tools_menu(f, state, content_area);
                }
                dialogs::render_tool_dialog(f, state);
            }
            AppMode::Installation => {
                installer::render_installation_ui(f, state, content_area);
            }
            AppMode::Complete => {
                installer::render_completion_ui(f, state, content_area);
            }
            AppMode::EmbeddedTerminal => {
                dialogs::render_embedded_terminal(f, state, content_area, pty_terminal);
            }
            AppMode::FloatingOutput => {
                if let Some(ref pre_mode) = state.pre_dialog_mode {
                    self.render_background(f, state, content_area, pre_mode);
                } else {
                    menus::render_tools_menu(f, state, content_area);
                }
                dialogs::render_floating_output(f, state);
            }
            AppMode::FileBrowser => {
                installer::render_automated_install_ui(f, state, content_area);
                dialogs::render_file_browser(f, state);
            }
            AppMode::ConfirmDialog => {
                if let Some(ref pre_mode) = state.pre_dialog_mode {
                    self.render_background(f, state, content_area, pre_mode);
                } else {
                    menus::render_tools_menu(f, state, content_area);
                }
                dialogs::render_confirm_dialog(f, state);
            }
            AppMode::DryRunSummary => {
                installer::render_dry_run_summary(f, state, content_area);
            }
        }

        // Render nav bar (redesigned component)
        let nav_items = keybinding_ctx.get_nav_items(&state.mode);
        let nav_bar = NavBar::new(nav_items);
        nav_bar.render(f, nav_bar_area);

        // Render help overlay if visible
        if state.help_visible {
            let help_overlay = HelpOverlay::new(&state.mode, &keybinding_ctx);
            help_overlay.render(f, f.area());
        }
    }

    fn render_background(
        &self,
        f: &mut Frame,
        state: &AppState,
        area: ratatui::layout::Rect,
        mode: &AppMode,
    ) {
        match mode {
            AppMode::DiskTools => menus::render_disk_tools_menu(f, state, area),
            AppMode::SystemTools => menus::render_system_tools_menu(f, state, area),
            AppMode::UserTools => menus::render_user_tools_menu(f, state, area),
            AppMode::NetworkTools => menus::render_network_tools_menu(f, state, area),
            AppMode::GuidedInstaller => installer::render_configuration_ui(f, state, area),
            _ => menus::render_tools_menu(f, state, area),
        }
    }
}
