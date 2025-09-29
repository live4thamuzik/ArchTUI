//! Configuration management module
//!
//! Handles all configuration options, validation, and environment variable mapping.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Package structure for structured package data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Package {
    pub repo: String,
    pub name: String,
    pub version: String,
    pub installed: bool,
    pub description: String,
}

/// Individual configuration option
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigOption {
    /// Display name of the option
    pub name: String,
    /// Current value
    pub value: String,
    /// Whether this option is required for installation
    pub required: bool,
    /// Description of the option
    pub description: String,
    /// Default value
    pub default_value: String,
}

impl ConfigOption {
    /// Create a new configuration option
    pub fn new(name: &str, required: bool, description: &str, default_value: &str) -> Self {
        Self {
            name: name.to_string(),
            value: String::new(),
            required,
            description: description.to_string(),
            default_value: default_value.to_string(),
        }
    }

    /// Get the current value, falling back to default if empty
    pub fn get_value(&self) -> String {
        if self.value.is_empty() {
            self.default_value.clone()
        } else {
            self.value.clone()
        }
    }

    /// Validate the current value
    pub fn is_valid(&self) -> bool {
        if self.required && self.value.trim().is_empty() {
            return false;
        }
        // Add specific validation based on field type
        match self.name.as_str() {
            "Username" | "Hostname" => {
                let value = self.get_value();
                value
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
                    && !value.starts_with('-')
                    && value.len() <= 32
            }
            "User Password" | "Root Password" => self.get_value().len() >= 8,
            "Disk" => self.get_value().starts_with("/dev/"),
            _ => true, // Default: any non-empty value is valid
        }
    }

    /// Get validation error message if invalid
    pub fn validation_error(&self) -> Option<String> {
        if !self.is_valid() {
            if self.required && self.value.trim().is_empty() {
                Some(format!("{} is required", self.name))
            } else {
                match self.name.as_str() {
                    "Username" | "Hostname" => {
                        Some(format!("{} must contain only alphanumeric characters, hyphens, and underscores, and be 32 characters or less", self.name))
                    }
                    "User Password" | "Root Password" => {
                        Some(format!("{} must be at least 8 characters long", self.name))
                    }
                    "Disk" => {
                        Some(format!("{} must be a valid device path (e.g., /dev/sda)", self.name))
                    }
                    _ => Some(format!("{} has an invalid value", self.name))
                }
            }
        } else {
            None
        }
    }
}

/// Complete configuration for the installation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Configuration {
    /// List of all configuration options
    pub options: Vec<ConfigOption>,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            options: vec![
                // Boot Setup (0-1)
                ConfigOption::new("Boot Mode", true, "Boot firmware type (Auto/UEFI/BIOS)", ""),
                ConfigOption::new(
                    "Secure Boot",
                    false,
                    "Enable Secure Boot (WARNING: Requires UEFI setup)",
                    "No",
                ),
                // System Locale and Input (2-3)
                ConfigOption::new("Locale", true, "System locale", "en_US.UTF-8"),
                ConfigOption::new("Keymap", true, "Keyboard layout", "us"),
                // Disk and Storage (4-14)
                ConfigOption::new("Disk", true, "Target disk for installation", ""),
                ConfigOption::new(
                    "Partitioning Strategy",
                    true,
                    "How to partition the disk",
                    "",
                ),
                ConfigOption::new("Encryption", false, "Enable disk encryption", "Auto"),
                ConfigOption::new("Root Filesystem", true, "Root partition filesystem", "ext4"),
                ConfigOption::new(
                    "Separate Home Partition",
                    false,
                    "Create separate /home partition",
                    "No",
                ),
                ConfigOption::new(
                    "Home Filesystem",
                    false,
                    "Home partition filesystem",
                    "ext4",
                ),
                ConfigOption::new("Swap", false, "Enable swap partition", "Yes"),
                ConfigOption::new("Swap Size", false, "Swap partition size", "2GB"),
                ConfigOption::new("Btrfs Snapshots", false, "Enable Btrfs snapshots", "No"),
                ConfigOption::new(
                    "Btrfs Frequency",
                    false,
                    "Btrfs snapshot frequency",
                    "weekly",
                ),
                ConfigOption::new(
                    "Btrfs Keep Count",
                    false,
                    "Number of snapshots to keep",
                    "3",
                ),
                ConfigOption::new("Btrfs Assistant", false, "Use Btrfs assistant", "No"),
                // Time and Location (15-17)
                ConfigOption::new("Timezone Region", true, "Timezone region", "America"),
                ConfigOption::new("Timezone", true, "Timezone city", "New_York"),
                ConfigOption::new(
                    "Time Sync (NTP)",
                    false,
                    "Enable NTP time synchronization",
                    "Yes",
                ),
                // System Packages (18-22)
                ConfigOption::new(
                    "Mirror Country",
                    true,
                    "Package mirror country",
                    "United States",
                ),
                ConfigOption::new("Kernel", true, "Linux kernel to install", "linux"),
                ConfigOption::new("Multilib", false, "Enable multilib repository", "Yes"),
                ConfigOption::new(
                    "Additional Pacman Packages",
                    false,
                    "Extra packages to install",
                    "",
                ),
                ConfigOption::new("GPU Drivers", false, "Graphics drivers", "Auto"),
                // Hostname (23)
                ConfigOption::new("Hostname", true, "System hostname", ""),
                // User Setup (24-26)
                ConfigOption::new("Username", true, "Primary user account", ""),
                ConfigOption::new("User Password", true, "User account password", ""),
                ConfigOption::new("Root Password", true, "Root account password", ""),
                // Package Management (27-29)
                ConfigOption::new("AUR Helper", false, "AUR package helper", "paru"),
                ConfigOption::new("Additional AUR Packages", false, "Extra AUR packages", ""),
                ConfigOption::new("Flatpak", false, "Enable Flatpak support", "No"),
                // Boot Configuration (30-32)
                ConfigOption::new("Bootloader", true, "Boot loader", "grub"),
                ConfigOption::new("OS Prober", false, "Enable OS detection", "Yes"),
                ConfigOption::new("GRUB Theme", false, "Enable GRUB themes", "No"),
                ConfigOption::new("GRUB Theme Selection", false, "GRUB theme to use", "arch"),
                // Desktop Environment (33-34)
                ConfigOption::new("Desktop Environment", false, "Desktop environment", "KDE"),
                ConfigOption::new("Display Manager", false, "Display manager", "sddm"),
                // Boot Splash and Final Setup (35-38)
                ConfigOption::new("Plymouth", false, "Boot splash screen", "Yes"),
                ConfigOption::new("Plymouth Theme", false, "Plymouth theme", "arch-glow"),
                ConfigOption::new("Numlock on Boot", false, "Enable numlock at boot", "Yes"),
                ConfigOption::new(
                    "Git Repository",
                    false,
                    "Clone installation repository",
                    "No",
                ),
                ConfigOption::new(
                    "Git Repository URL",
                    false,
                    "Git repository URL to clone",
                    "",
                ),
            ],
        }
    }
}

