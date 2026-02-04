//! Installer module
//!
//! Handles the execution of the bash installation script and communication with the TUI.
//!
//! # Type-Safe Disk Operations
//!
//! The `prepare_disks` function orchestrates disk operations using typed argument structs:
//! - `WipeDiskArgs` - Securely wipe the target disk
//! - `FormatPartitionArgs` - Format partitions with type-safe filesystem enum
//! - `MountPartitionArgs` - Mount partitions in correct order (root first, then boot)
//!
//! All operations go through `run_script_safe` which enforces process group isolation.
//!
//! # Base System Installation (Sprint 5)
//!
//! The `install_base_system` function uses the ALPM bindings to install packages
//! directly, replacing shell-based pacstrap calls. This provides full transparency
//! via log callbacks.

use crate::app::AppState;
use crate::config::Configuration;
#[cfg(feature = "alpm")]
use crate::package_manager::PackageManager;
use crate::process_guard::CommandProcessGroup;
use crate::script_runner::run_script_safe;
use crate::script_traits::ScriptArgs;
use crate::scripts::config::{GenFstabArgs, LocaleArgs, UserAddArgs};
use crate::scripts::disk::{
    FormatPartitionArgs, MountPartitionArgs, WipeDiskArgs, WipeMethod,
};
use crate::scripts::encryption::{LuksCipher, LuksFormatArgs, LuksOpenArgs, SecretFile};
use crate::scripts::network::{CheckConnectivityArgs, MirrorSortMethod, UpdateMirrorsArgs};
use crate::scripts::profiles::InstallDotfilesArgs;
#[cfg(feature = "alpm")]
use crate::scripts::profiles::EnableServicesArgs;
use crate::profiles::DotfilesConfig;
#[cfg(feature = "alpm")]
use crate::profiles::Profile;
use crate::types::Filesystem;
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
#[cfg(feature = "alpm")]
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

/// Installer instance
pub struct Installer {
    config: Configuration,
    app_state: Arc<Mutex<AppState>>,
}

impl Installer {
    /// Create a new installer instance
    pub fn new(config: Configuration, app_state: Arc<Mutex<AppState>>) -> Self {
        Self { config, app_state }
    }

    /// Validate the installation configuration
    fn validate_configuration(&self) -> bool {
        self.config.options.iter().all(|option| option.is_valid())
    }

    /// Start the installation process
    pub fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate configuration before starting
        if !self.validate_configuration() {
            return Err("Configuration validation failed".into());
        }

        // Update app state to installation mode
        {
            let mut state = self.app_state.lock().unwrap();
            state.mode = crate::app::AppMode::Installation;
            state.status_message = "Starting installation...".to_string();
            state.installation_progress = 10;

            // Add initial debug output
            state
                .installer_output
                .push("=== INSTALLATION ENGINE STARTED ===".to_string());
            state
                .installer_output
                .push("Script: scripts/install.sh".to_string());
            state.installer_output.push("Mode: TUI-only".to_string());
            state
                .installer_output
                .push("==========================================".to_string());
        }

        // Prepare environment variables (includes passwords)
        // Passwords are passed via environment because lint rules forbid `read` in bash
        let env_vars = self.config.to_env_vars();

        // Determine script path - use wrapper for TUI-friendly output
        let script_path = std::env::var("ARCHTUI_SCRIPTS_DIR")
            .map(|dir| format!("{}/install_wrapper.sh", dir))
            .unwrap_or_else(|_| "./scripts/install_wrapper.sh".to_string());

        // Launch the installation script
        // stdin is null - scripts are non-interactive per lint rules
        let mut child = Command::new("bash")
            .arg(&script_path)
            .envs(&env_vars)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .in_new_process_group()
            .spawn()?;

