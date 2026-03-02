//! Keybinding system — simplified for visual testing
//!
//! Provides NavBarItem and KeybindingContext with mode-aware nav items.

#![allow(dead_code)]

use crate::app::{AppMode, ConfigEditState};

/// Navigation bar item for display
#[derive(Debug, Clone)]
pub struct NavBarItem {
    pub key_display: String,
    pub action_label: String,
}

/// Help section for the help overlay
#[derive(Debug, Clone)]
pub struct HelpSection {
    pub title: String,
    pub items: Vec<(String, String)>,
}

/// Context-aware keybinding provider
pub struct KeybindingContext;

impl Default for KeybindingContext {
    fn default() -> Self {
        Self::new()
    }
}

impl KeybindingContext {
    pub fn new() -> Self {
        Self
    }

    /// Get navigation bar items for display
    pub fn get_nav_items(&self, mode: &AppMode, config_edit: &ConfigEditState) -> Vec<NavBarItem> {
        // When inline editing is active, show editor-specific hints
        if config_edit.is_active() {
            return match config_edit {
                ConfigEditState::Selection { .. } => vec![
                    NavBarItem { key_display: "\u{2191}/\u{2193}".into(), action_label: "Navigate".into() },
                    NavBarItem { key_display: "Enter".into(), action_label: "Confirm".into() },
                    NavBarItem { key_display: "Esc".into(), action_label: "Cancel".into() },
                    NavBarItem { key_display: "a-z".into(), action_label: "Jump".into() },
                ],
                ConfigEditState::TextInput { .. } => vec![
                    NavBarItem { key_display: "Enter".into(), action_label: "Confirm".into() },
                    NavBarItem { key_display: "Esc".into(), action_label: "Cancel".into() },
                ],
                ConfigEditState::PasswordInput { .. } => vec![
                    NavBarItem { key_display: "Enter".into(), action_label: "Confirm".into() },
                    NavBarItem { key_display: "Esc".into(), action_label: "Cancel".into() },
                ],
                ConfigEditState::PackageInput { .. } => vec![
                    NavBarItem { key_display: "Enter".into(), action_label: "Execute".into() },
                    NavBarItem { key_display: "Esc".into(), action_label: "Done".into() },
                    NavBarItem { key_display: "\u{2191}/\u{2193}".into(), action_label: "Navigate".into() },
                ],
                ConfigEditState::None => unreachable!(),
            };
        }

        match mode {
            AppMode::MainMenu => vec![
                NavBarItem { key_display: "\u{2191}/\u{2193}".into(), action_label: "Navigate".into() },
                NavBarItem { key_display: "Enter".into(), action_label: "Select".into() },
                NavBarItem { key_display: "?".into(), action_label: "Help".into() },
                NavBarItem { key_display: "q".into(), action_label: "Quit".into() },
            ],
            AppMode::GuidedInstaller => vec![
                NavBarItem { key_display: "\u{2191}/\u{2193}".into(), action_label: "Navigate".into() },
                NavBarItem { key_display: "Enter".into(), action_label: "Configure".into() },
                NavBarItem { key_display: "Space".into(), action_label: "Start".into() },
                NavBarItem { key_display: "b".into(), action_label: "Back".into() },
                NavBarItem { key_display: "?".into(), action_label: "Help".into() },
                NavBarItem { key_display: "q".into(), action_label: "Quit".into() },
            ],
            AppMode::ToolsMenu
            | AppMode::DiskTools
            | AppMode::SystemTools
            | AppMode::UserTools
            | AppMode::NetworkTools => vec![
                NavBarItem { key_display: "\u{2191}/\u{2193}".into(), action_label: "Navigate".into() },
                NavBarItem { key_display: "Enter".into(), action_label: "Select".into() },
                NavBarItem { key_display: "b".into(), action_label: "Back".into() },
                NavBarItem { key_display: "?".into(), action_label: "Help".into() },
                NavBarItem { key_display: "q".into(), action_label: "Quit".into() },
            ],
            AppMode::ToolDialog => vec![
                NavBarItem { key_display: "\u{2191}/\u{2193}".into(), action_label: "Navigate".into() },
                NavBarItem { key_display: "\u{2190}/\u{2192}".into(), action_label: "Change".into() },
                NavBarItem { key_display: "Enter".into(), action_label: "Execute".into() },
                NavBarItem { key_display: "Esc".into(), action_label: "Cancel".into() },
            ],
            AppMode::Installation => vec![
                NavBarItem { key_display: "\u{2191}/\u{2193}".into(), action_label: "Scroll".into() },
                NavBarItem { key_display: "q".into(), action_label: "Quit".into() },
            ],
            AppMode::Complete => vec![
                NavBarItem { key_display: "Enter".into(), action_label: "Continue".into() },
                NavBarItem { key_display: "b".into(), action_label: "Menu".into() },
                NavBarItem { key_display: "q".into(), action_label: "Quit".into() },
            ],
            AppMode::FloatingOutput => vec![
                NavBarItem { key_display: "\u{2191}/\u{2193}".into(), action_label: "Scroll".into() },
                NavBarItem { key_display: "Esc".into(), action_label: "Close".into() },
            ],
            AppMode::ConfirmDialog => vec![
                NavBarItem { key_display: "\u{2190}/\u{2192}".into(), action_label: "Toggle".into() },
                NavBarItem { key_display: "Enter".into(), action_label: "Confirm".into() },
                NavBarItem { key_display: "Esc".into(), action_label: "Cancel".into() },
            ],
            AppMode::FileBrowser => vec![
                NavBarItem { key_display: "\u{2191}/\u{2193}".into(), action_label: "Navigate".into() },
                NavBarItem { key_display: "Enter".into(), action_label: "Select".into() },
                NavBarItem { key_display: "Esc".into(), action_label: "Cancel".into() },
            ],
            AppMode::DryRunSummary => vec![
                NavBarItem { key_display: "\u{2191}/\u{2193}".into(), action_label: "Scroll".into() },
                NavBarItem { key_display: "b".into(), action_label: "Back".into() },
                NavBarItem { key_display: "Enter".into(), action_label: "Dismiss".into() },
            ],
            AppMode::AutomatedInstall => vec![
                NavBarItem { key_display: "Enter".into(), action_label: "Browse".into() },
                NavBarItem { key_display: "b".into(), action_label: "Back".into() },
                NavBarItem { key_display: "q".into(), action_label: "Quit".into() },
            ],
            AppMode::EmbeddedTerminal => vec![
                NavBarItem { key_display: "Ctrl+D".into(), action_label: "Exit".into() },
            ],
        }
    }

