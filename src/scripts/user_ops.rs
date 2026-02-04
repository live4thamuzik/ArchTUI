//! Type-safe arguments for user-space operations (Sprint 18).
//!
//! Operations that CANNOT run as root:
//! - AUR helper installation (`makepkg` forbids root)
//! - Dotfiles cloning (files must be owned by the target user)
//!
//! All operations in this module use privilege dropping (`sudo -u <user>`)
//! to run commands as the target user inside a chroot environment.

#![allow(dead_code)]

use std::path::PathBuf;

use crate::script_traits::ScriptArgs;
use crate::types::AurHelper;

// ============================================================================
// Run As User
// ============================================================================

/// Type-safe arguments for running a command as a non-root user.
///
/// Maps to `scripts/tools/run_as_user.sh`.
///
/// # Implementation
///
/// The bash script uses `arch-chroot <root> sudo -u <user> <command>`
/// to drop privileges inside the installed system.
///
/// # Field to Flag Mapping
///
/// | Rust Field    | CLI Flag   | Notes |
/// |---------------|------------|-------|
/// | `user`        | `--user`   | Target unprivileged user |
/// | `command`     | `--cmd`    | Command to execute as that user |
/// | `chroot_path` | `--root`   | Path to chroot root (e.g., /mnt) |
/// | `workdir`     | `--workdir`| Optional working directory inside chroot |
#[derive(Debug, Clone)]
pub struct UserRunArgs {
    /// Username to run the command as.
    pub user: String,
    /// Command to execute (passed to `sudo -u <user> bash -c "<command>"`).
    pub command: String,
    /// Chroot root path (e.g., `/mnt`).
    pub chroot_path: PathBuf,
    /// Optional working directory inside the chroot.
    pub workdir: Option<PathBuf>,
}

impl ScriptArgs for UserRunArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec![
            "--user".to_string(),
            self.user.clone(),
            "--cmd".to_string(),
            self.command.clone(),
            "--root".to_string(),
            self.chroot_path.display().to_string(),
        ];

        if let Some(ref dir) = self.workdir {
            args.push("--workdir".to_string());
            args.push(dir.display().to_string());
        }

        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "run_as_user.sh"
    }

    /// User commands may modify the system, treat as destructive.
    fn is_destructive(&self) -> bool {
        true
    }
}

// ============================================================================
// Install AUR Helper
// ============================================================================

/// Type-safe arguments for AUR helper installation.
///
/// Maps to `scripts/tools/install_aur_helper.sh`.
///
/// # Flow
///
/// 1. Clone the AUR helper repo to `/home/<user>/<helper>`
/// 2. Run `makepkg -si --noconfirm` as the target user
/// 3. Clean up the build directory
///
/// # Constraints
///
/// - `makepkg` forbids running as root — uses `sudo -u <user>`
/// - Runs inside `arch-chroot` to use the installed system's toolchain
/// - Failure is NON-FATAL (see Sprint 18 failure policy)
///
/// # Field to Flag Mapping
///
/// | Rust Field    | CLI Flag    | Notes |
/// |---------------|-------------|-------|
/// | `helper`      | `--helper`  | AUR helper name (paru, yay) |
/// | `target_user` | `--user`    | User to build as |
/// | `chroot_path` | `--root`    | Chroot root path |
#[derive(Debug, Clone)]
pub struct InstallAurHelperArgs {
    /// AUR helper to install.
    pub helper: AurHelper,
    /// User to run `makepkg` as (must exist, must NOT be root).
    pub target_user: String,
    /// Chroot root path (e.g., `/mnt`).
    pub chroot_path: PathBuf,
}

impl ScriptArgs for InstallAurHelperArgs {
    fn to_cli_args(&self) -> Vec<String> {
        vec![
            "--helper".to_string(),
            self.helper.to_string(),
            "--user".to_string(),
            self.target_user.clone(),
            "--root".to_string(),
            self.chroot_path.display().to_string(),
        ]
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "install_aur_helper.sh"
    }

    /// AUR helper installation is DESTRUCTIVE — installs packages.
    fn is_destructive(&self) -> bool {
        true
    }
}

// ============================================================================
// Clone Dotfiles (User-Space)
// ============================================================================

/// Type-safe arguments for cloning dotfiles as a non-root user.
///
/// Maps to `scripts/tools/install_dotfiles.sh` (reuses existing script).
///
/// This is a thin wrapper that ensures the git clone runs as the target user,
/// not as root. The existing `install_dotfiles.sh` already handles ownership.
///
/// # Field to Flag Mapping
///
/// | Rust Field    | CLI Flag   | Notes |
/// |---------------|------------|-------|
/// | `repo_url`    | `--repo`   | Git repository URL |
/// | `target_user` | `--user`   | User for ownership |
/// | `target_dir`  | `--target` | Clone target directory |
/// | `branch`      | `--branch` | Optional branch |
#[derive(Debug, Clone)]
pub struct CloneDotfilesArgs {
    /// Git repository URL.
    pub repo_url: String,
    /// Target user (sets ownership, runs git as this user).
    pub target_user: String,
    /// Target directory (default: user's home).
    pub target_dir: Option<PathBuf>,
    /// Branch to clone.
    pub branch: Option<String>,
}

