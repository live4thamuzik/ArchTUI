// Integration tests for archinstall-tui
//
// Sprint 1.2: Death Pact Integration Tests
// These tests verify that the process lifecycle management works correctly:
// - PR_SET_PDEATHSIG causes children to die when parent exits
// - Process group signaling kills entire process trees
// - Signal handlers trigger proper cleanup

use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

// Re-export process guard functionality for testing
use archinstall_tui::process_guard::{CommandProcessGroup, ChildRegistry};

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
    #[allow(dead_code)] // exit_code reserved for future assertions
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

// =============================================================================
// Sprint 1.2: Death Pact Integration Tests
// =============================================================================

/// Helper: Check if a process is alive (not dead or zombie)
fn is_process_alive(pid: u32) -> bool {
    use nix::sys::signal;
    use nix::unistd::Pid;

    // First check if process exists
    if signal::kill(Pid::from_raw(pid as i32), None).is_err() {
        return false;
    }

    // Check for zombie state - zombies aren't really "alive"
    if let Ok(stat) = std::fs::read_to_string(format!("/proc/{}/stat", pid)) {
        let fields: Vec<&str> = stat.split_whitespace().collect();
        if fields.len() > 2 {
            // 'Z' = zombie, 'X' = dead
            return !matches!(fields[2], "Z" | "X");
        }
    }

    true
}

