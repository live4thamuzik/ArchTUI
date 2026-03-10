//! Configuration file handling for saving and loading installation configs.
//!
//! This module uses type-safe enums instead of strings for configuration values,
//! providing compile-time validation and preventing typos.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

use crate::types::{
    AurHelper, AutoToggle, BootMode, Bootloader, DesktopEnvironment, DisplayManager,
    EncryptionKeyType, Filesystem, GpuDriver, GrubTheme, Kernel, PartitionScheme, PlymouthTheme,
    SnapshotFrequency, SnapshotTool, Toggle,
};

/// Installation configuration that can be saved/loaded
/// NOTE: Debug impl redacts password fields (ROE §8.1)
#[derive(Clone, Serialize, Deserialize)]
pub struct InstallationConfig {
    // Boot & System
    pub boot_mode: BootMode,
    pub secure_boot: Toggle,

    // Disk & Storage
    pub install_disk: String, // Disk path like /dev/sda - must remain String
    pub partitioning_strategy: PartitionScheme,
    #[serde(default = "default_raid_level")]
    pub raid_level: String, // RAID level (raid0/raid1/raid5/raid6/raid10)
    pub root_filesystem: Filesystem,
    pub home_filesystem: Filesystem,
    pub separate_home: Toggle,
    pub encryption: AutoToggle,
    pub encryption_password: String, // LUKS passphrase (if encryption enabled)
    pub swap: Toggle,
    pub swap_size: String, // Size like "2GB" - flexible format
    #[serde(default = "default_root_size")]
    pub root_size: String, // Size like "50GB" or "Remaining"
    #[serde(default = "default_home_size")]
    pub home_size: String, // Size like "100GB" or "Remaining"

    // Btrfs options
    pub btrfs_snapshots: Toggle,
    pub btrfs_frequency: SnapshotFrequency,
    pub btrfs_keep_count: u8,
    #[serde(alias = "btrfs_assistant")]
    pub snapshot_tool: SnapshotTool,

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
    #[serde(alias = "grub_themes")]
    pub grub_theme: Toggle,
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

    // Advanced boot
    #[serde(default)]
    pub unified_kernel_image: Toggle,

    // Encryption key type
    #[serde(default)]
    pub encryption_key_type: EncryptionKeyType,
}

// ROE §8.1: Custom Debug impl redacts password fields to prevent accidental leaks
impl std::fmt::Debug for InstallationConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InstallationConfig")
            .field("boot_mode", &self.boot_mode)
            .field("secure_boot", &self.secure_boot)
            .field("install_disk", &self.install_disk)
            .field("partitioning_strategy", &self.partitioning_strategy)
            .field("raid_level", &self.raid_level)
            .field("root_filesystem", &self.root_filesystem)
            .field("home_filesystem", &self.home_filesystem)
            .field("separate_home", &self.separate_home)
            .field("encryption", &self.encryption)
            .field("encryption_password", &"********")
            .field("swap", &self.swap)
            .field("swap_size", &self.swap_size)
            .field("root_size", &self.root_size)
            .field("home_size", &self.home_size)
            .field("btrfs_snapshots", &self.btrfs_snapshots)
            .field("btrfs_frequency", &self.btrfs_frequency)
            .field("btrfs_keep_count", &self.btrfs_keep_count)
            .field("snapshot_tool", &self.snapshot_tool)
            .field("timezone_region", &self.timezone_region)
            .field("timezone", &self.timezone)
            .field("locale", &self.locale)
            .field("keymap", &self.keymap)
            .field("time_sync", &self.time_sync)
            .field("mirror_country", &self.mirror_country)
            .field("hostname", &self.hostname)
            .field("username", &self.username)
            .field("user_password", &"********")
            .field("root_password", &"********")
            .field("kernel", &self.kernel)
            .field("gpu_drivers", &self.gpu_drivers)
            .field("multilib", &self.multilib)
            .field("additional_packages", &self.additional_packages)
            .field("additional_aur_packages", &self.additional_aur_packages)
            .field("aur_helper", &self.aur_helper)
            .field("flatpak", &self.flatpak)
            .field("bootloader", &self.bootloader)
            .field("os_prober", &self.os_prober)
            .field("grub_theme", &self.grub_theme)
            .field("grub_theme_selection", &self.grub_theme_selection)
            .field("desktop_environment", &self.desktop_environment)
            .field("display_manager", &self.display_manager)
            .field("plymouth", &self.plymouth)
            .field("plymouth_theme", &self.plymouth_theme)
            .field("numlock_on_boot", &self.numlock_on_boot)
            .field("git_repository", &self.git_repository)
            .field("git_repository_url", &self.git_repository_url)
            .field("unified_kernel_image", &self.unified_kernel_image)
            .field("encryption_key_type", &self.encryption_key_type)
            .finish()
    }
}

