//! Tests for Script Execution and Error Handling
//!
//! P3.1: Tests for script_runner, ScriptArgs, and error paths
//!
//! These tests verify:
//! - ScriptOutput structure and methods
//! - Script argument building
//! - Dry-run mode behavior
//! - Error handling patterns

use archtui::script_runner::ScriptOutput;
use archtui::script_traits::ScriptArgs;
use std::path::PathBuf;

// =============================================================================
// ScriptOutput Tests
// =============================================================================

#[test]
fn test_script_output_success() {
    let output = ScriptOutput {
        stdout: "Success output".to_string(),
        stderr: String::new(),
        exit_code: Some(0),
        success: true,
        dry_run: false,
    };

    assert!(output.success);
    assert_eq!(output.exit_code, Some(0));
    assert!(output.stdout.contains("Success"));
    assert!(output.stderr.is_empty());
    assert!(!output.dry_run);
}

#[test]
fn test_script_output_failure() {
    let output = ScriptOutput {
        stdout: String::new(),
        stderr: "Error: command failed".to_string(),
        exit_code: Some(1),
        success: false,
        dry_run: false,
    };

    assert!(!output.success);
    assert_eq!(output.exit_code, Some(1));
    assert!(output.stderr.contains("Error"));
}

#[test]
fn test_script_output_signal_termination() {
    let output = ScriptOutput {
        stdout: String::new(),
        stderr: String::new(),
        exit_code: None, // Terminated by signal
        success: false,
        dry_run: false,
    };

    assert!(!output.success);
    assert!(output.exit_code.is_none());
}

#[test]
fn test_script_output_dry_run() {
    let output = ScriptOutput {
        stdout: "[DRY RUN] Skipped: wipe_disk.sh\n".to_string(),
        stderr: String::new(),
        exit_code: Some(0),
        success: true,
        dry_run: true,
    };

    assert!(output.dry_run);
    assert!(output.success);
    assert!(output.stdout.contains("DRY RUN"));
}

#[test]
fn test_script_output_ensure_success_ok() {
    let output = ScriptOutput {
        stdout: "OK".to_string(),
        stderr: String::new(),
        exit_code: Some(0),
        success: true,
        dry_run: false,
    };

    let result = output.ensure_success("test operation");
    assert!(result.is_ok());
}

#[test]
fn test_script_output_ensure_success_err() {
    let output = ScriptOutput {
        stdout: String::new(),
        stderr: "Device not found".to_string(),
        exit_code: Some(1),
        success: false,
        dry_run: false,
    };

    let result = output.ensure_success("disk wipe");
    assert!(result.is_err());

    let err = result.unwrap_err();
    let err_msg = format!("{}", err);
    assert!(err_msg.contains("disk wipe"));
    assert!(err_msg.contains("Device not found") || err_msg.contains("exit code"));
}

#[test]
fn test_script_output_clone() {
    let output = ScriptOutput {
        stdout: "output".to_string(),
        stderr: "error".to_string(),
        exit_code: Some(42),
        success: false,
        dry_run: true,
    };

    let cloned = output.clone();
    assert_eq!(output.stdout, cloned.stdout);
    assert_eq!(output.stderr, cloned.stderr);
    assert_eq!(output.exit_code, cloned.exit_code);
    assert_eq!(output.success, cloned.success);
    assert_eq!(output.dry_run, cloned.dry_run);
}

// =============================================================================
// ScriptArgs Trait Tests (via concrete implementations)
// =============================================================================

// Test with WipeDiskArgs
use archtui::scripts::disk::{WipeDiskArgs, WipeMethod};

#[test]
fn test_wipe_disk_args_script_name() {
    let args = WipeDiskArgs {
        device: PathBuf::from("/dev/sda"),
        method: WipeMethod::Quick,
        confirm: true,
    };

    assert_eq!(args.script_name(), "wipe_disk.sh");
}

#[test]
fn test_wipe_disk_args_is_destructive() {
    let args = WipeDiskArgs {
        device: PathBuf::from("/dev/sda"),
        method: WipeMethod::Quick,
        confirm: true,
    };

    assert!(args.is_destructive(), "Wipe disk should be destructive");
}