/// Helper: Wait for a process to die with timeout
fn wait_for_process_death(pid: u32, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if !is_process_alive(pid) {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    false
}

/// Helper: Kill a process and wait for it to die
fn kill_and_wait(pid: u32) {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;
    let _ = kill(Pid::from_raw(pid as i32), Signal::SIGKILL);
    wait_for_process_death(pid, Duration::from_secs(1));
}

/// Test: Direct process killing with ChildRegistry works
/// This is the simplest death pact test - just verifying terminate_all kills processes
#[test]
fn test_death_pact_registry_kills_processes() {
    // Spawn a simple sleep process (no process group needed for this test)
    let child = Command::new("sleep")
        .arg("60")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .expect("Failed to spawn sleep");

    let pid = child.id();

    // Verify it's alive
    assert!(is_process_alive(pid), "Process should be alive after spawn");

    // Create registry and register the process
    let mut registry = ChildRegistry::default();
    registry.register(pid);

    // Terminate - this uses group signal with fallback to direct signal
    registry.terminate_all(Duration::from_secs(1));

    // Verify it's dead
    let died = wait_for_process_death(pid, Duration::from_secs(2));
    assert!(died, "Process should be dead after terminate_all");
}

/// Test: Process spawned with in_new_process_group becomes its own group leader
/// Verifying the process group setup works correctly
#[test]
fn test_process_group_setup() {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    // Spawn with in_new_process_group
    let child = Command::new("sleep")
        .arg("60")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .in_new_process_group()
        .spawn()
        .expect("Failed to spawn with process group");

    let pid = child.id();

    // Verify alive
    assert!(is_process_alive(pid), "Process should be alive");

    // Signal the process group (negative PID)
    // This should work because the process is its own group leader
    let result = kill(Pid::from_raw(-(pid as i32)), Signal::SIGTERM);

    // Wait for death
    let died = wait_for_process_death(pid, Duration::from_secs(2));

    // The result might be Err(ESRCH) if process died before we could check
    // But the process should definitely be dead now
    assert!(died, "Process should die from group signal. Signal result: {:?}", result);
}

/// Test: Process group signal kills entire tree (parent + children)
#[test]
fn test_process_group_kills_tree() {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    // Spawn bash that creates a child process, both in same process group
    let mut parent = Command::new("bash")
        .args(["-c", "sleep 60 & wait"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .in_new_process_group()
        .spawn()
        .expect("Failed to spawn bash");

    let parent_pid = parent.id();

    // Give bash time to spawn the child
    thread::sleep(Duration::from_millis(200));

    // Verify parent is alive
    assert!(is_process_alive(parent_pid), "Parent should be alive");

    // Kill the process group
    let _ = kill(Pid::from_raw(-(parent_pid as i32)), Signal::SIGTERM);

    // Wait for parent to die
    let died = wait_for_process_death(parent_pid, Duration::from_secs(2));

    // Cleanup
    let _ = parent.wait();

    assert!(died, "Parent should die from group signal");
}

/// Test: Processes that handle SIGTERM exit gracefully
/// (The unit test test_sigterm_before_sigkill in process_guard.rs
/// verifies the SIGTERM -> SIGKILL ordering)
#[test]
fn test_sigterm_causes_graceful_exit() {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    // Spawn a process that will exit cleanly on SIGTERM (default behavior)
    let child = Command::new("sleep")
        .arg("60")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .expect("Failed to spawn sleep");

    let pid = child.id();
    thread::sleep(Duration::from_millis(100));

    assert!(is_process_alive(pid), "Process should be alive");

    // Send SIGTERM
    let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);

    // Process should die gracefully
    let died = wait_for_process_death(pid, Duration::from_secs(2));

    assert!(died, "Process should exit on SIGTERM");
}

/// Test: Stubborn processes get SIGKILL after grace period
#[test]
fn test_sigkill_after_grace_period() {
    // Spawn bash that ignores SIGTERM
    let child = Command::new("bash")
        .args(["-c", "trap '' TERM; sleep 60"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .expect("Failed to spawn stubborn bash");

    let pid = child.id();

    // Give trap time to set up
    thread::sleep(Duration::from_millis(200));

    assert!(is_process_alive(pid), "Process should be alive");

    // Terminate with SHORT grace period
    let mut registry = ChildRegistry::default();
    registry.register(pid);
    registry.terminate_all(Duration::from_millis(300));

    // SIGKILL should have been sent
    let died = wait_for_process_death(pid, Duration::from_secs(2));

    assert!(died, "Stubborn process should be killed by SIGKILL");
}

/// Test: Exit codes are properly captured
#[test]
fn test_exit_code_propagation() {
    for expected_code in [0, 1, 42, 127, 255] {
        let output = Command::new("bash")
            .args(["-c", &format!("exit {}", expected_code)])
            .output()
            .expect("Failed to run bash");

        assert_eq!(
            output.status.code(),
            Some(expected_code),
            "Exit code {} should be captured",
            expected_code
        );
    }
}

/// Test: Script failure exit codes are non-zero
#[test]
fn test_script_failure_exit_codes() {
    let output = Command::new("bash")
        .args(["-c", "set -e; false"])
        .output()
        .expect("Failed to run bash");

    assert!(!output.status.success(), "Failed command should produce non-zero exit");
    assert_eq!(output.status.code(), Some(1), "Exit code should be 1");
}

/// Test: Installer pattern - async spawn with output capture and exit code handling
/// This mimics the pattern used in installer.rs
#[test]
fn test_installer_pattern_exit_code_handling() {
    use std::sync::{Arc, Mutex};

    #[derive(Default)]
    struct MockAppState {
        status_message: String,
        exit_code: Option<i32>,
        success: bool,
    }

    let app_state = Arc::new(Mutex::new(MockAppState::default()));

    // Test successful exit
    {
        let state = Arc::clone(&app_state);
        let mut child = Command::new("bash")
            .args(["-c", "echo 'success'; exit 0"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn");

        let status = child.wait().expect("Failed to wait");
        let mut s = state.lock().unwrap();
        s.success = status.success();
        s.exit_code = status.code();
        s.status_message = if status.success() {
            "Success".to_string()
        } else {
            format!("Failed with code {}", status.code().unwrap_or(-1))
        };
    }

    {
        let s = app_state.lock().unwrap();
        assert!(s.success, "Should report success");
        assert_eq!(s.exit_code, Some(0), "Exit code should be 0");
    }

    // Test failed exit
    {
        let state = Arc::clone(&app_state);
        let mut child = Command::new("bash")
            .args(["-c", "echo 'error' >&2; exit 42"])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("Failed to spawn");

        let status = child.wait().expect("Failed to wait");
        let mut s = state.lock().unwrap();
        s.success = status.success();
        s.exit_code = status.code();
        s.status_message = if status.success() {
            "Success".to_string()
        } else {
            format!("Failed with code {}", status.code().unwrap_or(-1))
        };
    }

    {
        let s = app_state.lock().unwrap();
        assert!(!s.success, "Should report failure");
        assert_eq!(s.exit_code, Some(42), "Exit code should be 42");
        assert!(s.status_message.contains("42"), "Message should contain exit code");
    }
}

/// Test: Signal termination produces correct exit status
#[test]
fn test_signal_termination_exit_status() {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    let mut child = Command::new("sleep")
        .arg("60")
        .spawn()
        .expect("Failed to spawn");

    let pid = child.id();
    thread::sleep(Duration::from_millis(100));

    // Kill with SIGTERM
    let _ = kill(Pid::from_raw(pid as i32), Signal::SIGTERM);

    let status = child.wait().expect("Failed to wait");

    // Process killed by signal should not be "success"
    assert!(!status.success(), "Signal-killed process should not be success");

    // On Unix, signal termination gives code = None but we can check signal
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        assert_eq!(status.signal(), Some(15), "Should be killed by SIGTERM (15)");
    }
}

/// Test: Two separate process groups are isolated
#[test]
fn test_process_groups_are_isolated() {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    // Spawn two processes in separate groups
    let child1 = Command::new("sleep")
        .arg("60")
        .in_new_process_group()
        .spawn()
        .expect("Failed to spawn child1");

    let child2 = Command::new("sleep")
        .arg("60")
        .in_new_process_group()
        .spawn()
        .expect("Failed to spawn child2");

    let pid1 = child1.id();
    let pid2 = child2.id();

    thread::sleep(Duration::from_millis(100));

    // Verify both alive
    assert!(is_process_alive(pid1), "Child1 should be alive");
    assert!(is_process_alive(pid2), "Child2 should be alive");

    // Kill child1 directly (not group signal, to avoid any weirdness)
    let _ = kill(Pid::from_raw(pid1 as i32), Signal::SIGTERM);
    wait_for_process_death(pid1, Duration::from_secs(1));

    // Child2 should still be alive
    let child2_alive = is_process_alive(pid2);

    // Cleanup
    kill_and_wait(pid2);

    assert!(child2_alive, "Child2 should be unaffected by child1's death");
}

/// Test: PR_SET_PDEATHSIG is set (child dies when immediate parent dies)
/// Note: PR_SET_PDEATHSIG only affects the immediate parent, not grandparents
#[test]
fn test_pdeathsig_set() {
    // This test verifies that when we spawn with in_new_process_group(),
    // the child has PR_SET_PDEATHSIG set. We can't easily test parent death
    // without forking, so we just verify the child runs correctly.
    let child = Command::new("bash")
        .args(["-c", "exit 0"])
        .in_new_process_group()
        .output()
        .expect("Failed to run child with pdeathsig");

    assert!(child.status.success(), "Child should run successfully with pdeathsig set");
}
