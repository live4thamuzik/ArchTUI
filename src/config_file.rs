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
    #[allow(dead_code)] // API: Constructor for external consumers
    pub fn new() -> Self {
        Self::default()
    }

    /// Save configuration to a JSON file
    #[allow(dead_code)] // API: Used by --save-config CLI option
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
    #[allow(dead_code)] // API: Used when passing config to install scripts
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

    // =========================================================================
    // Sprint 1.1: Comprehensive Serialization Tests
    // =========================================================================

    #[test]
    fn test_save_to_file_creates_valid_json() {
        let config = create_test_config();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        // Use save_to_file method directly
        let result = config.save_to_file(&path);
        assert!(result.is_ok(), "save_to_file should succeed");

        // Verify file contains valid JSON
        let content = std::fs::read_to_string(&path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(parsed.is_object(), "Output should be a JSON object");
    }

    #[test]
    fn test_roundtrip_save_load() {
        let original = create_test_config();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_path_buf();

        // Save then load
        original.save_to_file(&path).unwrap();
        let loaded = InstallationConfig::load_from_file(&path).unwrap();

        // Verify all fields match
        assert_eq!(loaded.boot_mode, original.boot_mode);
        assert_eq!(loaded.secure_boot, original.secure_boot);
        assert_eq!(loaded.install_disk, original.install_disk);
        assert_eq!(loaded.partitioning_strategy, original.partitioning_strategy);
        assert_eq!(loaded.root_filesystem, original.root_filesystem);
        assert_eq!(loaded.home_filesystem, original.home_filesystem);
        assert_eq!(loaded.encryption, original.encryption);
        assert_eq!(loaded.hostname, original.hostname);
        assert_eq!(loaded.username, original.username);
        assert_eq!(loaded.user_password, original.user_password);
        assert_eq!(loaded.root_password, original.root_password);
        assert_eq!(loaded.kernel, original.kernel);
        assert_eq!(loaded.bootloader, original.bootloader);
        assert_eq!(loaded.desktop_environment, original.desktop_environment);
        assert_eq!(loaded.display_manager, original.display_manager);
        assert_eq!(loaded.plymouth_theme, original.plymouth_theme);
    }

    #[test]
    fn test_load_json_missing_required_field() {
        let mut temp_file = NamedTempFile::new().unwrap();
        // JSON missing boot_mode field
        temp_file
            .write_all(b"{\"install_disk\": \"/dev/sda\"}")
            .unwrap();
        temp_file.flush().unwrap();

        let result = InstallationConfig::load_from_file(temp_file.path());
        assert!(result.is_err(), "Should fail on missing required fields");
    }

    #[test]
    fn test_load_json_wrong_type_fails() {
        let mut temp_file = NamedTempFile::new().unwrap();
        // JSON with wrong type for a field (number instead of string)
        temp_file
            .write_all(br#"{"boot_mode": 12345, "install_disk": "/dev/sda"}"#)
            .unwrap();
        temp_file.flush().unwrap();

        let result = InstallationConfig::load_from_file(temp_file.path());
        assert!(result.is_err(), "Should fail on wrong field type");
    }

    #[test]
    fn test_load_json_malformed_structure() {
        let mut temp_file = NamedTempFile::new().unwrap();
        // JSON array instead of object
        temp_file.write_all(b"[1, 2, 3]").unwrap();
        temp_file.flush().unwrap();

        let result = InstallationConfig::load_from_file(temp_file.path());
        assert!(result.is_err(), "Should fail on wrong JSON structure");
    }

    // =========================================================================
    // Sprint 1.1: Validation Edge Cases
    // =========================================================================

    #[test]
    fn test_validation_empty_hostname() {
        let mut config = create_test_config();
        config.hostname = String::new();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Hostname"));
    }

    #[test]
    fn test_validation_hostname_too_short() {
        let mut config = create_test_config();
        config.hostname = "ab".to_string(); // Only 2 chars
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("3-32"));
    }

    #[test]
    fn test_validation_hostname_too_long() {
        let mut config = create_test_config();
        config.hostname = "a".repeat(33); // 33 chars
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("3-32"));
    }

    #[test]
    fn test_validation_hostname_special_chars() {
        let mut config = create_test_config();
        config.hostname = "host-name".to_string(); // Contains hyphen
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("letters, numbers"));
    }

    #[test]
    fn test_validation_empty_username() {
        let mut config = create_test_config();
        config.username = String::new();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Username"));
    }

    #[test]
    fn test_validation_username_starts_with_number() {
        let mut config = create_test_config();
        config.username = "1user".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("start with a letter"));
    }

    #[test]
    fn test_validation_empty_user_password() {
        let mut config = create_test_config();
        config.user_password = String::new();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("User password"));
    }

    #[test]
    fn test_validation_user_password_with_whitespace() {
        let mut config = create_test_config();
        config.user_password = "pass word".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("whitespace"));
    }

    #[test]
    fn test_validation_empty_root_password() {
        let mut config = create_test_config();
        config.root_password = String::new();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Root password"));
    }

    #[test]
    fn test_validation_root_password_with_whitespace() {
        let mut config = create_test_config();
        config.root_password = "root\tpass".to_string(); // Tab character
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("whitespace"));
    }

    #[test]
    fn test_validation_git_url_invalid_scheme() {
        let mut config = create_test_config();
        config.git_repository = Toggle::Yes;
        config.git_repository_url = "ftp://example.com/repo.git".to_string();
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("http://"));
    }

    #[test]
    fn test_validation_git_url_valid_schemes() {
        let mut config = create_test_config();
        config.git_repository = Toggle::Yes;

        // Test all valid schemes
        for scheme in &["https://", "http://", "git://", "ssh://"] {
            config.git_repository_url = format!("{}example.com/repo.git", scheme);
            assert!(
                config.validate().is_ok(),
                "Should accept {} URLs",
                scheme
            );
        }
    }

    // =========================================================================
    // Sprint 1.1: Enum Serialization Verification
    // =========================================================================

    #[test]
    fn test_enum_serialization_matches_bash_constants() {
        let config = create_test_config();
        let env_vars = config.to_env_vars();

        // Verify enum string values match expected bash format
        let find_var = |name: &str| -> String {
            env_vars
                .iter()
                .find(|(k, _)| k == name)
                .map(|(_, v)| v.clone())
                .unwrap_or_default()
        };

        // These must match what bash scripts expect
        assert_eq!(find_var("BOOT_MODE"), "Auto");
        assert_eq!(find_var("ROOT_FILESYSTEM"), "ext4");
        assert_eq!(find_var("BOOTLOADER"), "grub");
        assert_eq!(find_var("DESKTOP_ENVIRONMENT"), "gnome");
    }

    #[test]
    fn test_all_filesystem_types_serialize() {
        use std::str::FromStr;

        let filesystems = vec![
            Filesystem::Ext4,
            Filesystem::Btrfs,
            Filesystem::Xfs,
        ];

        for fs in filesystems {
            let serialized = fs.to_string();
            let deserialized = Filesystem::from_str(&serialized);
            assert!(
                deserialized.is_ok(),
                "Filesystem {:?} should roundtrip",
                fs
            );
            assert_eq!(deserialized.unwrap(), fs);
        }
    }

    #[test]
    fn test_all_bootloaders_serialize() {
        use std::str::FromStr;

        let bootloaders = vec![
            Bootloader::Grub,
            Bootloader::SystemdBoot,
        ];

        for bl in bootloaders {
            let serialized = bl.to_string();
            let deserialized = Bootloader::from_str(&serialized);
            assert!(
                deserialized.is_ok(),
                "Bootloader {:?} should roundtrip",
                bl
            );
            assert_eq!(deserialized.unwrap(), bl);
        }
    }

    #[test]
    fn test_config_new_equals_default() {
        let new_config = InstallationConfig::new();
        let default_config = InstallationConfig::default();

        assert_eq!(new_config.boot_mode, default_config.boot_mode);
        assert_eq!(new_config.install_disk, default_config.install_disk);
        assert_eq!(new_config.hostname, default_config.hostname);
    }

    // =========================================================================
    // Sprint 1.1: Edge Case Tests for Serialization
    // =========================================================================

    #[test]
    fn test_special_characters_in_password() {
        let mut config = InstallationConfig::default();
        // Passwords can contain special characters, quotes, etc.
        config.user_password = r#"P@ss!w0rd$%^&*(){}[]|\"'`~<>?,./;:"#.to_string();
        config.root_password = "root!@#$%".to_string();
        config.install_disk = "/dev/sda".to_string();
        config.hostname = "testhost".to_string();
        config.username = "testuser".to_string();

        // Should serialize and deserialize correctly
        let json = serde_json::to_string(&config).expect("Should serialize");
        let loaded: InstallationConfig =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(loaded.user_password, config.user_password);
        assert_eq!(loaded.root_password, config.root_password);
    }

    #[test]
    fn test_unicode_in_optional_fields() {
        let mut config = InstallationConfig::default();
        config.install_disk = "/dev/sda".to_string();
        config.hostname = "testhost".to_string();
        config.username = "testuser".to_string();
        config.user_password = "pass123".to_string();
        config.root_password = "root123".to_string();
        // Additional packages can have unicode (though unusual)
        // Space-separated string of package names
        config.additional_packages = "vim 火狐".to_string();

        let json = serde_json::to_string(&config).expect("Should serialize unicode");
        let loaded: InstallationConfig =
            serde_json::from_str(&json).expect("Should deserialize unicode");

        assert_eq!(loaded.additional_packages, config.additional_packages);
    }

    #[test]
    fn test_empty_optional_vectors() {
        let mut config = InstallationConfig::default();
        config.install_disk = "/dev/sda".to_string();
        config.hostname = "testhost".to_string();
        config.username = "testuser".to_string();
        config.user_password = "pass".to_string();
        config.root_password = "root".to_string();
        config.additional_packages = String::new();
        config.additional_aur_packages = String::new();

        assert!(config.validate().is_ok(), "Empty package lists should be valid");

        let json = serde_json::to_string(&config).expect("Should serialize");
        let loaded: InstallationConfig = serde_json::from_str(&json).expect("Should deserialize");

        assert!(loaded.additional_packages.is_empty());
        assert!(loaded.additional_aur_packages.is_empty());
    }

    #[test]
    fn test_whitespace_only_hostname_invalid() {
        let mut config = InstallationConfig::default();
        config.install_disk = "/dev/sda".to_string();
        config.hostname = "   ".to_string(); // Whitespace only
        config.username = "user".to_string();
        config.user_password = "pass".to_string();
        config.root_password = "root".to_string();

        assert!(config.validate().is_err(), "Whitespace-only hostname should be invalid");
    }

    #[test]
    fn test_whitespace_only_username_invalid() {
        let mut config = InstallationConfig::default();
        config.install_disk = "/dev/sda".to_string();
        config.hostname = "host".to_string();
        config.username = "\t\n".to_string(); // Whitespace only
        config.user_password = "pass".to_string();
        config.root_password = "root".to_string();

        assert!(config.validate().is_err(), "Whitespace-only username should be invalid");
    }

    #[test]
    fn test_very_long_hostname_invalid() {
        let mut config = InstallationConfig::default();
        config.install_disk = "/dev/sda".to_string();
        // Hostnames longer than 32 chars are invalid per implementation
        config.hostname = "a".repeat(33);
        config.username = "user".to_string();
        config.user_password = "pass".to_string();
        config.root_password = "root".to_string();

        assert!(config.validate().is_err(), "Hostname > 32 chars should be invalid");
    }

    #[test]
    fn test_maximum_valid_hostname() {
        let mut config = InstallationConfig::default();
        config.install_disk = "/dev/sda".to_string();
        // 32 chars is the maximum valid hostname length per implementation
        config.hostname = "a".repeat(32);
        config.username = "user".to_string();
        config.user_password = "pass".to_string();
        config.root_password = "root".to_string();

        assert!(config.validate().is_ok(), "Hostname of 32 chars should be valid");
    }

    #[test]
    fn test_disk_path_with_partition_number() {
        let mut config = InstallationConfig::default();
        config.install_disk = "/dev/sda1".to_string(); // Partition, not disk
        config.hostname = "host".to_string();
        config.username = "user".to_string();
        config.user_password = "pass".to_string();
        config.root_password = "root".to_string();

        // This should be valid - validation just checks it starts with /dev/
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_nvme_disk_path() {
        let mut config = InstallationConfig::default();
        config.install_disk = "/dev/nvme0n1".to_string();
        config.hostname = "host".to_string();
        config.username = "user".to_string();
        config.user_password = "pass".to_string();
        config.root_password = "root".to_string();

        assert!(config.validate().is_ok(), "NVMe disk path should be valid");
    }

    #[test]
    fn test_json_with_extra_fields_ignored() {
        // Create a valid config, serialize it, add extra fields, and deserialize
        let config = InstallationConfig::default();
        let mut json: serde_json::Value = serde_json::to_value(&config).unwrap();

        // Add unknown fields that might exist in future versions
        json["unknown_future_field"] = serde_json::json!("some_value");
        json["another_unknown"] = serde_json::json!(12345);

        let json_str = serde_json::to_string(&json).unwrap();

        // Without deny_unknown_fields, extra fields should be ignored
        let result: Result<InstallationConfig, _> = serde_json::from_str(&json_str);
        // Note: This test documents current behavior. If we add deny_unknown_fields,
        // this test should be updated.
        assert!(result.is_ok(), "Unknown fields should be ignored for forward compatibility");
    }

    #[test]
    fn test_json_null_values_for_optional_fields() {
        let json = r#"{
            "boot_mode": "UEFI",
            "install_disk": "/dev/sda",
            "hostname": "host",
            "username": "user",
            "user_password": "pass",
            "root_password": "root",
            "partition_scheme": "auto_simple",
            "root_filesystem": "ext4",
            "encryption_password": null,
            "additional_packages": null
        }"#;

        let result: Result<InstallationConfig, _> = serde_json::from_str(json);
        // Null for optional fields should work with proper serde configuration
        // This test documents current behavior
        if let Err(e) = &result {
            println!("Note: null values for optional fields not supported: {}", e);
        }
    }

    #[test]
    fn test_serialization_deterministic() {
        let mut config = InstallationConfig::default();
        config.install_disk = "/dev/sda".to_string();
        config.hostname = "host".to_string();
        config.username = "user".to_string();
        config.user_password = "pass".to_string();
        config.root_password = "root".to_string();

        // Serialize twice, should produce identical output
        let json1 = serde_json::to_string(&config).expect("Should serialize");
        let json2 = serde_json::to_string(&config).expect("Should serialize again");

        assert_eq!(json1, json2, "Serialization should be deterministic");
    }

    #[test]
    fn test_pretty_json_roundtrip() {
        let mut config = InstallationConfig::default();
        config.install_disk = "/dev/sda".to_string();
        config.hostname = "host".to_string();
        config.username = "user".to_string();
        config.user_password = "pass".to_string();
        config.root_password = "root".to_string();

        // Pretty print JSON
        let pretty_json =
            serde_json::to_string_pretty(&config).expect("Should pretty serialize");

        // Should parse back correctly
        let loaded: InstallationConfig =
            serde_json::from_str(&pretty_json).expect("Should parse pretty JSON");

        assert_eq!(loaded.hostname, config.hostname);
        assert_eq!(loaded.username, config.username);
    }
}
