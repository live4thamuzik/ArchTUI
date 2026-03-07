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
//! # Base System Installation
//!
//! The `install_base_system` function uses the ALPM bindings to install packages
//! directly, replacing shell-based pacstrap calls. This provides full transparency
//! via log callbacks.

use crate::app::AppState;
use crate::config::Configuration;
#[cfg(feature = "alpm")]
use crate::package_manager::PackageManager;
use crate::process_guard::{ChildRegistry, CommandProcessGroup};
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
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
#[cfg(feature = "alpm")]
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

/// Strip ANSI escape sequences and handle carriage returns from a line of output.
///
/// Bash scripts emit ANSI color codes (e.g., `\x1b[31m`) that are invisible in real terminals
/// but ratatui renders as visible garbage, causing screen artifacts. Carriage returns from
/// progress-bar style output (e.g., pacman) cause overlapping text when stored as raw strings.
pub fn strip_ansi_and_cr(input: &str) -> String {
    let mut stripped = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            match chars.peek() {
                Some('[') => {
                    // CSI sequence: \x1b[ ... (final byte in @..~)
                    chars.next();
                    for next in chars.by_ref() {
                        if ('@'..='~').contains(&next) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    // OSC sequence: \x1b] ... (terminated by BEL or ST)
                    chars.next();
                    while let Some(next) = chars.next() {
                        if next == '\x07' {
                            break;
                        }
                        if next == '\x1b' && chars.peek() == Some(&'\\') {
                            chars.next();
                            break;
                        }
                    }
                }
                Some(_) => {
                    // Simple two-byte escape: skip next char
                    chars.next();
                }
                None => {}
            }
        } else {
            stripped.push(c);
        }
    }

    // Handle carriage returns: keep only content after the last \r
    // Simulates terminal behavior where \r returns cursor to column 0
    // and subsequent text overwrites from the start of the line
    match stripped.rfind('\r') {
        Some(pos) => stripped[pos + 1..].to_string(),
        None => stripped,
    }
}

