//! Package & Service Resolver (Sprint 16)
//!
//! Translates high-level configuration choices into concrete package names
//! and systemd service names.
//!
//! # Design
//!
//! - **No hardcoded strings**: All package lists come from `profiles/mod.rs` constants
//! - **Deduplication**: Output is deduplicated and sorted for deterministic results
//! - **Pure logic**: No I/O, no side effects — only resolves names
//!
//! # Resolution Rules
//!
//! | Config Field       | Resolved To |
//! |--------------------|-------------|
//! | `kernel`           | Kernel + headers packages |
//! | `gpu_drivers`      | Driver-specific packages |
//! | `desktop_env`      | Profile packages (from Profile enum) |
//! | `bootloader`       | Bootloader packages |
//! | `multilib`         | Enables lib32-* packages |
//! | `flatpak`          | Adds flatpak package |
//! | `display_manager`  | DM service name |
//! | `NetworkManager`   | Always enabled |

// Library API - consumed by installer orchestration
#![allow(dead_code)]

use crate::config_file::InstallationConfig;
use crate::profiles::{
    gpu_packages, kernel_packages, bootloader_packages, Profile, BASE_PACKAGES,
};
use crate::types::*;

// ============================================================================
// Package Resolution
// ============================================================================

/// Resolve all packages to install based on the installation configuration.
///
/// Collects packages from:
/// 1. Base system packages (always installed)
/// 2. Kernel packages (based on selected kernel)
/// 3. GPU driver packages (based on selected GPU driver)
/// 4. Bootloader packages
/// 5. Desktop/WM profile packages
/// 6. Additional user-specified packages
/// 7. Flatpak (if enabled)
///
/// # Returns
///
/// A deduplicated, sorted `Vec<String>` of package names ready for ALPM.
///
/// # What This Explicitly Refuses To Do
///
/// - Install AUR packages: Those require a separate AUR helper flow
/// - Validate package existence: That's ALPM's job at install time
/// - Handle package conflicts: pacman/ALPM resolves dependencies
pub fn resolve_packages(config: &InstallationConfig) -> Vec<String> {
    let mut packages: Vec<&str> = Vec::new();

    // 1. Base system — always installed
    packages.extend_from_slice(BASE_PACKAGES);

    // 2. Kernel
    let kernel_pkgs = match config.kernel {
        Kernel::Linux => kernel_packages::LINUX,
        Kernel::LinuxLts => kernel_packages::LINUX_LTS,
        Kernel::LinuxZen => kernel_packages::LINUX_ZEN,
        Kernel::LinuxHardened => kernel_packages::LINUX_HARDENED,
    };
    packages.extend_from_slice(kernel_pkgs);

    // 3. GPU drivers
    let gpu_pkgs = match config.gpu_drivers {
        GpuDriver::Nvidia => gpu_packages::NVIDIA,
        GpuDriver::Amd => gpu_packages::AMD,
        GpuDriver::Intel => gpu_packages::INTEL,
        GpuDriver::Auto => gpu_packages::AUTO,
    };
    // Only include lib32-* packages if multilib is enabled
    if config.multilib == Toggle::Yes {
        packages.extend_from_slice(gpu_pkgs);
    } else {
        packages.extend(gpu_pkgs.iter().filter(|p| !p.starts_with("lib32-")));
    }

    // 4. Bootloader
    let boot_pkgs = match config.bootloader {
        Bootloader::Grub => bootloader_packages::GRUB,
        Bootloader::SystemdBoot => bootloader_packages::SYSTEMD_BOOT,
    };
    packages.extend_from_slice(boot_pkgs);

    // 5. Desktop/WM profile
    let profile = desktop_to_profile(config.desktop_environment);
    packages.extend_from_slice(profile.get_packages());

    // 6. Flatpak
    if config.flatpak == Toggle::Yes {
        packages.push("flatpak");
    }

    // 7. Encryption tools (if encryption is enabled)
    if config.partitioning_strategy.uses_encryption() {
        packages.push("cryptsetup");
    }

    // 8. LVM tools (if LVM is enabled)
    if config.partitioning_strategy.uses_lvm() {
        packages.push("lvm2");
    }

    // 9. Btrfs tools (if Btrfs filesystem selected)
    if config.root_filesystem == Filesystem::Btrfs {
        packages.push("btrfs-progs");
    }

    // 10. Additional user-specified packages
    let additional = parse_package_list(&config.additional_packages);

    // Deduplicate and sort
    let mut result: Vec<String> = packages.iter().map(|s| s.to_string()).collect();
    result.extend(additional);
    result.sort();
    result.dedup();

    result
}

