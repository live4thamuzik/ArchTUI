//! Configuration file handling for saving and loading installation configs.
//!
//! This module uses type-safe enums instead of strings for configuration values,
//! providing compile-time validation and preventing typos.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::types::{
    AurHelper, AutoToggle, Bootloader, BootMode, DesktopEnvironment, DisplayManager, Filesystem,
    GpuDriver, GrubTheme, Kernel, PartitionScheme, PlymouthTheme, SnapshotFrequency, Toggle,
};

/// Installation configuration that can be saved/loaded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationConfig {
    // Boot & System
    pub boot_mode: BootMode,
    pub secure_boot: Toggle,

    // Disk & Storage
    pub install_disk: String, // Disk path like /dev/sda - must remain String
    pub partitioning_strategy: PartitionScheme,
    pub root_filesystem: Filesystem,
    pub home_filesystem: Filesystem,
    pub separate_home: Toggle,
    pub encryption: AutoToggle,
    pub swap: Toggle,
    pub swap_size: String, // Size like "2GB" - flexible format

    // Btrfs options
    pub btrfs_snapshots: Toggle,
    pub btrfs_frequency: SnapshotFrequency,
    pub btrfs_keep_count: u8,
    pub btrfs_assistant: Toggle,

    // Locale & Time
    pub timezone_region: String, // Too many options for enum
    pub timezone: String,        // Too many options for enum
    pub locale: String,          // Too many options for enum
    pub keymap: String,          // Too many options for enum
    pub time_sync: Toggle,

    // Network & Mirrors
    pub mirror_country: String, // Too many options for enum
    pub hostname: String,       // User-defined

    // User accounts
    pub username: String,      // User-defined
    pub user_password: String, // User-defined
    pub root_password: String, // User-defined

    // Packages
    pub kernel: Kernel,
    pub gpu_drivers: GpuDriver,
    pub multilib: Toggle,
    pub additional_packages: String,     // Space-separated list
    pub additional_aur_packages: String, // Space-separated list
    pub aur_helper: AurHelper,
    pub flatpak: Toggle,

    // Boot configuration
    pub bootloader: Bootloader,
    pub os_prober: Toggle,
    pub grub_themes: Toggle,
    pub grub_theme_selection: GrubTheme,

    // Desktop
    pub desktop_environment: DesktopEnvironment,
    pub display_manager: DisplayManager,

    // Final setup
    pub plymouth: Toggle,
    pub plymouth_theme: PlymouthTheme,
    pub numlock_on_boot: Toggle,
    pub git_repository: Toggle,
    pub git_repository_url: String, // User-defined URL
}

impl InstallationConfig {
    /// Create a new empty configuration with sensible defaults
    #[allow(dead_code)] // API method available for external use
    pub fn new() -> Self {
        Self::default()
    }

    /// Save configuration to a JSON file
    #[allow(dead_code)]
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize configuration to JSON")?;

        fs::write(&path, json)
            .with_context(|| format!("Failed to write configuration to {:?}", path.as_ref()))?;

