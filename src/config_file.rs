use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Installation configuration that can be saved/loaded
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallationConfig {
    pub boot_mode: String,
    pub install_disk: String,
    pub partitioning_strategy: String,
    pub root_filesystem: String,
    pub home_filesystem: String,
    pub separate_home: String,
    pub encryption: String,
    pub swap: String,
    pub swap_size: String,
    pub timezone_region: String,
    pub timezone: String,
    pub locale: String,
    pub keymap: String,
    pub hostname: String,
    pub username: String,
    pub user_password: String,
    pub root_password: String,
    pub mirror_country: String,
    pub bootloader: String,
    pub os_prober: String,
    pub desktop_environment: String,
    pub display_manager: String,
    pub additional_packages: String,
    pub additional_aur_packages: String,
    pub aur_helper: String,
    pub plymouth: String,
    pub plymouth_theme: String,
    pub grub_themes: String,
    pub grub_theme_selection: String,
    pub time_sync: String,
    pub git_repository: String,
    pub git_repository_url: String,
    pub numlock_on_boot: String,
    pub secure_boot: String,
}

impl InstallationConfig {
    /// Create a new empty configuration
    pub fn new() -> Self {
        Self {
            boot_mode: String::new(),
            install_disk: String::new(),
            partitioning_strategy: String::new(),
            root_filesystem: String::new(),
            home_filesystem: String::new(),
            separate_home: String::new(),
            encryption: String::new(),
            swap: String::new(),
            swap_size: String::new(),
            timezone_region: String::new(),
            timezone: String::new(),
            locale: String::new(),
            keymap: String::new(),
            hostname: String::new(),
            username: String::new(),
            user_password: String::new(),
            root_password: String::new(),
            mirror_country: String::new(),
            bootloader: String::new(),
            os_prober: String::new(),
            desktop_environment: String::new(),
            display_manager: String::new(),
            additional_packages: String::new(),
            additional_aur_packages: String::new(),
            aur_helper: String::new(),
            plymouth: String::new(),
            plymouth_theme: String::new(),
            grub_themes: String::new(),
            grub_theme_selection: String::new(),
            time_sync: String::new(),
            git_repository: String::new(),
            git_repository_url: String::new(),
            numlock_on_boot: String::new(),
            secure_boot: String::new(),
        }
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
        // Basic validation - ensure required fields are not empty
        if self.install_disk.trim().is_empty() {
            anyhow::bail!("Install disk must be specified");
        }

        if self.partitioning_strategy.trim().is_empty() {
            anyhow::bail!("Partitioning strategy must be specified");
        }

        // Validate hostname (3-32 chars, start with letter, alphanumeric + underscore)
        let hostname = self.hostname.trim();
        if hostname.is_empty() {
            anyhow::bail!("Hostname must be specified");
        }
        if hostname.len() < 3 || hostname.len() > 32 {
            anyhow::bail!("Hostname must be 3-32 characters long");
        }
        if !hostname.chars().next().unwrap().is_ascii_alphabetic() {
            anyhow::bail!("Hostname must start with a letter");
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
        if !username.chars().next().unwrap().is_ascii_alphabetic() {
            anyhow::bail!("Username must start with a letter");
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

        // Validate Git repository URL format
        if !self.git_repository_url.trim().is_empty() {
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

        Ok(())
    }

    /// Convert to environment variables for Bash scripts
    #[allow(dead_code)]
    pub fn to_env_vars(&self) -> Vec<(String, String)> {
        vec![
            ("BOOT_MODE".to_string(), self.boot_mode.clone()),
            ("INSTALL_DISK".to_string(), self.install_disk.clone()),
            (
                "PARTITIONING_STRATEGY".to_string(),
                self.partitioning_strategy.clone(),
            ),
            ("ROOT_FILESYSTEM".to_string(), self.root_filesystem.clone()),
            ("HOME_FILESYSTEM".to_string(), self.home_filesystem.clone()),
            ("SEPARATE_HOME".to_string(), self.separate_home.clone()),
            ("ENCRYPTION".to_string(), self.encryption.clone()),
            ("SWAP".to_string(), self.swap.clone()),
            ("SWAP_SIZE".to_string(), self.swap_size.clone()),
            ("TIMEZONE_REGION".to_string(), self.timezone_region.clone()),
            ("TIMEZONE".to_string(), self.timezone.clone()),
            ("LOCALE".to_string(), self.locale.clone()),
            ("KEYMAP".to_string(), self.keymap.clone()),
            ("HOSTNAME".to_string(), self.hostname.clone()),
            ("USERNAME".to_string(), self.username.clone()),
            ("USER_PASSWORD".to_string(), self.user_password.clone()),
            ("ROOT_PASSWORD".to_string(), self.root_password.clone()),
            ("MIRROR_COUNTRY".to_string(), self.mirror_country.clone()),
            ("BOOTLOADER".to_string(), self.bootloader.clone()),
            ("OS_PROBER".to_string(), self.os_prober.clone()),
            (
                "DESKTOP_ENVIRONMENT".to_string(),
                self.desktop_environment.clone(),
            ),
            ("DISPLAY_MANAGER".to_string(), self.display_manager.clone()),
            (
                "ADDITIONAL_PACKAGES".to_string(),
                self.additional_packages.clone(),
            ),
            (
                "ADDITIONAL_AUR_PACKAGES".to_string(),
                self.additional_aur_packages.clone(),
            ),
            ("AUR_HELPER".to_string(), self.aur_helper.clone()),
            ("PLYMOUTH".to_string(), self.plymouth.clone()),
            ("PLYMOUTH_THEME".to_string(), self.plymouth_theme.clone()),
            ("GRUB_THEMES".to_string(), self.grub_themes.clone()),
            (
                "GRUB_THEME_SELECTION".to_string(),
                self.grub_theme_selection.clone(),
            ),
            ("TIME_SYNC".to_string(), self.time_sync.clone()),
            ("GIT_REPOSITORY".to_string(), self.git_repository.clone()),
            (
                "GIT_REPOSITORY_URL".to_string(),
                self.git_repository_url.clone(),
            ),
            ("NUMLOCK_ON_BOOT".to_string(), self.numlock_on_boot.clone()),
            ("SECURE_BOOT".to_string(), self.secure_boot.clone()),
        ]
    }
}

impl Default for InstallationConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert from TUI Configuration to InstallationConfig
impl From<&crate::config::Configuration> for InstallationConfig {
    fn from(tui_config: &crate::config::Configuration) -> Self {
        let mut file_config = InstallationConfig::new();

        // Helper closure to find a value by name
        let get_value = |name: &str| -> String {
            tui_config.options.iter()
                .find(|opt| opt.name == name)
                .map(|opt| opt.get_value())
                .unwrap_or_default()
        };

        // Map all fields from the TUI config to the file config
        file_config.boot_mode = get_value("Boot Mode");
        file_config.secure_boot = get_value("Secure Boot");
        file_config.locale = get_value("Locale");
        file_config.keymap = get_value("Keymap");
        file_config.install_disk = get_value("Disk");
        file_config.partitioning_strategy = get_value("Partitioning Strategy");
        file_config.root_filesystem = get_value("Root Filesystem");
        file_config.home_filesystem = get_value("Home Filesystem");
        file_config.separate_home = get_value("Separate Home");
        file_config.encryption = get_value("Encryption");
        file_config.swap = get_value("Swap");
        file_config.swap_size = get_value("Swap Size");
        file_config.timezone_region = get_value("Timezone Region");
        file_config.timezone = get_value("Timezone");
        file_config.hostname = get_value("Hostname");
        file_config.username = get_value("Username");
        file_config.user_password = get_value("User Password");
        file_config.root_password = get_value("Root Password");
        file_config.mirror_country = get_value("Mirror Country");
        file_config.bootloader = get_value("Bootloader");
        file_config.os_prober = get_value("OS Prober");
        file_config.desktop_environment = get_value("Desktop Environment");
        file_config.display_manager = get_value("Display Manager");
        file_config.additional_packages = get_value("Additional Packages");
        file_config.additional_aur_packages = get_value("Additional AUR Packages");
        file_config.aur_helper = get_value("AUR Helper");
        file_config.plymouth = get_value("Plymouth");
        file_config.plymouth_theme = get_value("Plymouth Theme");
        file_config.grub_themes = get_value("GRUB Themes");
        file_config.grub_theme_selection = get_value("GRUB Theme Selection");
        file_config.time_sync = get_value("Time Sync");
        file_config.git_repository = get_value("Git Repository");
        file_config.git_repository_url = get_value("Git Repository URL");
        file_config.numlock_on_boot = get_value("Numlock on Boot");
        
        file_config
    }
}
