//! ArchInstall TUI - Main entry point
//!
//! A clean, modular TUI for Arch Linux installation with proper separation of concerns.

mod app;
mod cli;
mod config;
mod config_file;
mod error;
mod input;
mod installer;
mod package_utils;
mod scrolling;
mod ui;

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::stdout;

use crate::cli::Cli;
use crate::config_file::InstallationConfig;

/// Main application entry point
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse_args();

    match cli.command {
        Some(crate::cli::Commands::Validate { config }) => {
            // Validate configuration file
            match InstallationConfig::load_from_file(&config) {
                Ok(config) => match config.validate() {
                    Ok(_) => println!("✓ Configuration file is valid: {:?}", config),
                    Err(e) => {
                        eprintln!("✗ Configuration validation failed: {}", e);
                        std::process::exit(1);
                    }
                },
                Err(e) => {
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
                // Run installer with config file (skip TUI)
                run_installer_with_config(&config_path)?;
            } else if let Some(save_path) = save_config {
                // Run TUI and save config when done
                run_tui_installer_with_save(&save_path)?;
            } else {
                // Run normal TUI installer
                run_tui_installer()?;
            }
        }
        Some(crate::cli::Commands::Tools { tool }) => {
            // Handle tool commands
            run_tool_command(&tool)?;
        }
        None => {
            // Run the TUI installer (default behavior)
            run_tui_installer()?;
        }
    }

    Ok(())
}

