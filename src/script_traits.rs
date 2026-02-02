//! Type-safe script argument contracts.
//!
//! This module provides the `ScriptArgs` trait for ensuring compile-time correctness
//! of script arguments. Instead of raw string vectors, Rust structs implement
//! this trait to produce validated CLI arguments and environment variables.
//!
//! # Design Goals
//!
//! 1. **Compile-Time Safety**: Argument mismatches (e.g., `--device` vs `--disk`)
//!    are caught at compile time, not runtime.
//! 2. **Single Source of Truth**: The struct definition IS the contract.
//! 3. **Environment Contracts**: Confirmation flags are passed via env vars,
//!    matching the bash script expectations.
//!
//! # Dry-Run Mode (Sprint 8)
//!
//! Scripts can declare themselves as destructive via `is_destructive()`. When
//! the global `DRY_RUN` flag is set, destructive scripts are NOT executed.
//! Instead, a log message shows what WOULD have been executed.
//!
//! Non-destructive scripts (like `lsblk` or network checks) still execute
//! so the dry-run produces realistic output.

use std::sync::atomic::{AtomicBool, Ordering};

/// Global dry-run flag.
///
/// When set to `true`, destructive operations are skipped and logged instead.
/// This allows users to preview what the installer will do without risk.
///
/// # Thread Safety
///
/// Uses `AtomicBool` with `Ordering::SeqCst` for correct cross-thread visibility.
/// Set once at startup based on `--dry-run` CLI flag.
static DRY_RUN: AtomicBool = AtomicBool::new(false);

/// Enable dry-run mode globally.
///
/// Called at startup if `--dry-run` CLI flag is present.
pub fn enable_dry_run() {
    DRY_RUN.store(true, Ordering::SeqCst);
    log::info!("[DRY RUN] Mode enabled - destructive operations will be skipped");
}

/// Disable dry-run mode globally.
///
/// Used in tests and when toggling dry-run mode dynamically.
#[allow(dead_code)]
pub fn disable_dry_run() {
    DRY_RUN.store(false, Ordering::SeqCst);
}

/// Check if dry-run mode is enabled.
pub fn is_dry_run() -> bool {
    DRY_RUN.load(Ordering::SeqCst)
}

/// Trait for typed script arguments.
///
/// Implementors define the mapping between Rust struct fields and bash script
/// flags/environment variables. This ensures the compiler catches flag mismatches.
///
/// # Contract
///
/// - `to_cli_args()`: Returns CLI arguments exactly as the bash script expects them.
/// - `get_env_vars()`: Returns environment variables required by the script.
/// - `script_name()`: Returns the script filename (e.g., "wipe_disk.sh").
/// - `is_destructive()`: Returns `true` if the script modifies disk/system state.
///
/// # Invariants
///
/// - The returned CLI args MUST match the bash script's argument parser.
/// - Environment variables MUST match the script's environment contract.
/// - Scripts are identified by name only (path is resolved at execution time).
/// - Destructive scripts MUST return `true` from `is_destructive()`.
///
/// # Example
///
/// ```ignore
/// use archinstall_tui::script_traits::ScriptArgs;
/// use archinstall_tui::scripts::disk::WipeDiskArgs;
///
/// let args = WipeDiskArgs {
///     device: PathBuf::from("/dev/sda"),
///     method: WipeMethod::Quick,
///     confirm: true,
/// };
///
/// // Compiler enforces correct flag names
/// let cli_args = args.to_cli_args();  // ["--disk", "/dev/sda", "--method", "quick"]
/// let env_vars = args.get_env_vars(); // [("CONFIRM_WIPE_DISK", "yes")]
/// assert!(args.is_destructive());     // Wipe is destructive
/// ```
pub trait ScriptArgs {
    /// Convert struct fields to CLI arguments.
    ///
    /// Returns a vector of strings exactly as they should be passed to the script.
    /// Example: `["--disk", "/dev/sda", "--method", "quick"]`
    fn to_cli_args(&self) -> Vec<String>;

    /// Get required environment variables.
    ///
    /// Returns key-value pairs for environment variables the script requires.
    /// Example: `[("CONFIRM_WIPE_DISK", "yes")]`
    fn get_env_vars(&self) -> Vec<(String, String)>;

    /// Get the script filename.
    ///
    /// Returns the script name without path (e.g., "wipe_disk.sh").
    /// The execution layer resolves the full path.
    fn script_name(&self) -> &'static str;

    /// Check if this script performs destructive operations.
    ///
    /// Returns `true` for scripts that modify disk, format partitions,
    /// install packages, or otherwise change system state.
    ///
    /// Returns `false` for read-only scripts like `lsblk`, `system_info`,
    /// or network connectivity checks.
    ///
    /// # Dry-Run Behavior
    ///
    /// When `is_dry_run()` is `true` AND this returns `true`:
    /// - The script is NOT executed
    /// - A log message shows the intended command
    /// - `run_script_safe` returns success with empty output
    ///
    /// # Default Implementation
    ///
    /// Defaults to `true` (conservative - assumes destructive).
    /// Override to return `false` for read-only scripts.
    fn is_destructive(&self) -> bool {
        true // Conservative default: assume destructive
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dry_run_flag() {
        // Ensure clean state
        disable_dry_run();
        assert!(!is_dry_run());

        enable_dry_run();
        assert!(is_dry_run());

        disable_dry_run();
        assert!(!is_dry_run());
    }
}