/// UTC timestamp formatted as HH:MM:SS for log file entries.
/// Uses raw SystemTime to avoid adding a chrono dependency.
pub fn now_hms() -> String {
    use std::time::SystemTime;
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

/// Write a line to the master log file (thread-safe, best-effort).
fn write_master_log(log_file: &Arc<Mutex<Option<File>>>, line: &str) {
    // SAFETY: poison recovery via into_inner — never panic on mutex
    if let Ok(mut guard) = log_file.lock() {
        if let Some(ref mut f) = *guard {
            let _ = writeln!(f, "[{}] {}", now_hms(), line);
        }
    }
}

/// Installer instance
pub struct Installer {
    env_vars: std::collections::HashMap<String, String>,
    app_state: Arc<Mutex<AppState>>,
}

impl Installer {
    /// Create a new installer from TUI configuration
    pub fn new(config: Configuration, app_state: Arc<Mutex<AppState>>) -> Self {
        Self {
            env_vars: config.to_env_vars(),
            app_state,
        }
    }

    /// Create a new installer from a file-based InstallationConfig
    pub fn from_file_config(
        config: &crate::config_file::InstallationConfig,
        app_state: Arc<Mutex<AppState>>,
    ) -> Self {
        let env_vars: std::collections::HashMap<String, String> =
            config.to_env_vars().into_iter().collect();
        Self { env_vars, app_state }
    }

    /// Start the installation process
    pub fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Update app state to installation mode
        {
            // SAFETY: poison recovery via into_inner — never panic on mutex
            let mut state = self.app_state.lock().unwrap_or_else(|e| e.into_inner());
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

        let mut env_vars = self.env_vars.clone();

        // Pass LOG_LEVEL to child process (ARCHTUI_LOG_LEVEL env var → LOG_LEVEL in bash)
        let log_level = std::env::var("ARCHTUI_LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string());
        env_vars.insert("LOG_LEVEL".to_string(), log_level.clone());

        // Inject manual partition assignments (if set)
        {
            // SAFETY: poison recovery via into_inner — never panic on mutex
            let state = self.app_state.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(ref map) = state.manual_partition_map {
                env_vars.insert("MANUAL_ROOT_PARTITION".to_string(), map.root.clone());
                env_vars.insert("MANUAL_ROOT_FS".to_string(), map.root_fs.clone());
                env_vars.insert("MANUAL_BOOT_PARTITION".to_string(), map.boot.clone());
                if !map.efi.is_empty() {
                    env_vars.insert("MANUAL_EFI_PARTITION".to_string(), map.efi.clone());
                }
                if !map.home.is_empty() {
                    env_vars.insert("MANUAL_HOME_PARTITION".to_string(), map.home.clone());
                    env_vars.insert("MANUAL_HOME_FS".to_string(), map.home_fs.clone());
                }
                if !map.swap.is_empty() {
                    env_vars.insert("MANUAL_SWAP_PARTITION".to_string(), map.swap.clone());
                }
            }
        }

        // --- Master Log File ---
        // Create a persistent log that captures everything the TUI sees (and more),
        // surviving the 500-line ringbuffer cap. ANSI-stripped, timestamped.
        let master_log: Arc<Mutex<Option<File>>> = {
            let log_dir = crate::script_runner::log_dir();
            let log_dir = log_dir.to_string_lossy().to_string();
            let _ = fs::create_dir_all(&log_dir);
            let timestamp = now_hms().replace(':', "");
            let log_path = format!("{}/install-{}-master.log", log_dir, timestamp);
            match OpenOptions::new().create(true).append(true).open(&log_path) {
                Ok(mut f) => {
                    // Write header block
                    let _ = writeln!(f, "=== ArchTUI Master Installation Log ===");
                    let _ = writeln!(f, "[{}] Log level: {}", now_hms(), log_level);
                    let _ = writeln!(f, "[{}] === Environment Variables ===", now_hms());
                    for (k, v) in &env_vars {
                        let display_val = if k.contains("PASSWORD") { "********" } else { v.as_str() };
                        let _ = writeln!(f, "[{}]   {}={}", now_hms(), k, display_val);
                    }
                    let _ = writeln!(f, "[{}] === End Environment Variables ===", now_hms());
                    let _ = writeln!(f);
                    Arc::new(Mutex::new(Some(f)))
                }
                Err(_) => Arc::new(Mutex::new(None)),
            }
        };

        // Determine script path - use wrapper for TUI-friendly output
        let script_path = crate::script_runner::scripts_base_dir()
            .join("install_wrapper.sh")
            .to_string_lossy()
            .to_string();

        // Launch the installation script
        // stdin is null - scripts are non-interactive per lint rules
        let mut child = Command::new("bash")
            .arg(&script_path)
            .envs(&env_vars)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .in_new_process_group()
            .spawn()
            .context("Failed to spawn install wrapper script")?;

        // Register child PID for Death Pact compliance and cancellation
        let child_pid = child.id();
        {
            // SAFETY: poison recovery via into_inner — never panic on mutex
            let mut state = self.app_state.lock().unwrap_or_else(|e| e.into_inner());
            state.installer_pid = Some(child_pid);
        }
        if let Ok(mut registry) = ChildRegistry::global().lock() {
            registry.register(child_pid);
        }

        // Handle stdout in separate thread
        if let Some(stdout) = child.stdout.take() {
            let app_state = Arc::clone(&self.app_state);
            let log_file = Arc::clone(&master_log);

            thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    // Strip ANSI color codes and carriage returns before storing.
                    // Bash scripts emit these for terminal display, but ratatui renders
                    // them as visible garbage causing screen artifacts.
                    let clean_line = strip_ansi_and_cr(&line);

                    // Write to master log BEFORE the ringbuffer cap discards old lines
                    write_master_log(&log_file, &clean_line);

                    // SAFETY: poison recovery via into_inner — never panic on mutex
                    let mut state = app_state.lock().unwrap_or_else(|e| e.into_inner());
                    state.installer_output.push(clean_line);

                    // Keep only last 500 lines
                    if state.installer_output.len() > 500 {
                        state.installer_output.remove(0);
                    }

                    // Update progress based on output content (match on raw line —
                    // ANSI codes don't interfere with contains() substring matching)
                    //
                    // Phases 1-4 (fast) = 2%-18%, pacstrap = 22%-40%,
                    // Phase 7 chroot (longest) = 48%-88% with 5 PROGRESS: sub-milestones,
                    // Phase 8 + completion = 92%-100%
                    let progress_msg = if line.contains("Starting Arch Linux installation") {
                        state.installation_progress = 2;
                        Some("Installation started")
                    } else if line.contains("Phase 1:") {
                        state.installation_progress = 5;
                        Some("Validating configuration")
                    } else if line.contains("Phase 2:") {
                        state.installation_progress = 8;
                        Some("Preparing system")
                    } else if line.contains("Mirrors ranked") {
                        state.installation_progress = 10;
                        Some("Mirrors configured")
                    } else if line.contains("Phase 3:") {
                        state.installation_progress = 12;
                        Some("Installing dependencies")
                    } else if line.contains("Phase 4:") {
                        state.installation_progress = 14;
                        Some("Partitioning disk")
                    } else if line.contains("Starting disk partitioning") {
                        state.installation_progress = 16;
                        Some("Partitioning disk")
                    } else if line.contains("Disk partitioning complete") {
                        state.installation_progress = 18;
                        Some("Partitioning complete")
                    } else if line.contains("Phase 5:") {
                        state.installation_progress = 20;
                        Some("Installing base system")
                    } else if line.contains("Starting pacstrap") {
                        state.installation_progress = 22;
                        Some("Running pacstrap (this takes several minutes)")
                    } else if line.contains("Base system installed") {
                        state.installation_progress = 40;
                        Some("Base system installed")
                    } else if line.contains("Phase 6:") {
                        state.installation_progress = 45;
                        Some("Generating fstab")
                    } else if line.contains("Phase 7:") {
                        state.installation_progress = 48;
                        Some("Configuring system in chroot")
                    } else if line.contains("PROGRESS: Configuring base system") {
                        state.installation_progress = 52;
                        Some("Configuring base system")
                    } else if line.contains("PROGRESS: Configuring bootloader") {
                        state.installation_progress = 58;
                        Some("Configuring bootloader")
                    } else if line.contains("PROGRESS: Installing desktop environment") {
                        state.installation_progress = 65;
                        Some("Installing desktop environment")
                    } else if line.contains("PROGRESS: Installing additional software") {
                        state.installation_progress = 78;
                        Some("Installing additional software")
                    } else if line.contains("PROGRESS: Running final configuration") {
                        state.installation_progress = 88;
                        Some("Running final configuration")
                    } else if line.contains("Phase 8:") {
                        state.installation_progress = 92;
                        Some("Finalizing installation")
                    } else if line.contains("Installation complete") {
                        state.installation_progress = 100;
                        Some("Installation completed successfully!")
                    } else {
                        None
                    };

                    if let Some(msg) = progress_msg {
                        state.status_message = msg.to_string();
                        // Log Rust-side state events to master log
                        write_master_log(
                            &log_file,
                            &format!("[RUST] Progress: {}% - {}", state.installation_progress, msg),
                        );
                    }
                }
            });
        }

        // Handle stderr in separate thread
        if let Some(stderr) = child.stderr.take() {
            let app_state = Arc::clone(&self.app_state);
            let log_file = Arc::clone(&master_log);

            thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    let clean_line = strip_ansi_and_cr(&line);

                    // Write stderr to master log with ERROR prefix
                    write_master_log(&log_file, &format!("STDERR: {}", clean_line));

                    // SAFETY: poison recovery via into_inner — never panic on mutex
                    let mut state = app_state.lock().unwrap_or_else(|e| e.into_inner());
                    state.installer_output.push(format!("ERROR: {}", clean_line));

                    // Keep only last 500 lines
                    if state.installer_output.len() > 500 {
                        state.installer_output.remove(0);
                    }

                    // Check progress triggers on stderr too (in case wrapper doesn't merge)
                    if line.contains("Phase 1:") {
                        state.installation_progress = 5;
                        state.status_message = "Validating configuration".to_string();
                    } else if line.contains("Phase 2:") {
                        state.installation_progress = 8;
                        state.status_message = "Preparing system".to_string();
                    } else if line.contains("Phase 3:") {
                        state.installation_progress = 12;
                        state.status_message = "Installing dependencies".to_string();
                    } else if line.contains("Phase 4:") {
                        state.installation_progress = 14;
                        state.status_message = "Partitioning disk".to_string();
                    } else if line.contains("Phase 5:") {
                        state.installation_progress = 20;
                        state.status_message = "Installing base system".to_string();
                    } else if line.contains("Phase 6:") {
                        state.installation_progress = 45;
                        state.status_message = "Generating fstab".to_string();
                    } else if line.contains("Phase 7:") {
                        state.installation_progress = 48;
                        state.status_message = "Configuring system in chroot".to_string();
                    } else if line.contains("Phase 8:") {
                        state.installation_progress = 92;
                        state.status_message = "Finalizing installation".to_string();
                    } else if line.contains("ERROR") || line.contains("FATAL") {
                        state.status_message = format!("Error: {}", line);
                    }
                }
            });
        }

        // Wait for installation completion in separate thread
        let app_state = Arc::clone(&self.app_state);
        let wait_log = Arc::clone(&master_log);

        thread::spawn(move || {
            let result = child.wait();

            // Unregister child PID (process has exited)
            if let Ok(mut registry) = ChildRegistry::global().lock() {
                registry.unregister(child_pid);
            }

            match result {
            Ok(status) => {
                // SAFETY: poison recovery via into_inner — never panic on mutex
                let mut state = app_state.lock().unwrap_or_else(|e| e.into_inner());
                state.installer_pid = None;

                if status.success() {
                    state.installation_progress = 100;
                    state.mode = crate::app::AppMode::Complete;
                    state.status_message = "Installation completed successfully!".to_string();
                    state
                        .installer_output
                        .push("Installation completed successfully!".to_string());
                    write_master_log(&wait_log, "[RUST] Installation completed successfully (exit code 0)");
                } else {
                    let exit_code = status.code().unwrap_or(-1);
                    state.status_message = format!(
                        "Installation failed with exit code: {}",
                        exit_code
                    );
                    state.installer_output.push(format!(
                        "Installation failed with exit code: {}",
                        exit_code
                    ));
                    state.installer_output.push(format!(
                        "Check {}/ for full details (master log + verbose trace)",
                        crate::script_runner::log_dir().display()
                    ));
                    state.mode = crate::app::AppMode::Complete;
                    write_master_log(&wait_log, &format!("[RUST] Installation FAILED (exit code {})", exit_code));
                }
            }
            Err(e) => {
                // SAFETY: poison recovery via into_inner — never panic on mutex
                let mut state = app_state.lock().unwrap_or_else(|e| e.into_inner());
                state.installer_pid = None;

                state
                    .installer_output
                    .push(format!("ERROR: Failed to wait for installer: {}", e));
                state.status_message = format!("Installation error: {}", e);
                state.mode = crate::app::AppMode::Complete;
                write_master_log(&wait_log, &format!("[RUST] Installation ERROR: Failed to wait: {}", e));
            }
            }
        });

        Ok(())
    }
}