        Ok(())
    }

    /// Load configuration from a JSON file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read configuration from {:?}", path.as_ref()))?;

        let config: Self =
            serde_json::from_str(&content).context("Failed to parse configuration JSON")?;

        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        // Validate disk path
        if self.install_disk.trim().is_empty() {
            anyhow::bail!("Install disk must be specified");
        }

        // Validate hostname (3-32 chars, start with letter, alphanumeric + underscore)
        let hostname = self.hostname.trim();
        if hostname.is_empty() {
            anyhow::bail!("Hostname must be specified");
        }
        if hostname.len() < 3 || hostname.len() > 32 {
            anyhow::bail!("Hostname must be 3-32 characters long");
        }
        if let Some(first_char) = hostname.chars().next() {
            if !first_char.is_ascii_alphabetic() {
                anyhow::bail!("Hostname must start with a letter");
            }
        }
        if !hostname
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            anyhow::bail!("Hostname can only contain letters, numbers, and underscores");
        }

        // Validate username (3-32 chars, start with letter, alphanumeric + underscore)
        let username = self.username.trim();
        if username.is_empty() {
            anyhow::bail!("Username must be specified");
        }
        if username.len() < 3 || username.len() > 32 {
            anyhow::bail!("Username must be 3-32 characters long");
        }
        if let Some(first_char) = username.chars().next() {
            if !first_char.is_ascii_alphabetic() {
                anyhow::bail!("Username must start with a letter");
            }
        }
        if !username
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            anyhow::bail!("Username can only contain letters, numbers, and underscores");
        }

        // Validate passwords (non-empty, no whitespace)
        if self.user_password.trim().is_empty() {
            anyhow::bail!("User password must be specified");
        }
        if self.user_password.contains(char::is_whitespace) {
            anyhow::bail!("User password cannot contain whitespace");
        }

        if self.root_password.trim().is_empty() {
            anyhow::bail!("Root password must be specified");
        }
        if self.root_password.contains(char::is_whitespace) {
            anyhow::bail!("Root password cannot contain whitespace");
        }

        // Validate Git repository URL format if enabled
        if self.git_repository == Toggle::Yes && !self.git_repository_url.trim().is_empty() {
            let url = self.git_repository_url.trim();
            if !url.starts_with("http://")
                && !url.starts_with("https://")
                && !url.starts_with("git://")
                && !url.starts_with("ssh://")
            {
                anyhow::bail!(
                    "Git repository URL must start with http://, https://, git://, or ssh://"
                );
            }
        }

        // Validate RAID configuration
        if self.partitioning_strategy.requires_raid() {
            // RAID validation would check multiple disks - handled at runtime
        }

        Ok(())
    }

    /// Convert to environment variables for Bash scripts
    #[allow(dead_code)]
    pub fn to_env_vars(&self) -> Vec<(String, String)> {
        vec![
            ("BOOT_MODE".to_string(), self.boot_mode.to_string()),
            ("SECURE_BOOT".to_string(), self.secure_boot.to_string()),
            ("INSTALL_DISK".to_string(), self.install_disk.clone()),
            (
                "PARTITIONING_STRATEGY".to_string(),
                self.partitioning_strategy.to_string(),
            ),
            (
                "ROOT_FILESYSTEM".to_string(),
                self.root_filesystem.to_string(),
            ),
            (
                "HOME_FILESYSTEM".to_string(),
                self.home_filesystem.to_string(),
            ),
            ("SEPARATE_HOME".to_string(), self.separate_home.to_string()),
            ("ENCRYPTION".to_string(), self.encryption.to_string()),
            ("SWAP".to_string(), self.swap.to_string()),
            ("SWAP_SIZE".to_string(), self.swap_size.clone()),
            (
                "BTRFS_SNAPSHOTS".to_string(),
                self.btrfs_snapshots.to_string(),
            ),
            (
                "BTRFS_FREQUENCY".to_string(),
                self.btrfs_frequency.to_string(),
            ),
            (
                "BTRFS_KEEP_COUNT".to_string(),
                self.btrfs_keep_count.to_string(),
            ),
            (
                "BTRFS_ASSISTANT".to_string(),
                self.btrfs_assistant.to_string(),
            ),
            ("TIMEZONE_REGION".to_string(), self.timezone_region.clone()),
            ("TIMEZONE".to_string(), self.timezone.clone()),
            ("LOCALE".to_string(), self.locale.clone()),
            ("KEYMAP".to_string(), self.keymap.clone()),
            ("TIME_SYNC".to_string(), self.time_sync.to_string()),
            ("MIRROR_COUNTRY".to_string(), self.mirror_country.clone()),
            ("SYSTEM_HOSTNAME".to_string(), self.hostname.clone()),
            ("MAIN_USERNAME".to_string(), self.username.clone()),
            ("MAIN_USER_PASSWORD".to_string(), self.user_password.clone()),
            ("ROOT_PASSWORD".to_string(), self.root_password.clone()),
            ("KERNEL".to_string(), self.kernel.to_string()),
            ("GPU_DRIVERS".to_string(), self.gpu_drivers.to_string()),
            ("MULTILIB".to_string(), self.multilib.to_string()),
            (
                "ADDITIONAL_PACKAGES".to_string(),
                self.additional_packages.clone(),
            ),
            (
                "ADDITIONAL_AUR_PACKAGES".to_string(),
                self.additional_aur_packages.clone(),
            ),
            ("AUR_HELPER".to_string(), self.aur_helper.to_string()),
            ("FLATPAK".to_string(), self.flatpak.to_string()),
            ("BOOTLOADER".to_string(), self.bootloader.to_string()),
            ("OS_PROBER".to_string(), self.os_prober.to_string()),
            ("GRUB_THEMES".to_string(), self.grub_themes.to_string()),
            (
                "GRUB_THEME_SELECTION".to_string(),
                self.grub_theme_selection.to_string(),
            ),
            (
                "DESKTOP_ENVIRONMENT".to_string(),
                self.desktop_environment.to_string(),
            ),
            (
                "DISPLAY_MANAGER".to_string(),
                self.display_manager.to_string(),
            ),
            ("PLYMOUTH".to_string(), self.plymouth.to_string()),
            (
                "PLYMOUTH_THEME".to_string(),
                self.plymouth_theme.to_string(),
            ),
            (
                "NUMLOCK_ON_BOOT".to_string(),
                self.numlock_on_boot.to_string(),
            ),
            (
                "GIT_REPOSITORY".to_string(),
                self.git_repository.to_string(),
            ),
            (
                "GIT_REPOSITORY_URL".to_string(),
                self.git_repository_url.clone(),
            ),
        ]
    }
}

