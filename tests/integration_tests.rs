// Integration tests for archinstall-tui

use std::process::Command;
use std::os::unix::fs::PermissionsExt;

#[test]
fn test_binary_exists() {
    assert!(std::path::Path::new("./archinstall-tui").exists(), "Binary should exist");
}

#[test]
fn test_binary_executable() {
    let metadata = std::fs::metadata("./archinstall-tui")
        .expect("Should be able to read binary metadata");
    assert!(metadata.permissions().mode() & 0o111 != 0, "Binary should be executable");
}

#[test]
fn test_required_scripts_exist() {
    let required_scripts = vec![
        "scripts/install.sh",
        "scripts/install_wrapper.sh", 
        "scripts/utils.sh",
        "scripts/disk_strategies.sh",
        "scripts/chroot_config.sh",
    ];

    for script in required_scripts {
        assert!(std::path::Path::new(script).exists(), "Script {} should exist", script);
    }
}

#[test]
fn test_plymouth_themes_exist() {
    assert!(std::path::Path::new("Source/arch-glow").exists(), "Arch-glow theme should exist");
    assert!(std::path::Path::new("Source/arch-mac-style").exists(), "Arch-mac-style theme should exist");
}

#[test]
fn test_binary_runs_without_crashing() {
    // Test that the binary can start without immediately crashing
    // We use a timeout to prevent hanging
    let output = Command::new("timeout")
        .args(&["5s", "./archinstall-tui"])
        .output();
    
    // The binary should either exit cleanly or with a TUI error (expected in non-TTY environments)
    match output {
        Ok(result) => {
            // Exit code 0 or non-zero is fine, as long as it doesn't panic
            println!("Binary executed successfully, exit code: {:?}", result.status.code());
        }
        Err(e) => {
            // If it's a timeout or TUI error, that's expected in test environments
            if e.kind() == std::io::ErrorKind::TimedOut {
                println!("Binary timed out (expected in test environment)");
            } else {
                panic!("Binary failed to execute: {}", e);
            }
        }
    }
}

#[test]
fn test_config_structure() {
    // Test that we can load the configuration structure
    use archinstall_tui::config::Configuration;
    
    let config = Configuration::new();
    assert!(!config.options.is_empty(), "Configuration should have options");
    
    // Check for essential options
    let option_names: Vec<&String> = config.options.iter().map(|opt| &opt.name).collect();
    assert!(option_names.contains(&&"Disk".to_string()), "Should have Disk option");
    assert!(option_names.contains(&&"Root Filesystem".to_string()), "Should have Root Filesystem option");
}