        // Handle stdout in separate thread
        if let Some(stdout) = child.stdout.take() {
            let app_state = Arc::clone(&self.app_state);

            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    let mut state = app_state.lock().unwrap();
                    state.installer_output.push(line.clone());

                    // Keep only last 100 lines
                    if state.installer_output.len() > 100 {
                        state.installer_output.remove(0);
                    }

                    // Update progress based on output content
                    if line.contains("Starting Arch Linux installation") {
                        state.installation_progress = 10;
                        state.status_message = "Installation started".to_string();
                    } else if line.contains("Preparing system") {
                        state.installation_progress = 15;
                        state.status_message = "Preparing system".to_string();
                    } else if line.contains("Starting disk partitioning") {
                        state.installation_progress = 25;
                        state.status_message = "Partitioning disk".to_string();
                    } else if line.contains("Installing base system") {
                        state.installation_progress = 40;
                        state.status_message = "Installing base system".to_string();
                    } else if line.contains("Configuring system") {
                        state.installation_progress = 60;
                        state.status_message = "Configuring system".to_string();
                    } else if line.contains("Installing packages") {
                        state.installation_progress = 75;
                        state.status_message = "Installing packages".to_string();
                    } else if line.contains("Configuring bootloader") {
                        state.installation_progress = 85;
                        state.status_message = "Configuring bootloader".to_string();
                    } else if line.contains("Finalizing installation") {
                        state.installation_progress = 95;
                        state.status_message = "Finalizing installation".to_string();
                    } else if line.contains("Installation complete") {
                        state.installation_progress = 100;
                        state.status_message = "Installation completed successfully!".to_string();
                    }
                }
            });
        }

        // Handle stderr in separate thread
        if let Some(stderr) = child.stderr.take() {
            let app_state = Arc::clone(&self.app_state);

            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    let mut state = app_state.lock().unwrap();
                    state.installer_output.push(format!("ERROR: {}", line));

                    // Keep only last 100 lines
                    if state.installer_output.len() > 100 {
                        state.installer_output.remove(0);
                    }

                    // Update app state
                    state.status_message = format!("Error: {}", line);
                }
            });
        }

        // Wait for installation completion in separate thread
        let app_state = Arc::clone(&self.app_state);

        thread::spawn(move || match child.wait() {
            Ok(status) => {
                let mut state = app_state.lock().unwrap();

                if status.success() {
                    state.installation_progress = 100;
                    state.mode = crate::app::AppMode::Complete;
                    state.status_message = "Installation completed successfully!".to_string();
                    state
                        .installer_output
                        .push("Installation completed successfully!".to_string());
                } else {
                    state.status_message = format!(
                        "Installation failed with exit code: {}",
                        status.code().unwrap_or(-1)
                    );
                    state.installer_output.push(format!(
                        "Installation failed with exit code: {}",
                        status.code().unwrap_or(-1)
                    ));
                }
            }
            Err(e) => {
                let mut state = app_state.lock().unwrap();

                state
                    .installer_output
                    .push(format!("ERROR: Failed to wait for installer: {}", e));
                state.status_message = format!("Installation error: {}", e);
            }
        });

        Ok(())
    }
}

// ============================================================================
// Type-Safe Disk Operations (Sprint 4)
// ============================================================================

/// Disk layout configuration for installation.
///
/// Defines the partition structure Rust controls before Bash executes.
/// This ensures the installer knows the exact layout before any destructive operations.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Library API - will be used when installer is integrated
pub struct DiskLayout {
    /// Target disk device (e.g., `/dev/sda` or `/dev/nvme0n1`).
    pub disk: PathBuf,
    /// EFI/Boot partition device (e.g., `/dev/sda1`).
    pub boot_partition: PathBuf,
    /// Root partition device (e.g., `/dev/sda2`).
    pub root_partition: PathBuf,
    /// Filesystem for root partition.
    pub root_filesystem: Filesystem,
    /// Optional swap partition device.
    pub swap_partition: Option<PathBuf>,
    /// Target root mountpoint (typically `/mnt`).
    pub target_root: PathBuf,
}

impl Default for DiskLayout {
    fn default() -> Self {
        Self {
            disk: PathBuf::from("/dev/sda"),
            boot_partition: PathBuf::from("/dev/sda1"),
            root_partition: PathBuf::from("/dev/sda2"),
            root_filesystem: Filesystem::Ext4,
            swap_partition: None,
            target_root: PathBuf::from("/mnt"),
        }
    }
}

