//! Post-install orchestration (Sprint 18)
//!
//! Handles operations that run AFTER the base system is installed:
//! - AUR helper installation (paru/yay)
//! - Dotfiles cloning
//!
//! # Failure Policy
//!
//! All operations in this module are **non-fatal**. A broken AUR helper
//! or missing dotfiles should never brick the installation. Failures are
//! logged as warnings and the installation continues.
//!
//! # Privilege Dropping
//!
//! AUR builds and dotfiles clones run as the target user, not as root.
//! `makepkg` explicitly forbids running as root. This module coordinates
//! the privilege drop via `UserRunArgs` / `InstallAurHelperArgs`.

// Library API — consumed by installer orchestration
#![allow(dead_code)]

use crate::script_runner::run_script_safe;
use crate::scripts::user_ops::{CloneDotfilesArgs, InstallAurHelperArgs};
use crate::types::AurHelper;

use std::fmt;
use std::path::PathBuf;

// ============================================================================
// Post-install Result
// ============================================================================

/// Outcome of post-install operations.
#[derive(Debug, Clone)]
pub enum PostInstallResult {
    /// All post-install operations succeeded.
    Success,
    /// Some operations succeeded, some failed (with warnings).
    PartialSuccess(Vec<String>),
    /// All operations were skipped (nothing to do).
    Skipped,
}

impl fmt::Display for PostInstallResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Success => write!(f, "All post-install operations succeeded"),
            Self::PartialSuccess(warnings) => {
                write!(f, "Post-install completed with {} warning(s)", warnings.len())
            }
            Self::Skipped => write!(f, "No post-install operations to run"),
        }
    }
}

// ============================================================================
// Post-install Configuration
// ============================================================================

/// Configuration for post-install operations.
#[derive(Debug, Clone)]
pub struct PostInstallConfig {
    /// AUR helper to install (None = skip).
    pub aur_helper: AurHelper,
    /// Target user for AUR helper and dotfiles.
    pub target_user: String,
    /// Chroot root path (e.g., `/mnt`).
    pub chroot_path: PathBuf,
    /// Optional dotfiles repository URL.
    pub dotfiles_repo: Option<String>,
    /// Optional dotfiles branch.
    pub dotfiles_branch: Option<String>,
}

impl Default for PostInstallConfig {
    fn default() -> Self {
        Self {
            aur_helper: AurHelper::None,
            target_user: "user".to_string(),
            chroot_path: PathBuf::from("/mnt"),
            dotfiles_repo: None,
            dotfiles_branch: None,
        }
    }
}

// ============================================================================
// AUR Helper Installation
// ============================================================================

/// Install an AUR helper (paru or yay) as the target user.
///
/// # Failure Policy
///
/// **NON-FATAL**: If installation fails, logs a warning and returns
/// the error message. The caller should NOT abort the installation.
///
/// # Why Non-Fatal?
///
/// - The base system is fully functional without an AUR helper
/// - The user can install one manually post-reboot
/// - Common failure modes (no network, missing git) are recoverable
///
/// # Returns
///
/// - `Ok(())` — AUR helper installed successfully
/// - `Err(String)` — Installation failed (message for logging)
pub fn install_aur_helper_safe(
    helper: AurHelper,
    user: &str,
    chroot_path: &PathBuf,
) -> Result<(), String> {
    if helper == AurHelper::None {
        log::info!("No AUR helper selected — skipping");
        return Ok(());
    }

    log::info!("Installing AUR helper: {} for user {}", helper, user);

    let args = InstallAurHelperArgs {
        helper,
        target_user: user.to_string(),
        chroot_path: chroot_path.clone(),
    };

    match run_script_safe(&args) {
        Ok(output) => {
            if output.success {
                log::info!("AUR helper {} installed successfully", helper);
                Ok(())
            } else {
                let msg = format!(
                    "AUR helper {} installation failed (exit code {}) — \
                     user can install manually after reboot",
                    helper,
                    output.exit_code.unwrap_or(-1)
                );
                log::warn!("{}", msg);
                if !output.stderr.is_empty() {
                    log::warn!("AUR stderr: {}", output.stderr.trim());
                }
                Err(msg)
            }
        }
        Err(e) => {
            let msg = format!(
                "AUR helper {} failed to execute: {} — \
                 user can install manually after reboot",
                helper, e
            );
            log::warn!("{}", msg);
            Err(msg)
        }
    }
}

// ============================================================================
// Dotfiles Cloning
// ============================================================================

