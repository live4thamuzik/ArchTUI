//! Application state management and main event loop
//!
//! Handles the main application lifecycle, event processing, and state transitions.

use crate::config::Configuration;
use crate::error;
use crate::input::InputHandler;
use crate::installer::Installer;
use crate::ui::UiRenderer;
use crossterm::event::{Event, KeyCode, KeyEvent};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Main application state
#[derive(Debug, Clone)]
pub struct AppState {
    /// Current application mode
    pub mode: AppMode,
    /// Configuration options
    pub config: Configuration,
    /// Scroll state for configuration list
    pub config_scroll: crate::scrolling::ScrollState,
    /// Status message for user feedback
    pub status_message: String,
    /// Installer output lines
    pub installer_output: Vec<String>,
    /// Installation progress percentage
    pub installation_progress: u8,
}

/// Application operating modes
#[derive(Debug, Clone, PartialEq)]
pub enum AppMode {
    /// Configuration phase - user setting up installation options
    Configuration,
    /// Installation phase - running the actual installation
    Installation,
    /// Installation complete
    Complete,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            mode: AppMode::Configuration,
            config: Configuration::default(),
            config_scroll: crate::scrolling::ScrollState::new(42, 30), // 42 config options, default 30 visible
            status_message: "Configure installation options".to_string(),
            installer_output: Vec::new(),
            installation_progress: 0,
        }
    }
}

/// Main application struct
pub struct App {
    state: Arc<Mutex<AppState>>,
    installer: Option<Installer>,
    ui_renderer: UiRenderer,
    input_handler: InputHandler,
    save_config_path: Option<std::path::PathBuf>,
}

