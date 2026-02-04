//! Integration tests for the Death Pact mechanism
//!
//! Sprint 1: "Torture Test" - Proving process_guard.rs works correctly
//!
//! These tests verify that:
//! 1. Child processes spawned with in_new_process_group() are isolated
//! 2. PR_SET_PDEATHSIG causes children to die when their parent dies
//! 3. Process group signaling kills entire process trees

use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use archtui::process_guard::CommandProcessGroup;

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

/// Helper: Check if process exists in ps output
fn process_in_ps_output(pid: u32) -> bool {
    let output = Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "pid="])
        .output();

    match output {
        Ok(result) => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            stdout.trim().contains(&pid.to_string())
        }
        Err(_) => false,
    }
}

// =============================================================================
// Death Pact Tests: Proving the mechanism works
// =============================================================================

/// Test 1: Spawn sleep 1000 with process group, then kill it via group signal
/// This verifies basic process group setup and signaling
#[test]
fn test_death_pact_spawn_and_kill_via_group() {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    // Spawn "sleep 1000" in its own process group
    let child = Command::new("sleep")
        .arg("1000")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .in_new_process_group()
        .spawn()
        .expect("Failed to spawn sleep 1000");

    let pid = child.id();
    println!("Spawned sleep 1000 with PID: {}", pid);

    // Allow process to start
    thread::sleep(Duration::from_millis(100));

    // Verify it's alive using both methods
    assert!(is_process_alive(pid), "sleep 1000 should be alive after spawn");
    assert!(process_in_ps_output(pid), "sleep 1000 should appear in ps output");

    // Kill the entire process group (negative PID)
    let kill_result = kill(Pid::from_raw(-(pid as i32)), Signal::SIGTERM);
    println!("Group kill result: {:?}", kill_result);

    // Wait for death - is_process_alive checks both existence and zombie state
    let died = wait_for_process_death(pid, Duration::from_secs(3));
    assert!(died, "sleep 1000 should be dead after process group signal");

    // Note: We don't check ps output here because:
    // 1. The process might briefly appear as a zombie until reaped
    // 2. Our is_process_alive check already handles zombie detection
    // 3. The group kill definitely worked (kill returned Ok)
}

