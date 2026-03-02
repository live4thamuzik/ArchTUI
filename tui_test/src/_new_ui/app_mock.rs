//! Mock application state for visual testing
//!
//! Minimal stubs of AppState, AppMode, and dialog types.
//! Only fields the UI reads are included — no real logic.

#![allow(dead_code, clippy::field_reassign_with_default)]

use crate::config::Configuration;
use crate::scrolling::ScrollState;

// =============================================================================
// AppMode
// =============================================================================

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppMode {
    MainMenu,
    GuidedInstaller,
    AutomatedInstall,
    ToolsMenu,
    DiskTools,
    SystemTools,
    UserTools,
    NetworkTools,
    ToolDialog,
    Installation,
    Complete,
    FloatingOutput,
    FileBrowser,
    ConfirmDialog,
    DryRunSummary,
}

// =============================================================================
// Tool parameter types
// =============================================================================

#[derive(Debug, Clone)]
pub enum ToolParameter {
    Text(String),
    Number(i32),
    Boolean(bool),
    Selection(Vec<String>, usize),
    Password(String),
}

#[derive(Debug, Clone)]
pub struct ToolParam {
    pub name: String,
    pub description: String,
    pub param_type: ToolParameter,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct ToolDialogState {
    pub tool_name: String,
    pub parameters: Vec<ToolParam>,
    pub current_param: usize,
    pub param_values: Vec<String>,
    pub is_executing: bool,
}

// =============================================================================
// Floating output state (simplified)
// =============================================================================

#[derive(Debug, Clone)]
pub struct FloatingOutputState {
    pub title: String,
    pub content: Vec<String>,
    pub scroll_offset: usize,
    pub auto_scroll: bool,
    pub complete: bool,
    pub progress: Option<u8>,
    pub status: String,
}

impl Default for FloatingOutputState {
    fn default() -> Self {
        Self {
            title: "Output".to_string(),
            content: Vec::new(),
            scroll_offset: 0,
            auto_scroll: true,
            complete: false,
            progress: None,
            status: String::new(),
        }
    }
}

// =============================================================================
// Confirm dialog state (simplified)
// =============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmSeverity {
    Info,
    Warning,
    Danger,
}

#[derive(Debug, Clone)]
pub struct ConfirmDialogState {
    pub title: String,
    pub message: String,
    pub details: Vec<String>,
    pub severity: ConfirmSeverity,
    pub selected: usize,
    pub confirm_action: String,
}

// =============================================================================
// File browser state (simplified)
// =============================================================================

#[derive(Debug, Clone)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct FileBrowserState {
    pub current_dir: String,
    pub entries: Vec<FileEntry>,
    pub selected: usize,
    pub error: Option<String>,
    pub scroll_offset: usize,
}

// =============================================================================
// Config editing state — right panel interaction
// =============================================================================

/// A package search result
#[derive(Debug, Clone)]
pub struct PackageResult {
    pub repo: String,
    pub name: String,
    pub version: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum ConfigEditState {
    /// Right panel shows static details (default)
    None,
    /// Picking from a list of choices
    Selection {
        choices: Vec<String>,
        selected: usize,
    },
    /// Text input with cursor
    TextInput {
        value: String,
        cursor: usize,
    },
    /// Password input (masked)
    PasswordInput {
        value: String,
        cursor: usize,
    },
    /// Interactive package selection (search/add/remove/list/done)
    PackageInput {
        packages: Vec<String>,
        current_input: String,
        output_lines: Vec<String>,
        is_pacman: bool,
        /// Search results from pacman -Ss / AUR RPC
        search_results: Vec<PackageResult>,
        /// Index of highlighted result in search results view
        results_selected: usize,
        /// Whether we're browsing search results (true) or in command mode (false)
        show_search_results: bool,
    },
}

// =============================================================================
// AppState
// =============================================================================

#[derive(Debug, Clone)]
pub struct AppState {
    pub mode: AppMode,
    pub config: Configuration,
    pub config_scroll: ScrollState,
    pub status_message: String,
    pub installer_output: Vec<String>,
    pub installation_progress: u8,
    pub installer_scroll_offset: usize,
    pub installer_auto_scroll: bool,
    pub installer_visible_height: usize,
    pub main_menu_selection: usize,
    pub tools_menu_selection: usize,
    pub tool_dialog: Option<ToolDialogState>,
    pub help_visible: bool,
    pub floating_output: Option<FloatingOutputState>,
    pub file_browser: Option<FileBrowserState>,
    pub confirm_dialog: Option<ConfirmDialogState>,
    pub pre_dialog_mode: Option<AppMode>,
    pub dry_run_summary: Option<Vec<String>>,
    pub dry_run_scroll_offset: usize,
    pub installer_button_selection: usize,
    pub config_edit: ConfigEditState,
    /// Cached disk layout lines for the currently selected device
    pub disk_layout: Vec<String>,
}

impl Default for AppState {
    fn default() -> Self {
        let config = Configuration::default();
        let total_rows = config.options.len() + 1;
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
            tool_dialog: None,
            help_visible: false,
            floating_output: None,
            file_browser: None,
            confirm_dialog: None,
            pre_dialog_mode: None,
            dry_run_summary: None,
            dry_run_scroll_offset: 0,
            installer_button_selection: 2,
            config_edit: ConfigEditState::None,
            disk_layout: Vec::new(),
        }
    }
}