#[allow(dead_code)] // Library API - will be called from main installer flow
/// Prepare disks for installation using type-safe operations.
///
/// This function orchestrates the disk preparation sequence:
/// 1. Wipe the target disk (destructive - requires confirmation)
/// 2. Format the EFI/boot partition as FAT32
/// 3. Format the root partition with the specified filesystem
/// 4. Mount root partition first (to /mnt)
/// 5. Mount boot partition second (to /mnt/boot)
///
/// # Mount Order
///
/// **CRITICAL**: Root must be mounted BEFORE boot. If boot is mounted first,
/// the mount point `/mnt/boot` won't exist and the operation will fail.
///
/// # Arguments
///
/// * `layout` - The disk layout configuration
/// * `wipe` - Whether to wipe the disk (requires confirmation)
/// * `confirm_wipe` - Explicit confirmation for disk wipe operation
///
/// # Returns
///
/// - `Ok(())` - All operations completed successfully
/// - `Err` - Any operation failed (fail fast)
///
/// # Example
///
/// ```ignore
/// use archtui::installer::{DiskLayout, prepare_disks};
/// use archtui::types::Filesystem;
/// use std::path::PathBuf;
///
/// let layout = DiskLayout {
///     disk: PathBuf::from("/dev/sda"),
///     boot_partition: PathBuf::from("/dev/sda1"),
///     root_partition: PathBuf::from("/dev/sda2"),
///     root_filesystem: Filesystem::Ext4,
///     swap_partition: None,
///     target_root: PathBuf::from("/mnt"),
/// };
///
/// prepare_disks(&layout, true, true)?;
/// ```
pub fn prepare_disks(layout: &DiskLayout, wipe: bool, confirm_wipe: bool) -> Result<()> {
    log::info!("Starting disk preparation for {:?}", layout.disk);

    // Step 1: Optionally wipe the disk
    if wipe {
        log::info!("Wiping disk: {:?}", layout.disk);
        let wipe_args = WipeDiskArgs {
            device: layout.disk.clone(),
            method: WipeMethod::Quick,
            confirm: confirm_wipe,
        };
        let output = run_script_safe(&wipe_args)
            .context("Failed to execute disk wipe")?;
        output.ensure_success("Disk wipe")?;
        log::info!("Disk wipe completed");
    }

    // Step 2: Format EFI/boot partition as FAT32
    log::info!("Formatting boot partition: {:?} as FAT32", layout.boot_partition);
    let format_boot = FormatPartitionArgs {
        device: layout.boot_partition.clone(),
        filesystem: Filesystem::Fat32,
        label: Some("EFI".to_string()),
        force: false,
    };
    let output = run_script_safe(&format_boot)
        .context("Failed to execute boot partition format")?;
    output.ensure_success("Boot partition format")?;
    log::info!("Boot partition formatted");

    // Step 3: Format root partition
    log::info!(
        "Formatting root partition: {:?} as {:?}",
        layout.root_partition,
        layout.root_filesystem
    );
    let format_root = FormatPartitionArgs {
        device: layout.root_partition.clone(),
        filesystem: layout.root_filesystem,
        label: Some("archroot".to_string()),
        force: false,
    };
    let output = run_script_safe(&format_root)
        .context("Failed to execute root partition format")?;
    output.ensure_success("Root partition format")?;
    log::info!("Root partition formatted");

    // Step 4: Mount root partition FIRST
    // CRITICAL: Root must be mounted before boot so /mnt/boot exists
    log::info!(
        "Mounting root partition: {:?} -> {:?}",
        layout.root_partition,
        layout.target_root
    );
    let mount_root = MountPartitionArgs {
        device: layout.root_partition.clone(),
        mountpoint: layout.target_root.clone(),
        options: None,
    };
    let output = run_script_safe(&mount_root)
        .context("Failed to execute root partition mount")?;
    output.ensure_success("Root partition mount")?;
    log::info!("Root partition mounted");

    // Step 5: Create boot mount point and mount boot partition
    let boot_mountpoint = layout.target_root.join("boot");
    log::info!(
        "Mounting boot partition: {:?} -> {:?}",
        layout.boot_partition,
        boot_mountpoint
    );

    // Ensure boot directory exists
    std::fs::create_dir_all(&boot_mountpoint)
        .with_context(|| format!("Failed to create boot mountpoint: {:?}", boot_mountpoint))?;

    let mount_boot = MountPartitionArgs {
        device: layout.boot_partition.clone(),
        mountpoint: boot_mountpoint,
        options: None,
    };
    let output = run_script_safe(&mount_boot)
        .context("Failed to execute boot partition mount")?;
    output.ensure_success("Boot partition mount")?;
    log::info!("Boot partition mounted");

    log::info!("Disk preparation complete");
    Ok(())
}

