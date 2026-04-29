//! Type-Safe Script Execution
//!
//! This module provides the ONLY sanctioned way to execute tool scripts.
//! All script execution MUST go through `run_script_safe` to ensure:
//!
//! - Process group isolation (death pact compliance)
//! - Proper PID registration for cleanup
//! - Type-safe argument passing via `ScriptArgs` trait
//! - Dry-run mode support
//!
//! # Architecture Rule
//!
//! `run_script_safe` is the execution gatekeeper. Any attempt to use
//! `Command::new("bash")` directly for tool scripts violates the architecture.
//!
//! # Dry-Run Mode
//!
//! When `is_dry_run()` returns `true` AND the script is destructive:
//! - The script is NOT executed
//! - A log message shows what WOULD have been executed
//! - Returns success with empty output
//!
//! Non-destructive scripts (like `lsblk`) still execute so the dry-run
//! produces realistic output for validation.

use crate::process_guard::{ChildRegistry, CommandProcessGroup};
use crate::script_traits::{ScriptArgs, is_dry_run};
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tracing::{info, warn};

/// Resolve the log directory.
/// Priority: ARCHTUI_LOG_DIR env > /var/log/archtui (if writable) > $XDG_STATE_HOME/archtui > ~/.local/state/archtui
pub fn log_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("ARCHTUI_LOG_DIR") {
        return PathBuf::from(dir);
    }
    // Prefer /var/log/archtui during installation (running as root)
    let var_log = PathBuf::from("/var/log/archtui");
    if std::fs::create_dir_all(&var_log).is_ok() {
        // Verify we can actually write there
        let test_path = var_log.join(".write_test");
        if std::fs::write(&test_path, b"").is_ok() {
            let _ = std::fs::remove_file(&test_path);
            return var_log;
        }
    }
    // Non-root fallback: XDG state directory
    if let Ok(xdg) = std::env::var("XDG_STATE_HOME") {
        return PathBuf::from(xdg).join("archtui");
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".local/state/archtui");
    }
    // Last resort
    PathBuf::from("/tmp/archtui-logs")
}

/// Resolve the scripts base directory.
/// Priority: ARCHTUI_SCRIPTS_DIR env > exe-adjacent scripts/ > FHS /usr/share/archtui/scripts > cwd-relative scripts/
pub fn scripts_base_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("ARCHTUI_SCRIPTS_DIR") {
        return PathBuf::from(dir);
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        let candidate = parent.join("scripts");
        if candidate.is_dir() {
            return candidate;
        }
    }
    // FHS fallback for system-wide installation (e.g. /usr/bin/archtui + /usr/share/archtui/scripts)
    let fhs = PathBuf::from("/usr/share/archtui/scripts");
    if fhs.is_dir() {
        return fhs;
    }
    PathBuf::from("./scripts")
}

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
    let script_path = scripts_base_dir().join("tools").join(script_name);
    let script_path = script_path.to_string_lossy().to_string();

    // Validate script exists before attempting execution
    if !Path::new(&script_path).exists() {
        warn!("Script not found: {}", script_path);
        bail!(
            "Script not found: {}. Ensure scripts/tools/ directory is accessible.",
            script_name
        );
    }

    let cli_args = args.to_cli_args();
    let env_vars = args.get_env_vars();

    args.validate()
        .map_err(|e| anyhow::anyhow!("Argument validation failed: {}", e))?;
    tracing::debug!(script = %script_name, "Argument validation passed");

    // Log exact command and environment for transparency
    // redact password-containing env vars
    let redacted_env = redact_env_vars(&env_vars);
    info!(
        script = %script_name,
        path = %script_path,
        args = ?cli_args,
        env = ?redacted_env,
        destructive = args.is_destructive(),
        "Spawning script"
    );

    // ========================================================================
    // DRY-RUN CHECK
    // Skip destructive operations when dry-run is enabled
    // ========================================================================
    if is_dry_run() && args.is_destructive() {
        // Format the command that WOULD have been executed (passwords redacted)
        let env_display = redact_env_vars(&env_vars);
        let env_prefix = if env_display.is_empty() {
            String::new()
        } else {
            format!("{} ", env_display.join(" "))
        };

        let would_execute = format!("{}bash {} {}", env_prefix, script_path, cli_args.join(" "));

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
        // Recover from poison - the registry state is still usable
        let mut guard = registry
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.register(pid);
    }

    // Wait for completion
    let output = child
        .wait_with_output()
        .with_context(|| format!("Failed waiting for script: {}", script_name))?;

    // Unregister PID after completion
    {
        let registry = ChildRegistry::global();
        // Recover from poison - the registry state is still usable
        let mut guard = registry
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        guard.unregister(pid);
    }

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code();

    if output.status.success() {
        info!(script = %script_name, exit_code = ?exit_code, "Script completed successfully");
        Ok(ScriptOutput {
            stdout,
            stderr,
            exit_code,
            success: true,
            dry_run: false,
        })
    } else {
        let code = exit_code.unwrap_or(-1);
        warn!(script = %script_name, exit_code = code, "Script failed");
        Ok(ScriptOutput {
            stdout,
            stderr,
            exit_code,
            success: false,
            dry_run: false,
        })
    }
}

