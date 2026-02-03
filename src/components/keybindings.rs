//! Keybinding system for context-aware keyboard shortcuts
//!
//! Provides a registry of keybindings that change based on the current application mode.

#![allow(dead_code)]

use crate::app::AppMode;
use crossterm::event::{KeyCode, KeyModifiers};
use std::collections::HashMap;

/// Actions that can be triggered by keybindings
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyAction {
    NavigateUp,
    NavigateDown,
    PageUp,
    PageDown,
    Home,
    End,
    Select,
    Back,
    Quit,
    Help,
    StartInstall,
    Confirm,
    Cancel,
    Toggle,
    ScrollUp,
    ScrollDown,
    Dismiss,
    ExitTerminal,
}

/// A keybinding definition
#[derive(Debug, Clone)]
pub struct Keybinding {
    pub key: KeyCode,
    pub modifiers: KeyModifiers,
    pub action: KeyAction,
    pub display: String,
    pub description: String,
}

impl Keybinding {
    /// Create a new keybinding with no modifiers
    pub fn new(key: KeyCode, action: KeyAction, display: &str, description: &str) -> Self {
        Self {
            key,
            modifiers: KeyModifiers::NONE,
            action,
            display: display.to_string(),
            description: description.to_string(),
        }
    }

    /// Create a keybinding with modifiers
    pub fn with_modifiers(
        key: KeyCode,
        modifiers: KeyModifiers,
        action: KeyAction,
        display: &str,
        description: &str,
    ) -> Self {
        Self {
            key,
            modifiers,
            action,
            display: display.to_string(),
            description: description.to_string(),
        }
    }
}

/// Context-aware keybinding registry
pub struct KeybindingContext {
    /// Mode-specific keybindings
    mode_bindings: HashMap<AppMode, Vec<Keybinding>>,
    /// Global keybindings (available in all modes)
    global_bindings: Vec<Keybinding>,
}

impl Default for KeybindingContext {
    fn default() -> Self {
        Self::new()
    }
}

impl KeybindingContext {
    /// Create a new keybinding context with default bindings
    pub fn new() -> Self {
        let mut ctx = Self {
            mode_bindings: HashMap::new(),
            global_bindings: Vec::new(),
        };
        ctx.register_defaults();
        ctx
    }