// ============================================================================
// Base System Installation (Sprint 5)
// Requires `alpm` feature - only available on Arch Linux
// ============================================================================

#[cfg(feature = "alpm")]
/// Base packages required for a minimal Arch Linux installation.
///
/// These packages provide:
/// - `base`: Essential packages (filesystem, glibc, bash, etc.)
/// - `linux`: The Linux kernel
/// - `linux-firmware`: Firmware files for common hardware
/// - `base-devel`: Build tools (gcc, make, etc.) needed for AUR
const BASE_PACKAGES: &[&str] = &["base", "linux", "linux-firmware", "base-devel"];

#[cfg(feature = "alpm")]
/// Install the base system to the target root using ALPM.
///
/// This replaces shell-based `pacstrap` calls with direct ALPM bindings,
/// providing full transparency via log callbacks. All package operations
/// are logged through the Rust `log` crate.
///
/// # Arguments
///
/// * `target_root` - The mount point of the root partition (typically `/mnt`)
///
/// # Transparency
///
/// The ALPM handle is configured with `log_cb` which routes all library
/// messages to `log::info!`, `log::warn!`, and `log::error!`. This ensures
/// the TUI sees "Downloading...", "Installing...", etc.
///
/// # Fail Fast
///
/// If any package fails to download or install, this function returns an
/// error immediately. No silent retries are attempted.
///
/// # Example
///
/// ```ignore
/// use archtui::installer::install_base_system;
/// use std::path::Path;
///
/// // After mounting /mnt
/// install_base_system(Path::new("/mnt"))?;
/// ```
#[allow(dead_code)] // Library API - will be called from main installer flow
pub fn install_base_system(target_root: &Path) -> Result<()> {
    log::info!("Installing base system to {:?}", target_root);

    // Verify target root exists and is mounted
    if !target_root.exists() {
        anyhow::bail!(
            "Target root does not exist: {:?}. Mount the root partition first.",
            target_root
        );
    }

    // Verify it's actually a mount point (has lost+found or is non-empty)
    let is_mount = target_root.join("lost+found").exists()
        || std::fs::read_dir(target_root)
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);

    if !is_mount {
        log::warn!(
            "Target root {:?} appears to be empty - ensure it's properly mounted",
            target_root
        );
    }

    // Initialize ALPM with target root
    let db_path = target_root.join("var/lib/pacman");

    // Ensure pacman db directory exists
    std::fs::create_dir_all(&db_path)
        .with_context(|| format!("Failed to create pacman db path: {:?}", db_path))?;

    log::info!("Initializing ALPM: root={:?}, db={:?}", target_root, db_path);

    let mut pm = PackageManager::new(target_root, &db_path)
        .context("Failed to initialize ALPM package manager")?;

    // Read pacman.conf from the live system to get mirror list
    // In a real installation, we'd copy mirrorlist to target first
    let live_pacman_conf = Path::new("/etc/pacman.conf");
    if live_pacman_conf.exists() {
        log::info!("Loading mirror configuration from live system");
        pm = PackageManager::from_pacman_conf(target_root, live_pacman_conf)
            .context("Failed to load pacman.conf")?;
    } else {
        log::warn!("No pacman.conf found - using default mirrors");
    }

    // CRITICAL: Log exactly what we're installing (Self-Audit: linux-firmware included)
    log::info!(
        "Installing base packages: {:?}",
        BASE_PACKAGES
    );

    // Install base packages
    // Fail Fast: Any failure here aborts immediately
    pm.install_packages(BASE_PACKAGES)
        .context("Failed to install base packages")?;

    log::info!("Base system installation complete");
    Ok(())
}

#[cfg(feature = "alpm")]
/// Install the base system with additional packages.
///
/// This extends `install_base_system` to include custom packages like
/// a specific kernel, text editor, or network tools.
///
/// # Arguments
///
/// * `target_root` - The mount point of the root partition
/// * `extra_packages` - Additional packages to install beyond base
#[allow(dead_code)] // Library API - will be called from main installer flow
pub fn install_base_system_with_extras(
    target_root: &Path,
    extra_packages: &[&str],
) -> Result<()> {
    log::info!("Installing base system with {} extra packages", extra_packages.len());

    // Combine base packages with extras
    let mut all_packages: Vec<&str> = BASE_PACKAGES.to_vec();
    all_packages.extend_from_slice(extra_packages);

    log::info!("Full package list: {:?}", all_packages);

    // Initialize ALPM
    let db_path = target_root.join("var/lib/pacman");
    std::fs::create_dir_all(&db_path)
        .with_context(|| format!("Failed to create pacman db path: {:?}", db_path))?;

    let mut pm = PackageManager::new(target_root, &db_path)
        .context("Failed to initialize ALPM package manager")?;

    let live_pacman_conf = Path::new("/etc/pacman.conf");
    if live_pacman_conf.exists() {
        pm = PackageManager::from_pacman_conf(target_root, live_pacman_conf)
            .context("Failed to load pacman.conf")?;
    }

    // Install all packages
    pm.install_packages(&all_packages)
        .context("Failed to install packages")?;

    log::info!("Package installation complete");
    Ok(())
}

