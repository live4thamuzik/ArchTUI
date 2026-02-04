//! Type-Safe Script Execution
//!
//! This module provides the ONLY sanctioned way to execute tool scripts.
//! All script execution MUST go through `run_script_safe` to ensure:
//!
//! - Process group isolation (death pact compliance)
//! - Proper PID registration for cleanup
//! - Type-safe argument passing via `ScriptArgs` trait
//! - Dry-run mode support (Sprint 8)
//!
//! # Architecture Rule
//!
//! `run_script_safe` is the execution gatekeeper. Any attempt to use
//! `Command::new("bash")` directly for tool scripts violates the architecture.
//!
//! # Dry-Run Mode (Sprint 8)
//!
//! When `is_dry_run()` returns `true` AND the script is destructive:
//! - The script is NOT executed
//! - A log message shows what WOULD have been executed
//! - Returns success with empty output
//!
//! Non-destructive scripts (like `lsblk`) still execute so the dry-run
//! produces realistic output for validation.

use crate::process_guard::{ChildRegistry, CommandProcessGroup};
use crate::script_traits::{is_dry_run, ScriptArgs};
use anyhow::{Context, Result};
use log::info;
use std::process::{Command, Stdio};

/// Execute a tool script with type-safe arguments.
///
/// This is the ONLY way to execute scripts in the `scripts/tools/` directory.
/// Using raw `Command::new` for these scripts is forbidden by lint rules.
///
/// # Death Pact Compliance
///
/// - Spawns the script in a new process group via `.in_new_process_group()`
/// - Registers the child PID with `ChildRegistry::global()`
/// - Ensures cleanup if the parent process exits
///
/// # Dry-Run Mode
///
/// If `is_dry_run()` is `true` AND `args.is_destructive()` is `true`:
/// - The script is NOT executed
/// - A log message shows the intended command: `[DRY RUN] Would execute: bash scripts/tools/wipe_disk.sh --disk /dev/sda`
/// - Returns `Ok(ScriptOutput)` with `dry_run: true`
///
/// Non-destructive scripts (e.g., `lsblk`, `system_info`) still execute
/// so disk lists and system checks work during dry-run.
///
/// # Arguments
///
/// * `args` - A struct implementing `ScriptArgs` that provides CLI args and env vars
///
/// # Returns
///
/// - `Ok(output)` - Script executed successfully with stdout/stderr captured
/// - `Err` - Script not found, execution failed, or non-zero exit
///
/// # Example
///
/// ```ignore
/// use archtui::scripts::disk::{WipeDiskArgs, WipeMethod};
/// use archtui::script_runner::run_script_safe;
///
/// let args = WipeDiskArgs {
///     device: PathBuf::from("/dev/sda"),
///     method: WipeMethod::Quick,
///     confirm: true,
/// };
///
/// run_script_safe(&args)?;
/// ```
pub fn run_script_safe<T: ScriptArgs>(args: &T) -> Result<ScriptOutput> {
    let script_name = args.script_name();
    let script_path = format!("scripts/tools/{}", script_name);
    let cli_args = args.to_cli_args();
    let env_vars = args.get_env_vars();

    // Log exact command and environment for transparency
    info!(
        "run_script_safe: {} args={:?} env={:?}",
        script_path, cli_args, env_vars
    );

    // ========================================================================
    // DRY-RUN CHECK (Sprint 8)
    // Skip destructive operations when dry-run is enabled
    // ========================================================================
    if is_dry_run() && args.is_destructive() {
        // Format the command that WOULD have been executed
        let env_display: Vec<String> = env_vars
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect();
        let env_prefix = if env_display.is_empty() {
            String::new()
        } else {
            format!("{} ", env_display.join(" "))
        };

        let would_execute = format!(
            "{}bash {} {}",
            env_prefix,
            script_path,
            cli_args.join(" ")
        );

        info!("[DRY RUN] Would execute: {}", would_execute);

        // Return success without executing
        return Ok(ScriptOutput {
            stdout: format!("[DRY RUN] Skipped: {}\n", script_name),
            stderr: String::new(),
            exit_code: Some(0),
            success: true,
            dry_run: true,
        });
    }

    // Build command with process group isolation
    let mut cmd = Command::new("bash");
    cmd.arg(&script_path)
        .args(&cli_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .in_new_process_group(); // CRITICAL: Enables death pact

    // Inject environment variables from typed args
    for (key, value) in &env_vars {
        cmd.env(key, value);
    }

    // Spawn and register with global registry
    let child = cmd
        .spawn()
        .with_context(|| format!("Failed to spawn script: {}", script_path))?;
    let pid = child.id();

    // Register PID for cleanup on parent exit
    {
        let registry = ChildRegistry::global();
        // Lock is held briefly, panic is acceptable if poisoned
        let mut guard = registry.lock().expect("ChildRegistry mutex poisoned");
        guard.register(pid);
    }

    // Wait for completion
    let output = child
        .wait_with_output()
        .with_context(|| format!("Failed waiting for script: {}", script_name))?;

    // Unregister PID after completion
    {
        let registry = ChildRegistry::global();
        let mut guard = registry.lock().expect("ChildRegistry mutex poisoned");
        guard.unregister(pid);
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code();

    if output.status.success() {
        info!("Script {} executed successfully", script_name);
        Ok(ScriptOutput {
            stdout,
            stderr,
            exit_code,
            success: true,
            dry_run: false,
        })
    } else {
        let code = exit_code.unwrap_or(-1);
        info!("Script {} failed with exit code {}", script_name, code);
        Ok(ScriptOutput {
            stdout,
            stderr,
            exit_code,
            success: false,
            dry_run: false,
        })
    }
}

/// Output from a script execution.
#[derive(Debug, Clone)]
pub struct ScriptOutput {
    /// Standard output from the script.
    pub stdout: String,
    /// Standard error from the script.
    pub stderr: String,
    /// Exit code (None if terminated by signal).
    pub exit_code: Option<i32>,
    /// Whether the script exited successfully (exit code 0).
    pub success: bool,
    /// Whether this was a dry-run (script not actually executed).
    /// Callers can check this to know if the output is real or simulated.
    #[allow(dead_code)]
    pub dry_run: bool,
}

impl ScriptOutput {
    /// Check if the script succeeded and return an error if not.
    #[allow(dead_code)] // Used by installer::prepare_disks
    pub fn ensure_success(&self, context: &str) -> Result<()> {
        if self.success {
            Ok(())
        } else {
            let code = self.exit_code.unwrap_or(-1);
            anyhow::bail!(
                "{} failed (exit code {}): {}",
                context,
                code,
                self.stderr.trim()
            )
        }
    }
}