impl ScriptArgs for CloneDotfilesArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec![
            "--repo".to_string(),
            self.repo_url.clone(),
            "--user".to_string(),
            self.target_user.clone(),
            "--backup".to_string(), // Always backup existing files
        ];

        if let Some(ref dir) = self.target_dir {
            args.push("--target".to_string());
            args.push(dir.display().to_string());
        }

        if let Some(ref branch) = self.branch {
            args.push("--branch".to_string());
            args.push(branch.clone());
        }

        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "install_dotfiles.sh"
    }

    /// Dotfiles installation is DESTRUCTIVE — may overwrite configs.
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
    fn test_user_run_args_basic() {
        let args = UserRunArgs {
            user: "archuser".to_string(),
            command: "whoami".to_string(),
            chroot_path: PathBuf::from("/mnt"),
            workdir: None,
        };

        let cli = args.to_cli_args();
        assert!(cli.contains(&"--user".to_string()));
        assert!(cli.contains(&"archuser".to_string()));
        assert!(cli.contains(&"--cmd".to_string()));
        assert!(cli.contains(&"whoami".to_string()));
        assert!(cli.contains(&"--root".to_string()));
        assert!(cli.contains(&"/mnt".to_string()));
        assert!(!cli.contains(&"--workdir".to_string()));

        assert!(args.is_destructive());
        assert_eq!(args.script_name(), "run_as_user.sh");
    }

    #[test]
    fn test_user_run_args_with_workdir() {
        let args = UserRunArgs {
            user: "archuser".to_string(),
            command: "ls -la".to_string(),
            chroot_path: PathBuf::from("/mnt"),
            workdir: Some(PathBuf::from("/home/archuser")),
        };

        let cli = args.to_cli_args();
        assert!(cli.contains(&"--workdir".to_string()));
        assert!(cli.contains(&"/home/archuser".to_string()));
    }

    #[test]
    fn test_install_aur_helper_paru() {
        let args = InstallAurHelperArgs {
            helper: AurHelper::Paru,
            target_user: "archuser".to_string(),
            chroot_path: PathBuf::from("/mnt"),
        };

        let cli = args.to_cli_args();
        assert!(cli.contains(&"--helper".to_string()));
        assert!(cli.contains(&"paru".to_string()));
        assert!(cli.contains(&"--user".to_string()));
        assert!(cli.contains(&"archuser".to_string()));
        assert!(cli.contains(&"--root".to_string()));

        assert!(args.is_destructive());
        assert_eq!(args.script_name(), "install_aur_helper.sh");
    }

    #[test]
    fn test_install_aur_helper_yay() {
        let args = InstallAurHelperArgs {
            helper: AurHelper::Yay,
            target_user: "archuser".to_string(),
            chroot_path: PathBuf::from("/mnt"),
        };

        let cli = args.to_cli_args();
        assert!(cli.contains(&"yay".to_string()));
    }

    #[test]
    fn test_clone_dotfiles_args() {
        let args = CloneDotfilesArgs {
            repo_url: "https://github.com/user/dotfiles".to_string(),
            target_user: "archuser".to_string(),
            target_dir: Some(PathBuf::from("/home/archuser/.dotfiles")),
            branch: Some("main".to_string()),
        };

        let cli = args.to_cli_args();
        assert!(cli.contains(&"--repo".to_string()));
        assert!(cli.contains(&"https://github.com/user/dotfiles".to_string()));
        assert!(cli.contains(&"--user".to_string()));
        assert!(cli.contains(&"archuser".to_string()));
        assert!(cli.contains(&"--backup".to_string()));
        assert!(cli.contains(&"--target".to_string()));
        assert!(cli.contains(&"/home/archuser/.dotfiles".to_string()));
        assert!(cli.contains(&"--branch".to_string()));
        assert!(cli.contains(&"main".to_string()));

        assert!(args.is_destructive());
        assert_eq!(args.script_name(), "install_dotfiles.sh");
    }

    #[test]
    fn test_clone_dotfiles_minimal() {
        let args = CloneDotfilesArgs {
            repo_url: "https://github.com/user/dots".to_string(),
            target_user: "archuser".to_string(),
            target_dir: None,
            branch: None,
        };

        let cli = args.to_cli_args();
        assert_eq!(cli.len(), 5); // --repo, url, --user, username, --backup
        assert!(!cli.contains(&"--target".to_string()));
        assert!(!cli.contains(&"--branch".to_string()));
    }

    #[test]
    fn test_user_run_env_vars_empty() {
        let args = UserRunArgs {
            user: "test".to_string(),
            command: "echo hi".to_string(),
            chroot_path: PathBuf::from("/mnt"),
            workdir: None,
        };
        assert!(args.get_env_vars().is_empty());
    }
}
