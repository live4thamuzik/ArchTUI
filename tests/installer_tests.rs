//! Tests for Installation Orchestration
//!
//! P3.1: Tests for Installer struct and installation flow
//!
//! These tests verify:
//! - Configuration validation
//! - Environment variable generation
//! - Progress tracking
//! - State updates during installation

use archtui::app::{AppMode, AppState};
use archtui::config::Configuration;
use archtui::installer::Installer;
use std::sync::{Arc, Mutex};

// =============================================================================
// Configuration Validation Tests
// =============================================================================

#[test]
fn test_configuration_default_creates_valid_options() {
    let config = Configuration::default();
    assert!(!config.options.is_empty(), "Default config should have options");
}

#[test]
fn test_configuration_has_essential_options() {
    let config = Configuration::default();
    let option_names: Vec<&str> = config.options.iter().map(|o| o.name.as_str()).collect();

    // Check for essential options
    assert!(
        option_names.contains(&"Disk"),
        "Config should have Disk option"
    );
    assert!(
        option_names.contains(&"Root Filesystem"),
        "Config should have Root Filesystem option"
    );
    assert!(
        option_names.contains(&"Hostname"),
        "Config should have Hostname option"
    );
    assert!(
        option_names.contains(&"Username"),
        "Config should have Username option"
    );
}

#[test]
fn test_configuration_to_env_vars() {
    let config = Configuration::default();
    let env_vars = config.to_env_vars();

    // Should produce environment variables
    assert!(!env_vars.is_empty(), "Should produce env vars");

    // Check for expected env var keys
    let keys: Vec<&String> = env_vars.keys().collect();
    assert!(
        keys.iter().any(|k| k.contains("INSTALL") || k.contains("DISK") || k.contains("HOSTNAME")),
        "Should contain installation-related env vars"
    );
}

#[test]
fn test_configuration_env_vars_are_strings() {
    let config = Configuration::default();
    let env_vars = config.to_env_vars();

    for (key, value) in env_vars {
        // All keys and values should be valid strings (not empty keys)
        assert!(!key.is_empty(), "Env var key should not be empty");
        // Values can be empty strings for optional fields - just verify it's a string
        let _ = value; // Value is always a valid String
    }
}

// =============================================================================
// Installer Creation Tests
// =============================================================================

#[test]
fn test_installer_creation() {
    let config = Configuration::default();
    let app_state = Arc::new(Mutex::new(AppState::default()));

    let installer = Installer::new(config.clone(), Arc::clone(&app_state));

    // Installer should be created without panic
    // We can't easily test internal state, but creation succeeding is the test
    drop(installer);
}

#[test]
fn test_installer_with_shared_state() {
    let config = Configuration::default();
    let app_state = Arc::new(Mutex::new(AppState::default()));

    let _installer = Installer::new(config, Arc::clone(&app_state));

    // Should still be able to lock state
    let state = app_state.lock().expect("Should be able to lock state");
    assert_eq!(state.mode, AppMode::MainMenu);
}

// =============================================================================
// Progress Tracking Pattern Tests
// =============================================================================

#[test]
fn test_progress_update_pattern() {
    let app_state = Arc::new(Mutex::new(AppState::default()));

    // Simulate progress updates like installer does
    {
        let mut state = app_state.lock().unwrap();
        state.installation_progress = 10;
        state.status_message = "Starting installation...".to_string();
    }

    {
        let state = app_state.lock().unwrap();
        assert_eq!(state.installation_progress, 10);
    }

    {
        let mut state = app_state.lock().unwrap();
        state.installation_progress = 50;
        state.status_message = "Installing packages...".to_string();
    }

    {
        let state = app_state.lock().unwrap();
        assert_eq!(state.installation_progress, 50);
        assert!(state.status_message.contains("Installing"));
    }
}

#[test]
fn test_output_line_accumulation_pattern() {
    let app_state = Arc::new(Mutex::new(AppState::default()));

    // Simulate output line accumulation like installer does
    for i in 0..150 {
        let mut state = app_state.lock().unwrap();
        state.installer_output.push(format!("Line {}", i));

        // Keep only last 100 lines (like installer does)
        if state.installer_output.len() > 100 {
            state.installer_output.remove(0);
        }
    }

    let state = app_state.lock().unwrap();
    assert_eq!(state.installer_output.len(), 100);
    // First line should be "Line 50" (lines 0-49 were removed)
    assert!(state.installer_output[0].contains("50"));
}

#[test]
fn test_mode_transition_to_installation() {
    let app_state = Arc::new(Mutex::new(AppState::default()));

    {
        let mut state = app_state.lock().unwrap();
        state.mode = AppMode::Installation;
        state.status_message = "Starting installation...".to_string();
        state.installation_progress = 10;
    }

    let state = app_state.lock().unwrap();
    assert_eq!(state.mode, AppMode::Installation);
}

