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

/// Partition assignments for manual partitioning strategy
///
/// Stores user-selected partition→role mappings collected via TUI dialogs
/// after cfdisk creates partition tables. Injected as MANUAL_* env vars
/// for manual.sh to format and mount.
#[derive(Debug, Clone, Default)]
pub struct ManualPartitionMap {
    /// Root partition device (required), e.g. "/dev/sda2"
    pub root: String,
    /// Root filesystem type (required), e.g. "ext4"
    pub root_fs: String,
    /// Boot partition device (required), e.g. "/dev/sda1"
    pub boot: String,
    /// EFI partition device (UEFI only, empty for BIOS)
    pub efi: String,
    /// Home partition device (optional)
    pub home: String,
    /// Home filesystem type (optional)
    pub home_fs: String,
    /// Swap partition device (optional)
    pub swap: String,
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
    /// Scroll offset for installer output (lines from top)
    pub installer_scroll_offset: usize,
    /// Whether installer output auto-scrolls to bottom
    pub installer_auto_scroll: bool,
    /// Actual visible height of installer output viewport (set by renderer)
    pub installer_visible_height: usize,
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
    /// Scroll offset for dry-run summary display
    pub dry_run_scroll_offset: usize,
    /// Button selection in guided installer (0 = Test Config, 1 = Export Config, 2 = Start Install)
    pub installer_button_selection: usize,
    /// PID of the running installer process (for cancellation)
    pub installer_pid: Option<u32>,
    /// Logging verbosity for installation ("INFO" or "VERBOSE")
    pub log_level: String,
    /// Manual partitioning assignments (set via TUI dialogs after cfdisk)
    pub manual_partition_map: Option<ManualPartitionMap>,
    /// Device path stored across the disk layout → action → layout loop.
    /// Set after DiskSelection, consumed when returning from FloatingOutput.
    pub pending_tool_device: Option<String>,
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
        let config = Configuration::default();
        let total_rows = config.options.len() + 1; // +1 for "Start Installation" button
        Self {
            mode: AppMode::MainMenu,
            config_scroll: ScrollState::new(total_rows, 30),
            config,
            status_message: "Welcome to Arch Linux Toolkit".to_string(),
            installer_output: Vec::new(),
            installation_progress: 0,
            installer_scroll_offset: 0,
            installer_auto_scroll: true,
            installer_visible_height: 30,
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
            dry_run_scroll_offset: 0,
            installer_button_selection: 2, // Default to "Start Install"
            installer_pid: None,
            log_level: std::env::var("ARCHTUI_LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string()),
            manual_partition_map: None,
            pending_tool_device: None,
        }
    }
}
