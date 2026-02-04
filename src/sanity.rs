//! Pre-flight sanity checks for runtime environment
//!
//! This module verifies the system environment before the TUI starts:
//! - Required runtime binaries are present
//! - Running with root privileges (EUID 0)
//!
//! If any check fails, the program exits with a clear error message
//! before the TUI is initialized.

use crate::process_guard::CommandProcessGroup;
use std::process::Command;

/// Result of environment verification
#[derive(Debug)]
pub struct SanityCheckResult {
    pub missing_binaries: Vec<String>,
    pub is_root: bool,
}

impl SanityCheckResult {
    /// Returns true if all checks passed
    pub fn is_ok(&self) -> bool {
        self.missing_binaries.is_empty() && self.is_root
    }
}

/// Required runtime binaries for installation
const REQUIRED_BINARIES: &[&str] = &[
    "bash",       // Script execution
    "sgdisk",     // GPT partitioning (gdisk package)
    "mkfs.ext4",  // Filesystem creation (e2fsprogs)
    "ip",         // Network configuration (iproute2)
    "lsblk",      // Block device listing (util-linux)
];

/// Optional binaries (warn if missing but don't fail)
const OPTIONAL_BINARIES: &[&str] = &[
    "jq", // JSON processing (only needed for direct bash script usage)
];

/// Check if a binary is available in PATH
fn binary_exists(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .in_new_process_group()
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Check if running as root (EUID 0)
fn is_running_as_root() -> bool {
    // Using nix crate for reliable EUID check
    nix::unistd::geteuid().is_root()
}

/// Perform all sanity checks and return the result
pub fn verify_environment() -> SanityCheckResult {
    let mut missing = Vec::new();

    // Check required binaries
    for binary in REQUIRED_BINARIES {
        if !binary_exists(binary) {
            missing.push((*binary).to_string());
        }
    }

    // Check optional binaries (just warn, don't add to missing)
    for binary in OPTIONAL_BINARIES {
        if !binary_exists(binary) {
            log::debug!("Optional binary not found: {} (not required for TUI mode)", binary);
        }
    }

    SanityCheckResult {
        missing_binaries: missing,
        is_root: is_running_as_root(),
    }
}

/// Print a pretty error message to stderr and exit
/// This is called before TUI initialization, so we can safely print to stderr
pub fn print_error_and_exit(result: &SanityCheckResult) -> ! {
    eprintln!();
    eprintln!("╔══════════════════════════════════════════════════════════════════╗");
    eprintln!("║              ArchTUI - Pre-flight Check Failed           ║");
    eprintln!("╚══════════════════════════════════════════════════════════════════╝");
    eprintln!();

    if !result.is_root {
        eprintln!("❌ ERROR: Root privileges required");
        eprintln!("   This installer must be run as root to partition disks and install packages.");
        eprintln!();
        eprintln!("   Solution: Run with sudo or as root user:");
        eprintln!("     sudo ./archtui");
        eprintln!();
    }

    if !result.missing_binaries.is_empty() {
        eprintln!("❌ ERROR: Missing required binaries");
        eprintln!();
        for binary in &result.missing_binaries {
            let package = get_package_for_binary(binary);
            eprintln!("   • {} (install: pacman -S {})", binary, package);
        }
        eprintln!();
        eprintln!("   Solution: Install missing packages:");
        let packages: Vec<&str> = result
            .missing_binaries
            .iter()
            .map(|b| get_package_for_binary(b))
            .collect();
        eprintln!("     pacman -S {}", packages.join(" "));
        eprintln!();
    }

    eprintln!("╔══════════════════════════════════════════════════════════════════╗");
    eprintln!("║  Fix the above issues and try again.                             ║");
    eprintln!("╚══════════════════════════════════════════════════════════════════╝");
    eprintln!();

    std::process::exit(1);
}

/// Map binary names to their Arch Linux package names
fn get_package_for_binary(binary: &str) -> &'static str {
    match binary {
        "bash" => "bash",
        "jq" => "jq",
        "sgdisk" => "gptfdisk",
        "mkfs.ext4" => "e2fsprogs",
        "ip" => "iproute2",
        "lsblk" => "util-linux",
        _ => "unknown", // Fallback for unknown binaries
    }
}

/// Main entry point: verify environment and exit if checks fail
/// Call this before initializing the TUI
pub fn run_preflight_checks() {
    log::debug!("Running pre-flight sanity checks...");

    let result = verify_environment();

    if !result.is_ok() {
        print_error_and_exit(&result);
    }

    log::info!("Pre-flight checks passed: root={}, all binaries present", result.is_root);
}

/// Skip root check (for development/testing)
/// Set ARCHTUI_SKIP_ROOT_CHECK=1 to skip
pub fn should_skip_root_check() -> bool {
    std::env::var("ARCHTUI_SKIP_ROOT_CHECK")
        .map(|v| v == "1" || v.to_lowercase() == "true")
        .unwrap_or(false)
}

/// Run pre-flight checks with optional root check skip
pub fn run_preflight_checks_with_options(skip_root: bool) {
    log::debug!("Running pre-flight sanity checks (skip_root={})...", skip_root);

    let mut result = verify_environment();

    // Allow skipping root check for development
    if skip_root || should_skip_root_check() {
        log::warn!("Root check skipped (ARCHTUI_SKIP_ROOT_CHECK=1)");
        result.is_root = true; // Pretend we're root
    }

    if !result.is_ok() {
        print_error_and_exit(&result);
    }

    log::info!("Pre-flight checks passed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_binary_exists_bash() {
        // bash should always exist
        assert!(binary_exists("bash"), "bash should be available");
    }

    #[test]
    fn test_binary_exists_nonexistent() {
        assert!(!binary_exists("this_binary_definitely_does_not_exist_12345"));
    }

    #[test]
    fn test_verify_environment_finds_bash() {
        let result = verify_environment();
        assert!(
            !result.missing_binaries.contains(&"bash".to_string()),
            "bash should not be in missing binaries"
        );
    }

    #[test]
    fn test_package_mapping() {
        assert_eq!(get_package_for_binary("sgdisk"), "gptfdisk");
        assert_eq!(get_package_for_binary("mkfs.ext4"), "e2fsprogs");
        assert_eq!(get_package_for_binary("ip"), "iproute2");
    }

    #[test]
    fn test_sanity_result_is_ok() {
        let ok_result = SanityCheckResult {
            missing_binaries: vec![],
            is_root: true,
        };
        assert!(ok_result.is_ok());

        let missing_binary = SanityCheckResult {
            missing_binaries: vec!["test".to_string()],
            is_root: true,
        };
        assert!(!missing_binary.is_ok());

        let not_root = SanityCheckResult {
            missing_binaries: vec![],
            is_root: false,
        };
        assert!(!not_root.is_ok());
    }
}
