//! UI rendering module for tui_test
//!
//! Simplified renderer that dispatches to submodules based on AppMode.
//! No more separate header/title bars — identity embedded in borders.

mod descriptions;
mod dialogs;
mod header;
mod installer;
pub(crate) mod menus;

use crate::app::{AppMode, AppState};
use crate::components::help_overlay::HelpOverlay;
use crate::components::keybindings::KeybindingContext;
use crate::components::nav_bar::NavBar;
use ratatui::{
    layout::{Constraint, Direction, Layout},
    Frame,
};

/// Main UI renderer
pub struct UiRenderer;

impl UiRenderer {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, f: &mut Frame, state: &AppState) {
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

        // Render main content based on mode
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

        // Render nav bar
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
