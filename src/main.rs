//! ArchTUI - Main entry point
//!
//! A clean, modular TUI for Arch Linux installation with proper separation of concerns.
mod app;
mod cli;
mod components;
mod config;
mod config_file;
mod engine;
mod error;
mod hardware;
mod input;
mod install_state;
mod installer;
mod logic;
mod option_help;
#[cfg(feature = "alpm")]
mod package_manager;
mod package_utils;
mod process_guard;
mod profiles;
mod script_manifest;
mod script_runner;
mod script_traits;
mod scripts;
mod scrolling;
mod theme;
mod types;
mod ui;

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::{Terminal, backend::CrosstermBackend};
use std::io::stdout;
use std::path::PathBuf;
use anyhow::Context;
use tracing::{debug, error, info};

use crate::cli::Cli;
use crate::config_file::InstallationConfig;
use crate::process_guard::{ChildRegistry, CommandProcessGroup};
use crate::script_runner::run_script_safe;
use crate::script_traits::ScriptArgs;
use crate::scripts::config::{GenFstabArgs, UserAddArgs};
use crate::scripts::disk::{
    CheckDiskHealthArgs, FormatPartitionArgs, ManualPartitionArgs, MountPartitionsArgs,
    WipeDiskArgs, WipeMethod,
};
use crate::scripts::encryption::{
    LuksCipher, LuksCloseArgs, LuksFormatArgs, LuksOpenArgs, SecretFile,
};
use crate::scripts::network::{
    ConfigureNetworkArgs, FirewallArgs, MirrorSortMethod, NetworkDiagnosticsArgs, TestNetworkArgs,
    UpdateMirrorsArgs,
};
use crate::scripts::profiles::{EnableServicesArgs, InstallDotfilesArgs};
use crate::scripts::system::{BootloaderArgs, ChrootArgs, ServicesArgs, SystemInfoArgs};
use crate::scripts::user::{GroupsArgs, ResetPasswordArgs, SecurityAuditArgs, SshArgs};
use crate::scripts::user_ops::{InstallAurHelperArgs, UserRunArgs};
use crate::types::AurHelper;

/// Initialize the tracing subscriber for CLI mode (writes to stderr)
fn init_logger_cli() {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .init();
}

/// Initialize the tracing subscriber for TUI mode (writes to file to avoid corrupting the terminal)
fn init_logger_tui() {
    use tracing_subscriber::{EnvFilter, fmt};

    let target: Box<dyn std::io::Write + Send> = match std::fs::File::create("/tmp/archtui.log") {
        Ok(file) => Box::new(file),
        Err(_) => {
            // Fall back to silencing logs if we can't open the file
            Box::new(std::io::sink())
        }
    };

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_file(true)
        .with_line_number(true)
        .with_target(false)
        .with_writer(std::sync::Mutex::new(target))
        .init();
}