/// Names that are unconditionally treated as secret. New secret-bearing
/// env vars should be added here (or use a known suffix below) so they
/// cannot leak through `redact_env_vars` by accident.
const KNOWN_SECRET_ENV: &[&str] = &[
    "MAIN_USER_PASSWORD",
    "ROOT_PASSWORD",
    "ENCRYPTION_PASSWORD",
    "USER_PASSWORD",
];

/// Returns true if an env var name should be treated as sensitive.
///
/// Combines an explicit allowlist (`KNOWN_SECRET_ENV`) with conservative
/// suffix patterns so future names like `LUKS_KEY_PASSPHRASE`,
/// `GITHUB_TOKEN`, or `SSH_PRIVATE_KEY` are redacted by default rather
/// than silently leaking through a substring match like `contains("PASSWORD")`.
pub fn is_secret_env(name: &str) -> bool {
    let upper = name.to_uppercase();
    if KNOWN_SECRET_ENV.contains(&upper.as_str()) {
        return true;
    }
    upper.ends_with("_PASSWORD")
        || upper.ends_with("_PASSPHRASE")
        || upper.ends_with("_SECRET")
        || upper.ends_with("_TOKEN")
        || upper.ends_with("_PRIVATE_KEY")
        || upper.ends_with("_KEYFILE")
}

/// Redact password-bearing environment variables for safe logging.
///
/// Secret values must NEVER appear in logs or tracing output.
pub fn redact_env_vars(env_vars: &[(String, String)]) -> Vec<String> {
    env_vars
        .iter()
        .map(|(k, v)| {
            if is_secret_env(k) {
                format!("{}=<REDACTED>", k)
            } else {
                format!("{}={}", k, v)
            }
        })
        .collect()
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
    #[allow(dead_code)] // API: Checked by callers to distinguish real vs simulated output
    pub dry_run: bool,
}

impl ScriptOutput {
    /// Check if the script succeeded and return an error if not.
    #[allow(dead_code)] // API: Used by installer::prepare_disks and script_execution_tests
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_env_vars_redacts_passwords() {
        let env = vec![
            ("INSTALL_DISK".to_string(), "/dev/sda".to_string()),
            ("MAIN_USER_PASSWORD".to_string(), "s3cret!".to_string()),
            ("ROOT_PASSWORD".to_string(), "r00t!".to_string()),
            ("ENCRYPTION_PASSWORD".to_string(), "luk$".to_string()),
            ("HOSTNAME".to_string(), "archbox".to_string()),
        ];

        let redacted = redact_env_vars(&env);

        assert_eq!(redacted[0], "INSTALL_DISK=/dev/sda");
        assert_eq!(redacted[1], "MAIN_USER_PASSWORD=<REDACTED>");
        assert_eq!(redacted[2], "ROOT_PASSWORD=<REDACTED>");
        assert_eq!(redacted[3], "ENCRYPTION_PASSWORD=<REDACTED>");
        assert_eq!(redacted[4], "HOSTNAME=archbox");

        // Verify no actual password values leak
        for entry in &redacted {
            assert!(
                !entry.contains("s3cret"),
                "Password value leaked: {}",
                entry
            );
            assert!(!entry.contains("r00t"), "Password value leaked: {}", entry);
            assert!(!entry.contains("luk$"), "Password value leaked: {}", entry);
        }
    }