// ============================================================================
// Service Resolution
// ============================================================================

/// Resolve all systemd services to enable based on the installation configuration.
///
/// # Returns
///
/// A deduplicated, sorted `Vec<String>` of service names for `systemctl enable`.
///
/// # Resolution Rules
///
/// - `NetworkManager.service` — always enabled
/// - Display manager service — based on desktop profile
/// - `bluetooth.service` — enabled for desktop profiles (not Minimal)
/// - `fstrim.timer` — enabled for SSD optimization
pub fn resolve_services(config: &InstallationConfig) -> Vec<String> {
    let mut services: Vec<&str> = Vec::new();

    // NetworkManager — always enabled
    services.push("NetworkManager");

    // Display manager from profile
    let profile = desktop_to_profile(config.desktop_environment);
    if let Some(dm) = profile.get_display_manager() {
        services.push(dm);
    }

    // Profile-specific services
    services.extend_from_slice(profile.get_services());

    // Bluetooth for desktop environments
    if config.desktop_environment != DesktopEnvironment::None {
        services.push("bluetooth");
    }

    // NTP time sync
    if config.time_sync == Toggle::Yes {
        services.push("systemd-timesyncd");
    }

    // SSD optimization
    services.push("fstrim.timer");

    // Deduplicate and sort
    let mut result: Vec<String> = services.iter().map(|s| s.to_string()).collect();
    result.sort();
    result.dedup();

    result
}

// ============================================================================
// Helpers
// ============================================================================

/// Map `DesktopEnvironment` enum to `Profile` enum.
///
/// The `DesktopEnvironment` enum in types.rs has fewer variants than `Profile`.
/// This maps the config-level choice to the profile-level detail.
fn desktop_to_profile(de: DesktopEnvironment) -> Profile {
    match de {
        DesktopEnvironment::None => Profile::Minimal,
        DesktopEnvironment::Gnome => Profile::Gnome,
        DesktopEnvironment::Kde => Profile::Kde,
        DesktopEnvironment::Hyprland => Profile::Hyprland,
    }
}

