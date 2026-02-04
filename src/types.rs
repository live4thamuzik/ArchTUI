//! Type-safe configuration types for the ArchTUI
//!
//! This module replaces stringly-typed configuration with proper Rust enums
//! that provide compile-time validation and exhaustive matching.

use serde::{Deserialize, Serialize};
use strum::{Display, EnumIter, EnumString};

/// Boot firmware mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[derive(Display, EnumString, EnumIter)]
#[strum(serialize_all = "UPPERCASE")]
pub enum BootMode {
    #[default]
    #[strum(serialize = "Auto")]
    Auto,
    #[strum(serialize = "UEFI")]
    Uefi,
    #[strum(serialize = "BIOS")]
    Bios,
}

/// Filesystem type for partitions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[derive(Display, EnumString, EnumIter)]
#[strum(serialize_all = "lowercase")]
pub enum Filesystem {
    #[default]
    #[strum(serialize = "ext4")]
    Ext4,
    #[strum(serialize = "xfs")]
    Xfs,
    #[strum(serialize = "btrfs")]
    Btrfs,
    #[strum(serialize = "f2fs")]
    F2fs,
    /// FAT32 filesystem for EFI System Partition
    #[strum(serialize = "fat32")]
    Fat32,
}

/// Disk partitioning strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[derive(Display, EnumString, EnumIter)]
pub enum PartitionScheme {
    #[default]
    #[strum(serialize = "auto_simple")]
    AutoSimple,
    #[strum(serialize = "auto_simple_luks")]
    AutoSimpleLuks,
    #[strum(serialize = "auto_lvm")]
    AutoLvm,
    #[strum(serialize = "auto_luks_lvm")]
    AutoLuksLvm,
    #[strum(serialize = "auto_raid")]
    AutoRaid,
    #[strum(serialize = "auto_raid_luks")]
    AutoRaidLuks,
    #[strum(serialize = "auto_raid_lvm")]
    AutoRaidLvm,
    #[strum(serialize = "auto_raid_lvm_luks")]
    AutoRaidLvmLuks,
    #[strum(serialize = "manual")]
    Manual,
}

#[allow(dead_code)] // Methods available for future use
impl PartitionScheme {
    /// Check if this scheme requires RAID (multiple disks)
    pub fn requires_raid(&self) -> bool {
        matches!(
            self,
            Self::AutoRaid | Self::AutoRaidLuks | Self::AutoRaidLvm | Self::AutoRaidLvmLuks
        )
    }

    /// Check if this scheme uses encryption
    pub fn uses_encryption(&self) -> bool {
        matches!(
            self,
            Self::AutoSimpleLuks | Self::AutoLuksLvm | Self::AutoRaidLuks | Self::AutoRaidLvmLuks
        )
    }

    /// Check if this scheme uses LVM
    pub fn uses_lvm(&self) -> bool {
        matches!(
            self,
            Self::AutoLvm | Self::AutoLuksLvm | Self::AutoRaidLvm | Self::AutoRaidLvmLuks
        )
    }
}

/// Desktop environment selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[derive(Display, EnumString, EnumIter)]
#[strum(serialize_all = "lowercase")]
pub enum DesktopEnvironment {
    #[default]
    #[strum(serialize = "none")]
    None,
    #[strum(serialize = "gnome")]
    Gnome,
    #[strum(serialize = "kde")]
    Kde,
    #[strum(serialize = "hyprland")]
    Hyprland,
}

/// Display manager selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[derive(Display, EnumString, EnumIter)]
#[strum(serialize_all = "lowercase")]
pub enum DisplayManager {
    #[default]
    #[strum(serialize = "none")]
    None,
    #[strum(serialize = "gdm")]
    Gdm,
    #[strum(serialize = "sddm")]
    Sddm,
}

/// Bootloader selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[derive(Display, EnumString, EnumIter)]
#[strum(serialize_all = "lowercase")]
pub enum Bootloader {
    #[default]
    #[strum(serialize = "grub")]
    Grub,
    #[strum(serialize = "systemd-boot")]
    SystemdBoot,
}

/// AUR helper selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[derive(Display, EnumString, EnumIter)]
#[strum(serialize_all = "lowercase")]
pub enum AurHelper {
    #[default]
    #[strum(serialize = "paru")]
    Paru,
    #[strum(serialize = "yay")]
    Yay,
    #[strum(serialize = "none")]
    None,
}

/// Linux kernel selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[derive(Display, EnumString, EnumIter)]
pub enum Kernel {
    #[default]
    #[strum(serialize = "linux")]
    Linux,
    #[strum(serialize = "linux-lts")]
    LinuxLts,
    #[strum(serialize = "linux-zen")]
    LinuxZen,
    #[strum(serialize = "linux-hardened")]
    LinuxHardened,
}

/// GPU driver selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[derive(Display, EnumString, EnumIter)]
pub enum GpuDriver {
    #[default]
    #[strum(serialize = "Auto")]
    Auto,
    #[strum(serialize = "NVIDIA")]
    Nvidia,
    #[strum(serialize = "AMD")]
    Amd,
    #[strum(serialize = "Intel")]
    Intel,
}