impl Default for InstallationConfig {
    fn default() -> Self {
        Self {
            boot_mode: BootMode::Auto,
            secure_boot: Toggle::No,
            install_disk: String::new(),
            partitioning_strategy: PartitionScheme::AutoSimple,
            root_filesystem: Filesystem::Ext4,
            home_filesystem: Filesystem::Ext4,
            separate_home: Toggle::No,
            encryption: AutoToggle::Auto,
            swap: Toggle::Yes,
            swap_size: "2GB".to_string(),
            btrfs_snapshots: Toggle::No,
            btrfs_frequency: SnapshotFrequency::Weekly,
            btrfs_keep_count: 3,
            btrfs_assistant: Toggle::No,
            timezone_region: "America".to_string(),
            timezone: "New_York".to_string(),
            locale: "en_US.UTF-8".to_string(),
            keymap: "us".to_string(),
            time_sync: Toggle::Yes,
            mirror_country: "United States".to_string(),
            hostname: String::new(),
            username: String::new(),
            user_password: String::new(),
            root_password: String::new(),
            kernel: Kernel::Linux,
            gpu_drivers: GpuDriver::Auto,
            multilib: Toggle::Yes,
            additional_packages: String::new(),
            additional_aur_packages: String::new(),
            aur_helper: AurHelper::Paru,
            flatpak: Toggle::No,
            bootloader: Bootloader::Grub,
            os_prober: Toggle::Yes,
            grub_themes: Toggle::No,
            grub_theme_selection: GrubTheme::PolyDark,
            desktop_environment: DesktopEnvironment::None,
            display_manager: DisplayManager::None,
            plymouth: Toggle::Yes,
            plymouth_theme: PlymouthTheme::ArchGlow,
            numlock_on_boot: Toggle::Yes,
            git_repository: Toggle::No,
            git_repository_url: String::new(),
        }
    }
}