/// Main application entry point
fn main() -> anyhow::Result<()> {
    // Parse CLI first to determine if we're in TUI or CLI mode
    let cli = Cli::parse_args();

    // Determine if we're entering TUI mode (logs go to file instead of stderr)
    let is_tui_mode = match &cli.command {
        Some(crate::cli::Commands::Validate { .. }) => false,
        Some(crate::cli::Commands::Tools { .. }) => false,
        Some(crate::cli::Commands::Install { config, .. }) => config.is_none(),
        None => true,
    };

    // Set RUST_LOG before logger init so EnvFilter picks it up
    if cli.verbose {
        // SAFETY: set_var is safe here — called once at startup before any threads are spawned
        unsafe { std::env::set_var("RUST_LOG", "debug") };
    }

    if is_tui_mode {
        init_logger_tui();
    } else {
        init_logger_cli();
    }
    info!("ArchTUI starting up");

    // Initialize signal handlers for graceful child process cleanup
    // This ensures bash scripts are terminated if we receive SIGINT/SIGTERM
    if let Err(e) = process_guard::init_signal_handlers() {
        tracing::warn!("Failed to initialize signal handlers: {}", e);
        // Continue anyway - cleanup will still work via Drop
    }
    debug!("Signal handlers initialized");
    debug!("CLI arguments parsed");

    // Fail fast if scripts directory is not found
    let scripts_dir = script_runner::scripts_base_dir();
    if !scripts_dir.join("utils.sh").exists() {
        eprintln!("Fatal: scripts directory not found at {:?}", scripts_dir);
        eprintln!("Run archtui from the repository root or set ARCHTUI_SCRIPTS_DIR");
        std::process::exit(1);
    }
    debug!("Scripts directory verified: {:?}", scripts_dir);

    // Enable dry-run mode if requested
    if cli.dry_run {
        script_traits::enable_dry_run();
        info!("Dry-run mode enabled - destructive operations will be skipped");
        println!("Running in DRY-RUN mode: destructive operations will be skipped");
    }

    // Enable verbose logging for bash scripts (RUST_LOG already set above before logger init)
    if cli.verbose {
        // SAFETY: set_var is safe here — called once at startup before any threads are spawned
        unsafe { std::env::set_var("ARCHTUI_LOG_LEVEL", "VERBOSE") };
        info!("Verbose logging enabled — RUST_LOG=debug, bash LOG_LEVEL=VERBOSE");
    }

    match cli.command {
        Some(crate::cli::Commands::Validate { config }) => {
            info!("Validating configuration file: {:?}", config);
            match InstallationConfig::load_from_file(&config) {
                Ok(config) => match config.validate() {
                    Ok(_) => {
                        info!("Configuration validation successful");
                        println!("✓ Configuration file is valid (all fields validated)");
                    }
                    Err(e) => {
                        error!("Configuration validation failed: {}", e);
                        eprintln!("✗ Configuration validation failed: {}", e);
                        std::process::exit(1);
                    }
                },
                Err(e) => {
                    error!("Failed to load configuration file: {}", e);
                    eprintln!("✗ Failed to load configuration file: {}", e);
                    std::process::exit(1);
                }
            }
        }
        Some(crate::cli::Commands::Install {
            config,
            save_config,
        }) => {
            if let Some(config_path) = config {
                info!(
                    "Running headless installation with config: {:?}",
                    config_path
                );
                run_installer_with_config(&config_path)?;
            } else if let Some(save_path) = save_config {
                info!(
                    "Running TUI installer with config save path: {:?}",
                    save_path
                );
                run_tui_installer_with_save(&save_path)?;
            } else {
                info!("Running TUI installer in interactive mode");
                run_tui_installer()?;
            }
        }
        Some(crate::cli::Commands::Tools { tool }) => {
            debug!("Running tool command");
            run_tool_command(&tool)?;
        }
        None => {
            info!("No command specified, launching TUI installer");
            run_tui_installer()?;
        }
    }

    Ok(())
}

/// Run the TUI installer
fn run_tui_installer() -> anyhow::Result<()> {
    debug!("Initializing terminal for TUI mode");

    // Detect hardware environment before entering TUI
    let hw = hardware::HardwareInfo::detect();
    info!("Hardware detected: {}", hw);

    // Initialize terminal
    enable_raw_mode()
        .map_err(|e| error::general_error(format!("Failed to enable raw mode: {}", e)))?;
    crossterm::execute!(stdout(), crossterm::terminal::EnterAlternateScreen)
        .map_err(|e| error::general_error(format!("Failed to enter alternate screen: {}", e)))?;

    // Create terminal backend
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)
        .map_err(|e| error::general_error(format!("Failed to create terminal: {}", e)))?;

    // Create and run application with detected hardware
    let mut app = app::App::new(None, hw);
    let result = app.run(&mut terminal);

    // Cleanup terminal (always attempt cleanup, even if app failed)
    let _ = disable_raw_mode();
    let _ = crossterm::execute!(stdout(), crossterm::terminal::LeaveAlternateScreen);

    result.map_err(|e| anyhow::anyhow!("{}", e))
}