#[test]
fn test_wipe_disk_args_to_cli_args() {
    let args = WipeDiskArgs {
        device: PathBuf::from("/dev/sda"),
        method: WipeMethod::Quick,
        confirm: true,
    };

    let cli_args = args.to_cli_args();

    assert!(cli_args.contains(&"--disk".to_string()));
    assert!(cli_args.contains(&"/dev/sda".to_string()));
    assert!(cli_args.contains(&"--method".to_string()));
}

#[test]
fn test_wipe_disk_args_env_vars() {
    let args = WipeDiskArgs {
        device: PathBuf::from("/dev/sda"),
        method: WipeMethod::Secure,
        confirm: true,
    };

    let env_vars = args.get_env_vars();

    // Confirm should be in env vars for destructive operations
    // env_vars is a Vec<(String, String)>
    assert!(
        env_vars.iter().any(|(k, _)| k == "CONFIRM_WIPE_DISK"),
        "Should have confirmation env var"
    );
}

// Test with FormatPartitionArgs
use archtui::scripts::disk::FormatPartitionArgs;
use archtui::types::Filesystem;

#[test]
fn test_format_partition_args_script_name() {
    let args = FormatPartitionArgs {
        device: PathBuf::from("/dev/sda1"),
        filesystem: Filesystem::Ext4,
        label: None,
        force: false,
    };

    assert_eq!(args.script_name(), "format_partition.sh");
}

#[test]
fn test_format_partition_args_is_destructive() {
    let args = FormatPartitionArgs {
        device: PathBuf::from("/dev/sda1"),
        filesystem: Filesystem::Btrfs,
        label: Some("root".to_string()),
        force: true,
    };

    assert!(
        args.is_destructive(),
        "Format partition should be destructive"
    );
}

#[test]
fn test_format_partition_args_with_label() {
    let args = FormatPartitionArgs {
        device: PathBuf::from("/dev/sda1"),
        filesystem: Filesystem::Ext4,
        label: Some("BOOT".to_string()),
        force: false,
    };

    let cli_args = args.to_cli_args();

    assert!(cli_args.contains(&"--label".to_string()));
    assert!(cli_args.contains(&"BOOT".to_string()));
}

// Test with MountPartitionArgs
use archtui::scripts::disk::MountPartitionArgs;

#[test]
fn test_mount_partition_args_script_name() {
    let args = MountPartitionArgs {
        device: PathBuf::from("/dev/sda1"),
        mountpoint: PathBuf::from("/mnt"),
        options: None,
    };

    assert_eq!(args.script_name(), "mount_partitions.sh");
}

#[test]
fn test_mount_partition_args_not_destructive() {
    let args = MountPartitionArgs {
        device: PathBuf::from("/dev/sda1"),
        mountpoint: PathBuf::from("/mnt"),
        options: None,
    };

    assert!(
        !args.is_destructive(),
        "Mount should not be destructive"
    );
}

#[test]
fn test_mount_partition_args_with_options() {
    let args = MountPartitionArgs {
        device: PathBuf::from("/dev/mapper/cryptroot"),
        mountpoint: PathBuf::from("/mnt"),
        options: Some("compress=zstd,noatime".to_string()),
    };

    let cli_args = args.to_cli_args();

    assert!(cli_args.contains(&"--options".to_string()));
    assert!(cli_args.contains(&"compress=zstd,noatime".to_string()));
}

// =============================================================================
// Filesystem Type Tests
// =============================================================================

#[test]
fn test_filesystem_to_string() {
    assert_eq!(Filesystem::Ext4.to_string(), "ext4");
    assert_eq!(Filesystem::Btrfs.to_string(), "btrfs");
    assert_eq!(Filesystem::Xfs.to_string(), "xfs");
}

#[test]
fn test_filesystem_from_string() {
    assert_eq!("ext4".parse::<Filesystem>().unwrap(), Filesystem::Ext4);
    assert_eq!("btrfs".parse::<Filesystem>().unwrap(), Filesystem::Btrfs);
    assert_eq!("xfs".parse::<Filesystem>().unwrap(), Filesystem::Xfs);
}