    /// Get full help content for a mode
    pub fn get_help_content(&self, mode: &AppMode) -> Vec<HelpSection> {
        let mut sections = Vec::new();

        match mode {
            AppMode::MainMenu => {
                sections.push(HelpSection {
                    title: "Navigation".into(),
                    items: vec![
                        ("Up".into(), "Navigate up".into()),
                        ("Down".into(), "Navigate down".into()),
                    ],
                });
                sections.push(HelpSection {
                    title: "Actions".into(),
                    items: vec![("Enter".into(), "Select option".into())],
                });
            }
            AppMode::GuidedInstaller => {
                sections.push(HelpSection {
                    title: "Navigation".into(),
                    items: vec![
                        ("Up/Down".into(), "Navigate options".into()),
                        ("PgUp/PgDn".into(), "Page scroll".into()),
                        ("Home/End".into(), "Jump to first/last".into()),
                        ("Esc / b".into(), "Go back".into()),
                    ],
                });
                sections.push(HelpSection {
                    title: "Actions".into(),
                    items: vec![
                        ("Enter".into(), "Configure selected option".into()),
                        ("Space".into(), "Start installation".into()),
                    ],
                });
                sections.push(HelpSection {
                    title: "Inline Editing".into(),
                    items: vec![
                        ("Enter".into(), "Confirm value".into()),
                        ("Esc".into(), "Cancel editing".into()),
                        ("a-z".into(), "Jump to option (selection)".into()),
                    ],
                });
            }
            _ => {
                sections.push(HelpSection {
                    title: "Navigation".into(),
                    items: vec![
                        ("Up/Down".into(), "Navigate".into()),
                        ("Enter".into(), "Select".into()),
                    ],
                });
            }
        }

        sections.push(HelpSection {
            title: "General".into(),
            items: vec![
                ("?".into(), "Toggle help".into()),
                ("q".into(), "Quit".into()),
            ],
        });

        sections
    }
}