// ============================================================================
// Type-Safe Disk Operations
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
    tracing::info!(disk = %layout.disk.display(), wipe, "Starting disk preparation");

    // Step 1: Optionally wipe the disk
    if wipe {
        tracing::info!(disk = ?layout.disk, "Wiping disk");
        let wipe_args = WipeDiskArgs {
            device: layout.disk.clone(),
            method: WipeMethod::Quick,
            confirm: confirm_wipe,
        };
        let output = run_script_safe(&wipe_args)
            .context("Failed to execute disk wipe")?;
        output.ensure_success("Disk wipe")?;
        tracing::info!("Disk wipe completed");
    }

    // Step 2: Format EFI/boot partition as FAT32
    tracing::info!(partition = ?layout.boot_partition, fs = "FAT32", "Formatting boot partition");
    let format_boot = FormatPartitionArgs {
        device: layout.boot_partition.clone(),
        filesystem: Filesystem::Fat32,
        label: Some("EFI".to_string()),
        force: false,
    };
    let output = run_script_safe(&format_boot)
        .context("Failed to execute boot partition format")?;
    output.ensure_success("Boot partition format")?;
    tracing::info!("Boot partition formatted");

    // Step 3: Format root partition
    tracing::info!(partition = ?layout.root_partition, fs = ?layout.root_filesystem, "Formatting root partition");
    let format_root = FormatPartitionArgs {
        device: layout.root_partition.clone(),
        filesystem: layout.root_filesystem,
        label: Some("archroot".to_string()),
        force: false,
    };
    let output = run_script_safe(&format_root)
        .context("Failed to execute root partition format")?;
    output.ensure_success("Root partition format")?;
    tracing::info!("Root partition formatted");

    // Step 4: Mount root partition FIRST
    // CRITICAL: Root must be mounted before boot so /mnt/boot exists
    tracing::info!(partition = ?layout.root_partition, mountpoint = ?layout.target_root, "Mounting root partition");
    let mount_root = MountPartitionArgs {
        device: layout.root_partition.clone(),
        mountpoint: layout.target_root.clone(),
        options: None,
    };
    let output = run_script_safe(&mount_root)
        .context("Failed to execute root partition mount")?;
    output.ensure_success("Root partition mount")?;
    tracing::info!("Root partition mounted");

    // Step 5: Create boot mount point and mount boot partition
    let boot_mountpoint = layout.target_root.join("boot");
    tracing::info!(partition = ?layout.boot_partition, mountpoint = ?boot_mountpoint, "Mounting boot partition");

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
    tracing::info!("Boot partition mounted");

    tracing::info!("Disk preparation complete");
    Ok(())
}