impl AppState {
    /// Create state for Installation mode demo
    pub fn demo_installation() -> Self {
        let mut state = Self::default();
        state.mode = AppMode::Installation;
        state.installation_progress = 42;
        state.status_message = "Phase 3: Installing base packages...".to_string();
        state.installer_output = vec![
            "==> Phase 1: Partitioning disk".to_string(),
            "  Creating GPT partition table on /dev/sda".to_string(),
            "  Creating EFI partition (512MB)".to_string(),
            "  Creating root partition (remaining)".to_string(),
            "SUCCESS: Partitioning complete".to_string(),
            "==> Phase 2: Formatting partitions".to_string(),
            "  Formatting /dev/sda1 as FAT32 (EFI)".to_string(),
            "  Formatting /dev/sda2 as ext4 (root)".to_string(),
            "SUCCESS: Formatting complete".to_string(),
            "==> Phase 3: Installing base packages".to_string(),
            ":: Synchronizing package databases...".to_string(),
            "  Installing base linux linux-firmware...".to_string(),
            "  Downloading packages (42%)...".to_string(),
            "WARNING: Slow mirror detected, trying next...".to_string(),
            "  Downloading packages (58%)...".to_string(),
        ];
        state
    }

    /// Create state for Complete mode demo
    pub fn demo_complete() -> Self {
        let mut state = Self::default();
        state.mode = AppMode::Complete;
        state.installation_progress = 100;
        state.status_message = "Installation completed successfully!".to_string();
        state.installer_output = vec![
            "==> Phase 8: Final configuration".to_string(),
            "  Generating fstab...".to_string(),
            "  Setting hostname to 'archlinux'".to_string(),
            "  Enabling NetworkManager service".to_string(),
            "SUCCESS: Installation complete!".to_string(),
        ];
        state
    }

    /// Create state for ToolDialog demo
    pub fn demo_tool_dialog() -> Self {
        let mut state = Self::default();
        state.mode = AppMode::ToolDialog;
        state.pre_dialog_mode = Some(AppMode::DiskTools);
        state.tool_dialog = Some(ToolDialogState {
            tool_name: "format_partition".to_string(),
            parameters: vec![
                ToolParam {
                    name: "device".to_string(),
                    description: "Block device to format (e.g. /dev/sda1)".to_string(),
                    param_type: ToolParameter::Text("/dev/sda1".to_string()),
                    required: true,
                },
                ToolParam {
                    name: "filesystem".to_string(),
                    description: "Filesystem type to create".to_string(),
                    param_type: ToolParameter::Selection(
                        vec!["ext4".into(), "xfs".into(), "btrfs".into(), "fat32".into()],
                        0,
                    ),
                    required: true,
                },
                ToolParam {
                    name: "label".to_string(),
                    description: "Optional filesystem label".to_string(),
                    param_type: ToolParameter::Text(String::new()),
                    required: false,
                },
            ],
            current_param: 0,
            param_values: vec!["/dev/sda1".to_string(), "ext4".to_string(), String::new()],
            is_executing: false,
        });
        state
    }