    /// Register default keybindings for all modes
    fn register_defaults(&mut self) {
        // Global bindings (available everywhere except EmbeddedTerminal)
        self.global_bindings = vec![
            Keybinding::new(KeyCode::Char('?'), KeyAction::Help, "?", "Help"),
            Keybinding::new(KeyCode::Char('q'), KeyAction::Quit, "Q", "Quit"),
        ];

        // Main Menu
        self.mode_bindings.insert(
            AppMode::MainMenu,
            vec![
                Keybinding::new(KeyCode::Up, KeyAction::NavigateUp, "Up", "Navigate up"),
                Keybinding::new(KeyCode::Down, KeyAction::NavigateDown, "Down", "Navigate down"),
                Keybinding::new(KeyCode::Enter, KeyAction::Select, "Enter", "Select"),
            ],
        );

        // Guided Installer
        self.mode_bindings.insert(
            AppMode::GuidedInstaller,
            vec![
                Keybinding::new(KeyCode::Up, KeyAction::NavigateUp, "Up", "Navigate up"),
                Keybinding::new(KeyCode::Down, KeyAction::NavigateDown, "Down", "Navigate down"),
                Keybinding::new(KeyCode::PageUp, KeyAction::PageUp, "PgUp", "Page up"),
                Keybinding::new(KeyCode::PageDown, KeyAction::PageDown, "PgDn", "Page down"),
                Keybinding::new(KeyCode::Home, KeyAction::Home, "Home", "Go to first"),
                Keybinding::new(KeyCode::End, KeyAction::End, "End", "Go to last"),
                Keybinding::new(KeyCode::Enter, KeyAction::Select, "Enter", "Configure"),
                Keybinding::new(KeyCode::Char(' '), KeyAction::StartInstall, "Space", "Start install"),
                Keybinding::new(KeyCode::Char('b'), KeyAction::Back, "B", "Back"),
            ],
        );

        // Tools Menu and submenus
        let tools_bindings = vec![
            Keybinding::new(KeyCode::Up, KeyAction::NavigateUp, "Up", "Navigate up"),
            Keybinding::new(KeyCode::Down, KeyAction::NavigateDown, "Down", "Navigate down"),
            Keybinding::new(KeyCode::Enter, KeyAction::Select, "Enter", "Select"),
            Keybinding::new(KeyCode::Char('b'), KeyAction::Back, "B", "Back"),
        ];

        self.mode_bindings
            .insert(AppMode::ToolsMenu, tools_bindings.clone());
        self.mode_bindings
            .insert(AppMode::DiskTools, tools_bindings.clone());
        self.mode_bindings
            .insert(AppMode::SystemTools, tools_bindings.clone());
        self.mode_bindings
            .insert(AppMode::UserTools, tools_bindings.clone());
        self.mode_bindings
            .insert(AppMode::NetworkTools, tools_bindings.clone());

        // Automated Install
        self.mode_bindings.insert(
            AppMode::AutomatedInstall,
            vec![
                Keybinding::new(KeyCode::Enter, KeyAction::Select, "Enter", "Select config"),
                Keybinding::new(KeyCode::Char('b'), KeyAction::Back, "B", "Back"),
            ],
        );

        // Tool Dialog
        self.mode_bindings.insert(
            AppMode::ToolDialog,
            vec![
                Keybinding::new(KeyCode::Up, KeyAction::NavigateUp, "Up", "Previous field"),
                Keybinding::new(KeyCode::Down, KeyAction::NavigateDown, "Down", "Next field"),
                Keybinding::new(KeyCode::Enter, KeyAction::Confirm, "Enter", "Confirm"),
                Keybinding::new(KeyCode::Esc, KeyAction::Cancel, "Esc", "Cancel"),
            ],
        );

        // Tool Execution / Floating Output
        self.mode_bindings.insert(
            AppMode::ToolExecution,
            vec![
                Keybinding::new(KeyCode::Up, KeyAction::ScrollUp, "Up", "Scroll up"),
                Keybinding::new(KeyCode::Down, KeyAction::ScrollDown, "Down", "Scroll down"),
                Keybinding::new(KeyCode::PageUp, KeyAction::PageUp, "PgUp", "Page up"),
                Keybinding::new(KeyCode::PageDown, KeyAction::PageDown, "PgDn", "Page down"),
                Keybinding::new(KeyCode::Esc, KeyAction::Dismiss, "Esc", "Close"),
                Keybinding::new(KeyCode::Char('b'), KeyAction::Back, "B", "Back"),
            ],
        );

        // Installation
        self.mode_bindings.insert(
            AppMode::Installation,
            vec![
                Keybinding::new(KeyCode::Up, KeyAction::ScrollUp, "Up", "Scroll up"),
                Keybinding::new(KeyCode::Down, KeyAction::ScrollDown, "Down", "Scroll down"),
                Keybinding::new(KeyCode::PageUp, KeyAction::PageUp, "PgUp", "Page up"),
                Keybinding::new(KeyCode::PageDown, KeyAction::PageDown, "PgDn", "Page down"),
            ],
        );

        // Complete
        self.mode_bindings.insert(
            AppMode::Complete,
            vec![
                Keybinding::new(KeyCode::Enter, KeyAction::Dismiss, "Enter", "Continue"),
                Keybinding::new(KeyCode::Char('b'), KeyAction::Back, "B", "Back to menu"),
            ],
        );

        // Embedded Terminal
        self.mode_bindings.insert(
            AppMode::EmbeddedTerminal,
            vec![Keybinding::with_modifiers(
                KeyCode::Char('q'),
                KeyModifiers::CONTROL,
                KeyAction::ExitTerminal,
                "Ctrl+Q",
                "Exit terminal",
            )],
        );

        // Floating Output
        self.mode_bindings.insert(
            AppMode::FloatingOutput,
            vec![
                Keybinding::new(KeyCode::Up, KeyAction::ScrollUp, "Up", "Scroll up"),
                Keybinding::new(KeyCode::Down, KeyAction::ScrollDown, "Down", "Scroll down"),
                Keybinding::new(KeyCode::PageUp, KeyAction::PageUp, "PgUp", "Page up"),
                Keybinding::new(KeyCode::PageDown, KeyAction::PageDown, "PgDn", "Page down"),
                Keybinding::new(KeyCode::Esc, KeyAction::Dismiss, "Esc", "Close"),
                Keybinding::new(KeyCode::Enter, KeyAction::Dismiss, "Enter", "Close"),
            ],
        );

        // File Browser
        self.mode_bindings.insert(
            AppMode::FileBrowser,
            vec![
                Keybinding::new(KeyCode::Up, KeyAction::NavigateUp, "Up", "Move up"),
                Keybinding::new(KeyCode::Down, KeyAction::NavigateDown, "Down", "Move down"),
                Keybinding::new(KeyCode::Enter, KeyAction::Select, "Enter", "Select"),
                Keybinding::new(KeyCode::Char('~'), KeyAction::Home, "~", "Home dir"),
                Keybinding::new(KeyCode::Char('/'), KeyAction::Back, "/", "Root dir"),
                Keybinding::new(KeyCode::Esc, KeyAction::Dismiss, "Esc", "Cancel"),
            ],
        );

        // Confirm Dialog
        self.mode_bindings.insert(
            AppMode::ConfirmDialog,
            vec![
                Keybinding::new(KeyCode::Left, KeyAction::Toggle, "Left", "Select No"),
                Keybinding::new(KeyCode::Right, KeyAction::Toggle, "Right", "Select Yes"),
                Keybinding::new(KeyCode::Tab, KeyAction::Toggle, "Tab", "Toggle selection"),
                Keybinding::new(KeyCode::Enter, KeyAction::Confirm, "Enter", "Confirm"),
                Keybinding::new(KeyCode::Esc, KeyAction::Cancel, "Esc", "Cancel"),
            ],
        );
    }