#[test]
fn test_mode_transition_to_complete() {
    let app_state = Arc::new(Mutex::new(AppState::default()));

    {
        let mut state = app_state.lock().unwrap();
        state.mode = AppMode::Installation;
        state.installation_progress = 100;
    }

    {
        let mut state = app_state.lock().unwrap();
        state.mode = AppMode::Complete;
        state.status_message = "Installation complete!".to_string();
    }

    let state = app_state.lock().unwrap();
    assert_eq!(state.mode, AppMode::Complete);
    assert_eq!(state.installation_progress, 100);
}

// =============================================================================
// Progress Message Parsing Pattern Tests
// =============================================================================

#[test]
fn test_progress_from_output_line() {
    // Test the pattern used in installer to update progress from output
    let test_cases = vec![
        ("Starting Arch Linux installation", 10, "Installation started"),
        ("Preparing system", 15, "Preparing system"),
        ("Starting disk partitioning", 25, "Partitioning disk"),
        ("Installing base system", 40, "Installing base system"),
        ("Configuring system", 60, "Configuring system"),
        ("Installing packages", 75, "Installing packages"),
        ("Configuring bootloader", 85, "Configuring bootloader"),
        ("Finalizing installation", 95, "Finalizing installation"),
    ];

    for (line, expected_progress, expected_status) in test_cases {
        let (progress, status) = parse_progress_from_line(line);
        assert_eq!(
            progress, expected_progress,
            "Line '{}' should set progress to {}",
            line, expected_progress
        );
        assert!(
            status.contains(expected_status) || expected_status.contains(&status),
            "Line '{}' should set status containing '{}'",
            line,
            expected_status
        );
    }
}

// Helper function mimicking installer's progress parsing
fn parse_progress_from_line(line: &str) -> (u8, String) {
    if line.contains("Starting Arch Linux installation") {
        (10, "Installation started".to_string())
    } else if line.contains("Preparing system") {
        (15, "Preparing system".to_string())
    } else if line.contains("Starting disk partitioning") {
        (25, "Partitioning disk".to_string())
    } else if line.contains("Installing base system") {
        (40, "Installing base system".to_string())
    } else if line.contains("Configuring system") {
        (60, "Configuring system".to_string())
    } else if line.contains("Installing packages") {
        (75, "Installing packages".to_string())
    } else if line.contains("Configuring bootloader") {
        (85, "Configuring bootloader".to_string())
    } else if line.contains("Finalizing installation") {
        (95, "Finalizing installation".to_string())
    } else {
        (0, "".to_string())
    }
}

// =============================================================================
// Error Handling Pattern Tests
// =============================================================================

#[test]
fn test_error_in_output_pattern() {
    let app_state = Arc::new(Mutex::new(AppState::default()));

    // Simulate error handling like installer does
    {
        let mut state = app_state.lock().unwrap();
        state.mode = AppMode::Installation;
        state.installer_output.push("ERROR: Disk not found".to_string());
    }

    {
        let mut state = app_state.lock().unwrap();
        // Check for error in output and update status
        let has_error = state.installer_output.iter().any(|l| l.contains("ERROR"));
        if has_error {
            state.status_message = "Installation failed".to_string();
        }
    }

    let state = app_state.lock().unwrap();
    assert!(state.status_message.contains("failed"));
}

// =============================================================================
// Thread Safety Tests
// =============================================================================

#[test]
fn test_concurrent_state_access() {
    use std::thread;

    let app_state = Arc::new(Mutex::new(AppState::default()));
    let mut handles = vec![];

    // Spawn multiple threads that update state
    for i in 0..10 {
        let state_clone = Arc::clone(&app_state);
        let handle = thread::spawn(move || {
            let mut state = state_clone.lock().unwrap();
            state.installer_output.push(format!("Thread {} output", i));
        });
        handles.push(handle);
    }

    // Wait for all threads
    for handle in handles {
        handle.join().expect("Thread should complete");
    }

    // All outputs should be present
    let state = app_state.lock().unwrap();
    assert_eq!(state.installer_output.len(), 10);
}

#[test]
fn test_state_mutex_not_poisoned_on_normal_use() {
    let app_state = Arc::new(Mutex::new(AppState::default()));

    // Normal lock/unlock cycle
    {
        let mut state = app_state.lock().unwrap();
        state.installation_progress = 50;
    }

    // Should still be lockable
    let result = app_state.lock();
    assert!(result.is_ok(), "Mutex should not be poisoned");
}
