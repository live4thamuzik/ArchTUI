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
use crate::package_manager::PackageManager;
use crate::script_runner::run_script_safe;
use crate::scripts::disk::{
    FormatPartitionArgs, MountPartitionArgs, WipeDiskArgs, WipeMethod,
};
use crate::types::Filesystem;
use anyhow::{Context, Result};
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
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
        let script_path = std::env::var("ARCHINSTALL_SCRIPTS_DIR")
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
/// use archinstall_tui::installer::{DiskLayout, prepare_disks};
/// use archinstall_tui::types::Filesystem;
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
// ============================================================================

/// Base packages required for a minimal Arch Linux installation.
///
/// These packages provide:
/// - `base`: Essential packages (filesystem, glibc, bash, etc.)
/// - `linux`: The Linux kernel
/// - `linux-firmware`: Firmware files for common hardware
/// - `base-devel`: Build tools (gcc, make, etc.) needed for AUR
const BASE_PACKAGES: &[&str] = &["base", "linux", "linux-firmware", "base-devel"];

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
/// use archinstall_tui::installer::install_base_system;
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