/// Parse a space/comma-separated package list string into individual package names.
///
/// Handles both "pkg1 pkg2 pkg3" and "pkg1,pkg2,pkg3" formats.
/// Empty strings and whitespace-only entries are filtered out.
fn parse_package_list(input: &str) -> Vec<String> {
    input
        .split(|c: char| c == ',' || c.is_whitespace())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a basic config for testing
    fn test_config() -> InstallationConfig {
        InstallationConfig::new()
    }

    #[test]
    fn test_resolve_packages_always_has_base() {
        let config = test_config();
        let packages = resolve_packages(&config);

        assert!(packages.contains(&"base".to_string()));
        assert!(packages.contains(&"base-devel".to_string()));
        assert!(packages.contains(&"linux-firmware".to_string()));
        assert!(packages.contains(&"sudo".to_string()));
    }

    #[test]
    fn test_resolve_packages_default_kernel() {
        let config = test_config();
        let packages = resolve_packages(&config);

        // Default kernel is Linux
        assert!(packages.contains(&"linux".to_string()));
        assert!(packages.contains(&"linux-headers".to_string()));
    }

    #[test]
    fn test_resolve_packages_hardened_kernel() {
        let mut config = test_config();
        config.kernel = Kernel::LinuxHardened;
        let packages = resolve_packages(&config);

        assert!(packages.contains(&"linux-hardened".to_string()));
        assert!(packages.contains(&"linux-hardened-headers".to_string()));
        assert!(!packages.contains(&"linux-headers".to_string()));
    }

    #[test]
    fn test_resolve_packages_nvidia_gpu() {
        let mut config = test_config();
        config.gpu_drivers = GpuDriver::Nvidia;
        config.multilib = Toggle::Yes;
        let packages = resolve_packages(&config);

        assert!(packages.contains(&"nvidia".to_string()));
        assert!(packages.contains(&"nvidia-utils".to_string()));
        assert!(packages.contains(&"lib32-nvidia-utils".to_string()));
    }

    #[test]
    fn test_resolve_packages_nvidia_no_multilib() {
        let mut config = test_config();
        config.gpu_drivers = GpuDriver::Nvidia;
        config.multilib = Toggle::No;
        let packages = resolve_packages(&config);

        assert!(packages.contains(&"nvidia".to_string()));
        assert!(packages.contains(&"nvidia-utils".to_string()));
        // lib32 packages excluded
        assert!(!packages.contains(&"lib32-nvidia-utils".to_string()));
    }

    #[test]
    fn test_resolve_packages_intel_gpu() {
        let mut config = test_config();
        config.gpu_drivers = GpuDriver::Intel;
        config.multilib = Toggle::No;
        let packages = resolve_packages(&config);

        assert!(packages.contains(&"intel-ucode".to_string()));
        assert!(packages.contains(&"mesa".to_string()));
        assert!(packages.contains(&"vulkan-intel".to_string()));
    }

    #[test]
    fn test_resolve_packages_hyprland_desktop() {
        let mut config = test_config();
        config.desktop_environment = DesktopEnvironment::Hyprland;
        let packages = resolve_packages(&config);

        assert!(packages.contains(&"hyprland".to_string()));
        assert!(packages.contains(&"waybar".to_string()));
        assert!(packages.contains(&"kitty".to_string()));
        assert!(packages.contains(&"sddm".to_string()));
    }

    #[test]
    fn test_resolve_packages_kde_desktop() {
        let mut config = test_config();
        config.desktop_environment = DesktopEnvironment::Kde;
        let packages = resolve_packages(&config);

        assert!(packages.contains(&"plasma-meta".to_string()));
        assert!(packages.contains(&"sddm".to_string()));
        assert!(packages.contains(&"konsole".to_string()));
    }

    #[test]
    fn test_resolve_packages_grub_bootloader() {
        let mut config = test_config();
        config.bootloader = Bootloader::Grub;
        let packages = resolve_packages(&config);

        assert!(packages.contains(&"grub".to_string()));
        assert!(packages.contains(&"efibootmgr".to_string()));
    }

    #[test]
    fn test_resolve_packages_flatpak() {
        let mut config = test_config();
        config.flatpak = Toggle::Yes;
        let packages = resolve_packages(&config);
        assert!(packages.contains(&"flatpak".to_string()));
    }

    #[test]
    fn test_resolve_packages_no_flatpak() {
        let mut config = test_config();
        config.flatpak = Toggle::No;
        let packages = resolve_packages(&config);
        assert!(!packages.contains(&"flatpak".to_string()));
    }

    #[test]
    fn test_resolve_packages_luks_adds_cryptsetup() {
        let mut config = test_config();
        config.partitioning_strategy = PartitionScheme::AutoSimpleLuks;
        let packages = resolve_packages(&config);
        assert!(packages.contains(&"cryptsetup".to_string()));
    }

    #[test]
    fn test_resolve_packages_lvm_adds_lvm2() {
        let mut config = test_config();
        config.partitioning_strategy = PartitionScheme::AutoLvm;
        let packages = resolve_packages(&config);
        assert!(packages.contains(&"lvm2".to_string()));
    }

    #[test]
    fn test_resolve_packages_btrfs_adds_progs() {
        let mut config = test_config();
        config.root_filesystem = Filesystem::Btrfs;
        let packages = resolve_packages(&config);
        assert!(packages.contains(&"btrfs-progs".to_string()));
    }

    #[test]
    fn test_resolve_packages_deduplicated() {
        let config = test_config();
        let packages = resolve_packages(&config);

        // Check no duplicates
        let mut sorted = packages.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(packages.len(), sorted.len(), "packages list has duplicates");
    }

    #[test]
    fn test_resolve_packages_additional() {
        let mut config = test_config();
        config.additional_packages = "htop neovim tmux".to_string();
        let packages = resolve_packages(&config);

        assert!(packages.contains(&"htop".to_string()));
        assert!(packages.contains(&"neovim".to_string()));
        assert!(packages.contains(&"tmux".to_string()));
    }

    #[test]
    fn test_resolve_services_always_has_networkmanager() {
        let config = test_config();
        let services = resolve_services(&config);
        assert!(services.contains(&"NetworkManager".to_string()));
    }

    #[test]
    fn test_resolve_services_hyprland_has_sddm() {
        let mut config = test_config();
        config.desktop_environment = DesktopEnvironment::Hyprland;
        let services = resolve_services(&config);

        assert!(services.contains(&"sddm".to_string()));
        assert!(services.contains(&"bluetooth".to_string()));
    }

    #[test]
    fn test_resolve_services_gnome_has_gdm() {
        let mut config = test_config();
        config.desktop_environment = DesktopEnvironment::Gnome;
        let services = resolve_services(&config);

        assert!(services.contains(&"gdm".to_string()));
    }

    #[test]
    fn test_resolve_services_minimal_no_dm() {
        let mut config = test_config();
        config.desktop_environment = DesktopEnvironment::None;
        let services = resolve_services(&config);

        assert!(!services.contains(&"gdm".to_string()));
        assert!(!services.contains(&"sddm".to_string()));
        assert!(!services.contains(&"lightdm".to_string()));
        // No bluetooth for minimal
        assert!(!services.contains(&"bluetooth".to_string()));
    }

    #[test]
    fn test_resolve_services_ntp_enabled() {
        let mut config = test_config();
        config.time_sync = Toggle::Yes;
        let services = resolve_services(&config);
        assert!(services.contains(&"systemd-timesyncd".to_string()));
    }

    #[test]
    fn test_resolve_services_fstrim() {
        let config = test_config();
        let services = resolve_services(&config);
        assert!(services.contains(&"fstrim.timer".to_string()));
    }

    #[test]
    fn test_resolve_services_deduplicated() {
        let mut config = test_config();
        config.desktop_environment = DesktopEnvironment::Gnome;
        let services = resolve_services(&config);

        let mut sorted = services.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(services.len(), sorted.len(), "services list has duplicates");
    }

    #[test]
    fn test_parse_package_list_spaces() {
        let result = parse_package_list("htop neovim tmux");
        assert_eq!(result, vec!["htop", "neovim", "tmux"]);
    }

    #[test]
    fn test_parse_package_list_commas() {
        let result = parse_package_list("htop,neovim,tmux");
        assert_eq!(result, vec!["htop", "neovim", "tmux"]);
    }

    #[test]
    fn test_parse_package_list_mixed() {
        let result = parse_package_list("htop, neovim  tmux,,");
        assert_eq!(result, vec!["htop", "neovim", "tmux"]);
    }

    #[test]
    fn test_parse_package_list_empty() {
        let result = parse_package_list("");
        assert!(result.is_empty());
    }

    #[test]
    fn test_desktop_to_profile_mapping() {
        assert_eq!(desktop_to_profile(DesktopEnvironment::None), Profile::Minimal);
        assert_eq!(desktop_to_profile(DesktopEnvironment::Gnome), Profile::Gnome);
        assert_eq!(desktop_to_profile(DesktopEnvironment::Kde), Profile::Kde);
        assert_eq!(desktop_to_profile(DesktopEnvironment::Hyprland), Profile::Hyprland);
    }
}