/// Run installer with configuration file (headless mode)
fn run_installer_with_config(
    config_path: &std::path::Path,
) -> anyhow::Result<()> {
    use std::fs::{self, OpenOptions};
    use std::io::{BufRead, BufReader, Write};
    use std::process::{Command, Stdio};

    info!("Loading configuration from: {:?}", config_path);

    // Load and validate configuration
    let config = InstallationConfig::load_from_file(config_path)?;
    config.validate()?;

    info!("Configuration validated successfully");
    println!("✓ Configuration loaded and validated");
    println!("Starting installation with configuration file...");

    // Pass LOG_LEVEL to child process
    let log_level = std::env::var("ARCHTUI_LOG_LEVEL").unwrap_or_else(|_| "INFO".to_string());

    let script_path = script_runner::scripts_base_dir()
        .join("install.sh")
        .to_string_lossy()
        .to_string();
    info!("Spawning installer script: {}", script_path);

    let mut child = Command::new("/bin/bash")
        .arg(script_path)
        .arg("--config")
        .arg(config_path)
        .env("LOG_LEVEL", &log_level)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .in_new_process_group()
        .spawn()
        .map_err(|e| {
            error!("Failed to spawn installer script: {}", e);
            error::ArchTuiError::script(format!("Failed to spawn installer: {}", e))
        })?;

    // Death Pact: register headless child with ChildRegistry for signal handler cleanup
    let child_pid = child.id();
    if let Ok(mut registry) = ChildRegistry::global().lock() {
        registry.register(child_pid);
    }

    // Create master log for headless mode (best-effort)
    let log_dir = crate::script_runner::log_dir();
    let log_dir = log_dir.to_string_lossy();
    let _ = fs::create_dir_all(log_dir.as_ref());
    let master_log_path = format!(
        "{}/install-{}-master.log",
        log_dir,
        installer::now_hms().replace(':', "")
    );
    let mut master_log = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&master_log_path)
        .ok();

    if let Some(ref mut f) = master_log {
        let _ = writeln!(f, "=== ArchTUI Headless Master Log ===");
        let _ = writeln!(f, "[{}] Config: {:?}", installer::now_hms(), config_path);
        let _ = writeln!(f, "[{}] Log level: {}", installer::now_hms(), log_level);
        let _ = writeln!(f);
    }

    // Capture and print stdout in real-time, writing to master log
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line_content) => {
                    println!("{}", line_content);
                    if let Some(ref mut f) = master_log {
                        let clean = installer::strip_ansi_and_cr(&line_content);
                        let _ = writeln!(f, "[{}] {}", installer::now_hms(), clean);
                    }
                }
                Err(e) => {
                    // If there's an error reading stdout, still wait for the child
                    let _ = child.wait();
                    return Err(e.into());
                }
            }
        }
    }

    // Always wait for the child process to finish
    let output = child
        .wait_with_output()
        .context("Failed to wait for installer subprocess")?;

    // Death Pact: unregister child now that it has exited
    if let Ok(mut registry) = ChildRegistry::global().lock() {
        registry.unregister(child_pid);
    }

    if output.status.success() {
        info!("Installation completed successfully");
        println!("\n✓ Installation completed successfully!");
        if let Some(ref mut f) = master_log {
            let _ = writeln!(
                f,
                "[{}] [RUST] Installation completed successfully",
                installer::now_hms()
            );
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Installation failed. Exit code: {:?}", output.status.code());
        if !stderr.is_empty() {
            error!("Stderr: {}", stderr);
        }
        eprintln!("\n Installation failed");
        if !stderr.is_empty() {
            eprintln!("--- Errors ---");
            eprintln!("{}", stderr);
        }
        if let Some(ref mut f) = master_log {
            let _ = writeln!(
                f,
                "[{}] [RUST] Installation FAILED (exit code {:?})",
                installer::now_hms(),
                output.status.code()
            );
        }
        std::process::exit(1);
    }

    Ok(())
}

