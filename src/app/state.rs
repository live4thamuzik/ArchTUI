//! Application state definitions
//!
//! Contains all state-related types for the application including AppState,
//! AppMode, and tool-related state types.

#![allow(dead_code)]

use crate::components::confirm_dialog::ConfirmDialogState;
use crate::components::file_browser::FileBrowserState;
use crate::components::floating_window::FloatingOutputState;
use crate::components::pty_terminal::PtyTerminalState;
use crate::config::Configuration;
use crate::scrolling::ScrollState;

/// Tool parameter types for input dialogs
#[derive(Debug, Clone)]
pub enum ToolParameter {
    Text(String),
    Number(i32),
    Boolean(bool),
    Selection(Vec<String>, usize),
    /// Password input - NOT passed via command-line args (security)
    /// Instead, passed via stdin to prevent exposure in `ps aux`
    Password(String),
}

/// Tool parameter definition
#[derive(Debug, Clone)]
pub struct ToolParam {
    pub name: String,
    pub description: String,
    pub param_type: ToolParameter,
    pub required: bool,
}

/// Tool parameter collection state
#[derive(Debug, Clone)]
pub struct ToolDialogState {
    pub tool_name: String,
    pub parameters: Vec<ToolParam>,
    pub current_param: usize,
    pub param_values: Vec<String>,
    pub is_executing: bool,
}

/// Main application state
#[derive(Debug, Clone)]
pub struct AppState {
    /// Current application mode
    pub mode: AppMode,
    /// Configuration options
    pub config: Configuration,
    /// Scroll state for configuration list
    pub config_scroll: ScrollState,
    /// Status message for user feedback
    pub status_message: String,
    /// Installer output lines
    pub installer_output: Vec<String>,
    /// Installation progress percentage
    pub installation_progress: u8,
    /// Main menu selection state
    pub main_menu_selection: usize,
    /// Tools menu selection state
    pub tools_menu_selection: usize,
    /// Current tool being executed
    pub current_tool: Option<String>,
    /// Tool execution output
    pub tool_output: Vec<String>,
    /// Tool dialog state for parameter collection
    pub tool_dialog: Option<ToolDialogState>,
    /// Whether help overlay is visible
    pub help_visible: bool,
    /// Floating output window state
    pub floating_output: Option<FloatingOutputState>,
    /// Embedded terminal state
    pub embedded_terminal: Option<PtyTerminalState>,
    /// File browser state
    pub file_browser: Option<FileBrowserState>,
    /// Confirmation dialog state
    pub confirm_dialog: Option<ConfirmDialogState>,
    /// Previous mode to return to after dialog
    pub pre_dialog_mode: Option<AppMode>,
    /// Dry-run summary output (Sprint 8)
    pub dry_run_summary: Option<Vec<String>>,
    /// Button selection in guided installer (0 = Test Config, 1 = Start Install)
    pub installer_button_selection: usize,
}

/// Application operating modes
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppMode {
    /// Main menu - entry point for all functionality
    MainMenu,
    /// Guided installer - step-by-step configuration
    GuidedInstaller,
    /// Automated install - run from configuration file
    AutomatedInstall,
    /// Tools menu - system administration tools
    ToolsMenu,
    /// Disk tools submenu
    DiskTools,
    /// System tools submenu
    SystemTools,
    /// User tools submenu
    UserTools,
    /// Network tools submenu
    NetworkTools,
    /// Tool parameter input dialog
    ToolDialog,
    /// Tool execution in progress
    ToolExecution,
    /// Installation phase - running the actual installation
    Installation,
    /// Installation complete
    Complete,
    /// Embedded terminal for interactive tools
    EmbeddedTerminal,
    /// Floating output window
    FloatingOutput,
    /// File browser for selecting config files
    FileBrowser,
    /// Confirmation dialog for destructive operations
    ConfirmDialog,
    /// Dry-run summary display (Sprint 8)
    DryRunSummary,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: AppMode::MainMenu,
            config: Configuration::default(),
            config_scroll: ScrollState::new(42, 30), // 42 config options, default 30 visible
            status_message: "Welcome to Arch Linux Toolkit".to_string(),
            installer_output: Vec::new(),
            installation_progress: 0,
            main_menu_selection: 0,
            tools_menu_selection: 0,
            current_tool: None,
            tool_output: Vec::new(),
            tool_dialog: None,
            help_visible: false,
            floating_output: None,
            embedded_terminal: None,
            file_browser: None,
            confirm_dialog: None,
            pre_dialog_mode: None,
            dry_run_summary: None,
            installer_button_selection: 1, // Default to "Start Install"
        }
    }
}
