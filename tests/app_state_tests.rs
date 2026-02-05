//! Tests for Application State Management
//!
//! P3.1: Tests for AppMode, AppState, and state transitions
//!
//! These tests verify:
//! - AppState default initialization
//! - AppMode enum completeness
//! - ToolParameter and ToolDialogState behavior
//! - State field interactions

use archtui::app::{AppMode, AppState, ToolDialogState, ToolParam, ToolParameter};

// =============================================================================
// AppState Default Tests
// =============================================================================

#[test]
fn test_app_state_default_mode_is_main_menu() {
    let state = AppState::default();
    assert_eq!(state.mode, AppMode::MainMenu);
}

#[test]
fn test_app_state_default_has_welcome_message() {
    let state = AppState::default();
    assert!(state.status_message.contains("Welcome"));
}

#[test]
fn test_app_state_default_selections_are_zero() {
    let state = AppState::default();
    assert_eq!(state.main_menu_selection, 0);
    assert_eq!(state.tools_menu_selection, 0);
}

#[test]
fn test_app_state_default_progress_is_zero() {
    let state = AppState::default();
    assert_eq!(state.installation_progress, 0);
}

#[test]
fn test_app_state_default_help_not_visible() {
    let state = AppState::default();
    assert!(!state.help_visible);
}

#[test]
fn test_app_state_default_no_current_tool() {
    let state = AppState::default();
    assert!(state.current_tool.is_none());
}

#[test]
fn test_app_state_default_no_dialogs() {
    let state = AppState::default();
    assert!(state.tool_dialog.is_none());
    assert!(state.floating_output.is_none());
    assert!(state.embedded_terminal.is_none());
    assert!(state.file_browser.is_none());
    assert!(state.confirm_dialog.is_none());
}

#[test]
fn test_app_state_default_installer_button_is_start() {
    let state = AppState::default();
    // 1 = Start Install button (not 0 = Test Config)
    assert_eq!(state.installer_button_selection, 1);
}

#[test]
fn test_app_state_default_config_has_options() {
    let state = AppState::default();
    assert!(!state.config.options.is_empty());
}

// =============================================================================
// AppMode Enum Tests
// =============================================================================

#[test]
fn test_app_mode_equality() {
    assert_eq!(AppMode::MainMenu, AppMode::MainMenu);
    assert_ne!(AppMode::MainMenu, AppMode::ToolsMenu);
}

#[test]
fn test_app_mode_clone() {
    let mode = AppMode::GuidedInstaller;
    let cloned = mode.clone();
    assert_eq!(mode, cloned);
}

#[test]
fn test_app_mode_debug_format() {
    let mode = AppMode::DiskTools;
    let debug = format!("{:?}", mode);
    assert!(debug.contains("DiskTools"));
}

#[test]
fn test_app_mode_hash_consistency() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(AppMode::MainMenu);
    set.insert(AppMode::ToolsMenu);
    set.insert(AppMode::MainMenu); // Duplicate

    assert_eq!(set.len(), 2);
}

#[test]
fn test_all_app_modes_are_distinct() {
    use std::collections::HashSet;

    let modes = vec![
        AppMode::MainMenu,
        AppMode::GuidedInstaller,
        AppMode::AutomatedInstall,
        AppMode::ToolsMenu,
        AppMode::DiskTools,
        AppMode::SystemTools,
        AppMode::UserTools,
        AppMode::NetworkTools,
        AppMode::ToolDialog,
        AppMode::ToolExecution,
        AppMode::Installation,
        AppMode::Complete,
        AppMode::EmbeddedTerminal,
        AppMode::FloatingOutput,
        AppMode::FileBrowser,
        AppMode::ConfirmDialog,
        AppMode::DryRunSummary,
    ];

    let set: HashSet<_> = modes.iter().collect();
    assert_eq!(set.len(), modes.len(), "All AppModes should be distinct");
}

// =============================================================================
// ToolParameter Tests
// =============================================================================

#[test]
fn test_tool_parameter_text() {
    let param = ToolParameter::Text("default".to_string());
    if let ToolParameter::Text(value) = param {
        assert_eq!(value, "default");
    } else {
        panic!("Expected Text variant");
    }
}

#[test]
fn test_tool_parameter_number() {
    let param = ToolParameter::Number(42);
    if let ToolParameter::Number(value) = param {
        assert_eq!(value, 42);
    } else {
        panic!("Expected Number variant");
    }
}

#[test]
fn test_tool_parameter_boolean() {
    let param = ToolParameter::Boolean(true);
    if let ToolParameter::Boolean(value) = param {
        assert!(value);
    } else {
        panic!("Expected Boolean variant");
    }
}

#[test]
fn test_tool_parameter_selection() {
    let options = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let param = ToolParameter::Selection(options.clone(), 1);
    if let ToolParameter::Selection(opts, idx) = param {
        assert_eq!(opts, options);
        assert_eq!(idx, 1);
    } else {
        panic!("Expected Selection variant");
    }
}

#[test]
fn test_tool_parameter_password() {
    let param = ToolParameter::Password("secret".to_string());
    if let ToolParameter::Password(value) = param {
        assert_eq!(value, "secret");
    } else {
        panic!("Expected Password variant");
    }
}