// ============================================================================
// System Configuration (Sprint 6)
// ============================================================================

/// System configuration parameters for post-install setup.
///
/// Contains all the settings needed to configure the installed system:
/// - Locale and timezone
/// - User accounts
/// - Bootloader
#[derive(Debug, Clone)]
#[allow(dead_code)] // Library API - will be used when installer is integrated
pub struct SystemConfig {
    /// Target root mountpoint (typically `/mnt`).
    pub target_root: PathBuf,
    /// System hostname.
    pub hostname: String,
    /// Locale setting (e.g., "en_US.UTF-8").
    pub locale: String,
    /// Timezone (e.g., "America/New_York").
    pub timezone: String,
    /// Console keymap (e.g., "us").
    pub keymap: Option<String>,
    /// Username for the main user account.
    pub username: String,
    /// Password for the main user (passed via env var, NOT CLI).
    pub user_password: String,
    /// Whether the main user should have sudo access.
    pub user_sudo: bool,
    /// Optional root password (if None, root login disabled).
    pub root_password: Option<String>,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            target_root: PathBuf::from("/mnt"),
            hostname: "archlinux".to_string(),
            locale: "en_US.UTF-8".to_string(),
            timezone: "UTC".to_string(),
            keymap: Some("us".to_string()),
            username: "user".to_string(),
            user_password: String::new(),
            user_sudo: true,
            root_password: None,
        }
    }
}