/// Run TUI installer and save configuration when done
fn run_tui_installer_with_save(
    save_path: &std::path::Path,
) -> anyhow::Result<()> {
    println!(
        "🎯 TUI installer will save configuration to: {}",
        save_path.display()
    );
    println!("Configure your installation, then the config will be saved automatically!");
    println!(
        "After saving, you can run: ./archtui install --config {}",
        save_path.display()
    );
    println!();

    // Run TUI with save path
    run_tui_installer_with_save_path(save_path)
}

/// Run TUI installer with save path
fn run_tui_installer_with_save_path(
    save_path: &std::path::Path,
) -> anyhow::Result<()> {
    // Detect hardware environment before entering TUI
    let hw = hardware::HardwareInfo::detect();
    info!("Hardware detected: {}", hw);

    // Initialize terminal
    enable_raw_mode()
        .map_err(|e| error::general_error(format!("Failed to enable raw mode: {}", e)))?;
    crossterm::execute!(stdout(), crossterm::terminal::EnterAlternateScreen)
        .map_err(|e| error::general_error(format!("Failed to enter alternate screen: {}", e)))?;

    // Create terminal backend
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)
        .map_err(|e| error::general_error(format!("Failed to create terminal: {}", e)))?;

    // Create and run application with save path and detected hardware
    let mut app = app::App::new(Some(save_path.to_path_buf()), hw);
    let result = app.run(&mut terminal);

    // Cleanup terminal (always attempt cleanup, even if app failed)
    let _ = disable_raw_mode();
    let _ = crossterm::execute!(stdout(), crossterm::terminal::LeaveAlternateScreen);

    result.map_err(|e| anyhow::anyhow!("{}", e))
}

/// Run tool command — dispatches to category-specific handlers
fn run_tool_command(tool: &crate::cli::ToolCommands) -> anyhow::Result<()> {
    match tool {
        crate::cli::ToolCommands::Disk { disk_tool } => dispatch_disk_tool(disk_tool),
        crate::cli::ToolCommands::System { system_tool } => dispatch_system_tool(system_tool),
        crate::cli::ToolCommands::User { user_tool } => dispatch_user_tool(user_tool),
        crate::cli::ToolCommands::Network { network_tool } => dispatch_network_tool(network_tool),
    }
}

