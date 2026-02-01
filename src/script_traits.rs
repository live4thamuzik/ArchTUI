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
///
/// # Invariants
///
/// - The returned CLI args MUST match the bash script's argument parser.
/// - Environment variables MUST match the script's environment contract.
/// - Scripts are identified by name only (path is resolved at execution time).
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
}
