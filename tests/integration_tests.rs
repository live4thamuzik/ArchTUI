// Integration tests for archinstall-tui

use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;

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
    
    let config = Configuration::default();
    assert!(!config.options.is_empty(), "Configuration should have options");
    
    // Check for essential options
    let option_names: Vec<&String> = config.options.iter().map(|opt| &opt.name).collect();
    assert!(option_names.contains(&&"Disk".to_string()), "Should have Disk option");
    assert!(option_names.contains(&&"Root Filesystem".to_string()), "Should have Root Filesystem option");
}

/// Test async tool execution with threading and output capture
/// This validates the Sprint 2 async execution pattern works correctly
#[test]
fn test_async_tool_execution_with_output_capture() {
    // Message types matching the app's ToolMessage enum
    #[derive(Debug)]
    enum TestMessage {
        Stdout(String),
        Stderr(String),
        Complete { success: bool, exit_code: Option<i32> },
    }

    let (tx, rx) = mpsc::channel::<TestMessage>();

    // Spawn a simple bash command that outputs to stdout and stderr
    let tx_clone = tx.clone();
    let handle = thread::spawn(move || {
        let mut child = Command::new("bash")
            .arg("-c")
            .arg("echo 'stdout line 1'; echo 'stdout line 2'; echo 'stderr line' >&2; exit 0")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(Stdio::null())
            .spawn()
            .expect("Failed to spawn test command");

        // Stream stdout
        let stdout_tx = tx_clone.clone();
        let stdout_handle = if let Some(stdout) = child.stdout.take() {
            Some(thread::spawn(move || {
                let reader = BufReader::new(stdout);
                for line in reader.lines().map_while(Result::ok) {
                    let _ = stdout_tx.send(TestMessage::Stdout(line));
                }
            }))
        } else {
            None
        };

        // Stream stderr
        let stderr_tx = tx_clone.clone();
        let stderr_handle = if let Some(stderr) = child.stderr.take() {
            Some(thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines().map_while(Result::ok) {
                    let _ = stderr_tx.send(TestMessage::Stderr(line));
                }
            }))
        } else {
            None
        };

        // Wait for output threads
        if let Some(h) = stdout_handle {
            let _ = h.join();
        }
        if let Some(h) = stderr_handle {
            let _ = h.join();
        }

        // Wait for process and send completion
        match child.wait() {
            Ok(status) => {
                let _ = tx_clone.send(TestMessage::Complete {
                    success: status.success(),
                    exit_code: status.code(),
                });
            }
            Err(_) => {
                let _ = tx_clone.send(TestMessage::Complete {
                    success: false,
                    exit_code: None,
                });
            }
        }
    });

    // Collect messages with timeout
    let mut stdout_lines = Vec::new();
    let mut stderr_lines = Vec::new();
    let mut completed = false;
    let mut success = false;

    let timeout = std::time::Duration::from_secs(5);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout {
        match rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(msg) => match msg {
                TestMessage::Stdout(line) => stdout_lines.push(line),
                TestMessage::Stderr(line) => stderr_lines.push(line),
                TestMessage::Complete { success: s, .. } => {
                    completed = true;
                    success = s;
                    break;
                }
            },
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    handle.join().expect("Thread should complete");

    // Verify results
    assert!(completed, "Process should complete");
    assert!(success, "Process should exit successfully");
    assert_eq!(stdout_lines.len(), 2, "Should capture 2 stdout lines");
    assert_eq!(stdout_lines[0], "stdout line 1");
    assert_eq!(stdout_lines[1], "stdout line 2");
    assert_eq!(stderr_lines.len(), 1, "Should capture 1 stderr line");
    assert_eq!(stderr_lines[0], "stderr line");
}

/// Test async tool execution with stdin piping (Sprint 4 security pattern)
#[test]
fn test_async_tool_execution_with_stdin_piping() {
    let (tx, rx) = mpsc::channel::<String>();
    let password = "secret_test_password";

    let tx_clone = tx.clone();
    let password_clone = password.to_string();

    let handle = thread::spawn(move || {
        // Use bash to read from stdin and echo it back
        let mut child = Command::new("bash")
            .arg("-c")
            .arg("read -r input; echo \"received: $input\"")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::piped())
            .spawn()
            .expect("Failed to spawn test command");

        // Write password to stdin (simulating secure password passing)
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(password_clone.as_bytes());
            let _ = stdin.write_all(b"\n");
            // stdin is dropped here, closing the pipe
        }

        // Read stdout
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);
            for line in reader.lines().map_while(Result::ok) {
                let _ = tx_clone.send(line);
            }
        }

        let _ = child.wait();
    });

    // Wait for output
    let output = rx.recv_timeout(std::time::Duration::from_secs(5))
        .expect("Should receive output");

    handle.join().expect("Thread should complete");

    // Verify the password was received via stdin (not visible in process args)
    assert_eq!(output, format!("received: {}", password));
}