// ============================================================================
// Base System Installation
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
/// messages to `tracing::info!`, `tracing::warn!`, and `tracing::error!`. This ensures
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
    tracing::info!(target = ?target_root, "Installing base system");

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
        tracing::warn!(
            "Target root {:?} appears to be empty - ensure it's properly mounted",
            target_root
        );
    }

    // Initialize ALPM with target root
    let db_path = target_root.join("var/lib/pacman");

    // Ensure pacman db directory exists
    std::fs::create_dir_all(&db_path)
        .with_context(|| format!("Failed to create pacman db path: {:?}", db_path))?;

    tracing::info!("Initializing ALPM: root={:?}, db={:?}", target_root, db_path);

    let mut pm = PackageManager::new(target_root, &db_path)
        .context("Failed to initialize ALPM package manager")?;

    // Read pacman.conf from the live system to get mirror list
    // In a real installation, we'd copy mirrorlist to target first
    let live_pacman_conf = Path::new("/etc/pacman.conf");
    if live_pacman_conf.exists() {
        tracing::info!("Loading mirror configuration from live system");
        pm = PackageManager::from_pacman_conf(target_root, live_pacman_conf)
            .context("Failed to load pacman.conf")?;
    } else {
        tracing::warn!("No pacman.conf found - using default mirrors");
    }

    // CRITICAL: Log exactly what we're installing (Self-Audit: linux-firmware included)
    tracing::info!(
        "Installing base packages: {:?}",
        BASE_PACKAGES
    );

    // Install base packages
    // Fail Fast: Any failure here aborts immediately
    pm.install_packages(BASE_PACKAGES)
        .context("Failed to install base packages")?;

    tracing::info!("Base system installation complete");
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
    tracing::info!(extra_count = extra_packages.len(), "Installing base system with extra packages");

    // Combine base packages with extras
    let mut all_packages: Vec<&str> = BASE_PACKAGES.to_vec();
    all_packages.extend_from_slice(extra_packages);

    tracing::info!("Full package list: {:?}", all_packages);

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

    tracing::info!("Package installation complete");
    Ok(())
}