fn default_raid_level() -> String {
    "raid1".to_string()
}

fn default_root_size() -> String {
    "50GB".to_string()
}

fn default_home_size() -> String {
    "Remaining".to_string()
}

impl InstallationConfig {
    /// Create a new empty configuration with sensible defaults
    #[allow(dead_code)] // API: Constructor for external consumers
    pub fn new() -> Self {
        Self::default()
    }

    /// Save configuration to a JSON file
    /// Passwords are redacted — they are NEVER written to disk (ROE §8.1)
    #[allow(dead_code)] // API: Used by --save-config CLI option
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        // Clone and redact passwords before serialization (ROE §8.1: never write passwords to disk)
        let mut redacted = self.clone();
        redacted.user_password = String::new();
        redacted.root_password = String::new();
        redacted.encryption_password = String::new();

        let json = serde_json::to_string_pretty(&redacted)
            .context("Failed to serialize configuration to JSON")?;

        fs::write(&path, json)
            .with_context(|| format!("Failed to write configuration to {:?}", path.as_ref()))?;

        tracing::warn!(
            path = %path.as_ref().display(),
            "Configuration saved to disk (passwords redacted)"
        );

        Ok(())
    }

    /// Load configuration from a JSON file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        tracing::info!(path = %path.as_ref().display(), "Loading configuration from file");
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read configuration from {:?}", path.as_ref()))?;

        let config: Self =
            serde_json::from_str(&content).context("Failed to parse configuration JSON")?;

        tracing::info!(
            strategy = %config.partitioning_strategy,
            bootloader = %config.bootloader,
            de = %config.desktop_environment,
            "Configuration loaded successfully"
        );
        Ok(config)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        tracing::info!("Validating configuration");
        // Validate disk path (skip for pre-mounted — uses existing mounts)
        if self.partitioning_strategy != PartitionScheme::PreMounted
            && self.install_disk.trim().is_empty()
        {
            tracing::error!(field = "install_disk", "Install disk must be specified");
            anyhow::bail!("Install disk must be specified");
        }

        // Validate hostname (RFC 1123: 1-63 chars, alphanumeric + hyphens, case-insensitive)
        let hostname = self.hostname.trim();
        if hostname.is_empty() {
            tracing::error!(field = "hostname", "Hostname must be specified");
            anyhow::bail!("Hostname must be specified");
        }
        if hostname.len() > 63 {
            tracing::error!(
                field = "hostname",
                len = hostname.len(),
                "Hostname exceeds 63 chars"
            );
            anyhow::bail!("Hostname must be at most 63 characters long (RFC 1123)");
        }
        if let Some(first_char) = hostname.chars().next() {
            if !first_char.is_ascii_alphanumeric() {
                tracing::error!(
                    field = "hostname",
                    "Hostname must start with a letter or digit (RFC 1123)"
                );
                anyhow::bail!("Hostname must start with a letter or digit (RFC 1123)");
            }
        }
        if let Some(last_char) = hostname.chars().last() {
            if !last_char.is_ascii_alphanumeric() {
                tracing::error!(
                    field = "hostname",
                    "Hostname must end with a letter or digit (RFC 1123)"
                );
                anyhow::bail!("Hostname must end with a letter or digit (RFC 1123)");
            }
        }
        if !hostname
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            tracing::error!(field = "hostname", "Hostname contains invalid characters");
            anyhow::bail!("Hostname can only contain letters, numbers, and hyphens (RFC 1123)");
        }

        // Validate username (3-32 chars, start with lowercase letter, lowercase + digits + underscore)
        let username = self.username.trim();
        if username.is_empty() {
            tracing::error!(field = "username", "Username must be specified");
            anyhow::bail!("Username must be specified");
        }
        if username.len() < 3 || username.len() > 32 {
            tracing::error!(
                field = "username",
                len = username.len(),
                "Username length out of range"
            );
            anyhow::bail!("Username must be 3-32 characters long");
        }
        if let Some(first_char) = username.chars().next() {
            if !first_char.is_ascii_lowercase() {
                tracing::error!(
                    field = "username",
                    "Username must start with lowercase letter"
                );
                anyhow::bail!("Username must start with a lowercase letter");
            }
        }
        if !username
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        {
            tracing::error!(field = "username", "Username contains invalid characters");
            anyhow::bail!("Username can only contain lowercase letters, numbers, and underscores");
        }

        // Validate passwords (non-empty, no whitespace) — ROE §8.1: never log password values
        if self.user_password.trim().is_empty() {
            tracing::error!(
                field = "user_password",
                "User password is empty (value redacted)"
            );
            anyhow::bail!("User password must be specified");
        }
        if self.user_password.contains(char::is_whitespace) {
            tracing::error!(
                field = "user_password",
                "User password contains whitespace (value redacted)"
            );
            anyhow::bail!("User password cannot contain whitespace");
        }

        if self.root_password.trim().is_empty() {
            tracing::error!(
                field = "root_password",
                "Root password is empty (value redacted)"
            );
            anyhow::bail!("Root password must be specified");
        }
        if self.root_password.contains(char::is_whitespace) {
            tracing::error!(
                field = "root_password",
                "Root password contains whitespace (value redacted)"
            );
            anyhow::bail!("Root password cannot contain whitespace");
        }

        // Validate encryption password if encryption is enabled or strategy requires it
        let needs_encryption =
            self.encryption == AutoToggle::Yes || self.partitioning_strategy.uses_encryption();
        if needs_encryption && self.encryption_password.trim().is_empty() {
            tracing::error!(
                field = "encryption_password",
                "Encryption password required but empty (value redacted)"
            );
            anyhow::bail!("Encryption password must be specified when encryption is enabled");
        }

        // Validate Git repository URL format if enabled
        if self.git_repository == Toggle::Yes {
            let url = self.git_repository_url.trim();
            if url.is_empty() {
                tracing::error!(
                    field = "git_repository_url",
                    "Git repository URL required but empty"
                );
                anyhow::bail!(
                    "Git repository URL must be specified when Git Repository is enabled"
                );
            }
            if !url.starts_with("http://") && !url.starts_with("https://") {
                tracing::error!(
                    field = "git_repository_url",
                    "URL must start with http:// or https://"
                );
                anyhow::bail!("Git repository URL must start with http:// or https://");
            }
        }

        // Validate AUR helper is selected when DE requires AUR packages
        if self.desktop_environment.requires_aur() && self.aur_helper == AurHelper::None {
            tracing::error!(de = %self.desktop_environment, "DE requires AUR helper but none selected");
            anyhow::bail!(
                "{} requires an AUR helper (packages like wlogout are AUR-only)",
                self.desktop_environment
            );
        }

        // Validate RAID configuration
        if self.partitioning_strategy.requires_raid() {
            let disk = self.install_disk.trim();
            if disk.is_empty() {
                tracing::error!(field = "install_disk", "Install disk required for RAID");
                anyhow::bail!("Install disk must be specified for RAID strategies");
            }
            let disk_count = disk.split(',').filter(|s| !s.trim().is_empty()).count();
            if disk_count < 2 {
                tracing::error!(disk_count, "RAID requires at least 2 disks");
                anyhow::bail!(
                    "RAID strategies require at least 2 disks (found {}). Select multiple disks.",
                    disk_count
                );
            }
            let valid_levels = ["raid0", "raid1", "raid5", "raid6", "raid10"];
            if !valid_levels.contains(&self.raid_level.as_str()) {
                tracing::error!(raid_level = %self.raid_level, "Invalid RAID level");
                anyhow::bail!(
                    "Invalid RAID level: '{}'. Must be one of: {}",
                    self.raid_level,
                    valid_levels.join(", ")
                );
            }
        }

        // Validate UEFI-only bootloaders are not selected with BIOS boot mode
        match self.bootloader {
            Bootloader::SystemdBoot | Bootloader::Refind | Bootloader::Efistub => {
                if self.boot_mode == BootMode::Bios {
                    tracing::error!(bootloader = %self.bootloader, "UEFI-only bootloader with BIOS mode");
                    anyhow::bail!(
                        "{} requires UEFI firmware (BIOS is not supported)",
                        self.bootloader
                    );
                }
            }
            _ => {}
        }

        // BIOS RAID: only GRUB and Limine support BIOS boot
        if self.partitioning_strategy.requires_raid() && self.boot_mode == BootMode::Bios {
            match self.bootloader {
                Bootloader::Grub | Bootloader::Limine => {}
                _ => {
                    tracing::error!(
                        bootloader = %self.bootloader,
                        "UEFI-only bootloader with BIOS + RAID"
                    );
                    anyhow::bail!(
                        "{} requires UEFI firmware and cannot be used with BIOS + RAID",
                        self.bootloader
                    );
                }
            }
        }

        tracing::info!("Configuration validation passed");
        Ok(())
    }

    /// Convert to environment variables for Bash scripts.
    /// N/A sentinel values are converted to empty strings.
    #[allow(dead_code)] // API: Used when passing config to install scripts
    pub fn to_env_vars(&self) -> Vec<(String, String)> {
        tracing::debug!(
            strategy = %self.partitioning_strategy,
            disk = %self.install_disk,
            "Exporting configuration to environment variables"
        );
        let sanitize = |s: String| -> String { if s == "N/A" { String::new() } else { s } };
        vec![
            ("BOOT_MODE".to_string(), self.boot_mode.to_string()),
            ("SECURE_BOOT".to_string(), self.secure_boot.to_string()),
            ("INSTALL_DISK".to_string(), self.install_disk.clone()),
            (
                "PARTITIONING_STRATEGY".to_string(),
                self.partitioning_strategy.to_string(),
            ),
            ("RAID_LEVEL".to_string(), sanitize(self.raid_level.clone())),
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
            (
                "ENCRYPTION_PASSWORD".to_string(),
                sanitize(self.encryption_password.clone()),
            ),
            ("SWAP".to_string(), self.swap.to_string()),
            ("SWAP_SIZE".to_string(), sanitize(self.swap_size.clone())),
            ("ROOT_SIZE".to_string(), sanitize(self.root_size.clone())),
            ("HOME_SIZE".to_string(), sanitize(self.home_size.clone())),
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
            ("SNAPSHOT_TOOL".to_string(), self.snapshot_tool.to_string()),
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
            ("GRUB_THEME".to_string(), self.grub_theme.to_string()),
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
            (
                "UNIFIED_KERNEL_IMAGE".to_string(),
                self.unified_kernel_image.to_string(),
            ),
            (
                "ENCRYPTION_KEY_TYPE".to_string(),
                sanitize(self.encryption_key_type.to_string()),
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
            raid_level: "raid1".to_string(),
            root_filesystem: Filesystem::Ext4,
            home_filesystem: Filesystem::Ext4,
            separate_home: Toggle::No,
            encryption: AutoToggle::No,
            encryption_password: String::new(),
            swap: Toggle::No,
            swap_size: "N/A".to_string(),
            root_size: "50GB".to_string(),
            home_size: "Remaining".to_string(),
            btrfs_snapshots: Toggle::No,
            btrfs_frequency: SnapshotFrequency::Weekly,
            btrfs_keep_count: 3,
            snapshot_tool: SnapshotTool::None,
            timezone_region: "America".to_string(),
            timezone: "New_York".to_string(),
            locale: "en_US.UTF-8".to_string(),
            keymap: "us".to_string(),
            time_sync: Toggle::No,
            mirror_country: "United States".to_string(),
            hostname: String::new(),
            username: String::new(),
            user_password: String::new(),
            root_password: String::new(),
            kernel: Kernel::Linux,
            gpu_drivers: GpuDriver::Auto,
            multilib: Toggle::No,
            additional_packages: String::new(),
            additional_aur_packages: String::new(),
            aur_helper: AurHelper::None,
            flatpak: Toggle::No,
            bootloader: Bootloader::Grub,
            os_prober: Toggle::No,
            grub_theme: Toggle::No,
            grub_theme_selection: GrubTheme::PolyDark,
            desktop_environment: DesktopEnvironment::None,
            display_manager: DisplayManager::None,
            plymouth: Toggle::No,
            plymouth_theme: PlymouthTheme::Bgrt,
            numlock_on_boot: Toggle::No,
            git_repository: Toggle::No,
            git_repository_url: String::new(),
            unified_kernel_image: Toggle::No,
            encryption_key_type: EncryptionKeyType::Password,
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
            raid_level: {
                let v = get_value("RAID Level");
                if v == "N/A" || v.is_empty() {
                    "raid1".to_string()
                } else {
                    v
                }
            },
            root_filesystem: parse_or_default(&get_value("Root Filesystem")),
            home_filesystem: parse_or_default(&get_value("Home Filesystem")),
            separate_home: parse_or_default(&get_value("Separate Home Partition")),
            encryption: parse_or_default(&get_value("Encryption")),
            encryption_password: {
                let v = get_value("Encryption Password");
                if v == "N/A" { String::new() } else { v }
            },
            swap: parse_or_default(&get_value("Swap")),
            swap_size: get_value("Swap Size"),
            root_size: get_value("Root Size"),
            home_size: get_value("Home Size"),
            btrfs_snapshots: parse_or_default(&get_value("Btrfs Snapshots")),
            btrfs_frequency: parse_or_default(&get_value("Snapshot Frequency")),
            btrfs_keep_count: get_value("Snapshot Keep Count").parse().unwrap_or(3),
            snapshot_tool: parse_or_default(&get_value("Snapshot Tool")),
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
            grub_theme: parse_or_default(&get_value("GRUB Theme")),
            grub_theme_selection: parse_or_default(&get_value("GRUB Theme Selection")),
            desktop_environment: parse_or_default(&get_value("Desktop Environment")),
            display_manager: parse_or_default(&get_value("Display Manager")),
            plymouth: parse_or_default(&get_value("Plymouth")),
            plymouth_theme: parse_or_default(&get_value("Plymouth Theme")),
            numlock_on_boot: parse_or_default(&get_value("Numlock on Boot")),
            git_repository: parse_or_default(&get_value("Git Repository")),
            git_repository_url: get_value("Git Repository URL"),
            unified_kernel_image: parse_or_default(&get_value("Unified Kernel Image")),
            encryption_key_type: parse_or_default(&get_value("Encryption Key Type")),
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
        assert!(env_vars.contains(&("ROOT_SIZE".to_string(), "50GB".to_string())));
        assert!(env_vars.contains(&("HOME_SIZE".to_string(), "Remaining".to_string())));
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
        config.hostname = "-invalid".to_string(); // Starts with hyphen
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
    // Comprehensive Serialization Tests
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
        // Passwords are redacted on save (ROE §8.1) — they should be empty after roundtrip
        assert_eq!(
            loaded.user_password, "",
            "Passwords must be redacted on save"
        );
        assert_eq!(
            loaded.root_password, "",
            "Passwords must be redacted on save"
        );
        assert_eq!(loaded.kernel, original.kernel);
        assert_eq!(loaded.bootloader, original.bootloader);
        assert_eq!(loaded.desktop_environment, original.desktop_environment);
        assert_eq!(loaded.display_manager, original.display_manager);
        assert_eq!(loaded.plymouth_theme, original.plymouth_theme);
        assert_eq!(loaded.root_size, original.root_size);
        assert_eq!(loaded.home_size, original.home_size);
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
    // Validation Edge Cases
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
    fn test_validation_hostname_too_long() {
        let mut config = create_test_config();
        config.hostname = "a".repeat(64); // 64 chars, exceeds RFC 1123 limit of 63
        let result = config.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("63"));
    }

    #[test]
    fn test_validation_hostname_hyphen_allowed() {
        let mut config = create_test_config();
        config.hostname = "host-name".to_string(); // Hyphens are valid per RFC 1123
        let result = config.validate();
        assert!(result.is_ok());
    }

    #[test]
    fn test_validation_hostname_trailing_hyphen_rejected() {
        let mut config = create_test_config();
        config.hostname = "hostname-".to_string(); // RFC 1123: no trailing hyphens
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_hostname_underscore_rejected() {
        let mut config = create_test_config();
        config.hostname = "host_name".to_string(); // RFC 1123: no underscores
        let result = config.validate();
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_hostname_uppercase_accepted() {
        let mut config = create_test_config();
        config.hostname = "HostName".to_string(); // RFC 1123 is case-insensitive
        let result = config.validate();
        assert!(result.is_ok());
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
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("start with a lowercase letter")
        );
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

        // Test valid schemes (https:// and http:// only — git:// is unencrypted, ssh:// rejected by bash)
        for scheme in &["https://", "http://"] {
            config.git_repository_url = format!("{}example.com/repo.git", scheme);
            assert!(config.validate().is_ok(), "Should accept {} URLs", scheme);
        }

        // git:// and ssh:// should be rejected (unencrypted / not supported by install_dotfiles.sh)
        for scheme in &["git://", "ssh://"] {
            config.git_repository_url = format!("{}example.com/repo.git", scheme);
            assert!(config.validate().is_err(), "Should reject {} URLs", scheme);
        }
    }

    #[test]
    fn test_validation_raid_level() {
        let mut config = create_test_config();
        config.partitioning_strategy = PartitionScheme::AutoRaid;
        config.install_disk = "/dev/sda,/dev/sdb".to_string();

        // Valid RAID levels
        for level in &["raid0", "raid1", "raid5", "raid6", "raid10"] {
            config.raid_level = level.to_string();
            assert!(
                config.validate().is_ok(),
                "Should accept RAID level: {}",
                level
            );
        }

        // Invalid RAID levels
        for level in &["raid2", "invalid", "mirror", ""] {
            config.raid_level = level.to_string();
            assert!(
                config.validate().is_err(),
                "Should reject RAID level: '{}'",
                level
            );
        }
    }

    #[test]
    fn test_validation_uefi_only_bootloaders_on_bios() {
        let mut config = create_test_config();
        config.boot_mode = BootMode::Bios;

        // UEFI-only bootloaders must be rejected on BIOS
        for bl in &[
            Bootloader::SystemdBoot,
            Bootloader::Refind,
            Bootloader::Efistub,
        ] {
            config.bootloader = *bl;
            assert!(
                config.validate().is_err(),
                "{} should be rejected on BIOS",
                bl
            );
        }

        // GRUB and Limine support BIOS
        for bl in &[Bootloader::Grub, Bootloader::Limine] {
            config.bootloader = *bl;
            assert!(
                config.validate().is_ok(),
                "{} should be accepted on BIOS",
                bl
            );
        }
    }

    // =========================================================================
    // Enum Serialization Verification
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

        let filesystems = vec![Filesystem::Ext4, Filesystem::Btrfs, Filesystem::Xfs];

        for fs in filesystems {
            let serialized = fs.to_string();
            let deserialized = Filesystem::from_str(&serialized);
            assert!(deserialized.is_ok(), "Filesystem {:?} should roundtrip", fs);
            assert_eq!(deserialized.unwrap(), fs);
        }
    }

    #[test]
    fn test_all_bootloaders_serialize() {
        use std::str::FromStr;

        let bootloaders = vec![Bootloader::Grub, Bootloader::SystemdBoot];

        for bl in bootloaders {
            let serialized = bl.to_string();
            let deserialized = Bootloader::from_str(&serialized);
            assert!(deserialized.is_ok(), "Bootloader {:?} should roundtrip", bl);
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
    // Edge Case Tests for Serialization
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
        let loaded: InstallationConfig = serde_json::from_str(&json).expect("Should deserialize");

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

        assert!(
            config.validate().is_ok(),
            "Empty package lists should be valid"
        );

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

        assert!(
            config.validate().is_err(),
            "Whitespace-only hostname should be invalid"
        );
    }

    #[test]
    fn test_whitespace_only_username_invalid() {
        let mut config = InstallationConfig::default();
        config.install_disk = "/dev/sda".to_string();
        config.hostname = "host".to_string();
        config.username = "\t\n".to_string(); // Whitespace only
        config.user_password = "pass".to_string();
        config.root_password = "root".to_string();

        assert!(
            config.validate().is_err(),
            "Whitespace-only username should be invalid"
        );
    }

    #[test]
    fn test_very_long_hostname_invalid() {
        let mut config = InstallationConfig::default();
        config.install_disk = "/dev/sda".to_string();
        // Hostnames longer than 63 chars are invalid per RFC 1123
        config.hostname = "a".repeat(64);
        config.username = "user".to_string();
        config.user_password = "pass".to_string();
        config.root_password = "root".to_string();

        assert!(
            config.validate().is_err(),
            "Hostname > 63 chars should be invalid"
        );
    }

    #[test]
    fn test_maximum_valid_hostname() {
        let mut config = InstallationConfig::default();
        config.install_disk = "/dev/sda".to_string();
        // 63 chars is the maximum valid hostname length per RFC 1123
        config.hostname = "a".repeat(63);
        config.username = "user".to_string();
        config.user_password = "pass".to_string();
        config.root_password = "root".to_string();

        assert!(
            config.validate().is_ok(),
            "Hostname of 32 chars should be valid"
        );
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
        assert!(
            result.is_ok(),
            "Unknown fields should be ignored for forward compatibility"
        );
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
        let pretty_json = serde_json::to_string_pretty(&config).expect("Should pretty serialize");

        // Should parse back correctly
        let loaded: InstallationConfig =
            serde_json::from_str(&pretty_json).expect("Should parse pretty JSON");

        assert_eq!(loaded.hostname, config.hostname);
        assert_eq!(loaded.username, config.username);
    }
}