/// Dispatch disk tool subcommands
fn dispatch_disk_tool(disk_tool: &crate::cli::DiskToolCommands) -> anyhow::Result<()> {
    match disk_tool {
            crate::cli::DiskToolCommands::Format {
                device,
                filesystem,
                label,
            } => {
                // Parse filesystem string into typed enum
                let fs: crate::types::Filesystem = filesystem.parse().unwrap_or_else(|_| {
                    eprintln!("❌ Invalid filesystem: {}", filesystem);
                    eprintln!("   Valid types: ext4, xfs, btrfs, fat32");
                    std::process::exit(1);
                });
                let format_args = FormatPartitionArgs {
                    device: PathBuf::from(device),
                    filesystem: fs,
                    label: label.clone(),
                    force: false,
                };
                execute_tool(&format_args)?;
            }
            crate::cli::DiskToolCommands::Wipe {
                device,
                method,
                confirm,
            } => {
                if !confirm {
                    eprintln!("❌ Wipe operation requires --confirm flag");
                    std::process::exit(1);
                }

                // Parse method string into typed enum
                let wipe_method: WipeMethod = method.parse().unwrap_or_else(|e| {
                    eprintln!("❌ {}", e);
                    eprintln!("   Valid methods: quick, secure, auto");
                    std::process::exit(1);
                });

                // Use typed args - compiler enforces correct flag names
                let wipe_args = WipeDiskArgs {
                    device: PathBuf::from(device),
                    method: wipe_method,
                    confirm: *confirm,
                };

                execute_tool(&wipe_args)?;
            }
            crate::cli::DiskToolCommands::Health { device } => {
                let health_args = CheckDiskHealthArgs {
                    device: PathBuf::from(device),
                };
                execute_tool(&health_args)?;
            }
            crate::cli::DiskToolCommands::Mount {
                action,
                device,
                mountpoint,
                filesystem,
            } => {
                let mount_args = MountPartitionsArgs {
                    action: action.clone(),
                    device: PathBuf::from(device),
                    mountpoint: mountpoint.as_ref().map(PathBuf::from),
                    filesystem: filesystem.clone(),
                };
                execute_tool(&mount_args)?;
            }
            crate::cli::DiskToolCommands::Manual { device } => {
                let manual_args = ManualPartitionArgs {
                    device: PathBuf::from(device),
                };
                execute_tool(&manual_args)?;
            }
            crate::cli::DiskToolCommands::Encrypt {
                action,
                device,
                mapper,
            } => {
                match action.as_str() {
                    "format" => {
                        let dev = device.as_ref().unwrap_or_else(|| {
                            eprintln!("❌ --device is required for format action");
                            std::process::exit(1);
                        });
                        // Password is read from stdin for CLI
                        eprintln!("Enter LUKS passphrase:");
                        let mut password = String::new();
                        std::io::stdin()
                            .read_line(&mut password)
                            .context("Failed to read LUKS passphrase from stdin")?;
                        let password = password.trim().to_string();
                        let secret_file = SecretFile::new(&password)
                            .context("Failed to create temporary keyfile")?;
                        let format_args = LuksFormatArgs {
                            device: PathBuf::from(dev),
                            key_file: secret_file.path().to_path_buf(),
                            cipher: LuksCipher::default(),
                            label: None,
                            confirm: true,
                        };
                        execute_tool(&format_args)?;
                        // SecretFile dropped here, cleaned up
                    }
                    "open" => {
                        let dev = device.as_ref().unwrap_or_else(|| {
                            eprintln!("❌ --device is required for open action");
                            std::process::exit(1);
                        });
                        eprintln!("Enter LUKS passphrase:");
                        let mut password = String::new();
                        std::io::stdin()
                            .read_line(&mut password)
                            .context("Failed to read LUKS passphrase from stdin")?;
                        let password = password.trim().to_string();
                        let secret_file = SecretFile::new(&password)
                            .context("Failed to create temporary keyfile")?;
                        let open_args = LuksOpenArgs {
                            device: PathBuf::from(dev),
                            key_file: secret_file.path().to_path_buf(),
                            mapper_name: mapper.clone(),
                        };
                        execute_tool(&open_args)?;
                    }
                    "close" => {
                        let close_args = LuksCloseArgs {
                            mapper_name: mapper.clone(),
                        };
                        execute_tool(&close_args)?;
                    }
                    _ => {
                        eprintln!("❌ Invalid action: {}. Valid: format, open, close", action);
                        std::process::exit(1);
                    }
                }
            }
    }
    Ok(())
}