// ============================================================================
// System Configuration
// ============================================================================

/// System configuration parameters for post-install setup.
///
/// Contains all the settings needed to configure the installed system:
/// - Locale and timezone
/// - User accounts
/// - Bootloader
#[derive(Clone)]
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

// ROE §8.1: Custom Debug impl redacts password fields
impl std::fmt::Debug for SystemConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SystemConfig")
            .field("target_root", &self.target_root)
            .field("hostname", &self.hostname)
            .field("locale", &self.locale)
            .field("timezone", &self.timezone)
            .field("keymap", &self.keymap)
            .field("username", &self.username)
            .field("user_password", &"********")
            .field("user_sudo", &self.user_sudo)
            .field("root_password", &"********")
            .finish()
    }
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
/// 4. Install Bootloader (TODO)
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
    tracing::info!(target = ?config.target_root, "Starting system configuration");

    // ========================================================================
    // Step 1: Generate Fstab
    // CRITICAL: Must be done AFTER partitions are mounted, BEFORE chroot/reboot
    // Uses genfstab to read currently mounted filesystems
    // ========================================================================
    tracing::info!("Generating /etc/fstab");
    let fstab_args = GenFstabArgs {
        root: config.target_root.clone(),
    };
    let output = run_script_safe(&fstab_args)
        .context("Failed to execute fstab generation")?;
    output.ensure_success("Fstab generation")?;
    tracing::info!("Fstab generated successfully");

    // ========================================================================
    // Step 2: Configure Hostname and Locale
    // Sets /etc/hostname, /etc/locale.gen, /etc/locale.conf, /etc/localtime
    // ========================================================================
    tracing::info!(
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
    tracing::info!("Locale configuration complete");

    // ========================================================================
    // Step 3: Create User Accounts
    // SECURITY: Passwords via env vars (USER_PASSWORD), NOT CLI flags
    // ========================================================================

    // Create main user with sudo access if requested
    tracing::info!("Creating user: {} (sudo={})", config.username, config.user_sudo);
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
    tracing::info!(user = %config.username, "User created successfully");

    // Set root password if provided
    if let Some(ref root_pw) = config.root_password {
        tracing::info!("Setting root password");
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
        tracing::info!("Root password configured");
    } else {
        tracing::info!("No root password specified - root login will be disabled");
    }

    // ========================================================================
    // Step 4: Install Bootloader (TODO)
    // This will configure GRUB/systemd-boot based on boot mode (UEFI/BIOS)
    // ========================================================================
    tracing::info!("Bootloader installation: TODO");

    tracing::info!("System configuration complete");
    Ok(())
}

