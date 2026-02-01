//! Type-safe arguments for system tool scripts.
//!
//! This module provides typed argument structs for system-related scripts:
//! - `BootloaderArgs` for `install_bootloader.sh`
//! - `FstabArgs` for `generate_fstab.sh`
//! - `ChrootArgs` for `chroot_system.sh`
//! - `SystemInfoArgs` for `system_info.sh`
//! - `ServicesArgs` for `manage_services.sh`

use std::path::PathBuf;

use crate::script_traits::ScriptArgs;

// ============================================================================
// Install Bootloader
// ============================================================================

/// Type-safe arguments for `scripts/tools/install_bootloader.sh`.
#[derive(Debug, Clone)]
pub struct BootloaderArgs {
    /// Bootloader type (e.g., `grub`, `systemd-boot`).
    pub bootloader_type: String,
    /// Target disk for bootloader installation.
    pub disk: PathBuf,
    /// Boot mode (`uefi` or `bios`).
    pub mode: String,
    /// Optional EFI partition path.
    pub efi_path: Option<PathBuf>,
}

impl ScriptArgs for BootloaderArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec![
            "--type".to_string(),
            self.bootloader_type.clone(),
            "--disk".to_string(),
            self.disk.display().to_string(),
            "--mode".to_string(),
            self.mode.clone(),
        ];
        if let Some(ref efi) = self.efi_path {
            args.push("--efi-path".to_string());
            args.push(efi.display().to_string());
        }
        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "install_bootloader.sh"
    }
}

// ============================================================================
// Generate Fstab
// ============================================================================

/// Type-safe arguments for `scripts/tools/generate_fstab.sh`.
#[derive(Debug, Clone)]
pub struct FstabArgs {
    /// Root mount path for fstab generation.
    pub root: PathBuf,
}

impl ScriptArgs for FstabArgs {
    fn to_cli_args(&self) -> Vec<String> {
        vec!["--root".to_string(), self.root.display().to_string()]
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "generate_fstab.sh"
    }
}

// ============================================================================
// Chroot System
// ============================================================================

/// Type-safe arguments for `scripts/tools/chroot_system.sh`.
#[derive(Debug, Clone)]
pub struct ChrootArgs {
    /// Root path for chroot.
    pub root: PathBuf,
    /// Skip mounting filesystems.
    pub no_mount: bool,
}

impl ScriptArgs for ChrootArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec!["--root".to_string(), self.root.display().to_string()];
        if self.no_mount {
            args.push("--no-mount".to_string());
        }
        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "chroot_system.sh"
    }
}

// ============================================================================
// System Info
// ============================================================================

/// Type-safe arguments for `scripts/tools/system_info.sh`.
#[derive(Debug, Clone)]
pub struct SystemInfoArgs {
    /// Show detailed information.
    pub detailed: bool,
}

impl ScriptArgs for SystemInfoArgs {
    fn to_cli_args(&self) -> Vec<String> {
        if self.detailed {
            vec!["--detailed".to_string()]
        } else {
            vec![]
        }
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "system_info.sh"
    }
}

// ============================================================================
// Manage Services
// ============================================================================

/// Type-safe arguments for `scripts/tools/manage_services.sh`.
#[derive(Debug, Clone)]
pub struct ServicesArgs {
    /// Action to perform (e.g., `enable`, `disable`, `start`, `stop`).
    pub action: String,
    /// Optional service name.
    pub service: Option<String>,
}

impl ScriptArgs for ServicesArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec!["--action".to_string(), self.action.clone()];
        if let Some(ref svc) = self.service {
            args.push("--service".to_string());
            args.push(svc.clone());
        }
        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "manage_services.sh"
    }
}