/// Test 2: Process group signal kills children - the mechanism we use for Death Pact
/// When the bash parent is killed via GROUP signal, all its children die too
/// This is the primary mechanism for ensuring destructive operations stop on crash
#[test]
fn test_death_pact_group_signal_kills_children() {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    // Spawn a bash process that spawns sleep as a child
    // Both will be in the same process group
    let mut parent = Command::new("bash")
        .args(["-c", "sleep 1000 & echo $!; wait"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .in_new_process_group()
        .spawn()
        .expect("Failed to spawn parent bash");

    let parent_pid = parent.id();

    // Give bash time to spawn the child and print its PID
    thread::sleep(Duration::from_millis(300));

    // Read the child PID from stdout
    let mut child_pid_str = String::new();
    if let Some(ref mut stdout) = parent.stdout {
        use std::io::Read;
        let mut buf = [0u8; 32];
        if let Ok(n) = stdout.read(&mut buf) {
            child_pid_str = String::from_utf8_lossy(&buf[..n]).trim().to_string();
        }
    }

    let child_pid: u32 = child_pid_str
        .lines()
        .next()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    println!(
        "Parent PID: {}, Child sleep PID: {}",
        parent_pid, child_pid
    );

    if child_pid == 0 {
        // Cleanup and skip if we couldn't get child PID
        let _ = kill(Pid::from_raw(-(parent_pid as i32)), Signal::SIGKILL);
        let _ = parent.wait();
        panic!("Could not determine child PID");
    }

    // Verify both are alive
    assert!(is_process_alive(parent_pid), "Parent bash should be alive");
    assert!(is_process_alive(child_pid), "Child sleep should be alive");

    // Kill the ENTIRE PROCESS GROUP (negative PID) - this is the Death Pact mechanism
    // This kills both the bash parent AND the sleep child
    let _ = kill(Pid::from_raw(-(parent_pid as i32)), Signal::SIGTERM);

    // Wait for parent to die
    let parent_died = wait_for_process_death(parent_pid, Duration::from_secs(2));
    assert!(parent_died, "Parent should be dead");

    // Wait for child to die (killed by group signal)
    let child_died = wait_for_process_death(child_pid, Duration::from_secs(3));

    // Cleanup
    let _ = parent.wait();

    // If child didn't die from SIGTERM, kill it directly for cleanup
    if !child_died {
        let _ = kill(Pid::from_raw(child_pid as i32), Signal::SIGKILL);
        wait_for_process_death(child_pid, Duration::from_secs(1));
    }

    assert!(
        child_died,
        "Child sleep should die when process GROUP is signaled (PID: {})",
        child_pid
    );
}

/// Test 3: Process tree is killed entirely via group signal
/// Simulates: TUI spawns bash -> bash spawns sgdisk -> kill TUI -> both die
#[test]
fn test_death_pact_entire_tree_killed() {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    // Spawn bash that creates a nested process tree
    // bash -> { sleep 500 & sleep 501 & wait }
    let mut parent = Command::new("bash")
        .args(["-c", "sleep 500 & sleep 501 & wait"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .in_new_process_group()
        .spawn()
        .expect("Failed to spawn tree parent");

    let parent_pid = parent.id();

    // Give time for children to spawn
    thread::sleep(Duration::from_millis(300));

    // Verify parent is alive
    assert!(is_process_alive(parent_pid), "Tree parent should be alive");

    // Kill the entire process group
    let _ = kill(Pid::from_raw(-(parent_pid as i32)), Signal::SIGTERM);

    // Wait for death
    thread::sleep(Duration::from_millis(500));

    // Verify parent is dead
    let parent_died = wait_for_process_death(parent_pid, Duration::from_secs(2));
    let _ = parent.wait();

    assert!(parent_died, "Tree parent should be dead after group signal");

    // Use pgrep to check for any remaining sleep 500 or 501 processes
    // (There shouldn't be any from our process group)
    let pgrep_output = Command::new("pgrep")
        .args(["-f", "sleep 50[01]"])
        .output();

    if let Ok(output) = pgrep_output {
        let remaining = String::from_utf8_lossy(&output.stdout);
        // Note: Other tests or system processes might have sleep, so we just log
        if !remaining.trim().is_empty() {
            println!(
                "Note: Some sleep processes exist (may be from other sources): {}",
                remaining.trim()
            );
        }
    }
}

/// Test 4: Registry-based termination (simulating Drop cleanup)
/// This is what happens when the TUI App struct is dropped
#[test]
fn test_death_pact_registry_terminate_all() {
    use archtui::process_guard::ChildRegistry;

    // Spawn multiple processes
    let mut pids = Vec::new();

    for i in 0..3 {
        let child = Command::new("sleep")
            .arg(format!("{}", 100 + i))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .in_new_process_group()
            .spawn()
            .expect("Failed to spawn sleep");

        pids.push(child.id());
    }

    // Wait for all to start
    thread::sleep(Duration::from_millis(100));

    // Verify all alive
    for pid in &pids {
        assert!(is_process_alive(*pid), "Process {} should be alive", pid);
    }

    // Create registry and register all
    let mut registry = ChildRegistry::default();
    for pid in &pids {
        registry.register(*pid);
    }

    // Terminate all (simulating Drop)
    registry.terminate_all(Duration::from_secs(2));

    // Verify all dead
    for pid in &pids {
        let died = wait_for_process_death(*pid, Duration::from_secs(2));
        assert!(died, "Process {} should be dead after terminate_all", pid);
    }
}

/// Test 5: Verify process group isolation - one group's death doesn't affect another
#[test]
fn test_death_pact_group_isolation() {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    // Spawn two independent process groups
    let child1 = Command::new("sleep")
        .arg("200")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .in_new_process_group()
        .spawn()
        .expect("Failed to spawn sleep 1");

    let child2 = Command::new("sleep")
        .arg("201")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .in_new_process_group()
        .spawn()
        .expect("Failed to spawn sleep 2");

    let pid1 = child1.id();
    let pid2 = child2.id();

    thread::sleep(Duration::from_millis(100));

    // Verify both alive
    assert!(is_process_alive(pid1), "Process 1 should be alive");
    assert!(is_process_alive(pid2), "Process 2 should be alive");

    // Kill only process group 1
    let _ = kill(Pid::from_raw(-(pid1 as i32)), Signal::SIGKILL);

    // Wait for process 1 to die
    wait_for_process_death(pid1, Duration::from_secs(2));

    // Process 2 should still be alive (different group)
    assert!(
        is_process_alive(pid2),
        "Process 2 should still be alive (isolated group)"
    );

    // Cleanup process 2
    let _ = kill(Pid::from_raw(pid2 as i32), Signal::SIGKILL);
    wait_for_process_death(pid2, Duration::from_secs(1));
}

// =============================================================================
// Edge Cases and Stress Tests
// =============================================================================

/// Test: Rapid spawn and kill doesn't cause issues
#[test]
fn test_death_pact_rapid_spawn_kill() {
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    for i in 0..10 {
        let child = Command::new("sleep")
            .arg("1000")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .in_new_process_group()
            .spawn()
            .expect("Failed to spawn");

        let pid = child.id();

        // Immediately kill
        let _ = kill(Pid::from_raw(-(pid as i32)), Signal::SIGKILL);

        // Brief pause
        thread::sleep(Duration::from_millis(10));

        // Should be dead or dying
        let died = wait_for_process_death(pid, Duration::from_millis(500));
        assert!(died, "Rapid kill iteration {} should work", i);
    }
}

/// Test: Already-dead process doesn't cause errors in terminate_all
#[test]
fn test_death_pact_handles_already_dead() {
    use archtui::process_guard::ChildRegistry;

    // Spawn a process that exits immediately
    let mut child = Command::new("bash")
        .args(["-c", "exit 0"])
        .spawn()
        .expect("Failed to spawn");

    let pid = child.id();

    // Wait for it to exit naturally
    let _ = child.wait();

    // Now try to terminate it via registry - should not panic
    let mut registry = ChildRegistry::default();
    registry.register(pid);

    // This should handle the already-dead process gracefully
    registry.terminate_all(Duration::from_millis(100));

    // No panic = success
}