// ============================================================================
// LUKS Encryption
// ============================================================================

/// Encryption configuration for a partition.
#[derive(Clone)]
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

// ROE §8.1: Custom Debug impl redacts password field
impl std::fmt::Debug for EncryptionConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncryptionConfig")
            .field("password", &"********")
            .field("mapper_name", &self.mapper_name)
            .field("label", &self.label)
            .field("cipher", &self.cipher)
            .finish()
    }
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
    tracing::info!(device = %device.display(), "Starting LUKS encryption");

    // SECURITY: Create temporary keyfile with password
    // SecretFile ensures cleanup via Drop trait (even on panic)
    let keyfile = SecretFile::new(&config.password)
        .context("Failed to create temporary keyfile")?;

    tracing::info!("Temporary keyfile created: {:?}", keyfile.path());

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

    tracing::info!("Executing LUKS format on {:?}", device);
    let output = run_script_safe(&format_args)
        .context("Failed to execute LUKS format")?;
    output.ensure_success("LUKS format")?;
    tracing::info!("LUKS format completed successfully");

    // Open the encrypted device
    let open_args = LuksOpenArgs {
        device: device.clone(),
        mapper_name: config.mapper_name.clone(),
        key_file: keyfile.path().to_path_buf(),
    };

    tracing::info!("Opening LUKS device as {:?}", config.mapper_name);
    let output = run_script_safe(&open_args)
        .context("Failed to open LUKS device")?;
    output.ensure_success("LUKS open")?;

    // SecretFile is dropped here, securely wiping the keyfile
    let decrypted_device = PathBuf::from(format!("/dev/mapper/{}", config.mapper_name));
    tracing::info!(
        "LUKS encryption complete. Decrypted device: {:?}",
        decrypted_device
    );

    Ok(decrypted_device)
}