    /// Get keybindings for a specific mode (includes global bindings)
    pub fn get_bindings(&self, mode: &AppMode) -> Vec<&Keybinding> {
        let mut bindings: Vec<&Keybinding> = Vec::new();

        // Add mode-specific bindings
        if let Some(mode_bindings) = self.mode_bindings.get(mode) {
            bindings.extend(mode_bindings.iter());
        }

        // Add global bindings (except for EmbeddedTerminal)
        if *mode != AppMode::EmbeddedTerminal {
            bindings.extend(self.global_bindings.iter());
        }

        bindings
    }

    /// Get navigation bar items for display
    pub fn get_nav_items(&self, mode: &AppMode) -> Vec<NavBarItem> {
        let bindings = self.get_bindings(mode);

        // Select key bindings to show in nav bar (most important ones)
        let priority_actions = match mode {
            AppMode::MainMenu => vec![
                KeyAction::NavigateUp,
                KeyAction::NavigateDown,
                KeyAction::Select,
                KeyAction::Help,
                KeyAction::Quit,
            ],
            AppMode::GuidedInstaller => vec![
                KeyAction::NavigateUp,
                KeyAction::NavigateDown,
                KeyAction::Select,
                KeyAction::StartInstall,
                KeyAction::Back,
                KeyAction::Help,
                KeyAction::Quit,
            ],
            AppMode::ToolsMenu
            | AppMode::DiskTools
            | AppMode::SystemTools
            | AppMode::UserTools
            | AppMode::NetworkTools => vec![
                KeyAction::NavigateUp,
                KeyAction::NavigateDown,
                KeyAction::Select,
                KeyAction::Back,
                KeyAction::Help,
                KeyAction::Quit,
            ],
            AppMode::EmbeddedTerminal => vec![KeyAction::ExitTerminal],
            AppMode::FloatingOutput | AppMode::ToolExecution => vec![
                KeyAction::ScrollUp,
                KeyAction::ScrollDown,
                KeyAction::Dismiss,
            ],
            AppMode::Installation => vec![
                KeyAction::ScrollUp,
                KeyAction::ScrollDown,
                KeyAction::Quit,
            ],
            AppMode::Complete => vec![KeyAction::Dismiss, KeyAction::Back, KeyAction::Quit],
            AppMode::ToolDialog => vec![
                KeyAction::NavigateUp,
                KeyAction::NavigateDown,
                KeyAction::Confirm,
                KeyAction::Cancel,
            ],
            AppMode::AutomatedInstall => vec![
                KeyAction::Select,
                KeyAction::Back,
                KeyAction::Help,
                KeyAction::Quit,
            ],
            AppMode::FileBrowser => vec![
                KeyAction::NavigateUp,
                KeyAction::NavigateDown,
                KeyAction::Select,
                KeyAction::Dismiss,
            ],
            AppMode::ConfirmDialog => vec![
                KeyAction::Toggle,
                KeyAction::Confirm,
                KeyAction::Cancel,
            ],
            AppMode::DryRunSummary => vec![
                KeyAction::ScrollUp,
                KeyAction::ScrollDown,
                KeyAction::Back,
                KeyAction::Dismiss,
            ],
        };

        // Combine Up/Down into single item for cleaner display
        let mut items: Vec<NavBarItem> = Vec::new();
        let mut has_nav = false;
        let mut has_scroll = false;

        for action in priority_actions {
            // Skip if we already added a combined nav item
            if (action == KeyAction::NavigateUp || action == KeyAction::NavigateDown) && has_nav {
                continue;
            }
            if (action == KeyAction::ScrollUp || action == KeyAction::ScrollDown) && has_scroll {
                continue;
            }

            if let Some(binding) = bindings.iter().find(|b| b.action == action) {
                // Combine Up/Down navigation
                if action == KeyAction::NavigateUp || action == KeyAction::NavigateDown {
                    items.push(NavBarItem {
                        key_display: "Up/Dn".to_string(),
                        action_label: "Navigate".to_string(),
                    });
                    has_nav = true;
                } else if action == KeyAction::ScrollUp || action == KeyAction::ScrollDown {
                    items.push(NavBarItem {
                        key_display: "Up/Dn".to_string(),
                        action_label: "Scroll".to_string(),
                    });
                    has_scroll = true;
                } else {
                    items.push(NavBarItem {
                        key_display: binding.display.clone(),
                        action_label: binding.description.clone(),
                    });
                }
            }
        }

        items
    }

