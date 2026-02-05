//! Property-Based Tests for ArchTUI
//!
//! P4.3: Uses proptest for testing invariants and edge cases
//!
//! These tests verify:
//! - Enum string round-trips (parse → to_string → parse)
//! - Configuration invariants
//! - Type safety properties

use proptest::prelude::*;

// =============================================================================
// Filesystem Enum Property Tests
// =============================================================================

use archtui::types::Filesystem;

/// Strategy for generating valid Filesystem variants
fn filesystem_strategy() -> impl Strategy<Value = Filesystem> {
    prop_oneof![
        Just(Filesystem::Ext4),
        Just(Filesystem::Btrfs),
        Just(Filesystem::Xfs),
        Just(Filesystem::F2fs),
        Just(Filesystem::Fat32),
    ]
}

proptest! {
    /// Filesystem: to_string → parse round-trip is identity
    #[test]
    fn filesystem_roundtrip(fs in filesystem_strategy()) {
        let s = fs.to_string();
        let parsed: Filesystem = s.parse().expect("Should parse");
        prop_assert_eq!(fs, parsed);
    }

    /// Filesystem: Display output is non-empty lowercase
    #[test]
    fn filesystem_display_is_valid(fs in filesystem_strategy()) {
        let s = fs.to_string();
        prop_assert!(!s.is_empty());
        let lowercase = s.to_lowercase();
        prop_assert_eq!(s, lowercase);
    }
}

// =============================================================================
// WipeMethod Enum Property Tests
// =============================================================================

use archtui::scripts::disk::WipeMethod;

/// Strategy for generating valid WipeMethod variants
fn wipe_method_strategy() -> impl Strategy<Value = WipeMethod> {
    prop_oneof![
        Just(WipeMethod::Quick),
        Just(WipeMethod::Secure),
        Just(WipeMethod::Auto),
    ]
}

proptest! {
    /// WipeMethod: to_string → parse round-trip is identity
    #[test]
    fn wipe_method_roundtrip(method in wipe_method_strategy()) {
        let s = method.to_string();
        let parsed: WipeMethod = s.parse().expect("Should parse");
        prop_assert_eq!(method, parsed);
    }

    /// WipeMethod: as_str returns valid lowercase string
    #[test]
    fn wipe_method_as_str_valid(method in wipe_method_strategy()) {
        let s = method.as_str();
        prop_assert!(!s.is_empty());
        prop_assert_eq!(s, s.to_lowercase());
        // Must be one of the valid values
        prop_assert!(["quick", "secure", "auto"].contains(&s));
    }
}

// =============================================================================
// AppMode Enum Property Tests
// =============================================================================

use archtui::app::AppMode;

/// Strategy for generating valid AppMode variants
fn app_mode_strategy() -> impl Strategy<Value = AppMode> {
    prop_oneof![
        Just(AppMode::MainMenu),
        Just(AppMode::GuidedInstaller),
        Just(AppMode::AutomatedInstall),
        Just(AppMode::ToolsMenu),
        Just(AppMode::DiskTools),
        Just(AppMode::SystemTools),
        Just(AppMode::UserTools),
        Just(AppMode::NetworkTools),
        Just(AppMode::ToolDialog),
        Just(AppMode::ToolExecution),
        Just(AppMode::Installation),
        Just(AppMode::Complete),
        Just(AppMode::EmbeddedTerminal),
        Just(AppMode::FloatingOutput),
        Just(AppMode::FileBrowser),
        Just(AppMode::ConfirmDialog),
        Just(AppMode::DryRunSummary),
    ]
}

proptest! {
    /// AppMode: clone is equal to original
    #[test]
    fn app_mode_clone_equals_original(mode in app_mode_strategy()) {
        let cloned = mode.clone();
        prop_assert_eq!(mode, cloned);
    }

    /// AppMode: equality is reflexive
    #[test]
    fn app_mode_equality_reflexive(mode in app_mode_strategy()) {
        let cloned = mode.clone();
        prop_assert_eq!(mode, cloned);
    }

    /// AppMode: Debug format is non-empty
    #[test]
    fn app_mode_debug_non_empty(mode in app_mode_strategy()) {
        let debug = format!("{:?}", mode);
        prop_assert!(!debug.is_empty());
    }
}

// =============================================================================
// ToolParameter Property Tests
// =============================================================================

use archtui::app::ToolParameter;

/// Strategy for generating ToolParameter variants
fn tool_parameter_strategy() -> impl Strategy<Value = ToolParameter> {
    prop_oneof![
        any::<String>().prop_map(ToolParameter::Text),
        any::<i32>().prop_map(ToolParameter::Number),
        any::<bool>().prop_map(ToolParameter::Boolean),
        any::<String>().prop_map(ToolParameter::Password),
        (prop::collection::vec(any::<String>(), 1..5), 0usize..5)
            .prop_map(|(opts, idx)| {
                let safe_idx = idx.min(opts.len().saturating_sub(1));
                ToolParameter::Selection(opts, safe_idx)
            }),
    ]
}