impl App {
    /// Helper function to safely lock the state mutex
    fn lock_state(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, AppState>, Box<dyn std::error::Error>> {
        self.state
            .lock()
            .map_err(|e| error::general_error(format!("Mutex poisoned: {}", e)).into())
    }

    /// Helper function to safely lock the state mutex mutably
    fn lock_state_mut(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, AppState>, Box<dyn std::error::Error>> {
        self.state
            .lock()
            .map_err(|e| error::general_error(format!("Mutex poisoned: {}", e)).into())
    }

    /// Create a new application instance
    pub fn new(save_config_path: Option<std::path::PathBuf>) -> Self {
        Self {
            state: Arc::new(Mutex::new(AppState::default())),
            installer: None,
            ui_renderer: UiRenderer::new(),
            input_handler: InputHandler::new(),
            save_config_path,
        }
    }

    /// Run the main application loop
    pub fn run(
        &mut self,
        terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        loop {
            // Handle input events
            if crossterm::event::poll(Duration::from_millis(100))? {
                match crossterm::event::read()? {
                    Event::Key(key_event) => {
                        if self.handle_key_event(key_event)? {
                            break; // Exit requested
                        }
                    }
                    Event::Resize(width, height) => {
                        // Handle window resize - update scroll state
                        self.handle_resize(width, height)?;
                    }
                    _ => {}
                }
            }

            // Check if installation is complete
            {
                let state = self
                    .state
                    .lock()
                    .map_err(|e| error::general_error(format!("Mutex poisoned: {}", e)))?;
                if state.mode == AppMode::Complete {
                    break;
                }
            }

            // Render UI
            terminal.draw(|f| {
                let mut state = match self.state.lock() {
                    Ok(state) => state,
                    Err(_) => {
                        // If mutex is poisoned, we can't continue safely
                        eprintln!("Fatal error: Mutex poisoned, cannot continue");
                        std::process::exit(1);
                    }
                };
                // Update scroll state with actual available space for config options
                if state.mode == AppMode::Configuration {
                    // Calculate the config area height (total height minus reserved space)
                    let config_area_height = f.area().height.saturating_sub(16); // 16 lines reserved
                    let visible_items = config_area_height.saturating_sub(2); // Account for borders
                    state
                        .config_scroll
                        .update_visible_items(visible_items as usize);
                }
                self.ui_renderer.render(f, &state, &mut self.input_handler);
            })?;
        }

        Ok(())
    }

    /// Handle keyboard input events
    fn handle_key_event(
        &mut self,
        key_event: KeyEvent,
    ) -> Result<bool, Box<dyn std::error::Error>> {
        // Check if we're in an input dialog
        if self.input_handler.is_dialog_active() {
            if let Some(value) = self.input_handler.handle_input(key_event) {
                // User confirmed input, update configuration
                self.update_configuration_value(value)?;
            }
            return Ok(false);
        }

        // Handle main application navigation
        match key_event.code {
            KeyCode::Char('q') => {
                // Exit application
                return Ok(true);
            }
            KeyCode::Up => {
                self.navigate_up();
            }
            KeyCode::Down => {
                self.navigate_down();
            }
            KeyCode::PageUp => {
                self.page_up();
            }
            KeyCode::PageDown => {
                self.page_down();
            }
            KeyCode::Home => {
                self.move_to_first();
            }
            KeyCode::End => {
                self.move_to_last();
            }
            KeyCode::Enter => {
                self.handle_enter()?;
            }
            _ => {}
        }

        Ok(false)
    }

    /// Navigate to previous configuration option
    fn navigate_up(&self) {
        if let Ok(mut state) = self.lock_state_mut() {
            if state.mode == AppMode::Configuration {
                state.config_scroll.move_up();
            }
        }
    }

    /// Navigate to next configuration option
    fn navigate_down(&self) {
        if let Ok(mut state) = self.lock_state_mut() {
            if state.mode == AppMode::Configuration {
                state.config_scroll.move_down();
            }
        }
    }

    /// Page up in configuration list
    fn page_up(&self) {
        if let Ok(mut state) = self.lock_state_mut() {
            if state.mode == AppMode::Configuration {
                state.config_scroll.page_up();
            }
        }
    }

    /// Page down in configuration list
    fn page_down(&self) {
        if let Ok(mut state) = self.lock_state_mut() {
            if state.mode == AppMode::Configuration {
                state.config_scroll.page_down();
            }
        }
    }

    /// Move to first configuration option
    fn move_to_first(&self) {
        if let Ok(mut state) = self.lock_state_mut() {
            if state.mode == AppMode::Configuration {
                state.config_scroll.move_to_first();
            }
        }
    }

    /// Move to last configuration option
    fn move_to_last(&self) {
        if let Ok(mut state) = self.lock_state_mut() {
            if state.mode == AppMode::Configuration {
                state.config_scroll.move_to_last();
            }
        }
    }

    /// Handle Enter key press
    fn handle_enter(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (should_open_input, should_start_installation) = {
            let state = self.lock_state()?;
            match state.mode {
                AppMode::Configuration => {
                    // Check if we're on the green button (one step past the last config option)
                    if state.config_scroll.selected_index == state.config.options.len() {
                        (false, true) // Start installation
                    } else {
                        (true, false) // Open input dialog
                    }
                }
                AppMode::Installation => {
                    // Installation is running, no action needed
                    (false, false)
                }
                AppMode::Complete => {
                    // Installation complete, no action needed
                    (false, false)
                }
            }
        };

        // Open input dialog if needed
        if should_open_input {
            self.open_input_dialog()?;
        }

        // Start installation if needed
        if should_start_installation {
            if self.validate_configuration_for_installation() {
                {
                    let mut state = self.lock_state_mut()?;
                    state.status_message =
                        "Validation passed. Starting installation...".to_string();
                }
                self.start_installation()?;
            } else {
                // Validation failed - status message already set in validate_configuration_for_installation
                // User will see the error message
            }
        }

        Ok(())
    }

    /// Validate that all required configuration options are set and valid
    fn validate_configuration(&self, config: &Configuration) -> bool {
        // First check basic option validation
        if !config.options.iter().all(|option| option.is_valid()) {
            return false;
        }

        // Then check secure boot requirements
        self.validate_secure_boot_requirements(config)
    }

    /// Validate secure boot requirements
    fn validate_secure_boot_requirements(&self, config: &Configuration) -> bool {
        // Find the Secure Boot option
        if let Some(secure_boot_option) =
            config.options.iter().find(|opt| opt.name == "Secure Boot")
        {
            if secure_boot_option.value.to_lowercase() == "yes" {
                // Check if Boot Mode is UEFI
                if let Some(boot_mode_option) =
                    config.options.iter().find(|opt| opt.name == "Boot Mode")
                {
                    let boot_mode = boot_mode_option.value.to_lowercase();
                    if boot_mode != "uefi" && boot_mode != "auto" {
                        // Secure boot validation failed - this will be handled in the main validation
                        return false;
                    }
                }
            }
        }
        true
    }

    /// Check if secure boot warning should be shown after setting value
    fn check_secure_boot_warning(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Secure boot warning is now always shown in the dialog itself
        Ok(())
    }

    /// Check if UEFI is supported on this system
    fn is_uefi_supported(&self) -> bool {
        // Check for UEFI support by looking at /sys/firmware/efi
        std::path::Path::new("/sys/firmware/efi").exists()
    }

    /// Show secure boot warning dialog
    fn show_secure_boot_warning(&mut self) {
        let warning_message = vec![
            "SECURE BOOT REQUIREMENTS NOT MET".to_string(),
            "".to_string(),
            "Secure Boot requires UEFI firmware configuration:".to_string(),
            "".to_string(),
            "1. Boot into UEFI firmware settings".to_string(),
            "2. Enable UEFI mode (disable Legacy/CSM)".to_string(),
            "3. Enable Secure Boot in firmware".to_string(),
            "4. Set Secure Boot to 'Windows UEFI' mode".to_string(),
            "5. Save and exit firmware".to_string(),
            "".to_string(),
            "⚠️  WARNING: Secure Boot can prevent booting".to_string(),
            "if not configured properly!".to_string(),
            "".to_string(),
            "See: https://wiki.archlinux.org/title/Unified_Extensible_Firmware_Interface#UEFI_variables".to_string(),
        ];

        self.input_handler
            .start_warning("Secure Boot Warning".to_string(), warning_message);
    }

    /// Get detailed validation errors
    fn get_validation_errors(&self, config: &Configuration) -> Vec<String> {
        let mut errors: Vec<String> = config
            .options
            .iter()
            .filter_map(|option| option.validation_error())
            .collect();

        // Add secure boot validation errors
        if let Some(secure_boot_option) =
            config.options.iter().find(|opt| opt.name == "Secure Boot")
        {
            if secure_boot_option.value.to_lowercase() == "yes" {
                if let Some(boot_mode_option) =
                    config.options.iter().find(|opt| opt.name == "Boot Mode")
                {
                    let boot_mode = boot_mode_option.value.to_lowercase();
                    if boot_mode != "uefi" && boot_mode != "auto" {
                        errors.push("Secure Boot requires UEFI boot mode. Please configure UEFI firmware first.".to_string());
                    }
                }
            }
        }

        errors
    }

    /// Validate configuration for installation (with user feedback)
    fn validate_configuration_for_installation(&mut self) -> bool {
        let config = {
            let state = match self.lock_state() {
                Ok(state) => state,
                Err(_) => return false, // If we can't lock the state, validation fails
            };
            state.config.clone()
        };

        // Check for secure boot issues first (show warning dialog)
        if let Some(secure_boot_option) =
            config.options.iter().find(|opt| opt.name == "Secure Boot")
        {
            if secure_boot_option.value.to_lowercase() == "yes" {
                if let Some(boot_mode_option) =
                    config.options.iter().find(|opt| opt.name == "Boot Mode")
                {
                    let boot_mode = boot_mode_option.value.to_lowercase();
                    if boot_mode != "uefi" && boot_mode != "auto" {
                        self.show_secure_boot_warning();
                        return false;
                    }
                }
            }
        }

        if self.validate_configuration(&config) {
            // All validation passed - installation can proceed
            true
        } else {
            let mut state = match self.lock_state_mut() {
                Ok(state) => state,
                Err(_) => return false, // If we can't lock the state, validation fails
            };
            let errors = self.get_validation_errors(&config);

            if errors.len() == 1 {
                state.status_message = format!("❌ Cannot start installation: {}", errors[0]);
            } else {
                state.status_message = format!(
                    "❌ Cannot start installation: {} (and {} more errors)",
                    errors[0],
                    errors.len() - 1
                );
            }
            false
        }
    }

    /// Start the installation process
    fn start_installation(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // Check if we need to save the config before starting
        if let Some(save_path) = &self.save_config_path {
            let state = self.lock_state()?;
            let file_config = crate::config_file::InstallationConfig::from(&state.config);
            file_config.save_to_file(save_path)?;
            
            let mut state_mut = self.lock_state_mut()?;
            state_mut.status_message = format!("✓ Config saved to {}", save_path.display());
        }

        // Update state to installation mode
        {
            let mut state = self.lock_state_mut()?;
            state.mode = AppMode::Installation;
            state.status_message = "Starting installation...".to_string();
        }

        // Create installer with current configuration
        let config = {
            let state = self.lock_state()?;
            state.config.clone()
        };

        self.installer = Some(Installer::new(config, Arc::clone(&self.state)));

        // Start installation in background
        if let Some(ref mut installer) = self.installer {
            installer.start()?;
        }

        Ok(())
    }

    /// Open input dialog for the current configuration option
    fn open_input_dialog(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let option = {
            let state = self.lock_state()?;
            let current_step = state.config_scroll.selected_index;
            state.config.options[current_step].clone()
        };

        match option.name.as_str() {
            "Boot Mode" => {
                // Show normal selection dialog
                let options = InputHandler::get_predefined_options(&option.name);
                self.input_handler
                    .start_selection(option.name.clone(), options, option.value);
            }
            "Secure Boot" => {
                // Always show selection dialog with static warning about requirements
                let mut options = InputHandler::get_predefined_options(&option.name);

                // Check if UEFI is supported
                let uefi_supported = self.is_uefi_supported();

                // Insert static warning at the top of the options
                options.insert(
                    0,
                    "⚠️  WARNING: Secure Boot requires UEFI firmware!".to_string(),
                );
                options.insert(
                    1,
                    "Make sure your motherboard supports UEFI and".to_string(),
                );
                options.insert(2, "Secure Boot is properly configured in BIOS.".to_string());
                options.insert(3, "See: https://wiki.archlinux.org/title/UEFI".to_string());
                options.insert(4, "".to_string());

                // If UEFI is not supported, only show "No" option
                if !uefi_supported {
                    options = vec!["No".to_string()];
                    options.insert(
                        0,
                        "⚠️  WARNING: Secure Boot requires UEFI firmware!".to_string(),
                    );
                    options.insert(1, "UEFI is not supported on this system.".to_string());
                    options.insert(2, "Secure Boot is not available.".to_string());
                    options.insert(3, "".to_string());
                }

                self.input_handler
                    .start_selection(option.name.clone(), options, option.value);
            }
            "Encryption" => {
                // Only allow encryption Yes/No for manual partitioning
                let partitioning_strategy = {
                    let state = match self.lock_state() {
                        Ok(state) => state,
                        Err(_) => return Ok(()), // If we can't lock the state, skip this option
                    };
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Partitioning Strategy")
                        .map(|opt| opt.value.clone())
                        .unwrap_or_default()
                };

                if partitioning_strategy == "manual" {
                    let options = vec!["Yes".to_string(), "No".to_string()];
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else {
                    // Show message that encryption is auto-set for non-manual strategies
                    if let Ok(mut state) = self.lock_state_mut() {
                        state.status_message = "Encryption is auto-set based on partitioning strategy. Use manual partitioning to control encryption.".to_string();
                    }
                }
            }
            "Swap Size" => {
                // Only allow swap size configuration if swap is enabled
                let swap_enabled = {
                    let state = match self.lock_state() {
                        Ok(state) => state,
                        Err(_) => return Ok(()),
                    };
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Swap")
                        .map(|opt| opt.value.to_lowercase() == "yes")
                        .unwrap_or(false)
                };

                if swap_enabled {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else {
                    if let Ok(mut state) = self.lock_state_mut() {
                        state.status_message =
                            "Swap size can only be configured when swap is enabled.".to_string();
                    }
                }
            }
            "Btrfs Frequency" | "Btrfs Keep Count" | "Btrfs Assistant" => {
                // Only allow btrfs configuration if snapshots are enabled
                let snapshots_enabled = {
                    let state = match self.lock_state() {
                        Ok(state) => state,
                        Err(_) => return Ok(()),
                    };
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Btrfs Snapshots")
                        .map(|opt| opt.value.to_lowercase() == "yes")
                        .unwrap_or(false)
                };

                if snapshots_enabled {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else {
                    if let Ok(mut state) = self.lock_state_mut() {
                        state.status_message = format!(
                            "{} can only be configured when Btrfs snapshots are enabled.",
                            option.name
                        );
                    }
                }
            }
            "GRUB Theme Selection" => {
                // Only allow theme selection if GRUB themes are enabled
                let themes_enabled = {
                    let state = match self.lock_state() {
                        Ok(state) => state,
                        Err(_) => return Ok(()),
                    };
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "GRUB Theme")
                        .map(|opt| opt.value.to_lowercase() == "yes")
                        .unwrap_or(false)
                };

                if themes_enabled {
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else {
                    if let Ok(mut state) = self.lock_state_mut() {
                        state.status_message =
                            "GRUB theme selection is only available when GRUB themes are enabled."
                                .to_string();
                    }
                }
            }
            "Git Repository URL" => {
                // Only allow URL input if git repository is enabled
                let git_enabled = {
                    let state = match self.lock_state() {
                        Ok(state) => state,
                        Err(_) => return Ok(()),
                    };
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Git Repository")
                        .map(|opt| opt.value.to_lowercase() == "yes")
                        .unwrap_or(false)
                };

                if git_enabled {
                    self.input_handler.start_text_input(
                        option.name.clone(),
                        option.value,
                        "Enter git repository URL".to_string(),
                    );
                } else {
                    if let Ok(mut state) = self.lock_state_mut() {
                        state.status_message =
                            "Git repository URL can only be configured when git repository is enabled."
                                .to_string();
                    }
                }
            }
            "Disk" => {
                // Check if we need multi-disk selection
                let state = match self.lock_state() {
                    Ok(state) => state,
                    Err(_) => return Ok(()),
                };
                let partitioning_strategy = state
                    .config
                    .options
                    .iter()
                    .find(|opt| opt.name == "Partitioning Strategy")
                    .map(|opt| opt.value.clone())
                    .unwrap_or_default();
                drop(state); // Release the lock

                match partitioning_strategy.as_str() {
                    "auto_raid" | "auto_raid_luks" | "auto_raid_lvm" | "auto_raid_lvm_luks"
                    | "manual" => {
                        self.input_handler
                            .start_multi_disk_selection(&partitioning_strategy);
                    }
                    _ => {
                        self.input_handler.start_disk_selection(option.value);
                    }
                }
            }
            "Username" | "Hostname" => {
                let placeholder = match option.name.as_str() {
                    "Username" => "Enter username",
                    "Hostname" => "Enter hostname",
                    _ => "Enter value",
                }
                .to_string();

                self.input_handler
                    .start_text_input(option.name.clone(), option.value, placeholder);
            }
            "User Password" | "Root Password" => {
                let placeholder = match option.name.as_str() {
                    "User Password" => "Enter user password",
                    "Root Password" => "Enter root password",
                    _ => "Enter password",
                }
                .to_string();

                self.input_handler.start_password_input(
                    option.name.clone(),
                    option.value,
                    placeholder,
                );
            }
            "Additional Pacman Packages" | "Additional AUR Packages" => {
                self.input_handler
                    .start_package_selection(option.name.clone(), option.value);
            }
            "Timezone Region" => {
                let options = InputHandler::get_predefined_options(&option.name);
                self.input_handler
                    .start_selection(option.name.clone(), options, option.value);
            }
            "Timezone" => {
                // Get timezone options based on selected region
                let timezone_region = {
                    let state = match self.lock_state() {
                        Ok(state) => state,
                        Err(_) => return Ok(()),
                    };
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Timezone Region")
                        .map(|opt| opt.value.clone())
                        .unwrap_or_default()
                };

                if !timezone_region.is_empty() {
                    let options = InputHandler::get_timezones_for_region(&timezone_region);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                } else {
                    if let Ok(mut state) = self.lock_state_mut() {
                        state.status_message = "Please select a timezone region first.".to_string();
                    }
                }
            }
            "Display Manager" => {
                // Check if desktop environment is set to something other than "none"
                let desktop_env = {
                    let state = match self.lock_state() {
                        Ok(state) => state,
                        Err(_) => return Ok(()),
                    };
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Desktop Environment")
                        .map(|opt| opt.value.clone())
                        .unwrap_or_default()
                };

                if desktop_env != "none" && !desktop_env.is_empty() {
                    // Desktop environment is selected, display manager should be auto-set
                    if let Ok(mut state) = self.lock_state_mut() {
                        state.status_message =
                            "Display Manager is auto-set based on Desktop Environment selection."
                                .to_string();
                    }
                } else {
                    // No desktop environment or "none" selected, allow manual selection
                    let options = InputHandler::get_predefined_options(&option.name);
                    self.input_handler
                        .start_selection(option.name.clone(), options, option.value);
                }
            }
            _ => {
                // Use predefined options for selection fields
                let options = InputHandler::get_predefined_options(&option.name);
                self.input_handler
                    .start_selection(option.name.clone(), options, option.value);
            }
        }

        Ok(())
    }

    /// Update configuration value after input dialog
    fn update_configuration_value(
        &mut self,
        value: String,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let (current_step, option_name) = {
            let state = self
                .state
                .lock()
                .map_err(|e| error::general_error(format!("Mutex poisoned: {}", e)))?;
            if state.config_scroll.selected_index >= state.config.options.len() {
                return Err(error::general_error("Invalid configuration option index").into());
            }
            (
                state.config_scroll.selected_index,
                state.config.options[state.config_scroll.selected_index]
                    .name
                    .clone(),
            )
        };

        // Update the configuration value
        {
            let mut state = self
                .state
                .lock()
                .map_err(|e| error::general_error(format!("Mutex poisoned: {}", e)))?;
            if current_step < state.config.options.len() {
                // Parse disk selection to extract only device path
                let parsed_value = if option_name == "Disk" {
                    // Check if this is multi-disk selection (contains commas)
                    if value.contains(',') {
                        // Multi-disk selection - extract all disk paths
                        let disk_paths: Vec<String> = value
                            .split(',')
                            .map(|d| d.split_whitespace().next().unwrap_or("").to_string())
                            .filter(|d| !d.is_empty())
                            .collect();

                        // Check if this is manual partitioning
                        let partitioning_strategy = state
                            .config
                            .options
                            .iter()
                            .find(|opt| opt.name == "Partitioning Strategy")
                            .map(|opt| opt.value.as_str())
                            .unwrap_or("");

                        if partitioning_strategy == "manual" {
                            // For manual partitioning, show confirmation dialog
                            drop(state); // Release the lock
                            self.input_handler
                                .start_manual_partitioning_confirmation(&disk_paths);
                            return Ok(());
                        } else {
                            // For RAID strategies, join with commas
                            disk_paths.join(",")
                        }
                    } else {
                        // Single disk selection - extract just the device path from "/dev/sda (128G)" -> "/dev/sda"
                        value
                            .split_whitespace()
                            .next()
                            .unwrap_or(&value)
                            .to_string()
                    }
                } else {
                    value.clone()
                };

                state.config.options[current_step].value = parsed_value.clone();
                state.status_message = format!(
                    "Set {} to: {}",
                    state.config.options[current_step].name, parsed_value
                );
            }
        }

        // Handle manual partitioning confirmation
        if option_name == "manual_partitioning_confirm" {
            if value == "Yes, start partitioning" {
                // User confirmed manual partitioning
                let state = match self.lock_state() {
                    Ok(state) => state,
                    Err(_) => return Ok(()),
                };
                let disk_value = state
                    .config
                    .options
                    .iter()
                    .find(|opt| opt.name == "Disk")
                    .map(|opt| opt.value.clone())
                    .unwrap_or_default();
                drop(state);

                // Extract disk paths
                let disk_paths: Vec<String> = disk_value
                    .split(',')
                    .map(|d| d.split_whitespace().next().unwrap_or("").to_string())
                    .filter(|d| !d.is_empty())
                    .collect();

                // Launch partitioning tool
                if let Err(e) = self.input_handler.launch_partitioning_tool(&disk_paths) {
                    if let Ok(mut state) = self.lock_state_mut() {
                        state.status_message = format!("Partitioning failed: {}", e);
                        return Ok(());
                    }
                }

                // Validate partitioning after user finishes
                let boot_mode = {
                    let state = match self.lock_state() {
                        Ok(state) => state,
                        Err(_) => return Ok(()),
                    };
                    state
                        .config
                        .options
                        .iter()
                        .find(|opt| opt.name == "Boot Mode")
                        .map(|opt| opt.value.as_str())
                        .unwrap_or("BIOS")
                        .to_string()
                };

                match self
                    .input_handler
                    .validate_manual_partitioning(&disk_paths, &boot_mode)
                {
                    Ok(layout) => {
                        if let Ok(mut state) = self.lock_state_mut() {
                            state.status_message = format!(
                                "Manual partitioning validated successfully! Found {} partitions with {} table",
                                layout.partitions.len(),
                                layout.table_type
                            );
                        }
                    }
                    Err(e) => {
                        if let Ok(mut state) = self.lock_state_mut() {
                            state.status_message = format!("Partitioning validation failed: {}", e);
                        }
                    }
                }
            }
            // If user chose "No, go back", just return (dialog will close)
            return Ok(());
        }

        // Auto-set encryption based on partitioning strategy
        if option_name == "Partitioning Strategy" {
            self.auto_set_encryption(&value)?;
        }

        // Auto-set display manager based on desktop environment
        if option_name == "Desktop Environment" {
            self.auto_set_display_manager(&value)?;
        }

        // Handle warning dialog acknowledgment
        if value == "acknowledged" {
            // Warning was acknowledged, proceed to show normal selection dialog
        }

        // Check for secure boot warning after setting Secure Boot
        if option_name == "Secure Boot" && value.to_lowercase() == "yes" {
            self.check_secure_boot_warning()?;
        }

        // Handle dependent option updates
        self.handle_dependent_options(&option_name, &value)?;

        // Move to next step
        {
            if let Ok(mut state) = self.lock_state_mut() {
                if state.config_scroll.selected_index < state.config.options.len() - 1 {
                    let next_index = state.config_scroll.selected_index + 1;
                    state.config_scroll.set_selected(next_index);
                }
            }
        }

        Ok(())
    } // Close the update_configuration_value function

    /// Auto-set encryption based on partitioning strategy
    fn auto_set_encryption(
        &mut self,
        partitioning_strategy: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Only auto-set if not manual partitioning
        if partitioning_strategy != "manual" {
            let encryption_value = if partitioning_strategy.contains("luks") {
                "Yes"
            } else {
                "No"
            };

            {
                if let Ok(mut state) = self.lock_state_mut() {
                    // Find encryption option (index 6)
                    if state.config.options.len() > 6 {
                        state.config.options[6].value = encryption_value.to_string();
                        state.status_message = format!(
                            "Auto-set Encryption to: {} (based on partitioning strategy)",
                            encryption_value
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Auto-set display manager based on desktop environment
    fn auto_set_display_manager(
        &mut self,
        desktop_env: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let display_manager = match desktop_env {
            "kde" => "sddm",
            "gnome" => "gdm",
            "hyprland" => "sddm",
            "none" => "", // Don't auto-set when "none" - let user choose
            _ => "",
        };

        if !display_manager.is_empty() {
            {
                if let Ok(mut state) = self.lock_state_mut() {
                    // Find display manager option by name
                    if let Some(display_manager_option) = state
                        .config
                        .options
                        .iter_mut()
                        .find(|opt| opt.name == "Display Manager")
                    {
                        display_manager_option.value = display_manager.to_string();
                        state.status_message = format!(
                            "Auto-set Display Manager to: {} (based on desktop environment)",
                            display_manager
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Handle dependent option updates based on user selections
    fn handle_dependent_options(
        &mut self,
        option_name: &str,
        value: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(mut state) = self.lock_state_mut() {
            match option_name {
                "Swap" => {
                    if value.to_lowercase() == "no" {
                        // Disable swap size when swap is disabled
                        if let Some(swap_size_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Swap Size")
                        {
                            swap_size_option.value = "N/A".to_string();
                        }
                    } else if value.to_lowercase() == "yes" {
                        // Reset swap size to default when swap is enabled
                        if let Some(swap_size_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Swap Size")
                        {
                            swap_size_option.value = "2GB".to_string();
                        }
                    }
                }
                "Btrfs Snapshots" => {
                    if value.to_lowercase() == "no" {
                        // Disable btrfs frequency, keep count, and assistant when snapshots are disabled
                        if let Some(freq_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Btrfs Frequency")
                        {
                            freq_option.value = "N/A".to_string();
                        }
                        if let Some(keep_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Btrfs Keep Count")
                        {
                            keep_option.value = "N/A".to_string();
                        }
                        if let Some(assistant_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Btrfs Assistant")
                        {
                            assistant_option.value = "No".to_string();
                        }
                    } else if value.to_lowercase() == "yes" {
                        // Reset btrfs options to defaults when snapshots are enabled
                        if let Some(freq_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Btrfs Frequency")
                        {
                            freq_option.value = "weekly".to_string();
                        }
                        if let Some(keep_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Btrfs Keep Count")
                        {
                            keep_option.value = "3".to_string();
                        }
                    }
                }
                "Plymouth" => {
                    if value.to_lowercase() == "no" {
                        // Set plymouth theme to none when plymouth is disabled
                        if let Some(theme_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Plymouth Theme")
                        {
                            theme_option.value = "none".to_string();
                        }
                    }
                }
                "GRUB Theme" => {
                    if value.to_lowercase() == "no" {
                        // Set GRUB theme selection to none when themes are disabled
                        if let Some(theme_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "GRUB Theme Selection")
                        {
                            theme_option.value = "none".to_string();
                        }
                    }
                }
                "Timezone Region" => {
                    // Reset timezone when region changes
                    if let Some(timezone_option) = state
                        .config
                        .options
                        .iter_mut()
                        .find(|opt| opt.name == "Timezone")
                    {
                        timezone_option.value = "".to_string(); // Reset to empty to force selection
                    }

                    // Auto-select mirror country based on region for quality of life
                    let mirror_country = match value {
                        "US" => "United States",
                        "Europe" => "Germany", // Default to Germany for Europe
                        "Asia" => "Japan",     // Default to Japan for Asia
                        "Australia" => "Australia",
                        "America" => "United States", // For America region, default to US
                        _ => "",                      // Don't auto-select for other regions
                    };

                    if !mirror_country.is_empty() {
                        if let Some(mirror_option) = state
                            .config
                            .options
                            .iter_mut()
                            .find(|opt| opt.name == "Mirror Country")
                        {
                            mirror_option.value = mirror_country.to_string();
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    /// Handle window resize events
    fn handle_resize(
        &mut self,
        _width: u16,
        height: u16,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Update scroll state with new visible height
        if let Ok(mut state) = self.lock_state_mut() {
            if state.mode == AppMode::Configuration {
                // Calculate available height for config list
                // Header(7) + Title(3) + Instructions(3) + Start Button(3) = 16 lines reserved
                let available_height = (height as usize).saturating_sub(16);
                // Use most of the available space, with a minimum of 5 lines
                let visible_height = available_height.max(5);
                state.config_scroll.update_visible_items(visible_height);
            }
        }
        Ok(())
    }
}