/// Dispatch system tool subcommands
fn dispatch_system_tool(system_tool: &crate::cli::SystemToolCommands) -> anyhow::Result<()> {
    match system_tool {
            crate::cli::SystemToolCommands::Bootloader {
                r#type,
                disk,
                efi_path,
                mode,
            } => {
                let bootloader_args = BootloaderArgs {
                    bootloader_type: r#type.clone(),
                    disk: PathBuf::from(disk),
                    mode: mode.clone(),
                    efi_path: efi_path.as_ref().map(PathBuf::from),
                };
                execute_tool(&bootloader_args)?;
            }
            crate::cli::SystemToolCommands::Fstab { root } => {
                let fstab_args = GenFstabArgs {
                    root: PathBuf::from(root),
                };
                execute_tool(&fstab_args)?;
            }
            crate::cli::SystemToolCommands::Chroot { root, no_mount } => {
                let chroot_args = ChrootArgs {
                    root: PathBuf::from(root),
                    no_mount: *no_mount,
                };
                execute_tool(&chroot_args)?;
            }
            crate::cli::SystemToolCommands::Info { detailed } => {
                let info_args = SystemInfoArgs {
                    detailed: *detailed,
                };
                execute_tool(&info_args)?;
            }
            crate::cli::SystemToolCommands::Services { action, service } => {
                let services_args = ServicesArgs {
                    action: action.clone(),
                    service: service.clone(),
                };
                execute_tool(&services_args)?;
            }
            crate::cli::SystemToolCommands::EnableServices { services, root } => {
                let service_list: Vec<String> =
                    services.split(',').map(|s| s.trim().to_string()).collect();
                let enable_args = EnableServicesArgs {
                    services: service_list,
                    root: PathBuf::from(root),
                };
                execute_tool(&enable_args)?;
            }
            crate::cli::SystemToolCommands::AurHelper { helper, user, root } => {
                let aur_helper: AurHelper = match helper.to_lowercase().as_str() {
                    "paru" => AurHelper::Paru,
                    "yay" => AurHelper::Yay,
                    "pikaur" => AurHelper::Pikaur,
                    _ => {
                        eprintln!("Invalid AUR helper: {}. Valid: paru, yay, pikaur", helper);
                        std::process::exit(1);
                    }
                };
                let aur_args = InstallAurHelperArgs {
                    helper: aur_helper,
                    target_user: user.clone(),
                    chroot_path: PathBuf::from(root),
                };
                execute_tool(&aur_args)?;
            }
    }
    Ok(())
}

/// Dispatch user tool subcommands
fn dispatch_user_tool(user_tool: &crate::cli::UserToolCommands) -> anyhow::Result<()> {
    match user_tool {
            crate::cli::UserToolCommands::Add {
                username,
                full_name,
                groups,
                shell,
            } => {
                let add_user_args = UserAddArgs {
                    username: username.clone(),
                    password: None,
                    groups: groups.clone(),
                    shell: Some(shell.clone()),
                    full_name: full_name.clone(),
                    home_dir: None,
                    create_home: true,
                    sudo: false,
                };
                execute_tool(&add_user_args)?;
            }
            crate::cli::UserToolCommands::ResetPassword { username } => {
                // Read password from env var, never from CLI args (/proc/PID/cmdline)
                let password = std::env::var("USER_PASSWORD").unwrap_or_else(|_| {
                    eprintln!("❌ USER_PASSWORD environment variable must be set");
                    eprintln!("   Usage: USER_PASSWORD='secret' archtui tool user reset-password -u <user>");
                    std::process::exit(1);
                });
                let reset_args = ResetPasswordArgs {
                    username: username.clone(),
                    password,
                };
                execute_tool(&reset_args)?;
            }
            crate::cli::UserToolCommands::Groups {
                action,
                user,
                group,
            } => {
                let groups_args = GroupsArgs {
                    action: action.clone(),
                    user: user.clone(),
                    group: group.clone(),
                };
                execute_tool(&groups_args)?;
            }
            crate::cli::UserToolCommands::Ssh {
                action,
                port,
                root_login,
                password_auth,
            } => {
                let ssh_args = SshArgs {
                    action: action.clone(),
                    port: *port,
                    enable_root_login: *root_login,
                    enable_password_auth: *password_auth,
                };
                execute_tool(&ssh_args)?;
            }
            crate::cli::UserToolCommands::Security { action } => {
                let security_args = SecurityAuditArgs {
                    action: action.clone(),
                };
                execute_tool(&security_args)?;
            }
            crate::cli::UserToolCommands::Dotfiles {
                repo,
                user,
                branch,
                backup,
            } => {
                let dotfiles_args = InstallDotfilesArgs {
                    repo_url: repo.clone(),
                    target_user: user.clone(),
                    target_dir: None,
                    branch: branch.clone(),
                    backup: *backup,
                };
                execute_tool(&dotfiles_args)?;
            }
            crate::cli::UserToolCommands::RunAs { user, cmd, root } => {
                let run_args = UserRunArgs {
                    user: user.clone(),
                    command: cmd.clone(),
                    chroot_path: PathBuf::from(root),
                    workdir: None,
                };
                execute_tool(&run_args)?;
            }
    }
    Ok(())
}