/// Configure the installed system.
///
/// This function runs the post-install configuration sequence:
/// 1. **Generate Fstab** - CRITICAL: Must be done after mount, before chroot
/// 2. **Set Hostname/Locale** - Configure system identity
/// 3. **Create Users** - Set up root and main user accounts
/// 4. Install Bootloader (TODO: Sprint 7)
///
/// # Security: Password Handling
///
/// **CRITICAL**: Passwords are passed via environment variables, NOT CLI flags.
/// CLI arguments are visible in `/proc/<pid>/cmdline` to all users on the system.
/// The `UserAddArgs` struct enforces this by excluding password from `to_cli_args()`
/// and including it in `get_env_vars()`.
///
/// # Arguments
///
/// * `config` - System configuration parameters
///
/// # Returns
///
/// - `Ok(())` - All configuration completed successfully
/// - `Err` - Any operation failed (fail fast)
///
/// # Example
///
/// ```ignore
/// use archtui::installer::{SystemConfig, configure_system};
/// use std::path::PathBuf;
///
/// let config = SystemConfig {
///     target_root: PathBuf::from("/mnt"),
///     hostname: "myarch".to_string(),
///     locale: "en_US.UTF-8".to_string(),
///     timezone: "America/Chicago".to_string(),
///     keymap: Some("us".to_string()),
///     username: "archuser".to_string(),
///     user_password: "secret123".to_string(),
///     user_sudo: true,
///     root_password: Some("rootpw".to_string()),
/// };
///
/// configure_system(&config)?;
/// ```
#[allow(dead_code)] // Library API - will be called from main installer flow
pub fn configure_system(config: &SystemConfig) -> Result<()> {
    log::info!("Starting system configuration for {:?}", config.target_root);

    // ========================================================================
    // Step 1: Generate Fstab
    // CRITICAL: Must be done AFTER partitions are mounted, BEFORE chroot/reboot
    // Uses genfstab to read currently mounted filesystems
    // ========================================================================
    log::info!("Generating /etc/fstab");
    let fstab_args = GenFstabArgs {
        root: config.target_root.clone(),
    };
    let output = run_script_safe(&fstab_args)
        .context("Failed to execute fstab generation")?;
    output.ensure_success("Fstab generation")?;
    log::info!("Fstab generated successfully");

    // ========================================================================
    // Step 2: Configure Hostname and Locale
    // Sets /etc/hostname, /etc/locale.gen, /etc/locale.conf, /etc/localtime
    // ========================================================================
    log::info!(
        "Configuring locale: hostname={}, locale={}, timezone={}",
        config.hostname,
        config.locale,
        config.timezone
    );
    let locale_args = LocaleArgs {
        root: config.target_root.clone(),
        hostname: config.hostname.clone(),
        locale: config.locale.clone(),
        timezone: config.timezone.clone(),
        keymap: config.keymap.clone(),
    };
    let output = run_script_safe(&locale_args)
        .context("Failed to execute locale configuration")?;
    output.ensure_success("Locale configuration")?;
    log::info!("Locale configuration complete");

    // ========================================================================
    // Step 3: Create User Accounts
    // SECURITY: Passwords via env vars (USER_PASSWORD), NOT CLI flags
    // ========================================================================

    // Create main user with sudo access if requested
    log::info!("Creating user: {} (sudo={})", config.username, config.user_sudo);
    let user_args = UserAddArgs {
        username: config.username.clone(),
        password: Some(config.user_password.clone()),
        groups: Some("wheel,audio,video,storage,optical".to_string()),
        shell: Some("/bin/bash".to_string()),
        full_name: None,
        home_dir: None,
        create_home: true,
        sudo: config.user_sudo,
    };

    // SELF-AUDIT: Verify password is NOT in CLI args
    let cli_args = user_args.to_cli_args();
    debug_assert!(
        !cli_args.iter().any(|a| a.contains(&config.user_password)),
        "BUG: Password found in CLI args! This is a security vulnerability."
    );

    let output = run_script_safe(&user_args)
        .context("Failed to create user")?;
    output.ensure_success("User creation")?;
    log::info!("User {} created successfully", config.username);

    // Set root password if provided
    if let Some(ref root_pw) = config.root_password {
        log::info!("Setting root password");
        let root_args = UserAddArgs {
            username: "root".to_string(),
            password: Some(root_pw.clone()),
            groups: None,
            shell: None,
            full_name: None,
            home_dir: None,
            create_home: false, // Root home already exists
            sudo: false,
        };

        // SELF-AUDIT: Verify password is NOT in CLI args
        let root_cli = root_args.to_cli_args();
        debug_assert!(
            !root_cli.iter().any(|a| a.contains(root_pw)),
            "BUG: Root password found in CLI args!"
        );

        let output = run_script_safe(&root_args)
            .context("Failed to set root password")?;
        output.ensure_success("Root password setup")?;
        log::info!("Root password configured");
    } else {
        log::info!("No root password specified - root login will be disabled");
    }

    // ========================================================================
    // Step 4: Install Bootloader (TODO: Sprint 7)
    // This will configure GRUB/systemd-boot based on boot mode (UEFI/BIOS)
    // ========================================================================
    log::info!("Bootloader installation: TODO in Sprint 7");

    log::info!("System configuration complete");
    Ok(())
}

// ============================================================================
// LUKS Encryption (Sprint 11)
// ============================================================================

/// Encryption configuration for a partition.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Library API
pub struct EncryptionConfig {
    /// Password for the encrypted volume (stored temporarily in SecretFile).
    pub password: String,
    /// Device mapper name (e.g., "cryptroot").
    pub mapper_name: String,
    /// Optional LUKS label.
    pub label: Option<String>,
    /// Cipher configuration.
    pub cipher: LuksCipher,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            password: String::new(),
            mapper_name: "cryptroot".to_string(),
            label: Some("archcrypt".to_string()),
            cipher: LuksCipher::default(),
        }
    }
}

