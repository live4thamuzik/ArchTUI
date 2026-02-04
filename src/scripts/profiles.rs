//! Type-safe arguments for profile/dotfiles scripts (Sprint 12).
//!
//! This module provides typed argument structs for profile-related scripts:
//! - `InstallDotfilesArgs` for `install_dotfiles.sh`
//! - `EnableServicesArgs` for enabling systemd services
//!

#![allow(dead_code)]
//! # Dotfiles Installation
//!
//! Dotfiles are cloned from a Git repository. The script ensures:
//! - Git is installed before attempting clone
//! - Proper ownership of cloned files
//! - Safe handling of existing files (backup vs overwrite)

use std::path::PathBuf;

use crate::script_traits::ScriptArgs;

// ============================================================================
// Install Dotfiles
// ============================================================================

/// Type-safe arguments for `scripts/tools/install_dotfiles.sh`.
///
/// Clones a dotfiles repository for a user.
///
/// # Field to Flag Mapping
///
/// | Rust Field    | CLI Flag      | Notes |
/// |---------------|---------------|-------|
/// | `repo_url`    | `--repo`      | Git repository URL |
/// | `target_user` | `--user`      | Username for ownership |
/// | `target_dir`  | `--target`    | Optional target directory |
/// | `branch`      | `--branch`    | Optional branch name |
/// | `backup`      | `--backup`    | Backup existing files |
///
/// # Prerequisites
///
/// The script will check that `git` is installed before cloning.
#[derive(Debug, Clone)]
pub struct InstallDotfilesArgs {
    /// Git repository URL (https:// or git://).
    pub repo_url: String,
    /// Target user for dotfiles (sets ownership).
    pub target_user: String,
    /// Target directory (default: user's home directory).
    pub target_dir: Option<PathBuf>,
    /// Branch to clone (default: main or master).
    pub branch: Option<String>,
    /// Whether to backup existing files before overwriting.
    pub backup: bool,
}

impl ScriptArgs for InstallDotfilesArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec![
            "--repo".to_string(),
            self.repo_url.clone(),
            "--user".to_string(),
            self.target_user.clone(),
        ];

        if let Some(ref dir) = self.target_dir {
            args.push("--target".to_string());
            args.push(dir.display().to_string());
        }

        if let Some(ref branch) = self.branch {
            args.push("--branch".to_string());
            args.push(branch.clone());
        }

        if self.backup {
            args.push("--backup".to_string());
        }

        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "install_dotfiles.sh"
    }

    /// Dotfiles installation is DESTRUCTIVE - may overwrite existing configs.
    fn is_destructive(&self) -> bool {
        true
    }
}

// ============================================================================
// Enable Services
// ============================================================================

/// Type-safe arguments for enabling systemd services.
///
/// Used to enable display managers and other services in the chroot.
///
/// # Field to Flag Mapping
///
/// | Rust Field  | CLI Flag     | Notes |
/// |-------------|--------------|-------|
/// | `services`  | `--services` | Comma-separated service names |
/// | `root`      | `--root`     | Target root for arch-chroot |
#[derive(Debug, Clone)]
pub struct EnableServicesArgs {
    /// Services to enable (e.g., ["sddm", "NetworkManager"]).
    pub services: Vec<String>,
    /// Target root (for chroot execution).
    pub root: PathBuf,
}

impl ScriptArgs for EnableServicesArgs {
    fn to_cli_args(&self) -> Vec<String> {
        vec![
            "--root".to_string(),
            self.root.display().to_string(),
            "--services".to_string(),
            self.services.join(","),
        ]
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "enable_services.sh"
    }

    /// Enabling services is DESTRUCTIVE - modifies system configuration.
    fn is_destructive(&self) -> bool {
        true
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_dotfiles_args() {
        let args = InstallDotfilesArgs {
            repo_url: "https://github.com/user/dotfiles".to_string(),
            target_user: "archuser".to_string(),
            target_dir: Some(PathBuf::from("/home/archuser")),
            branch: Some("main".to_string()),
            backup: true,
        };

        let cli_args = args.to_cli_args();
        assert!(cli_args.contains(&"--repo".to_string()));
        assert!(cli_args.contains(&"https://github.com/user/dotfiles".to_string()));
        assert!(cli_args.contains(&"--user".to_string()));
        assert!(cli_args.contains(&"archuser".to_string()));
        assert!(cli_args.contains(&"--target".to_string()));
        assert!(cli_args.contains(&"/home/archuser".to_string()));
        assert!(cli_args.contains(&"--branch".to_string()));
        assert!(cli_args.contains(&"main".to_string()));
        assert!(cli_args.contains(&"--backup".to_string()));

        assert!(args.is_destructive());
        assert_eq!(args.script_name(), "install_dotfiles.sh");
    }

    #[test]
    fn test_install_dotfiles_minimal() {
        let args = InstallDotfilesArgs {
            repo_url: "https://github.com/user/dotfiles".to_string(),
            target_user: "archuser".to_string(),
            target_dir: None,
            branch: None,
            backup: false,
        };

        let cli_args = args.to_cli_args();
        assert_eq!(cli_args.len(), 4); // Only --repo, url, --user, username
        assert!(!cli_args.contains(&"--backup".to_string()));
        assert!(!cli_args.contains(&"--branch".to_string()));
        assert!(!cli_args.contains(&"--target".to_string()));
    }

    #[test]
    fn test_enable_services_args() {
        let args = EnableServicesArgs {
            services: vec!["sddm".to_string(), "NetworkManager".to_string()],
            root: PathBuf::from("/mnt"),
        };

        let cli_args = args.to_cli_args();
        assert!(cli_args.contains(&"--root".to_string()));
        assert!(cli_args.contains(&"/mnt".to_string()));
        assert!(cli_args.contains(&"--services".to_string()));
        assert!(cli_args.contains(&"sddm,NetworkManager".to_string()));

        assert!(args.is_destructive());
        assert_eq!(args.script_name(), "enable_services.sh");
    }
}