/// Dispatch network tool subcommands
fn dispatch_network_tool(network_tool: &crate::cli::NetworkToolCommands) -> anyhow::Result<()> {
    match network_tool {
            crate::cli::NetworkToolCommands::Configure {
                interface,
                ip,
                gateway,
            } => {
                let network_args = ConfigureNetworkArgs {
                    interface: interface.clone(),
                    ip: ip.clone(),
                    gateway: gateway.clone(),
                };
                execute_tool(&network_args)?;
            }
            crate::cli::NetworkToolCommands::Test {
                action,
                host,
                timeout,
            } => {
                let test_args = TestNetworkArgs {
                    action: action.clone(),
                    host: host.clone(),
                    timeout: u32::from(*timeout),
                };
                execute_tool(&test_args)?;
            }
            crate::cli::NetworkToolCommands::Firewall {
                action,
                r#type,
                port,
                protocol,
                allow,
                deny,
            } => {
                let firewall_args = FirewallArgs {
                    action: action.clone(),
                    firewall_type: r#type.clone(),
                    port: *port,
                    protocol: protocol.clone(),
                    allow: *allow,
                    deny: *deny,
                };
                execute_tool(&firewall_args)?;
            }
            crate::cli::NetworkToolCommands::Diagnostics { action } => {
                let diagnostics_args = NetworkDiagnosticsArgs {
                    action: action.clone(),
                };
                execute_tool(&diagnostics_args)?;
            }
            crate::cli::NetworkToolCommands::Mirrors {
                country,
                limit,
                sort,
            } => {
                let sort_method = match sort.to_lowercase().as_str() {
                    "rate" => MirrorSortMethod::Rate,
                    "age" => MirrorSortMethod::Age,
                    "score" => MirrorSortMethod::Score,
                    "country" => MirrorSortMethod::Country,
                    _ => {
                        eprintln!(
                            "❌ Invalid sort method: {}. Valid: rate, age, score, country",
                            sort
                        );
                        std::process::exit(1);
                    }
                };
                let mirrors_args = UpdateMirrorsArgs {
                    country: country.clone(),
                    limit: *limit,
                    sort: sort_method,
                    protocol: Some("https".to_string()),
                    save: true,
                };
                execute_tool(&mirrors_args)?;
            }
    }
    Ok(())
}

/// Execute a tool script with typed arguments and print output (CLI helper).
///
/// This wraps the shared `run_script_safe` from `script_runner` module
/// to provide CLI-friendly output and process exit on failure.
fn execute_tool<T: ScriptArgs>(args: &T) -> anyhow::Result<()> {
    let script_name = args.script_name();
    let cli_args = args.to_cli_args();
    let env_vars = args.get_env_vars();

    // Print what we're executing
    println!(
        "🔧 Executing: scripts/tools/{} {}",
        script_name,
        cli_args.join(" ")
    );
    if !env_vars.is_empty() {
        println!(
            "   ENV: {}",
            env_vars
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect::<Vec<_>>()
                .join(" ")
        );
    }

    // Execute via shared runner
    let output = run_script_safe(args)?;

    // Print stdout
    if !output.stdout.is_empty() {
        print!("{}", output.stdout);
    }

    // Print stderr
    if !output.stderr.is_empty() {
        eprint!("{}", output.stderr);
    }

    if output.success {
        info!("Tool {} executed successfully", script_name);
        println!("✅ Tool executed successfully");
        Ok(())
    } else {
        error!(
            "Tool {} execution failed with exit code: {:?}",
            script_name, output.exit_code
        );
        eprintln!("❌ Tool execution failed");
        std::process::exit(1);
    }
}
