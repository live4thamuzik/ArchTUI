//! Installer module
//!
//! Handles the execution of the bash installation script and communication with the TUI.

use crate::app::AppState;
use crate::config::Configuration;
use std::io::{BufRead, BufReader};
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