#[test]
fn test_tool_parameter_clone() {
    let param = ToolParameter::Text("test".to_string());
    let cloned = param.clone();
    if let (ToolParameter::Text(a), ToolParameter::Text(b)) = (param, cloned) {
        assert_eq!(a, b);
    }
}

// =============================================================================
// ToolParam Tests
// =============================================================================

#[test]
fn test_tool_param_required_field() {
    let param = ToolParam {
        name: "device".to_string(),
        description: "Target device".to_string(),
        param_type: ToolParameter::Text("/dev/sda".to_string()),
        required: true,
    };

    assert!(param.required);
    assert_eq!(param.name, "device");
}

#[test]
fn test_tool_param_optional_field() {
    let param = ToolParam {
        name: "label".to_string(),
        description: "Partition label (optional)".to_string(),
        param_type: ToolParameter::Text("".to_string()),
        required: false,
    };

    assert!(!param.required);
}

// =============================================================================
// ToolDialogState Tests
// =============================================================================

#[test]
fn test_tool_dialog_state_creation() {
    let state = ToolDialogState {
        tool_name: "format_partition".to_string(),
        parameters: vec![
            ToolParam {
                name: "device".to_string(),
                description: "Device to format".to_string(),
                param_type: ToolParameter::Text("".to_string()),
                required: true,
            },
            ToolParam {
                name: "filesystem".to_string(),
                description: "Filesystem type".to_string(),
                param_type: ToolParameter::Selection(
                    vec!["ext4".to_string(), "xfs".to_string()],
                    0,
                ),
                required: true,
            },
        ],
        current_param: 0,
        param_values: vec!["".to_string(), "ext4".to_string()],
        is_executing: false,
    };

    assert_eq!(state.tool_name, "format_partition");
    assert_eq!(state.parameters.len(), 2);
    assert_eq!(state.current_param, 0);
    assert!(!state.is_executing);
}

#[test]
fn test_tool_dialog_state_param_navigation() {
    let mut state = ToolDialogState {
        tool_name: "test".to_string(),
        parameters: vec![
            ToolParam {
                name: "a".to_string(),
                description: "".to_string(),
                param_type: ToolParameter::Text("".to_string()),
                required: true,
            },
            ToolParam {
                name: "b".to_string(),
                description: "".to_string(),
                param_type: ToolParameter::Text("".to_string()),
                required: true,
            },
        ],
        current_param: 0,
        param_values: vec!["".to_string(), "".to_string()],
        is_executing: false,
    };

    assert_eq!(state.current_param, 0);

    // Simulate navigation down
    if state.current_param < state.parameters.len() - 1 {
        state.current_param += 1;
    }
    assert_eq!(state.current_param, 1);

    // Simulate navigation up
    if state.current_param > 0 {
        state.current_param -= 1;
    }
    assert_eq!(state.current_param, 0);
}

// =============================================================================
// AppState Mutation Tests
// =============================================================================

#[test]
fn test_app_state_mode_transition() {
    let mut state = AppState::default();
    assert_eq!(state.mode, AppMode::MainMenu);

    state.mode = AppMode::ToolsMenu;
    assert_eq!(state.mode, AppMode::ToolsMenu);

    state.mode = AppMode::DiskTools;
    assert_eq!(state.mode, AppMode::DiskTools);
}

#[test]
fn test_app_state_menu_selection_bounds() {
    let mut state = AppState::default();

    // Main menu has 4 items (0-3)
    state.main_menu_selection = 0;
    assert_eq!(state.main_menu_selection, 0);

    state.main_menu_selection = 3;
    assert_eq!(state.main_menu_selection, 3);
}

#[test]
fn test_app_state_progress_update() {
    let mut state = AppState::default();

    state.installation_progress = 50;
    assert_eq!(state.installation_progress, 50);

    state.installation_progress = 100;
    assert_eq!(state.installation_progress, 100);
}

#[test]
fn test_app_state_output_accumulation() {
    let mut state = AppState::default();

    state.installer_output.push("Line 1".to_string());
    state.installer_output.push("Line 2".to_string());

    assert_eq!(state.installer_output.len(), 2);
    assert_eq!(state.installer_output[0], "Line 1");
}

#[test]
fn test_app_state_status_message_update() {
    let mut state = AppState::default();

    state.status_message = "Installing...".to_string();
    assert_eq!(state.status_message, "Installing...");
}

#[test]
fn test_app_state_pre_dialog_mode() {
    let mut state = AppState::default();

    // Set up confirm dialog flow
    state.pre_dialog_mode = Some(AppMode::DiskTools);
    state.mode = AppMode::ConfirmDialog;

    // After dialog, restore previous mode
    if let Some(prev_mode) = state.pre_dialog_mode.take() {
        state.mode = prev_mode;
    }

    assert_eq!(state.mode, AppMode::DiskTools);
    assert!(state.pre_dialog_mode.is_none());
}

#[test]
fn test_app_state_clone() {
    let state = AppState::default();
    let cloned = state.clone();

    assert_eq!(state.mode, cloned.mode);
    assert_eq!(state.main_menu_selection, cloned.main_menu_selection);
    assert_eq!(state.status_message, cloned.status_message);
}