/// Convert from TUI Configuration to InstallationConfig
impl From<&crate::config::Configuration> for InstallationConfig {
    fn from(tui_config: &crate::config::Configuration) -> Self {
        use std::str::FromStr;

        // Helper closure to find a value by name
        let get_value = |name: &str| -> String {
            tui_config
                .options
                .iter()
                .find(|opt| opt.name == name)
                .map(|opt| opt.get_value())
                .unwrap_or_default()
        };

        // Helper to parse enum with fallback to default
        fn parse_or_default<T: FromStr + Default>(s: &str) -> T {
            T::from_str(s).unwrap_or_default()
        }

        Self {
            boot_mode: parse_or_default(&get_value("Boot Mode")),
            secure_boot: parse_or_default(&get_value("Secure Boot")),
            install_disk: get_value("Disk"),
            partitioning_strategy: parse_or_default(&get_value("Partitioning Strategy")),
            root_filesystem: parse_or_default(&get_value("Root Filesystem")),
            home_filesystem: parse_or_default(&get_value("Home Filesystem")),
            separate_home: parse_or_default(&get_value("Separate Home Partition")),
            encryption: parse_or_default(&get_value("Encryption")),
            swap: parse_or_default(&get_value("Swap")),
            swap_size: get_value("Swap Size"),
            btrfs_snapshots: parse_or_default(&get_value("Btrfs Snapshots")),
            btrfs_frequency: parse_or_default(&get_value("Btrfs Frequency")),
            btrfs_keep_count: get_value("Btrfs Keep Count").parse().unwrap_or(3),
            btrfs_assistant: parse_or_default(&get_value("Btrfs Assistant")),
            timezone_region: get_value("Timezone Region"),
            timezone: get_value("Timezone"),
            locale: get_value("Locale"),
            keymap: get_value("Keymap"),
            time_sync: parse_or_default(&get_value("Time Sync (NTP)")),
            mirror_country: get_value("Mirror Country"),
            hostname: get_value("Hostname"),
            username: get_value("Username"),
            user_password: get_value("User Password"),
            root_password: get_value("Root Password"),
            kernel: parse_or_default(&get_value("Kernel")),
            gpu_drivers: parse_or_default(&get_value("GPU Drivers")),
            multilib: parse_or_default(&get_value("Multilib")),
            additional_packages: get_value("Additional Pacman Packages"),
            additional_aur_packages: get_value("Additional AUR Packages"),
            aur_helper: parse_or_default(&get_value("AUR Helper")),
            flatpak: parse_or_default(&get_value("Flatpak")),
            bootloader: parse_or_default(&get_value("Bootloader")),
            os_prober: parse_or_default(&get_value("OS Prober")),
            grub_themes: parse_or_default(&get_value("GRUB Theme")),
            grub_theme_selection: parse_or_default(&get_value("GRUB Theme Selection")),
            desktop_environment: parse_or_default(&get_value("Desktop Environment")),
            display_manager: parse_or_default(&get_value("Display Manager")),
            plymouth: parse_or_default(&get_value("Plymouth")),
            plymouth_theme: parse_or_default(&get_value("Plymouth Theme")),
            numlock_on_boot: parse_or_default(&get_value("Numlock on Boot")),
            git_repository: parse_or_default(&get_value("Git Repository")),
            git_repository_url: get_value("Git Repository URL"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_config() -> InstallationConfig {
        InstallationConfig {
            install_disk: "/dev/sda".to_string(),
            partitioning_strategy: PartitionScheme::AutoSimple,
            root_filesystem: Filesystem::Ext4,
            encryption: AutoToggle::No,
            locale: "en_US.UTF-8".to_string(),
            timezone_region: "America".to_string(),
            timezone: "New_York".to_string(),
            hostname: "archtest".to_string(),
            username: "testuser".to_string(),
            user_password: "password123".to_string(),
            root_password: "rootpass".to_string(),
            bootloader: Bootloader::Grub,
            desktop_environment: DesktopEnvironment::Gnome,
            ..Default::default()
        }
    }

    #[test]
    fn test_installation_config_default() {
        let config = InstallationConfig::default();
        assert!(config.install_disk.is_empty());
        assert!(config.hostname.is_empty());
        assert!(config.username.is_empty());
        assert_eq!(config.boot_mode, BootMode::Auto);
        assert_eq!(config.root_filesystem, Filesystem::Ext4);
    }

    #[test]
    fn test_installation_config_to_env_vars() {
        let config = create_test_config();
        let env_vars = config.to_env_vars();

        assert!(env_vars.contains(&("INSTALL_DISK".to_string(), "/dev/sda".to_string())));
        assert!(env_vars.contains(&("SYSTEM_HOSTNAME".to_string(), "archtest".to_string())));
        assert!(env_vars.contains(&("MAIN_USERNAME".to_string(), "testuser".to_string())));
        assert!(env_vars.contains(&("ROOT_FILESYSTEM".to_string(), "ext4".to_string())));
    }

    #[test]
    fn test_save_and_load_json_config() {
        let config = create_test_config();

        // Create a temp file
        let mut temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        // Save config
        let json = serde_json::to_string_pretty(&config).unwrap();
        temp_file.write_all(json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        // Load config
        let loaded = InstallationConfig::load_from_file(&path);
        assert!(loaded.is_ok());
        let loaded = loaded.unwrap();

        assert_eq!(loaded.install_disk, config.install_disk);
        assert_eq!(loaded.hostname, config.hostname);
        assert_eq!(loaded.username, config.username);
        assert_eq!(loaded.root_filesystem, config.root_filesystem);
        assert_eq!(loaded.boot_mode, config.boot_mode);
    }

    #[test]
    fn test_load_nonexistent_file() {
        let result = InstallationConfig::load_from_file(std::path::Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_invalid_json() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"{ invalid json }").unwrap();
        temp_file.flush().unwrap();

        let result = InstallationConfig::load_from_file(temp_file.path());
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_valid_config() {
        let config = create_test_config();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validation_empty_disk() {
        let mut config = create_test_config();
        config.install_disk = String::new();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_validation_invalid_hostname() {
        let mut config = create_test_config();
        config.hostname = "1invalid".to_string(); // Starts with number
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_partition_scheme_features() {
        let config = InstallationConfig {
            partitioning_strategy: PartitionScheme::AutoRaidLuks,
            ..Default::default()
        };

        assert!(config.partitioning_strategy.requires_raid());
        assert!(config.partitioning_strategy.uses_encryption());
    }
}
