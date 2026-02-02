//! Type-Safe Script Execution
//!
//! This module provides the ONLY sanctioned way to execute tool scripts.
//! All script execution MUST go through `run_script_safe` to ensure:
//!
//! - Process group isolation (death pact compliance)
//! - Proper PID registration for cleanup
//! - Type-safe argument passing via `ScriptArgs` trait
//!
//! # Architecture Rule
//!
//! `run_script_safe` is the execution gatekeeper. Any attempt to use
//! `Command::new("bash")` directly for tool scripts violates the architecture.

use crate::process_guard::{ChildRegistry, CommandProcessGroup};
use crate::script_traits::ScriptArgs;
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
/// use archinstall_tui::scripts::disk::{WipeDiskArgs, WipeMethod};
/// use archinstall_tui::script_runner::run_script_safe;
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
        })
    } else {
        let code = exit_code.unwrap_or(-1);
        info!("Script {} failed with exit code {}", script_name, code);
        Ok(ScriptOutput {
            stdout,
            stderr,
            exit_code,
            success: false,
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
