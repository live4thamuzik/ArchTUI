//! Type-safe arguments for system configuration scripts.
//!
//! This module provides typed argument structs for post-installation configuration:
//! - `GenFstabArgs` for `generate_fstab.sh`
//! - `UserAddArgs` for `add_user.sh` (password via env var, NOT CLI)
//! - `LocaleArgs` for hostname/locale configuration
//!
//! # Security: Password Handling
//!
//! **CRITICAL**: Passwords MUST be passed via environment variables, not CLI flags.
//! CLI arguments are visible in `/proc/<pid>/cmdline` to all users on the system.
//!
//! The `UserAddArgs` struct enforces this by:
//! - NOT including password in `to_cli_args()`
//! - Including password in `get_env_vars()` as `USER_PASSWORD`

use std::path::PathBuf;

use crate::script_traits::ScriptArgs;

// ============================================================================
// Generate Fstab
// ============================================================================

/// Type-safe arguments for `scripts/tools/generate_fstab.sh`.
///
/// Generates `/etc/fstab` for the target system based on current mounts.
/// Must be called AFTER partitions are mounted but BEFORE chroot/reboot.
///
/// # Mount Order Dependency
///
/// This script uses `genfstab -U` which reads currently mounted filesystems.
/// Ensure all partitions are mounted in the correct order before calling.
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use archtui::scripts::config::GenFstabArgs;
/// use archtui::script_traits::ScriptArgs;
///
/// let args = GenFstabArgs {
///     root: PathBuf::from("/mnt"),
/// };
///
/// assert_eq!(args.to_cli_args(), vec!["--root", "/mnt"]);
/// ```
#[derive(Debug, Clone)]
pub struct GenFstabArgs {
    /// Root mount path (e.g., `/mnt`) - where the target system is mounted.
    pub root: PathBuf,
}

impl ScriptArgs for GenFstabArgs {
    fn to_cli_args(&self) -> Vec<String> {
        vec!["--root".to_string(), self.root.display().to_string()]
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "generate_fstab.sh"
    }

    /// Fstab generation is DESTRUCTIVE - writes to /etc/fstab.
    fn is_destructive(&self) -> bool {
        true
    }
}

// ============================================================================
// Add User
// ============================================================================

/// Type-safe arguments for `scripts/tools/add_user.sh`.
///
/// # Security: Password via Environment
///
/// **CRITICAL**: The password is passed via the `USER_PASSWORD` environment variable,
/// NOT as a CLI flag. This prevents password exposure in `/proc/<pid>/cmdline`.
///
/// # Field to Flag/Env Mapping
///
/// | Rust Field | CLI Flag       | Env Var        | Notes |
/// |------------|----------------|----------------|-------|
/// | `username` | `--username`   | -              | Required |
/// | `password` | -              | `USER_PASSWORD`| Via env only |
/// | `groups`   | `--groups`     | -              | Comma-separated |
/// | `shell`    | `--shell`      | -              | Default: /bin/bash |
/// | `full_name`| `--full-name`  | -              | Optional |
/// | `home_dir` | `--home-dir`   | -              | Optional |
/// | `sudo`     | `--groups wheel` | -            | Added to groups |
///
/// # Example
///
/// ```
/// use archtui::scripts::config::UserAddArgs;
/// use archtui::script_traits::ScriptArgs;
///
/// let args = UserAddArgs {
///     username: "archuser".to_string(),
///     password: Some("secret123".to_string()),
///     groups: Some("wheel,audio,video".to_string()),
///     shell: Some("/bin/zsh".to_string()),
///     full_name: Some("Arch User".to_string()),
///     home_dir: None,
///     create_home: true,
///     sudo: true,
/// };
///
/// // Password NOT in CLI args
/// let cli = args.to_cli_args();
/// assert!(!cli.iter().any(|a| a.contains("secret")));
///
/// // Password in env vars
/// let env = args.get_env_vars();
/// assert!(env.iter().any(|(k, v)| k == "USER_PASSWORD" && v == "secret123"));
/// ```
#[derive(Debug, Clone)]
pub struct UserAddArgs {
    /// Username for the new user.
    pub username: String,
    /// Password for the user (passed via USER_PASSWORD env var, NOT CLI).
    pub password: Option<String>,
    /// Comma-separated list of groups (e.g., "wheel,audio,video").
    pub groups: Option<String>,
    /// Login shell (default: /bin/bash).
    pub shell: Option<String>,
    /// Full name / comment for the user.
    pub full_name: Option<String>,
    /// Custom home directory path.
    pub home_dir: Option<PathBuf>,
    /// Whether to create home directory (default: true).
    pub create_home: bool,
    /// Whether to add to wheel group for sudo access.
    pub sudo: bool,
}

impl ScriptArgs for UserAddArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec!["--username".to_string(), self.username.clone()];

        // Build groups string, adding wheel if sudo is requested
        let groups = if self.sudo {
            match &self.groups {
                Some(g) if !g.contains("wheel") => Some(format!("{},wheel", g)),
                Some(g) => Some(g.clone()),
                None => Some("wheel".to_string()),
            }
        } else {
            self.groups.clone()
        };

        if let Some(ref g) = groups {
            args.push("--groups".to_string());
            args.push(g.clone());
        }

        if let Some(ref shell) = self.shell {
            args.push("--shell".to_string());
            args.push(shell.clone());
        }

        if let Some(ref name) = self.full_name {
            args.push("--full-name".to_string());
            args.push(name.clone());
        }

        if let Some(ref home) = self.home_dir {
            args.push("--home-dir".to_string());
            args.push(home.display().to_string());
        }

        if !self.create_home {
            args.push("--no-create-home".to_string());
        }

        // CRITICAL: Password is NOT included here - see get_env_vars()
        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        // SECURITY: Password passed via environment, not CLI
        // CLI args are visible in /proc/<pid>/cmdline
        if let Some(ref pwd) = self.password {
            vec![("USER_PASSWORD".to_string(), pwd.clone())]
        } else {
            vec![]
        }
    }

    fn script_name(&self) -> &'static str {
        "add_user.sh"
    }

    /// User creation is DESTRUCTIVE - modifies /etc/passwd, /etc/shadow.
    fn is_destructive(&self) -> bool {
        true
    }
}