    /// Create state for FloatingOutput demo
    pub fn demo_floating_output() -> Self {
        let mut state = Self::default();
        state.mode = AppMode::FloatingOutput;
        state.pre_dialog_mode = Some(AppMode::DiskTools);
        state.floating_output = Some(FloatingOutputState {
            title: "Check Disk Health".to_string(),
            content: vec![
                "==> Running SMART diagnostics on /dev/sda".to_string(),
                "  Model: Samsung SSD 970 EVO Plus 1TB".to_string(),
                "  Serial: S4EWNF0M123456".to_string(),
                "  Firmware: 2B2QEXM7".to_string(),
                "".to_string(),
                "==> SMART Health Status: PASSED".to_string(),
                "  Temperature: 34C".to_string(),
                "  Power On Hours: 12,847".to_string(),
                "  Wear Leveling Count: 2%".to_string(),
                "  Available Spare: 100%".to_string(),
                "".to_string(),
                "SUCCESS: Disk health check complete".to_string(),
            ],
            scroll_offset: 0,
            auto_scroll: false,
            complete: true,
            progress: None,
            status: "Complete — press Esc to close".to_string(),
        });
        state
    }

    /// Create state for ConfirmDialog demo
    pub fn demo_confirm_dialog() -> Self {
        let mut state = Self::default();
        state.mode = AppMode::ConfirmDialog;
        state.pre_dialog_mode = Some(AppMode::DiskTools);
        state.confirm_dialog = Some(ConfirmDialogState {
            title: "WIPE ENTIRE DISK".to_string(),
            message: "Permanently erase ALL data on /dev/sda?".to_string(),
            details: vec![
                "ALL partitions will be destroyed".to_string(),
                "ALL data will be permanently erased".to_string(),
                "This operation CANNOT be undone".to_string(),
            ],
            severity: ConfirmSeverity::Danger,
            selected: 0,
            confirm_action: "wipe_disk".to_string(),
        });
        state
    }

    /// Create state for DryRunSummary demo
    pub fn demo_dry_run_summary() -> Self {
        let mut state = Self::default();
        state.mode = AppMode::DryRunSummary;
        state.dry_run_summary = Some(vec![
            "[DESTRUCTIVE] Partition /dev/sda (GPT)".to_string(),
            "  -> Create EFI partition: 512MB FAT32".to_string(),
            "  -> Create root partition: remaining ext4".to_string(),
            "".to_string(),
            "[DESTRUCTIVE] Format /dev/sda1 as FAT32".to_string(),
            "[DESTRUCTIVE] Format /dev/sda2 as ext4".to_string(),
            "".to_string(),
            "Install base packages via pacstrap".to_string(),
            "  -> base linux linux-firmware vim git".to_string(),
            "".to_string(),
            "Install GRUB bootloader".to_string(),
            "  -> Target: /dev/sda".to_string(),
            "".to_string(),
            "Configure system".to_string(),
            "  -> Hostname: archlinux".to_string(),
            "  -> Username: user".to_string(),
            "  -> Timezone: America/New_York".to_string(),
            "".to_string(),
            "[SKIP] Desktop environment: none".to_string(),
            "[SKIP] AUR helper: none".to_string(),
        ]);
        state
    }

    /// Create state for FileBrowser demo
    pub fn demo_file_browser() -> Self {
        let mut state = Self::default();
        state.mode = AppMode::FileBrowser;
        state.file_browser = Some(FileBrowserState {
            current_dir: "/home/user/configs".to_string(),
            entries: vec![
                FileEntry { name: "..".to_string(), is_dir: true, size: 0 },
                FileEntry { name: "arch-configs".to_string(), is_dir: true, size: 0 },
                FileEntry { name: "desktop.toml".to_string(), is_dir: false, size: 2048 },
                FileEntry { name: "minimal.toml".to_string(), is_dir: false, size: 1024 },
                FileEntry { name: "server.json".to_string(), is_dir: false, size: 3072 },
            ],
            selected: 2,
            error: None,
            scroll_offset: 0,
        });
        state
    }
}