// ============================================================================
// Desktop Profile Installation
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
    tracing::info!(profile = ?profile, "Installing profile");

    // Get packages for the profile
    let packages = profile.get_packages();
    tracing::info!(profile = ?profile, count = packages.len(), "Profile packages resolved");

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
        tracing::info!("Enabling display manager: {}", dm);

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

    tracing::info!("Profile {:?} installed successfully", profile);
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
    tracing::info!(
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

    tracing::info!("Dotfiles installed successfully");
    Ok(())
}

// ============================================================================
// Mirror Ranking
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
    tracing::info!("Updating pacman mirrors (country={:?}, limit={}, sort={})",
               country, limit, sort);

    // Check network connectivity first
    tracing::info!("Checking network connectivity...");
    let connectivity = CheckConnectivityArgs::default();
    let output = run_script_safe(&connectivity)
        .context("Failed to check network connectivity")?;

    if !output.success {
        anyhow::bail!(
            "Network connectivity check failed. Cannot update mirrors without network access."
        );
    }
    tracing::info!("Network connectivity OK");

    // Update mirrors
    let args = UpdateMirrorsArgs {
        country: country.map(String::from),
        limit,
        sort,
        protocol: Some("https".to_string()),
        save: true,
    };

    tracing::info!("Running reflector to rank mirrors...");
    let output = run_script_safe(&args)
        .context("Failed to update mirrors")?;
    output.ensure_success("Mirror update")?;

    tracing::info!("Mirrors updated successfully");
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
    tracing::info!("Checking network connectivity...");

    let args = CheckConnectivityArgs::default();
    let output = run_script_safe(&args)
        .context("Failed to check network connectivity")?;

    Ok(output.success)
}

#[cfg(test)]
mod tests {
    use super::strip_ansi_and_cr;

    #[test]
    fn test_strip_plain_text_unchanged() {
        assert_eq!(strip_ansi_and_cr("hello world"), "hello world");
    }

    #[test]
    fn test_strip_csi_color_codes() {
        // Red text: \x1b[31m ... \x1b[0m
        assert_eq!(strip_ansi_and_cr("\x1b[31mERROR\x1b[0m"), "ERROR");
    }

    #[test]
    fn test_strip_multiple_csi_sequences() {
        let input = "\x1b[1m\x1b[36m  [pacstrap] downloading linux\x1b[0m";
        assert_eq!(strip_ansi_and_cr(input), "  [pacstrap] downloading linux");
    }

    #[test]
    fn test_strip_bold_and_color() {
        let input = "\x1b[1;33mWARNING: something\x1b[0m";
        assert_eq!(strip_ansi_and_cr(input), "WARNING: something");
    }

    #[test]
    fn test_strip_carriage_return_keeps_last_segment() {
        // Progress bar: overwrites from start of line
        assert_eq!(strip_ansi_and_cr("old text\rnew text"), "new text");
    }

    #[test]
    fn test_strip_cr_with_ansi() {
        let input = "\x1b[32mProgress: 50%\x1b[0m\r\x1b[32mProgress: 100%\x1b[0m";
        assert_eq!(strip_ansi_and_cr(input), "Progress: 100%");
    }

    #[test]
    fn test_strip_osc_sequence() {
        // OSC: terminal title setting \x1b]0;title\x07
        let input = "\x1b]0;terminal title\x07visible text";
        assert_eq!(strip_ansi_and_cr(input), "visible text");
    }

    #[test]
    fn test_strip_empty_string() {
        assert_eq!(strip_ansi_and_cr(""), "");
    }

    #[test]
    fn test_strip_only_ansi_codes() {
        assert_eq!(strip_ansi_and_cr("\x1b[0m\x1b[31m\x1b[0m"), "");
    }

    #[test]
    fn test_strip_preserves_brackets_in_normal_text() {
        assert_eq!(strip_ansi_and_cr("[INFO] Phase 5: Installing"), "[INFO] Phase 5: Installing");
    }
}