// ============================================================================
// Locale Configuration
// ============================================================================

/// Type-safe arguments for locale and hostname configuration.
///
/// This configures the basic system identity in chroot:
/// - Hostname in `/etc/hostname`
/// - Locale in `/etc/locale.gen` and `/etc/locale.conf`
/// - Timezone via symlink to `/etc/localtime`
///
/// # Note
///
/// This maps to operations typically done in `chroot_config.sh`.
/// The script will need to be created or the chroot script updated.
#[derive(Debug, Clone)]
pub struct LocaleArgs {
    /// Target root path where config files will be written.
    pub root: PathBuf,
    /// Hostname for the system (e.g., "archlinux").
    pub hostname: String,
    /// Locale setting (e.g., "en_US.UTF-8").
    pub locale: String,
    /// Timezone (e.g., "America/New_York").
    pub timezone: String,
    /// Keymap for console (e.g., "us").
    pub keymap: Option<String>,
}

impl ScriptArgs for LocaleArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec![
            "--root".to_string(),
            self.root.display().to_string(),
            "--hostname".to_string(),
            self.hostname.clone(),
            "--locale".to_string(),
            self.locale.clone(),
            "--timezone".to_string(),
            self.timezone.clone(),
        ];

        if let Some(ref keymap) = self.keymap {
            args.push("--keymap".to_string());
            args.push(keymap.clone());
        }

        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        // This would need a dedicated script or be part of chroot_config
        "configure_locale.sh"
    }

    /// Locale configuration is DESTRUCTIVE - writes to /etc/hostname, /etc/locale.*, etc.
    fn is_destructive(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_fstab_args() {
        let args = GenFstabArgs {
            root: PathBuf::from("/mnt"),
        };
        assert_eq!(args.to_cli_args(), vec!["--root", "/mnt"]);
        assert_eq!(args.script_name(), "generate_fstab.sh");
    }

    #[test]
    fn test_user_add_password_not_in_cli() {
        let args = UserAddArgs {
            username: "testuser".to_string(),
            password: Some("supersecret".to_string()),
            groups: None,
            shell: None,
            full_name: None,
            home_dir: None,
            create_home: true,
            sudo: false,
        };

        let cli = args.to_cli_args();

        // CRITICAL: Password must NOT appear in CLI args
        assert!(
            !cli.iter().any(|a| a.contains("supersecret")),
            "Password should not be in CLI args!"
        );
        assert!(
            !cli.iter().any(|a| a.contains("password")),
            "Password flag should not be in CLI args!"
        );
    }

    #[test]
    fn test_user_add_password_in_env() {
        let args = UserAddArgs {
            username: "testuser".to_string(),
            password: Some("supersecret".to_string()),
            groups: None,
            shell: None,
            full_name: None,
            home_dir: None,
            create_home: true,
            sudo: false,
        };

        let env = args.get_env_vars();

        // Password must be in env vars
        assert_eq!(env.len(), 1);
        assert_eq!(env[0].0, "USER_PASSWORD");
        assert_eq!(env[0].1, "supersecret");
    }

    #[test]
    fn test_user_add_sudo_adds_wheel() {
        let args = UserAddArgs {
            username: "admin".to_string(),
            password: None,
            groups: Some("audio,video".to_string()),
            shell: None,
            full_name: None,
            home_dir: None,
            create_home: true,
            sudo: true,
        };

        let cli = args.to_cli_args();

        // Should contain wheel in groups
        let groups_idx = cli.iter().position(|a| a == "--groups").unwrap();
        let groups_val = &cli[groups_idx + 1];
        assert!(
            groups_val.contains("wheel"),
            "sudo=true should add wheel group"
        );
    }

    #[test]
    fn test_user_add_no_password_empty_env() {
        let args = UserAddArgs {
            username: "nopwd".to_string(),
            password: None,
            groups: None,
            shell: None,
            full_name: None,
            home_dir: None,
            create_home: true,
            sudo: false,
        };

        let env = args.get_env_vars();
        assert!(env.is_empty(), "No password means no env vars");
    }

    #[test]
    fn test_locale_args() {
        let args = LocaleArgs {
            root: PathBuf::from("/mnt"),
            hostname: "archbox".to_string(),
            locale: "en_US.UTF-8".to_string(),
            timezone: "America/New_York".to_string(),
            keymap: Some("us".to_string()),
        };

        let cli = args.to_cli_args();
        assert!(cli.contains(&"--hostname".to_string()));
        assert!(cli.contains(&"archbox".to_string()));
        assert!(cli.contains(&"--locale".to_string()));
        assert!(cli.contains(&"en_US.UTF-8".to_string()));
        assert!(cli.contains(&"--keymap".to_string()));
        assert!(cli.contains(&"us".to_string()));
    }
}