/// Encrypt a partition with LUKS2.
///
/// This function handles the secure encryption workflow:
/// 1. Write password to a temporary keyfile (SecretFile with 0600 permissions)
/// 2. Call cryptsetup luksFormat with the keyfile
/// 3. Securely wipe and delete the keyfile (even on failure)
///
/// # Security Model
///
/// - Password is NEVER passed via CLI (visible in `ps aux`)
/// - Password is written to `/tmp` which is RAM-backed on Arch ISO
/// - `SecretFile` uses RAII to ensure cleanup on panic/error
/// - Requires `CONFIRM_LUKS_FORMAT=yes` environment variable
///
/// # Arguments
///
/// * `device` - Partition to encrypt (e.g., `/dev/sda2`)
/// * `config` - Encryption configuration (password, mapper name, cipher)
/// * `confirm` - Explicit confirmation for destructive operation
///
/// # Returns
///
/// - `Ok(PathBuf)` - Path to the decrypted device (`/dev/mapper/<mapper_name>`)
/// - `Err` - Encryption failed
///
/// # Example
///
/// ```ignore
/// use archtui::installer::{encrypt_partition, EncryptionConfig};
/// use std::path::PathBuf;
///
/// let config = EncryptionConfig {
///     password: "mysecretpassword".to_string(),
///     mapper_name: "cryptroot".to_string(),
///     label: Some("archcrypt".to_string()),
///     cipher: LuksCipher::default(),
/// };
///
/// let decrypted_device = encrypt_partition(
///     &PathBuf::from("/dev/sda2"),
///     &config,
///     true, // confirm
/// )?;
/// // decrypted_device = /dev/mapper/cryptroot
/// ```
#[allow(dead_code)] // Library API
pub fn encrypt_partition(
    device: &PathBuf,
    config: &EncryptionConfig,
    confirm: bool,
) -> Result<PathBuf> {
    log::info!("Starting LUKS encryption for {:?}", device);

    // SECURITY: Create temporary keyfile with password
    // SecretFile ensures cleanup via Drop trait (even on panic)
    let keyfile = SecretFile::new(&config.password)
        .context("Failed to create temporary keyfile")?;

    log::info!("Temporary keyfile created: {:?}", keyfile.path());

    // Format the device with LUKS2
    let format_args = LuksFormatArgs {
        device: device.clone(),
        cipher: config.cipher,
        key_file: keyfile.path().to_path_buf(),
        label: config.label.clone(),
        confirm,
    };

    // SELF-AUDIT: Verify password is NOT in CLI args
    let cli_args = format_args.to_cli_args();
    debug_assert!(
        !cli_args.iter().any(|a| a.contains(&config.password)),
        "BUG: Password found in LUKS format CLI args! This is a security vulnerability."
    );

    log::info!("Executing LUKS format on {:?}", device);
    let output = run_script_safe(&format_args)
        .context("Failed to execute LUKS format")?;
    output.ensure_success("LUKS format")?;
    log::info!("LUKS format completed successfully");

    // Open the encrypted device
    let open_args = LuksOpenArgs {
        device: device.clone(),
        mapper_name: config.mapper_name.clone(),
        key_file: keyfile.path().to_path_buf(),
    };

    log::info!("Opening LUKS device as {:?}", config.mapper_name);
    let output = run_script_safe(&open_args)
        .context("Failed to open LUKS device")?;
    output.ensure_success("LUKS open")?;

    // SecretFile is dropped here, securely wiping the keyfile
    let decrypted_device = PathBuf::from(format!("/dev/mapper/{}", config.mapper_name));
    log::info!(
        "LUKS encryption complete. Decrypted device: {:?}",
        decrypted_device
    );

    Ok(decrypted_device)
}

// ============================================================================
// Desktop Profile Installation (Sprint 12)
// ============================================================================

/// Install a desktop profile (DE/WM).
///
/// Installs the packages for the selected profile and enables the
/// display manager service.
///
/// # Arguments
///
/// * `target_root` - Target installation root (e.g., `/mnt`)
/// * `profile` - Desktop profile to install
///
/// # Returns
///
/// - `Ok(())` - Profile installed successfully
/// - `Err` - Installation failed
#[cfg(feature = "alpm")]
#[allow(dead_code)] // Library API
pub fn install_profile(target_root: &Path, profile: Profile) -> Result<()> {
    log::info!("Installing profile: {:?}", profile);

    // Get packages for the profile
    let packages = profile.get_packages();
    log::info!("Profile packages: {:?}", packages);

    // Install packages using ALPM
    let db_path = target_root.join("var/lib/pacman");
    std::fs::create_dir_all(&db_path)
        .with_context(|| format!("Failed to create pacman db path: {:?}", db_path))?;

    let mut pm = PackageManager::new(target_root, &db_path)
        .context("Failed to initialize ALPM package manager")?;

    let live_pacman_conf = Path::new("/etc/pacman.conf");
    if live_pacman_conf.exists() {
        pm = PackageManager::from_pacman_conf(target_root, live_pacman_conf)
            .context("Failed to load pacman.conf")?;
    }

    pm.install_packages(packages)
        .context("Failed to install profile packages")?;

    // Enable display manager if present
    if let Some(dm) = profile.get_display_manager() {
        log::info!("Enabling display manager: {}", dm);

        let mut services: Vec<String> = vec![dm.to_string()];
        services.extend(profile.get_services().iter().map(|s| s.to_string()));

        let enable_args = EnableServicesArgs {
            services,
            root: target_root.to_path_buf(),
        };

        let output = run_script_safe(&enable_args)
            .context("Failed to enable services")?;
        output.ensure_success("Enable services")?;
    }

    log::info!("Profile {:?} installed successfully", profile);
    Ok(())
}

