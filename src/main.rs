//! ArchInstall TUI - Main entry point
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
use log::{debug, error, info};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;
use std::path::PathBuf;

use crate::cli::Cli;
use crate::config_file::InstallationConfig;
use crate::process_guard::CommandProcessGroup;
use crate::script_runner::run_script_safe;
use crate::script_traits::ScriptArgs;
use crate::scripts::disk::{
    CheckDiskHealthArgs, FormatPartitionArgs, ManualPartitionArgs, MountPartitionsArgs,
    WipeDiskArgs, WipeMethod,
};
use crate::scripts::network::{
    ConfigureNetworkArgs, FirewallArgs, NetworkDiagnosticsArgs, TestNetworkArgs,
};
use crate::scripts::system::{
    BootloaderArgs, ChrootArgs, FstabArgs, ServicesArgs, SystemInfoArgs,
};
use crate::scripts::user::{
    AddUserArgs, GroupsArgs, ResetPasswordArgs, SecurityAuditArgs, SshArgs,
};

/// Initialize the logger with appropriate settings
fn init_logger() {
    use env_logger::Builder;
    use std::io::Write;

    Builder::from_default_env()
        .format(|buf, record| {
            writeln!(
                buf,
                "[{} {}:{}] {}",
                record.level(),
                record.file().unwrap_or("unknown"),
                record.line().unwrap_or(0),
                record.args()
            )
        })
        .filter_level(log::LevelFilter::Info)
        .parse_default_env() // Allows RUST_LOG env var to override
        .init();
}

/// Main application entry point
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging first
    init_logger();
    info!("ArchInstall TUI starting up");

    // Initialize signal handlers for graceful child process cleanup
    // This ensures bash scripts are terminated if we receive SIGINT/SIGTERM
    if let Err(e) = process_guard::init_signal_handlers() {
        log::warn!("Failed to initialize signal handlers: {}", e);
        // Continue anyway - cleanup will still work via Drop
    }
    debug!("Signal handlers initialized");

    let cli = Cli::parse_args();
    debug!("CLI arguments parsed");

    // Enable dry-run mode if requested
    if cli.dry_run {
        script_traits::enable_dry_run();
        info!("Dry-run mode enabled - destructive operations will be skipped");
        println!("Running in DRY-RUN mode: destructive operations will be skipped");
    }

    match cli.command {
        Some(crate::cli::Commands::Validate { config }) => {
            info!("Validating configuration file: {:?}", config);
            match InstallationConfig::load_from_file(&config) {
                Ok(config) => match config.validate() {
                    Ok(_) => {
                        info!("Configuration validation successful");
                        println!("✓ Configuration file is valid: {:?}", config);
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
                info!("Running headless installation with config: {:?}", config_path);
                run_installer_with_config(&config_path)?;
            } else if let Some(save_path) = save_config {
                info!("Running TUI installer with config save path: {:?}", save_path);
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
fn run_tui_installer() -> Result<(), Box<dyn std::error::Error>> {
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

    result
}

/// Run installer with configuration file (headless mode)
fn run_installer_with_config(
    config_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{BufRead, BufReader};
    use std::process::{Command, Stdio};

    info!("Loading configuration from: {:?}", config_path);

    // Load and validate configuration
    let config = InstallationConfig::load_from_file(config_path)?;
    config.validate()?;

    info!("Configuration validated successfully");
    println!("✓ Configuration loaded and validated");
    println!("🚀 Starting installation with configuration file...");

    let script_path = "./scripts/install.sh";
    info!("Spawning installer script: {}", script_path);

    let mut child = Command::new("bash")
        .arg(script_path)
        .arg("--config")
        .arg(config_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .in_new_process_group()
        .spawn()
        .map_err(|e| {
            error!("Failed to spawn installer script: {}", e);
            error::ArchInstallError::script(format!("Failed to spawn installer: {}", e))
        })?;

    // Capture and print stdout in real-time
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            match line {
                Ok(line_content) => println!("{}", line_content),
                Err(e) => {
                    // If there's an error reading stdout, still wait for the child
                    let _ = child.wait();
                    return Err(e.into());
                }
            }
        }
    }

    // Always wait for the child process to finish
    let output = child.wait_with_output()?;

    if output.status.success() {
        info!("Installation completed successfully");
        println!("\n✓ Installation completed successfully!");
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("Installation failed. Exit code: {:?}", output.status.code());
        if !stderr.is_empty() {
            error!("Stderr: {}", stderr);
        }
        eprintln!("\n✗ Installation failed");
        if !stderr.is_empty() {
            eprintln!("--- Errors ---");
            eprintln!("{}", stderr);
        }
        std::process::exit(1);
    }

    Ok(())
}

/// Run TUI installer and save configuration when done
fn run_tui_installer_with_save(
    save_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "🎯 TUI installer will save configuration to: {}",
        save_path.display()
    );
    println!("Configure your installation, then the config will be saved automatically!");
    println!(
        "After saving, you can run: ./archinstall-tui install --config {}",
        save_path.display()
    );
    println!();

    // Run TUI with save path
    run_tui_installer_with_save_path(save_path)
}

/// Run TUI installer with save path
fn run_tui_installer_with_save_path(
    save_path: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
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

    result
}

/// Run tool command
fn run_tool_command(tool: &crate::cli::ToolCommands) -> Result<(), Box<dyn std::error::Error>> {
    match tool {
        crate::cli::ToolCommands::Disk { disk_tool } => match disk_tool {
            crate::cli::DiskToolCommands::Format {
                device,
                filesystem,
                label,
            } => {
                // Parse filesystem string into typed enum
                let fs: crate::types::Filesystem = filesystem.parse().unwrap_or_else(|_| {
                    eprintln!("❌ Invalid filesystem: {}", filesystem);
                    eprintln!("   Valid types: ext4, xfs, btrfs, f2fs, fat32");
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
                    mountpoint: mountpoint.as_ref().map(|p| PathBuf::from(p)),
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
        },
        crate::cli::ToolCommands::System { system_tool } => match system_tool {
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
                    efi_path: efi_path.as_ref().map(|p| PathBuf::from(p)),
                };
                execute_tool(&bootloader_args)?;
            }
            crate::cli::SystemToolCommands::Fstab { root } => {
                let fstab_args = FstabArgs {
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
        },
        crate::cli::ToolCommands::User { user_tool } => match user_tool {
            crate::cli::UserToolCommands::Add {
                username,
                full_name,
                groups,
                shell,
            } => {
                let add_user_args = AddUserArgs {
                    username: username.clone(),
                    shell: shell.clone(),
                    full_name: full_name.clone(),
                    groups: groups.clone(),
                };
                execute_tool(&add_user_args)?;
            }
            crate::cli::UserToolCommands::ResetPassword { username } => {
                let reset_args = ResetPasswordArgs {
                    username: username.clone(),
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
        },
        crate::cli::ToolCommands::Network { network_tool } => match network_tool {
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
        },
    }
    Ok(())
}
/// Execute a tool script with typed arguments and print output (CLI helper).
///
/// This wraps the shared `run_script_safe` from `script_runner` module
/// to provide CLI-friendly output and process exit on failure.
fn execute_tool<T: ScriptArgs>(args: &T) -> Result<(), Box<dyn std::error::Error>> {
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