/// Clone dotfiles repository as the target user.
///
/// # Failure Policy
///
/// **NON-FATAL**: Dotfiles are cosmetic. If cloning fails, the system
/// is still fully functional. Logs a warning and continues.
///
/// # Returns
///
/// - `Ok(())` — Dotfiles cloned successfully
/// - `Err(String)` — Cloning failed (message for logging)
pub fn clone_dotfiles_safe(
    repo_url: &str,
    user: &str,
    branch: Option<&str>,
) -> Result<(), String> {
    log::info!("Cloning dotfiles from {} for user {}", repo_url, user);

    let args = CloneDotfilesArgs {
        repo_url: repo_url.to_string(),
        target_user: user.to_string(),
        target_dir: None, // Default: user's home directory
        branch: branch.map(String::from),
    };

    match run_script_safe(&args) {
        Ok(output) => {
            if output.success {
                log::info!("Dotfiles cloned successfully for user {}", user);
                Ok(())
            } else {
                let msg = format!(
                    "Dotfiles clone failed (exit code {}) — \
                     user can clone manually after reboot",
                    output.exit_code.unwrap_or(-1)
                );
                log::warn!("{}", msg);
                if !output.stderr.is_empty() {
                    log::warn!("Dotfiles stderr: {}", output.stderr.trim());
                }
                Err(msg)
            }
        }
        Err(e) => {
            let msg = format!(
                "Dotfiles clone failed to execute: {} — \
                 user can clone manually after reboot",
                e
            );
            log::warn!("{}", msg);
            Err(msg)
        }
    }
}

// ============================================================================
// Orchestrator
// ============================================================================

/// Run all post-install operations with fail-safe behavior.
///
/// Runs AUR helper installation and dotfiles cloning. Both are non-fatal:
/// failures produce warnings but the installation continues.
///
/// # Returns
///
/// - `PostInstallResult::Success` — Everything succeeded
/// - `PostInstallResult::PartialSuccess(warnings)` — Some ops failed
/// - `PostInstallResult::Skipped` — Nothing to do
pub fn run_postinstall(config: &PostInstallConfig) -> PostInstallResult {
    let mut warnings: Vec<String> = Vec::new();
    let mut did_something = false;

    // 1. AUR helper
    if config.aur_helper != AurHelper::None {
        did_something = true;
        if let Err(msg) = install_aur_helper_safe(
            config.aur_helper,
            &config.target_user,
            &config.chroot_path,
        ) {
            warnings.push(msg);
        }
    }

    // 2. Dotfiles
    if let Some(ref repo) = config.dotfiles_repo {
        did_something = true;
        if let Err(msg) = clone_dotfiles_safe(
            repo,
            &config.target_user,
            config.dotfiles_branch.as_deref(),
        ) {
            warnings.push(msg);
        }
    }

    if !did_something {
        return PostInstallResult::Skipped;
    }

    if warnings.is_empty() {
        PostInstallResult::Success
    } else {
        PostInstallResult::PartialSuccess(warnings)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postinstall_config_defaults() {
        let config = PostInstallConfig::default();
        assert_eq!(config.aur_helper, AurHelper::None);
        assert_eq!(config.target_user, "user");
        assert_eq!(config.chroot_path, PathBuf::from("/mnt"));
        assert!(config.dotfiles_repo.is_none());
        assert!(config.dotfiles_branch.is_none());
    }

    #[test]
    fn test_install_aur_helper_none_skips() {
        let result = install_aur_helper_safe(
            AurHelper::None,
            "user",
            &PathBuf::from("/mnt"),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_postinstall_skipped() {
        let config = PostInstallConfig {
            aur_helper: AurHelper::None,
            target_user: "user".to_string(),
            chroot_path: PathBuf::from("/mnt"),
            dotfiles_repo: None,
            dotfiles_branch: None,
        };
        let result = run_postinstall(&config);
        assert!(matches!(result, PostInstallResult::Skipped));
    }

    #[test]
    fn test_postinstall_result_display() {
        let success = PostInstallResult::Success;
        assert_eq!(
            success.to_string(),
            "All post-install operations succeeded"
        );

        let partial = PostInstallResult::PartialSuccess(vec!["warn1".to_string()]);
        assert_eq!(
            partial.to_string(),
            "Post-install completed with 1 warning(s)"
        );

        let skipped = PostInstallResult::Skipped;
        assert_eq!(skipped.to_string(), "No post-install operations to run");
    }
}