/// Generic Yes/No toggle for boolean-like options
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[derive(Display, EnumString, EnumIter)]
pub enum Toggle {
    #[default]
    #[strum(serialize = "Yes")]
    Yes,
    #[strum(serialize = "No")]
    No,
}

#[allow(dead_code)] // Method available for future use
impl Toggle {
    /// Convert to boolean
    pub fn as_bool(&self) -> bool {
        matches!(self, Self::Yes)
    }
}

impl From<bool> for Toggle {
    fn from(value: bool) -> Self {
        if value {
            Self::Yes
        } else {
            Self::No
        }
    }
}

/// Auto/Yes/No option for fields like Encryption that support auto-detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[derive(Display, EnumString, EnumIter)]
pub enum AutoToggle {
    #[default]
    #[strum(serialize = "Auto")]
    Auto,
    #[strum(serialize = "Yes")]
    Yes,
    #[strum(serialize = "No")]
    No,
}

/// Plymouth theme selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[derive(Display, EnumString, EnumIter)]
pub enum PlymouthTheme {
    #[default]
    #[strum(serialize = "arch-glow")]
    ArchGlow,
    #[strum(serialize = "arch-mac-style")]
    ArchMacStyle,
    #[strum(serialize = "none")]
    None,
}

/// GRUB theme selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[derive(Display, EnumString, EnumIter)]
pub enum GrubTheme {
    #[default]
    #[strum(serialize = "PolyDark")]
    PolyDark,
    #[strum(serialize = "CyberEXS")]
    CyberExs,
    #[strum(serialize = "CyberPunk")]
    CyberPunk,
    #[strum(serialize = "HyperFluent")]
    HyperFluent,
    #[strum(serialize = "none")]
    None,
}

/// Btrfs snapshot frequency
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[derive(Display, EnumString, EnumIter)]
#[strum(serialize_all = "lowercase")]
pub enum SnapshotFrequency {
    #[strum(serialize = "hourly")]
    Hourly,
    #[strum(serialize = "daily")]
    Daily,
    #[default]
    #[strum(serialize = "weekly")]
    Weekly,
    #[strum(serialize = "monthly")]
    Monthly,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use strum::IntoEnumIterator;

    #[test]
    fn test_boot_mode_serialization() {
        assert_eq!(BootMode::Uefi.to_string(), "UEFI");
        assert_eq!(BootMode::Bios.to_string(), "BIOS");
        assert_eq!(BootMode::Auto.to_string(), "Auto");
    }

    #[test]
    fn test_boot_mode_parsing() {
        assert_eq!(BootMode::from_str("UEFI").unwrap(), BootMode::Uefi);
        assert_eq!(BootMode::from_str("BIOS").unwrap(), BootMode::Bios);
        assert_eq!(BootMode::from_str("Auto").unwrap(), BootMode::Auto);
    }

    #[test]
    fn test_filesystem_iteration() {
        let filesystems: Vec<String> = Filesystem::iter().map(|f| f.to_string()).collect();
        assert!(filesystems.contains(&"ext4".to_string()));
        assert!(filesystems.contains(&"btrfs".to_string()));
        assert!(filesystems.contains(&"xfs".to_string()));
    }

    #[test]
    fn test_partition_scheme_features() {
        assert!(PartitionScheme::AutoRaid.requires_raid());
        assert!(!PartitionScheme::AutoSimple.requires_raid());

        assert!(PartitionScheme::AutoSimpleLuks.uses_encryption());
        assert!(!PartitionScheme::AutoSimple.uses_encryption());

        assert!(PartitionScheme::AutoLvm.uses_lvm());
        assert!(!PartitionScheme::AutoSimple.uses_lvm());
    }

    #[test]
    fn test_toggle_conversion() {
        assert!(Toggle::Yes.as_bool());
        assert!(!Toggle::No.as_bool());
        assert_eq!(Toggle::from(true), Toggle::Yes);
        assert_eq!(Toggle::from(false), Toggle::No);
    }

    #[test]
    fn test_serde_roundtrip() {
        let original = BootMode::Uefi;
        let json = serde_json::to_string(&original).unwrap();
        let parsed: BootMode = serde_json::from_str(&json).unwrap();
        assert_eq!(original, parsed);
    }

    #[test]
    fn test_all_enums_have_default() {
        // Verify all enums have sensible defaults
        assert_eq!(BootMode::default(), BootMode::Auto);
        assert_eq!(Filesystem::default(), Filesystem::Ext4);
        assert_eq!(PartitionScheme::default(), PartitionScheme::AutoSimple);
        assert_eq!(DesktopEnvironment::default(), DesktopEnvironment::None);
        assert_eq!(Bootloader::default(), Bootloader::Grub);
        assert_eq!(Toggle::default(), Toggle::Yes);
    }
}