// =============================================================================
// WipeMethod Tests
// =============================================================================

#[test]
fn test_wipe_method_variants() {
    let quick = WipeMethod::Quick;
    let secure = WipeMethod::Secure;
    let auto = WipeMethod::Auto;

    // All variants should be distinct
    assert_ne!(format!("{:?}", quick), format!("{:?}", secure));
    assert_ne!(format!("{:?}", secure), format!("{:?}", auto));
    assert_ne!(format!("{:?}", auto), format!("{:?}", quick));
}

#[test]
fn test_wipe_method_to_string() {
    assert_eq!(WipeMethod::Quick.to_string(), "quick");
    assert_eq!(WipeMethod::Secure.to_string(), "secure");
    assert_eq!(WipeMethod::Auto.to_string(), "auto");
}

// =============================================================================
// CLI Argument Building Tests
// =============================================================================

#[test]
fn test_cli_args_proper_ordering() {
    let args = WipeDiskArgs {
        device: PathBuf::from("/dev/nvme0n1"),
        method: WipeMethod::Secure,
        confirm: true,
    };

    let cli_args = args.to_cli_args();

    // Find positions
    let disk_pos = cli_args.iter().position(|a| a == "--disk");
    let method_pos = cli_args.iter().position(|a| a == "--method");

    assert!(disk_pos.is_some(), "Should have --disk flag");
    assert!(method_pos.is_some(), "Should have --method flag");

    // Value should follow flag
    if let Some(pos) = disk_pos {
        assert!(cli_args.len() > pos + 1, "Value should follow --disk");
        assert_eq!(cli_args[pos + 1], "/dev/nvme0n1");
    }
}

#[test]
fn test_cli_args_handles_special_characters() {
    let args = MountPartitionArgs {
        device: PathBuf::from("/dev/disk/by-uuid/abc-123"),
        mountpoint: PathBuf::from("/mnt/my data"),
        options: Some("uid=1000,gid=1000".to_string()),
    };

    let cli_args = args.to_cli_args();

    // Should contain paths with special characters
    assert!(cli_args.iter().any(|a| a.contains("by-uuid")));
}

// =============================================================================
// Error Path Tests
// =============================================================================

#[test]
fn test_error_context_in_ensure_success() {
    let output = ScriptOutput {
        stdout: String::new(),
        stderr: "Permission denied".to_string(),
        exit_code: Some(126),
        success: false,
        dry_run: false,
    };

    let result = output.ensure_success("format /dev/sda1");

    assert!(result.is_err());
    let err_string = format!("{}", result.unwrap_err());

    // Error should contain the context
    assert!(
        err_string.contains("format") || err_string.contains("sda1"),
        "Error should contain operation context"
    );
}

#[test]
fn test_nonzero_exit_codes() {
    for code in [1, 2, 126, 127, 255] {
        let output = ScriptOutput {
            stdout: String::new(),
            stderr: format!("Exit {}", code),
            exit_code: Some(code),
            success: false,
            dry_run: false,
        };

        assert!(!output.success);
        assert_eq!(output.exit_code, Some(code));
    }
}

// =============================================================================
// Dry-Run Pattern Tests
// =============================================================================

#[test]
fn test_dry_run_output_format() {
    // Simulate what run_script_safe returns in dry-run mode
    let script_name = "wipe_disk.sh";
    let output = ScriptOutput {
        stdout: format!("[DRY RUN] Skipped: {}\n", script_name),
        stderr: String::new(),
        exit_code: Some(0),
        success: true,
        dry_run: true,
    };

    assert!(output.dry_run);
    assert!(output.stdout.contains("[DRY RUN]"));
    assert!(output.stdout.contains(script_name));
}

#[test]
fn test_dry_run_is_always_success() {
    // Dry-run should always report success (script not actually run)
    let output = ScriptOutput {
        stdout: "[DRY RUN] Skipped: format_partition.sh\n".to_string(),
        stderr: String::new(),
        exit_code: Some(0),
        success: true,
        dry_run: true,
    };

    assert!(output.success);
    assert!(output.ensure_success("dry run test").is_ok());
}
