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
    let mut app = app::App::new();
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
    // Load and validate configuration
    let config = InstallationConfig::load_from_file(config_path)?;
    config.validate()?;

    println!("✓ Configuration loaded and validated");
    println!("🚀 Starting installation with configuration file...");

    // Run the Bash installer with the config file (real-time output)
    let mut child = std::process::Command::new("bash")
        .arg("scripts/install.sh")
        .arg("--config")
        .arg(config_path)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()?;

    let status = child.wait()?;

    if status.success() {
        println!("✓ Installation completed successfully!");
    } else {
        eprintln!("✗ Installation failed with exit code: {:?}", status.code());
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

    // For now, run the normal TUI and show instructions for manual config saving
    // TODO: Implement proper config saving from TUI state
    println!("⚠️  Note: Config saving from TUI is not yet fully implemented.");
    println!("   The TUI will run, but you'll need to manually create a config file.");
    println!("   See the examples in the repository for config file format.");
    println!();

    run_tui_installer()
}