impl Configuration {
    /// Convert configuration to environment variables for the installer
    pub fn to_env_vars(&self) -> HashMap<String, String> {
        let mut env_vars = HashMap::new();

        // Map configuration options to environment variables by name (more maintainable)
        for option in &self.options {
            let env_name = match option.name.as_str() {
                "Boot Mode" => "BOOT_MODE",
                "Secure Boot" => "SECURE_BOOT",
                "Locale" => "LOCALE",
                "Keymap" => "KEYMAP",
                "Disk" => "INSTALL_DISK",
                "Partitioning Strategy" => "PARTITIONING_STRATEGY",
                "Encryption" => "ENCRYPTION",
                "Root Filesystem" => "ROOT_FILESYSTEM",
                "Separate Home Partition" => "SEPARATE_HOME",
                "Home Filesystem" => "HOME_FILESYSTEM",
                "Swap Size" => "SWAP_SIZE",
                "Btrfs Snapshots" => "BTRFS_SNAPSHOTS",
                "Btrfs Frequency" => "BTRFS_FREQUENCY",
                "Btrfs Keep Count" => "BTRFS_KEEP_COUNT",
                "Btrfs Assistant" => "BTRFS_ASSISTANT",
                "Timezone Region" => "TIMEZONE_REGION",
                "Timezone" => "TIMEZONE",
                "Time Sync (NTP)" => "TIME_SYNC",
                "Mirror Country" => "MIRROR_COUNTRY",
                "Kernel" => "KERNEL",
                "Multilib" => "MULTILIB",
                "Additional Pacman Packages" => "ADDITIONAL_PACKAGES",
                "GPU Drivers" => "GPU_DRIVERS",
                "Hostname" => "SYSTEM_HOSTNAME",
                "Username" => "MAIN_USERNAME",
                "User Password" => "MAIN_USER_PASSWORD",
                "Root Password" => "ROOT_PASSWORD",
                "AUR Helper" => "AUR_HELPER",
                "Additional AUR Packages" => "ADDITIONAL_AUR_PACKAGES",
                "Flatpak" => "FLATPAK",
                "Bootloader" => "BOOTLOADER",
                "OS Prober" => "OS_PROBER",
                "GRUB Theme" => "GRUB_THEME",
                "Desktop Environment" => "DESKTOP_ENVIRONMENT",
                "Display Manager" => "DISPLAY_MANAGER",
                "Plymouth" => "PLYMOUTH",
                "Plymouth Theme" => "PLYMOUTH_THEME",
                "Numlock on Boot" => "NUMLOCK_ON_BOOT",
                "Git Repository" => "GIT_REPOSITORY",
                _ => continue, // Skip unknown options
            };

            env_vars.insert(env_name.to_string(), option.get_value());
        }

        env_vars
    }
}
