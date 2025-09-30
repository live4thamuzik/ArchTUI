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
            println!("{}", line?);
        }
    }

    // Capture and print stderr after the process finishes
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
    println!("Configure your installation, then the config will be saved and you can run:");
    println!(
        "  ./archinstall-tui install --config {}",
        save_path.display()
    );

    // Run TUI with save path
    run_tui_installer_with_save_path(save_path)?;

    Ok(())
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
        crate::cli::ToolCommands::Disk { disk_tool } => {
            match disk_tool {
                crate::cli::DiskToolCommands::Format { device, filesystem, label } => {
                    let mut args = vec!["--device", device, "--filesystem", filesystem];
                    if let Some(label) = label {
                        args.extend(&["--label", label]);
                    }
                    execute_tool_script("format_partition.sh", &args)?;
                }
                crate::cli::DiskToolCommands::Wipe { device, method, confirm } => {
                    if !confirm {
                        eprintln!("❌ Wipe operation requires --confirm flag");
                        std::process::exit(1);
                    }
                    let args = vec!["--device", device, "--method", method, "--confirm"];
                    execute_tool_script("wipe_disk.sh", &args)?;
                }
            }
        }
        crate::cli::ToolCommands::System { system_tool } => {
            match system_tool {
                crate::cli::SystemToolCommands::Bootloader { r#type, disk, efi_path, mode } => {
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
            }
        }
        crate::cli::ToolCommands::User { user_tool } => {
            match user_tool {
                crate::cli::UserToolCommands::Add { username, full_name, groups, shell } => {
                    let mut args = vec!["--username", username, "--shell", shell];
                    if let Some(full_name) = full_name {
                        args.extend(&["--full-name", full_name]);
                    }
                    if let Some(groups) = groups {
                        args.extend(&["--groups", groups]);
                    }
                    execute_tool_script("add_user.sh", &args)?;
                }
            }
        }
        crate::cli::ToolCommands::Network { network_tool } => {
            match network_tool {
                crate::cli::NetworkToolCommands::Configure { interface, ip, gateway } => {
                    println!("🔧 Network configuration tool not yet implemented");
                    println!("Interface: {}", interface);
                    if let Some(ip) = ip {
                        println!("IP: {}", ip);
                    }
                    if let Some(gateway) = gateway {
                        println!("Gateway: {}", gateway);
                    }
                }
            }
        }
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