/// Run the TUI installer
fn run_tui_installer() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize terminal
    enable_raw_mode()
        .map_err(|e| error::general_error(format!("Failed to enable raw mode: {}", e)))?;
    crossterm::execute!(stdout(), crossterm::terminal::EnterAlternateScreen)
        .map_err(|e| error::general_error(format!("Failed to enter alternate screen: {}", e)))?;

    // Create terminal backend
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)
        .map_err(|e| error::general_error(format!("Failed to create terminal: {}", e)))?;

    // Create and run application
    let mut app = app::App::new(None);
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

    // Load and validate configuration
    let config = InstallationConfig::load_from_file(config_path)?;
    config.validate()?;

    println!("✓ Configuration loaded and validated");
    println!("🚀 Starting installation with configuration file...");

    let script_path = "./scripts/install.sh";
    let mut child = Command::new("bash")
        .arg(script_path)
        .arg("--config")
        .arg(config_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn installer script");

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
        println!("\n✓ Installation completed successfully!");
    } else {
        eprintln!("\n✗ Installation failed");
        // Print any remaining stderr
        let stderr = String::from_utf8_lossy(&output.stderr);
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
    // Initialize terminal
    enable_raw_mode()
        .map_err(|e| error::general_error(format!("Failed to enable raw mode: {}", e)))?;
    crossterm::execute!(stdout(), crossterm::terminal::EnterAlternateScreen)
        .map_err(|e| error::general_error(format!("Failed to enter alternate screen: {}", e)))?;

    // Create terminal backend
    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)
        .map_err(|e| error::general_error(format!("Failed to create terminal: {}", e)))?;

    // Create and run application with save path
    let mut app = app::App::new(Some(save_path.to_path_buf()));
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
                let mut args = vec!["--device", device, "--filesystem", filesystem];
                if let Some(label) = label {
                    args.extend(&["--label", label]);
                }
                execute_tool_script("format_partition.sh", &args)?;
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
                let args = vec!["--device", device, "--method", method, "--confirm"];
                execute_tool_script("wipe_disk.sh", &args)?;
            }
            crate::cli::DiskToolCommands::Health { device } => {
                let args = vec!["--device", device];
                execute_tool_script("check_disk_health.sh", &args)?;
            }
            crate::cli::DiskToolCommands::Mount {
                action,
                device,
                mountpoint,
                filesystem,
            } => {
                let mut args = vec!["--action", action, "--device", device];
                if let Some(mountpoint) = mountpoint {
                    args.extend(&["--mountpoint", mountpoint]);
                }
                if let Some(filesystem) = filesystem {
                    args.extend(&["--filesystem", filesystem]);
                }
                execute_tool_script("mount_partitions.sh", &args)?;
            }
            crate::cli::DiskToolCommands::Manual { device } => {
                let args = vec!["--device", device];
                execute_tool_script("manual_partition.sh", &args)?;
            }
        },
        crate::cli::ToolCommands::System { system_tool } => match system_tool {
            crate::cli::SystemToolCommands::Bootloader {
                r#type,
                disk,
                efi_path,
                mode,
            } => {
                let mut args = vec!["--type", r#type, "--disk", disk, "--mode", mode];
                if let Some(efi_path) = efi_path {
                    args.extend(&["--efi-path", efi_path]);
                }
                execute_tool_script("install_bootloader.sh", &args)?;
            }
            crate::cli::SystemToolCommands::Fstab { root } => {
                let args = vec!["--root", root];
                execute_tool_script("generate_fstab.sh", &args)?;
            }
            crate::cli::SystemToolCommands::Chroot { root, no_mount } => {
                let mut args = vec!["--root", root];
                if *no_mount {
                    args.push("--no-mount");
                }
                execute_tool_script("chroot_system.sh", &args)?;
            }
            crate::cli::SystemToolCommands::Info { detailed } => {
                let mut args = vec![];
                if *detailed {
                    args.push("--detailed");
                }
                execute_tool_script("system_info.sh", &args)?;
            }
            crate::cli::SystemToolCommands::Services { action, service } => {
                let mut args = vec!["--action", action];
                if let Some(svc) = service {
                    args.extend(&["--service", svc]);
                }
                execute_tool_script("manage_services.sh", &args)?;
            }
        },
        crate::cli::ToolCommands::User { user_tool } => match user_tool {
            crate::cli::UserToolCommands::Add {
                username,
                full_name,
                groups,
                shell,
            } => {
                let mut args = vec!["--username", username, "--shell", shell];
                if let Some(full_name) = full_name {
                    args.extend(&["--full-name", full_name]);
                }
                if let Some(groups) = groups {
                    args.extend(&["--groups", groups]);
                }
                execute_tool_script("add_user.sh", &args)?;
            }
            crate::cli::UserToolCommands::ResetPassword { username } => {
                let args = vec!["--username", username];
                execute_tool_script("reset_password.sh", &args)?;
            }
            crate::cli::UserToolCommands::Groups {
                action,
                user,
                group,
            } => {
                let mut args = vec!["--action", action];
                if let Some(u) = user {
                    args.extend(&["--user", u]);
                }
                if let Some(g) = group {
                    args.extend(&["--group", g]);
                }
                execute_tool_script("manage_groups.sh", &args)?;
            }
            crate::cli::UserToolCommands::Ssh {
                action,
                port,
                root_login,
                password_auth,
            } => {
                let mut args: Vec<String> = vec!["--action".to_string(), action.to_string()];
                if let Some(p) = port {
                    args.push("--port".to_string());
                    args.push(p.to_string());
                }
                if let Some(rl) = root_login {
                    if *rl {
                        args.push("--enable-root-login".to_string());
                    } else {
                        args.push("--disable-root-login".to_string());
                    }
                }
                if let Some(pa) = password_auth {
                    if *pa {
                        args.push("--enable-password-auth".to_string());
                    } else {
                        args.push("--disable-password-auth".to_string());
                    }
                }
                let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                execute_tool_script("configure_ssh.sh", &args_refs)?;
            }
            crate::cli::UserToolCommands::Security { action } => {
                let args = vec!["--action", action];
                execute_tool_script("security_audit.sh", &args)?;
            }
        },
        crate::cli::ToolCommands::Network { network_tool } => match network_tool {
            crate::cli::NetworkToolCommands::Configure {
                interface,
                ip,
                gateway,
            } => {
                let mut args = vec!["--interface", interface];
                if let Some(ip) = ip {
                    args.extend(&["--ip", ip]);
                }
                if let Some(gateway) = gateway {
                    args.extend(&["--gateway", gateway]);
                }
                execute_tool_script("configure_network.sh", &args)?;
            }
            crate::cli::NetworkToolCommands::Test {
                action,
                host,
                timeout,
            } => {
                let mut args: Vec<String> = vec!["--action".to_string(), action.to_string()];
                if let Some(h) = host {
                    args.push("--host".to_string());
                    args.push(h.to_string());
                }
                args.push("--timeout".to_string());
                args.push(timeout.to_string());
                let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                execute_tool_script("test_network.sh", &args_refs)?;
            }
            crate::cli::NetworkToolCommands::Firewall {
                action,
                r#type,
                port,
                protocol,
                allow,
                deny,
            } => {
                let mut args: Vec<String> = vec![
                    "--action".to_string(),
                    action.to_string(),
                    "--type".to_string(),
                    r#type.to_string(),
                ];
                if let Some(p) = port {
                    args.push("--port".to_string());
                    args.push(p.to_string());
                }
                args.push("--protocol".to_string());
                args.push(protocol.to_string());
                if *allow {
                    args.push("--allow".to_string());
                }
                if *deny {
                    args.push("--deny".to_string());
                }
                let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                execute_tool_script("configure_firewall.sh", &args_refs)?;
            }
            crate::cli::NetworkToolCommands::Diagnostics { action } => {
                let args = vec!["--action", action];
                execute_tool_script("network_diagnostics.sh", &args)?;
            }
        },
    }
    Ok(())
}

/// Execute a tool script with arguments
fn execute_tool_script(script_name: &str, args: &[&str]) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::{Command, Stdio};

    let script_path = format!("scripts/tools/{}", script_name);
    println!("🔧 Executing: {} {}", script_path, args.join(" "));

    let output = Command::new("bash")
        .arg(&script_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;

    // Print stdout
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !stdout.is_empty() {
        print!("{}", stdout);
    }

    // Print stderr
    let stderr = String::from_utf8_lossy(&output.stderr);
    if !stderr.is_empty() {
        eprint!("{}", stderr);
    }

    if output.status.success() {
        println!("✅ Tool executed successfully");
    } else {
        eprintln!("❌ Tool execution failed");
        std::process::exit(1);
    }

    Ok(())
}