proptest! {
    /// ToolParameter: clone preserves data
    #[test]
    fn tool_parameter_clone_preserves_data(param in tool_parameter_strategy()) {
        let cloned = param.clone();
        match (param, cloned) {
            (ToolParameter::Text(a), ToolParameter::Text(b)) => prop_assert_eq!(a, b),
            (ToolParameter::Number(a), ToolParameter::Number(b)) => prop_assert_eq!(a, b),
            (ToolParameter::Boolean(a), ToolParameter::Boolean(b)) => prop_assert_eq!(a, b),
            (ToolParameter::Password(a), ToolParameter::Password(b)) => prop_assert_eq!(a, b),
            (ToolParameter::Selection(a, ai), ToolParameter::Selection(b, bi)) => {
                prop_assert_eq!(a, b);
                prop_assert_eq!(ai, bi);
            }
            _ => prop_assert!(false, "Mismatched variants after clone"),
        }
    }
}

// =============================================================================
// Configuration Property Tests
// =============================================================================

use archtui::config::Configuration;

proptest! {
    /// Configuration: default has non-empty options
    #[test]
    fn config_default_has_options(_seed in any::<u64>()) {
        let config = Configuration::default();
        prop_assert!(!config.options.is_empty());
    }

    /// Configuration: all option names are non-empty
    #[test]
    fn config_option_names_non_empty(_seed in any::<u64>()) {
        let config = Configuration::default();
        for option in &config.options {
            prop_assert!(!option.name.is_empty(), "Option name should not be empty");
        }
    }

    /// Configuration: to_env_vars produces valid key-value pairs
    #[test]
    fn config_env_vars_valid(_seed in any::<u64>()) {
        let config = Configuration::default();
        let env_vars = config.to_env_vars();

        for (key, _value) in &env_vars {
            // Keys should be uppercase with underscores (env var convention)
            prop_assert!(!key.is_empty(), "Env var key should not be empty");
            // Keys should not contain spaces
            prop_assert!(!key.contains(' '), "Env var key should not contain spaces: {}", key);
        }
    }
}

// =============================================================================
// String Input Validation Property Tests
// =============================================================================

proptest! {
    /// Arbitrary strings don't crash Filesystem parsing
    #[test]
    fn filesystem_parse_doesnt_crash(s in ".*") {
        // Should not panic, just return Err for invalid input
        let _ = s.parse::<Filesystem>();
    }

    /// Arbitrary strings don't crash WipeMethod parsing
    #[test]
    fn wipe_method_parse_doesnt_crash(s in ".*") {
        let _ = s.parse::<WipeMethod>();
    }

    /// Valid filesystem strings always parse (lowercase only per strum config)
    #[test]
    fn valid_filesystem_strings_parse(fs_str in prop_oneof![
        Just("ext4"),
        Just("btrfs"),
        Just("xfs"),
        Just("f2fs"),
        Just("fat32"),
    ]) {
        let result = fs_str.parse::<Filesystem>();
        prop_assert!(result.is_ok(), "Valid filesystem string '{}' should parse", fs_str);
    }
}

// =============================================================================
// ScriptOutput Property Tests
// =============================================================================

use archtui::script_runner::ScriptOutput;

proptest! {
    /// ScriptOutput: success=true means exit_code is 0
    #[test]
    fn script_output_success_implies_zero(
        stdout in ".*",
        stderr in ".*",
    ) {
        let output = ScriptOutput {
            stdout,
            stderr,
            exit_code: Some(0),
            success: true,
            dry_run: false,
        };

        prop_assert!(output.success);
        prop_assert_eq!(output.exit_code, Some(0));
        prop_assert!(output.ensure_success("test").is_ok());
    }

    /// ScriptOutput: success=false returns error from ensure_success
    #[test]
    fn script_output_failure_returns_error(
        stdout in ".*",
        stderr in ".*",
        exit_code in 1i32..256,
    ) {
        let output = ScriptOutput {
            stdout,
            stderr,
            exit_code: Some(exit_code),
            success: false,
            dry_run: false,
        };

        prop_assert!(!output.success);
        prop_assert!(output.ensure_success("test").is_err());
    }

    /// ScriptOutput: dry_run outputs are always successful
    #[test]
    fn script_output_dry_run_is_success(
        script_name in "[a-z_]+\\.sh",
    ) {
        let output = ScriptOutput {
            stdout: format!("[DRY RUN] Skipped: {}\n", script_name),
            stderr: String::new(),
            exit_code: Some(0),
            success: true,
            dry_run: true,
        };

        prop_assert!(output.dry_run);
        prop_assert!(output.success);
        prop_assert!(output.stdout.contains("DRY RUN"));
    }
}