/// Install dotfiles from a Git repository.
///
/// Clones the dotfiles repository for the specified user.
///
/// # Prerequisites
///
/// - Git must be installed (included in most profiles)
/// - Target user must exist
///
/// # Arguments
///
/// * `config` - Dotfiles configuration (repo URL, target user)
///
/// # Returns
///
/// - `Ok(())` - Dotfiles installed successfully
/// - `Err` - Installation failed
#[allow(dead_code)] // Library API
pub fn install_dotfiles(config: &DotfilesConfig) -> Result<()> {
    log::info!(
        "Installing dotfiles from {} for user {}",
        config.repo_url,
        config.target_user
    );

    let args = InstallDotfilesArgs {
        repo_url: config.repo_url.clone(),
        target_user: config.target_user.clone(),
        target_dir: config.target_dir.as_ref().map(PathBuf::from),
        branch: config.branch.clone(),
        backup: true, // Always backup existing files
    };

    let output = run_script_safe(&args)
        .context("Failed to install dotfiles")?;
    output.ensure_success("Dotfiles installation")?;

    log::info!("Dotfiles installed successfully");
    Ok(())
}

// ============================================================================
// Mirror Ranking (Sprint 13)
// ============================================================================

/// Update pacman mirrorlist using reflector.
///
/// Ranks mirrors by speed and updates `/etc/pacman.d/mirrorlist`.
/// Should be called at the start of installation for faster downloads.
///
/// # Network Requirement
///
/// This function requires network connectivity. Call `check_network_connectivity()`
/// first to verify the network is up.
///
/// # Arguments
///
/// * `country` - Optional country filter (ISO 3166-1 alpha-2 code)
/// * `limit` - Number of mirrors to keep (default: 20)
/// * `sort` - Sort method (default: rate)
///
/// # Returns
///
/// - `Ok(())` - Mirrors updated successfully
/// - `Err` - Update failed (possibly due to network)
#[allow(dead_code)] // Library API
pub fn update_mirrors(
    country: Option<&str>,
    limit: u32,
    sort: MirrorSortMethod,
) -> Result<()> {
    log::info!("Updating pacman mirrors (country={:?}, limit={}, sort={})",
               country, limit, sort);

    // Check network connectivity first
    log::info!("Checking network connectivity...");
    let connectivity = CheckConnectivityArgs::default();
    let output = run_script_safe(&connectivity)
        .context("Failed to check network connectivity")?;

    if !output.success {
        anyhow::bail!(
            "Network connectivity check failed. Cannot update mirrors without network access."
        );
    }
    log::info!("Network connectivity OK");

    // Update mirrors
    let args = UpdateMirrorsArgs {
        country: country.map(String::from),
        limit,
        sort,
        protocol: Some("https".to_string()),
        save: true,
    };

    log::info!("Running reflector to rank mirrors...");
    let output = run_script_safe(&args)
        .context("Failed to update mirrors")?;
    output.ensure_success("Mirror update")?;

    log::info!("Mirrors updated successfully");
    Ok(())
}

/// Check network connectivity before network-dependent operations.
///
/// # Returns
///
/// - `Ok(true)` - Network is available
/// - `Ok(false)` - Network is not available
/// - `Err` - Check failed
#[allow(dead_code)] // Library API
pub fn check_network_connectivity() -> Result<bool> {
    log::info!("Checking network connectivity...");

    let args = CheckConnectivityArgs::default();
    let output = run_script_safe(&args)
        .context("Failed to check network connectivity")?;

    Ok(output.success)
}