    /// Get full help content for a mode (for help overlay)
    pub fn get_help_content(&self, mode: &AppMode) -> Vec<HelpSection> {
        let mut sections = Vec::new();

        // Navigation section
        let nav_bindings: Vec<_> = self
            .get_bindings(mode)
            .into_iter()
            .filter(|b| {
                matches!(
                    b.action,
                    KeyAction::NavigateUp
                        | KeyAction::NavigateDown
                        | KeyAction::PageUp
                        | KeyAction::PageDown
                        | KeyAction::Home
                        | KeyAction::End
                        | KeyAction::ScrollUp
                        | KeyAction::ScrollDown
                )
            })
            .collect();

        if !nav_bindings.is_empty() {
            sections.push(HelpSection {
                title: "Navigation".to_string(),
                items: nav_bindings
                    .iter()
                    .map(|b| (b.display.clone(), b.description.clone()))
                    .collect(),
            });
        }

        // Actions section
        let action_bindings: Vec<_> = self
            .get_bindings(mode)
            .into_iter()
            .filter(|b| {
                matches!(
                    b.action,
                    KeyAction::Select
                        | KeyAction::Confirm
                        | KeyAction::Cancel
                        | KeyAction::StartInstall
                        | KeyAction::Toggle
                        | KeyAction::Dismiss
                        | KeyAction::ExitTerminal
                )
            })
            .collect();

        if !action_bindings.is_empty() {
            sections.push(HelpSection {
                title: "Actions".to_string(),
                items: action_bindings
                    .iter()
                    .map(|b| (b.display.clone(), b.description.clone()))
                    .collect(),
            });
        }

        // General section
        let general_bindings: Vec<_> = self
            .get_bindings(mode)
            .into_iter()
            .filter(|b| matches!(b.action, KeyAction::Back | KeyAction::Help | KeyAction::Quit))
            .collect();

        if !general_bindings.is_empty() {
            sections.push(HelpSection {
                title: "General".to_string(),
                items: general_bindings
                    .iter()
                    .map(|b| (b.display.clone(), b.description.clone()))
                    .collect(),
            });
        }

        sections
    }
}

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
