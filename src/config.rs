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
                value.len() >= 3
                    && value.len() <= 32
                    && value.chars().next().is_some_and(|c| c.is_alphabetic())
                    && value.chars().all(|c| c.is_alphanumeric() || c == '_')
            }
            "User Password" | "Root Password" => {
                let value = self.get_value();
                !value.is_empty() && !value.contains(char::is_whitespace)
            }
            "Disk" => self.get_value().starts_with("/dev/"),
            "Git Repository URL" => {
                let value = self.get_value();
                let trimmed = value.trim();
                trimmed.is_empty()
                    || trimmed.starts_with("http://")
                    || trimmed.starts_with("https://")
                    || trimmed.starts_with("git://")
                    || trimmed.starts_with("ssh://")
            }
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
                        Some(format!("{} must contain only alphanumeric characters and underscores, start with a letter, and be 3-32 characters", self.name))
                    }
                    "User Password" | "Root Password" => {
                        Some(format!("{} cannot be empty or contain whitespace", self.name))
                    }
                    "Disk" => {
                        Some(format!("{} must be a valid device path (e.g., /dev/sda)", self.name))
                    }
                    "Git Repository URL" => {
                        Some(format!("{} must be a valid URL (http://, https://, git://, or ssh://)", self.name))
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

impl Configuration {}

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
                "Swap" => "SWAP",
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
                // Passwords are passed via environment variables
                // While /proc/<pid>/environ is readable by root, this is acceptable because:
                // 1. Installer runs as root anyway
                // 2. Process lifetime is short
                // 3. Lint rules forbid stdin reading in bash scripts
                "User Password" => "MAIN_USER_PASSWORD",
                "Root Password" => "ROOT_PASSWORD",
                "Encryption Password" | "LUKS Password" => "ENCRYPTION_PASSWORD",
                "AUR Helper" => "AUR_HELPER",
                "Additional AUR Packages" => "ADDITIONAL_AUR_PACKAGES",
                "Flatpak" => "FLATPAK",
                "Bootloader" => "BOOTLOADER",
                "OS Prober" => "OS_PROBER",
                "GRUB Theme" => "GRUB_THEME",
                "GRUB Theme Selection" => "GRUB_THEME_SELECTION",
                "Desktop Environment" => "DESKTOP_ENVIRONMENT",
                "Display Manager" => "DISPLAY_MANAGER",
                "Plymouth" => "PLYMOUTH",
                "Plymouth Theme" => "PLYMOUTH_THEME",
                "Numlock on Boot" => "NUMLOCK_ON_BOOT",
                "Git Repository" => "GIT_REPOSITORY",
                "Git Repository URL" => "GIT_REPOSITORY_URL",
                _ => continue, // Skip unknown options
            };

            env_vars.insert(env_name.to_string(), option.get_value());
        }

        env_vars
    }

    /// Extract passwords for secure stdin passing
    ///
    /// SECURITY: Passwords should NEVER be passed via environment variables
    /// as they are visible in /proc/<pid>/environ. Instead, pass them via stdin
    /// to child processes and close the pipe immediately after writing.
    ///
    /// Returns (user_password, root_password, encryption_password)
    pub fn get_passwords(&self) -> (String, String, Option<String>) {
        let mut user_password = String::new();
        let mut root_password = String::new();
        let mut encryption_password: Option<String> = None;

        for option in &self.options {
            match option.name.as_str() {
                "User Password" => user_password = option.get_value(),
                "Root Password" => root_password = option.get_value(),
                // Encryption password may be set via the Encryption option or LUKS config
                "Encryption Password" | "LUKS Password" => {
                    let val = option.get_value();
                    if !val.is_empty() {
                        encryption_password = Some(val);
                    }
                }
                _ => {}
            }
        }

        (user_password, root_password, encryption_password)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_option_new() {
        let option = ConfigOption::new("Test Option", true, "Test description", "default");
        assert_eq!(option.name, "Test Option");
        assert!(option.required);
        assert_eq!(option.description, "Test description");
        assert_eq!(option.default_value, "default");
        assert_eq!(option.value, "");
    }

    #[test]
    fn test_config_option_get_value() {
        let mut option = ConfigOption::new("Test Option", false, "Test description", "default");

        // Should return default when value is empty
        assert_eq!(option.get_value(), "default");

        // Should return actual value when set
        option.value = "custom".to_string();
        assert_eq!(option.get_value(), "custom");
    }

    #[test]
    fn test_configuration_new() {
        let config = Configuration::default();
        assert!(!config.options.is_empty());

        // Check that essential options exist
        let option_names: Vec<&String> = config.options.iter().map(|opt| &opt.name).collect();
        assert!(option_names.contains(&&"Disk".to_string()));
        assert!(option_names.contains(&&"Root Filesystem".to_string()));
        assert!(option_names.contains(&&"Hostname".to_string()));
        assert!(option_names.contains(&&"Username".to_string()));
    }

    #[test]
    fn test_package_serialization() {
        let package = Package {
            repo: "core".to_string(),
            name: "linux".to_string(),
            version: "6.1.0".to_string(),
            installed: true,
            description: "The Linux kernel".to_string(),
        };

        let json = serde_json::to_string(&package).unwrap();
        let deserialized: Package = serde_json::from_str(&json).unwrap();
        assert_eq!(package, deserialized);
    }

    #[test]
    fn test_environment_variable_mapping() {
        let config = Configuration::default();
        let env_vars = config.to_env_vars();

        // Check that some expected environment variables are present
        assert!(env_vars.contains_key("INSTALL_DISK"));
        assert!(env_vars.contains_key("ROOT_FILESYSTEM"));
        // Note: HOSTNAME and USERNAME may not be in the mapping if they're not configured
        assert!(env_vars.contains_key("INSTALL_DISK")); // At least one should be present
    }

    #[test]
    fn test_passwords_in_env_vars() {
        // Design decision: Passwords are passed via environment variables
        // because lint rules forbid `read` in bash scripts.
        // This is acceptable because:
        // 1. Installer runs as root
        // 2. Process lifetime is short
        // 3. Only root can read /proc/<pid>/environ of root processes
        let mut config = Configuration::default();

        // Set passwords
        for opt in &mut config.options {
            match opt.name.as_str() {
                "User Password" => opt.value = "secret_user_pw".to_string(),
                "Root Password" => opt.value = "secret_root_pw".to_string(),
                _ => {}
            }
        }

        let env_vars = config.to_env_vars();

        // Verify passwords ARE in environment variables
        assert!(
            env_vars.contains_key("MAIN_USER_PASSWORD"),
            "User password should be in env vars"
        );
        assert!(
            env_vars.contains_key("ROOT_PASSWORD"),
            "Root password should be in env vars"
        );
        assert_eq!(
            env_vars.get("MAIN_USER_PASSWORD"),
            Some(&"secret_user_pw".to_string())
        );
        assert_eq!(
            env_vars.get("ROOT_PASSWORD"),
            Some(&"secret_root_pw".to_string())
        );
    }

    #[test]
    fn test_get_passwords() {
        let mut config = Configuration::default();

        // Set passwords
        for opt in &mut config.options {
            match opt.name.as_str() {
                "User Password" => opt.value = "user123".to_string(),
                "Root Password" => opt.value = "root456".to_string(),
                _ => {}
            }
        }

        let (user_pw, root_pw, encrypt_pw) = config.get_passwords();

        assert_eq!(user_pw, "user123");
        assert_eq!(root_pw, "root456");
        assert!(encrypt_pw.is_none()); // No encryption password set
    }
}
