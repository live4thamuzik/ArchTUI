//! User interface rendering module
//!
//! This module is organized into submodules for better maintainability:
//! - `header` - Header, title, and common widget rendering
//! - `menus` - Menu rendering (main, tools, categories)
//! - `installer` - Installation and configuration UI
//! - `dialogs` - Input and confirmation dialog rendering
//! - `descriptions` - Tool description text generation

#![allow(dead_code)]

mod descriptions;
mod dialogs;
mod header;
mod installer;
mod menus;

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
        }

        // Render navigation bar
        header::render_nav_bar(f, state, keybinding_ctx, nav_bar_area);

        // Render help overlay if visible (on top of everything)
        if state.help_visible {
            header::render_help_overlay(f, state, keybinding_ctx);
        }
    }
}