    #[test]
    fn test_redact_env_vars_case_insensitive() {
        // Lowercase forms of known secret names still redact (allowlist is uppercased).
        let env = vec![("user_password".to_string(), "pw1".to_string())];
        let redacted = redact_env_vars(&env);
        assert_eq!(redacted[0], "user_password=<REDACTED>");
    }

    #[test]
    fn test_redact_env_vars_empty() {
        let env: Vec<(String, String)> = vec![];
        let redacted = redact_env_vars(&env);
        assert!(redacted.is_empty());
    }

    #[test]
    fn is_secret_env_classifies_known_and_future_names() {
        // Currently-known secrets (allowlist).
        assert!(is_secret_env("MAIN_USER_PASSWORD"));
        assert!(is_secret_env("ROOT_PASSWORD"));
        assert!(is_secret_env("ENCRYPTION_PASSWORD"));
        assert!(is_secret_env("USER_PASSWORD"));
        // Future secret-shaped names caught by suffix pattern.
        assert!(is_secret_env("LUKS_KEY_PASSPHRASE"));
        assert!(is_secret_env("GITHUB_TOKEN"));
        assert!(is_secret_env("API_SECRET"));
        assert!(is_secret_env("SSH_PRIVATE_KEY"));
        assert!(is_secret_env("LUKS_KEYFILE"));
        // Common false-positive shapes — must NOT be redacted.
        assert!(!is_secret_env("KEYMAP"));
        assert!(!is_secret_env("ENCRYPTION_KEY_TYPE"));
        assert!(!is_secret_env("KEYBOARD_LAYOUT"));
        assert!(!is_secret_env("MAIN_USERNAME"));
        assert!(!is_secret_env("INSTALL_DISK"));
    }

    /// Regression test against future drift: every secret produced by the
    /// real `Configuration::to_env_vars()` pipeline must be redacted, and
    /// no plaintext password value can survive the redactor.
    #[test]
    fn redact_covers_every_secret_in_full_configuration() {
        let mut config = crate::config::Configuration::default();
        for opt in &mut config.options {
            match opt.name.as_str() {
                "User Password" => opt.value = "userpw_canary".into(),
                "Root Password" => opt.value = "rootpw_canary".into(),
                "Encryption Password" => opt.value = "luks_canary".into(),
                _ => {}
            }
        }
        let env: Vec<(String, String)> = config.to_env_vars().into_iter().collect();
        let redacted = redact_env_vars(&env).join("\n");

        assert!(redacted.contains("MAIN_USER_PASSWORD=<REDACTED>"));
        assert!(redacted.contains("ROOT_PASSWORD=<REDACTED>"));
        assert!(redacted.contains("ENCRYPTION_PASSWORD=<REDACTED>"));
        assert!(!redacted.contains("userpw_canary"));
        assert!(!redacted.contains("rootpw_canary"));
        assert!(!redacted.contains("luks_canary"));
    }

    #[test]
    fn test_scripts_base_dir_returns_path() {
        let dir = scripts_base_dir();
        // Should return a path (may not exist in test env but should not panic)
        assert!(!dir.as_os_str().is_empty());
    }

    #[test]
    fn test_log_dir_returns_path() {
        let dir = log_dir();
        assert!(!dir.as_os_str().is_empty());
    }

    #[test]
    fn test_log_dir_respects_env_var() {
        // SAFETY: test is single-threaded, no other threads reading env vars
        unsafe { std::env::set_var("ARCHTUI_LOG_DIR", "/tmp/archtui-test-logs") };
        let dir = log_dir();
        assert_eq!(dir, PathBuf::from("/tmp/archtui-test-logs"));
        unsafe { std::env::remove_var("ARCHTUI_LOG_DIR") };
    }
}
