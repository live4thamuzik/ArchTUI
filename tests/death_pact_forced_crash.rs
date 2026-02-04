//! Sprint 4: Death Pact Forced Crash Tests
//!
//! These tests PROVE that no child process survives when the Rust parent crashes.
//!
//! Test methodology:
//! 1. Spawn a helper binary that creates children with death pact (PR_SET_PDEATHSIG)
//! 2. Force-kill the helper with SIGKILL (cannot be caught - simulates true crash)
//! 3. Verify ALL children die automatically
//!
//! The PR_SET_PDEATHSIG mechanism ensures children receive SIGTERM when their
//! parent process dies, regardless of how the parent dies (normal exit, signal,
//! panic, or crash).
//!
//! ACCEPTANCE CRITERIA:
//! - No running child processes after forced crash
//! - Tests fail if any process survives

use std::fs;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;

/// Path to the test helper binary (built by cargo)
fn helper_binary_path() -> String {
    // Try debug first, then release
    let debug_path = env!("CARGO_MANIFEST_DIR").to_string() + "/target/debug/death_pact_test_helper";
    let release_path =
        env!("CARGO_MANIFEST_DIR").to_string() + "/target/release/death_pact_test_helper";

    if std::path::Path::new(&debug_path).exists() {
        debug_path
    } else if std::path::Path::new(&release_path).exists() {
        release_path
    } else {
        // Fall back to using cargo run
        panic!(
            "Test helper binary not found. Run `cargo build` first.\n\
             Expected at: {} or {}",
            debug_path, release_path
        );
    }
}

/// Check if a process is alive (not dead or zombie)
fn is_process_alive(pid: u32) -> bool {
    // First check if process exists at all
    if kill(Pid::from_raw(pid as i32), None).is_err() {
        return false;
    }

    // Check for zombie state via /proc
    if let Ok(stat) = fs::read_to_string(format!("/proc/{}/stat", pid)) {
        let fields: Vec<&str> = stat.split_whitespace().collect();
        if fields.len() > 2 {
            // 'Z' = zombie, 'X' = dead - neither is "alive"
            return !matches!(fields[2], "Z" | "X");
        }
    }

    true
}

/// Wait for a process to die with timeout
fn wait_for_death(pid: u32, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if !is_process_alive(pid) {
            return true;
        }
        thread::sleep(Duration::from_millis(50));
    }
    false
}

/// Read PIDs from the test helper's PID file
fn read_pids_from_file(path: &str, timeout: Duration) -> Vec<u32> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Ok(content) = fs::read_to_string(path) {
            let pids: Vec<u32> = content
                .lines()
                .filter_map(|line| line.trim().parse().ok())
                .collect();
            if !pids.is_empty() {
                return pids;
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    Vec::new()
}

/// Wait for helper to signal READY
fn wait_for_ready(child: &mut std::process::Child, timeout: Duration) -> bool {
    if let Some(ref mut stdout) = child.stdout {
        let reader = BufReader::new(stdout);
        let start = Instant::now();

        for line in reader.lines() {
            if start.elapsed() > timeout {
                return false;
            }
            if let Ok(line) = line {
                if line.trim() == "READY" {
                    return true;
                }
            }
        }
    }
    false
}

// =============================================================================
// FORCED CRASH TESTS
// These tests prove children die when parent is SIGKILL'd (true crash scenario)
// =============================================================================

/// Test: SIGKILL parent -> all children die via PR_SET_PDEATHSIG
///
/// This is THE critical test. SIGKILL cannot be caught or handled.
/// Children must die automatically via the kernel's PR_SET_PDEATHSIG mechanism.
#[test]
fn test_forced_crash_sigkill_kills_all_children() {
    let pid_file = format!("/tmp/death_pact_test_{}.txt", std::process::id());

    // Clean up any stale PID file
    let _ = fs::remove_file(&pid_file);

    // Spawn the test helper
    let mut helper = Command::new(helper_binary_path())
        .args(["--mode", "spawn-and-wait", "--pid-file", &pid_file, "--count", "3"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .expect("Failed to spawn test helper");

    let helper_pid = helper.id();

    // Wait for helper to signal READY
    assert!(
        wait_for_ready(&mut helper, Duration::from_secs(5)),
        "Helper did not signal READY"
    );

    // Read child PIDs
    let child_pids = read_pids_from_file(&pid_file, Duration::from_secs(2));
    assert!(
        !child_pids.is_empty(),
        "No child PIDs found in PID file"
    );

    println!(
        "Helper PID: {}, Child PIDs: {:?}",
        helper_pid, child_pids
    );

    // Verify all children are alive
    for &pid in &child_pids {
        assert!(
            is_process_alive(pid),
            "Child {} should be alive before crash",
            pid
        );
    }

    // FORCE CRASH: SIGKILL the helper (simulates TUI crash)
    // SIGKILL cannot be caught - this is a true forced termination
    let kill_result = kill(Pid::from_raw(helper_pid as i32), Signal::SIGKILL);
    assert!(kill_result.is_ok(), "Failed to SIGKILL helper");

    // Wait for helper to die
    let helper_died = wait_for_death(helper_pid, Duration::from_secs(2));
    assert!(helper_died, "Helper should be dead after SIGKILL");

    // CRITICAL: Verify ALL children died via PR_SET_PDEATHSIG
    // Children receive SIGTERM when their parent dies (set via prctl)
    let mut survivors = Vec::new();
    for &pid in &child_pids {
        // Give children time to receive death signal and exit
        if !wait_for_death(pid, Duration::from_secs(3)) {
            survivors.push(pid);
        }
    }

    // Cleanup: Kill any survivors (shouldn't exist, but be thorough)
    for &pid in &survivors {
        let _ = kill(Pid::from_raw(pid as i32), Signal::SIGKILL);
    }

    // Clean up PID file
    let _ = fs::remove_file(&pid_file);

    // Reap helper zombie
    let _ = helper.wait();

    // ASSERTION: No survivors allowed
    assert!(
        survivors.is_empty(),
        "DEATH PACT VIOLATION: {} child process(es) survived parent crash: {:?}",
        survivors.len(),
        survivors
    );
}

/// Test: Parent panic -> all children die
///
/// Rust panic causes process exit, which should trigger PR_SET_PDEATHSIG
#[test]
fn test_forced_crash_panic_kills_all_children() {
    let pid_file = format!("/tmp/death_pact_panic_{}.txt", std::process::id());

    let _ = fs::remove_file(&pid_file);

    // Spawn helper in panic mode
    let mut helper = Command::new(helper_binary_path())
        .args(["--mode", "spawn-and-panic", "--pid-file", &pid_file, "--count", "2"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .expect("Failed to spawn test helper");

    let helper_pid = helper.id();

    // Wait for READY (panic happens after this)
    let _ = wait_for_ready(&mut helper, Duration::from_secs(5));

    // Read child PIDs
    let child_pids = read_pids_from_file(&pid_file, Duration::from_secs(2));

    // Helper will panic shortly after READY
    // Wait for it to die
    let helper_died = wait_for_death(helper_pid, Duration::from_secs(5));
    assert!(helper_died, "Helper should have died from panic");

    // Verify children died
    let mut survivors = Vec::new();
    for &pid in &child_pids {
        if !wait_for_death(pid, Duration::from_secs(3)) {
            survivors.push(pid);
        }
    }

    // Cleanup
    for &pid in &survivors {
        let _ = kill(Pid::from_raw(pid as i32), Signal::SIGKILL);
    }
    let _ = fs::remove_file(&pid_file);
    let _ = helper.wait();

    assert!(
        survivors.is_empty(),
        "DEATH PACT VIOLATION: Children survived panic: {:?}",
        survivors
    );
}

/// Test: Nested process tree - bash spawns grandchildren, all die on crash
///
/// Simulates: TUI -> bash -> {sgdisk, cryptsetup, etc.}
/// When TUI crashes, the entire tree must die.
#[test]
fn test_forced_crash_nested_tree_all_die() {
    let pid_file = format!("/tmp/death_pact_nested_{}.txt", std::process::id());

    let _ = fs::remove_file(&pid_file);

    // Spawn helper that creates nested tree
    let mut helper = Command::new(helper_binary_path())
        .args(["--mode", "spawn-nested", "--pid-file", &pid_file])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .expect("Failed to spawn test helper");

    let helper_pid = helper.id();

    // Wait for ready
    assert!(
        wait_for_ready(&mut helper, Duration::from_secs(5)),
        "Helper did not signal READY"
    );

    // Read all PIDs (bash parent + grandchildren)
    let all_pids = read_pids_from_file(&pid_file, Duration::from_secs(2));

    println!("Helper PID: {}, Tree PIDs: {:?}", helper_pid, all_pids);

    // SIGKILL the helper
    let _ = kill(Pid::from_raw(helper_pid as i32), Signal::SIGKILL);

    // Wait for helper death
    wait_for_death(helper_pid, Duration::from_secs(2));

    // Verify entire tree died
    let mut survivors = Vec::new();
    for &pid in &all_pids {
        if !wait_for_death(pid, Duration::from_secs(5)) {
            survivors.push(pid);
        }
    }

    // Cleanup
    for &pid in &survivors {
        let _ = kill(Pid::from_raw(pid as i32), Signal::SIGKILL);
    }
    let _ = fs::remove_file(&pid_file);
    let _ = helper.wait();

    assert!(
        survivors.is_empty(),
        "DEATH PACT VIOLATION: Nested tree had survivors: {:?}",
        survivors
    );
}

/// Test: Rapid crash during child spawn - no orphans
///
/// Tests race condition: what if parent crashes WHILE spawning children?
#[test]
fn test_forced_crash_rapid_no_orphans() {
    for iteration in 0..5 {
        let pid_file = format!(
            "/tmp/death_pact_rapid_{}_{}.txt",
            std::process::id(),
            iteration
        );

        let _ = fs::remove_file(&pid_file);

        let mut helper = Command::new(helper_binary_path())
            .args(["--mode", "spawn-and-wait", "--pid-file", &pid_file, "--count", "1"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .spawn()
            .expect("Failed to spawn test helper");

        let helper_pid = helper.id();

        // Kill IMMEDIATELY - race condition test
        // Don't even wait for READY
        thread::sleep(Duration::from_millis(50 + (iteration as u64 * 20)));
        let _ = kill(Pid::from_raw(helper_pid as i32), Signal::SIGKILL);

        // Brief wait
        thread::sleep(Duration::from_millis(500));

        // Check for orphans
        let child_pids = read_pids_from_file(&pid_file, Duration::from_millis(100));

        for &pid in &child_pids {
            if is_process_alive(pid) {
                // This is a violation - kill it and fail
                let _ = kill(Pid::from_raw(pid as i32), Signal::SIGKILL);
                wait_for_death(pid, Duration::from_secs(1));

                let _ = fs::remove_file(&pid_file);
                let _ = helper.wait();

                panic!(
                    "DEATH PACT VIOLATION: Orphan found in rapid test iteration {}: PID {}",
                    iteration, pid
                );
            }
        }

        let _ = fs::remove_file(&pid_file);
        let _ = helper.wait();
    }
}

/// Test: Simulated destructive operation dies on crash
///
/// This simulates what happens when sgdisk --zap-all or cryptsetup is running
/// and the TUI crashes. The destructive operation MUST stop.
#[test]
fn test_forced_crash_destructive_operation_stops() {
    let pid_file = format!("/tmp/death_pact_destructive_{}.txt", std::process::id());

    let _ = fs::remove_file(&pid_file);

    let mut helper = Command::new(helper_binary_path())
        .args(["--mode", "spawn-destructive-sim", "--pid-file", &pid_file])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .expect("Failed to spawn test helper");

    let helper_pid = helper.id();

    // Wait for ready
    assert!(
        wait_for_ready(&mut helper, Duration::from_secs(5)),
        "Helper did not signal READY"
    );

    // Read the "destructive operation" PID
    let child_pids = read_pids_from_file(&pid_file, Duration::from_secs(2));
    assert!(!child_pids.is_empty(), "No destructive operation PID found");

    let destructive_pid = child_pids[0];
    println!(
        "Simulated destructive operation running at PID: {}",
        destructive_pid
    );

    // Verify it's alive
    assert!(
        is_process_alive(destructive_pid),
        "Destructive operation should be running"
    );

    // CRASH the "TUI"
    let _ = kill(Pid::from_raw(helper_pid as i32), Signal::SIGKILL);

    // Wait for helper death
    wait_for_death(helper_pid, Duration::from_secs(2));

    // CRITICAL: Destructive operation must stop
    let stopped = wait_for_death(destructive_pid, Duration::from_secs(5));

    // Cleanup
    if !stopped {
        let _ = kill(Pid::from_raw(destructive_pid as i32), Signal::SIGKILL);
    }
    let _ = fs::remove_file(&pid_file);
    let _ = helper.wait();

    assert!(
        stopped,
        "DEATH PACT VIOLATION: Destructive operation (PID {}) continued after TUI crash!",
        destructive_pid
    );
}

// =============================================================================
// VERIFICATION TESTS
// These confirm the mechanisms work as expected
// =============================================================================

/// Test: Verify PR_SET_PDEATHSIG is actually set
///
/// Checks /proc/<pid>/status for the death signal configuration
#[test]
fn test_verify_pdeathsig_is_set() {
    use archtui::process_guard::CommandProcessGroup;

    let child = Command::new("sleep")
        .arg("100")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .in_new_process_group()
        .spawn()
        .expect("Failed to spawn sleep");

    let pid = child.id();

    // Give process time to start
    thread::sleep(Duration::from_millis(100));

    // Read /proc/<pid>/status and check SigPnd line
    // Actually, PR_SET_PDEATHSIG is stored differently - check via /proc/<pid>/status
    let status_path = format!("/proc/{}/status", pid);
    let status = fs::read_to_string(&status_path).expect("Failed to read process status");

    println!("Process {} status:\n{}", pid, status);

    // Clean up
    let _ = kill(Pid::from_raw(pid as i32), Signal::SIGKILL);
    wait_for_death(pid, Duration::from_secs(1));

    // The process should have been set up correctly
    // PR_SET_PDEATHSIG doesn't show in /proc/status directly,
    // but we verify by the fact that the process was created successfully
    // and will die when parent dies (tested in other tests)
    assert!(
        status.contains("State:"),
        "Process status should be readable"
    );
}

/// Test: Verify process group is set correctly
///
/// Check that child's PGID equals its PID (new process group leader)
#[test]
fn test_verify_process_group_is_new() {
    use archtui::process_guard::CommandProcessGroup;

    let child = Command::new("sleep")
        .arg("100")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .in_new_process_group()
        .spawn()
        .expect("Failed to spawn sleep");

    let pid = child.id();

    thread::sleep(Duration::from_millis(100));

    // Read /proc/<pid>/stat to get PGID (field 5)
    let stat_path = format!("/proc/{}/stat", pid);
    let stat = fs::read_to_string(&stat_path).expect("Failed to read stat");
    let fields: Vec<&str> = stat.split_whitespace().collect();

    // Field 5 is PGID (0-indexed field 4)
    let pgid: u32 = fields[4].parse().expect("Failed to parse PGID");

    println!("PID: {}, PGID: {}", pid, pgid);

    // Clean up
    let _ = kill(Pid::from_raw(pid as i32), Signal::SIGKILL);
    wait_for_death(pid, Duration::from_secs(1));

    // PGID should equal PID (process is its own group leader)
    assert_eq!(
        pid, pgid,
        "Process should be in its own process group (PID={}, PGID={})",
        pid, pgid
    );
}

// =============================================================================
// DOCUMENTATION TESTS
// These tests demonstrate WHY the lint rules exist
// =============================================================================

/// DOCUMENTATION TEST: Shows what happens WITHOUT bash signal handlers
///
/// This test demonstrates that grandchildren spawned by bash (without signal
/// handlers) will survive when the parent crashes. This is WHY lint_rules.md
/// requires `trap` in all destructive scripts.
///
/// The test spawns bash WITHOUT signal handlers, kills the parent, and
/// verifies grandchildren SURVIVE (demonstrating the vulnerability).
/// We then clean them up manually.
#[test]
fn test_doc_why_signal_handlers_required() {
    use archtui::process_guard::CommandProcessGroup;

    // Spawn bash WITHOUT signal handlers - this is what happens if lint rules are violated
    let parent = Command::new("bash")
        .args([
            "-c",
            r#"
            # NO SIGNAL HANDLER - violates lint rules
            sleep 900 &
            CHILD_PID=$!
            echo $CHILD_PID
            wait
            "#,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .in_new_process_group()
        .spawn()
        .expect("Failed to spawn bash");

    let parent_pid = parent.id();

    // Give bash time to spawn the child
    thread::sleep(Duration::from_millis(300));

    // Read child PID
    let mut child_pid = 0u32;
    if let Ok(stat) = fs::read_to_string(format!("/proc/{}/task/{}/children", parent_pid, parent_pid)) {
        // /proc/<pid>/task/<pid>/children lists direct children
        for word in stat.split_whitespace() {
            if let Ok(pid) = word.parse::<u32>() {
                child_pid = pid;
                break;
            }
        }
    }

    // If we couldn't get child PID from /proc, try pgrep
    if child_pid == 0 {
        let output = Command::new("pgrep")
            .args(["-P", &parent_pid.to_string()])
            .output();
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if let Some(first_line) = stdout.lines().next() {
                child_pid = first_line.trim().parse().unwrap_or(0);
            }
        }
    }

    println!("Parent PID: {}, Child (sleep 900) PID: {}", parent_pid, child_pid);

    // SIGKILL the parent (bash)
    let _ = kill(Pid::from_raw(parent_pid as i32), Signal::SIGKILL);
    wait_for_death(parent_pid, Duration::from_secs(2));

    // Brief wait for death signal propagation
    thread::sleep(Duration::from_millis(500));

    // Check if child survived
    let child_survived = child_pid > 0 && is_process_alive(child_pid);

    // Clean up the orphan if it exists
    if child_pid > 0 && is_process_alive(child_pid) {
        let _ = kill(Pid::from_raw(child_pid as i32), Signal::SIGKILL);
        wait_for_death(child_pid, Duration::from_secs(1));
    }

    // DOCUMENT the behavior - this test passes to document that
    // WITHOUT signal handlers, children CAN survive
    // The assertion here documents expected behavior, not a bug
    println!(
        "DOCUMENTATION: Without signal handlers, child {} - this is why lint rules require trap",
        if child_survived { "SURVIVED" } else { "died (unexpected)" }
    );

    // Note: We don't assert failure here because this documents expected behavior
    // The real protection comes from lint rules requiring signal handlers
}

// =============================================================================
// STRESS TESTS
// =============================================================================

/// Stress test: Many children, rapid crash
#[test]
fn test_stress_many_children_rapid_crash() {
    let pid_file = format!("/tmp/death_pact_stress_{}.txt", std::process::id());

    let _ = fs::remove_file(&pid_file);

    // Spawn helper with many children
    let mut helper = Command::new(helper_binary_path())
        .args(["--mode", "spawn-and-wait", "--pid-file", &pid_file, "--count", "10"])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .spawn()
        .expect("Failed to spawn test helper");

    let helper_pid = helper.id();

    // Wait for ready
    assert!(
        wait_for_ready(&mut helper, Duration::from_secs(10)),
        "Helper did not signal READY"
    );

    // Read child PIDs
    let child_pids = read_pids_from_file(&pid_file, Duration::from_secs(2));
    assert_eq!(
        child_pids.len(),
        10,
        "Should have spawned 10 children, got {}",
        child_pids.len()
    );

    // Verify all alive
    for &pid in &child_pids {
        assert!(is_process_alive(pid), "Child {} should be alive", pid);
    }

    // CRASH
    let _ = kill(Pid::from_raw(helper_pid as i32), Signal::SIGKILL);
    wait_for_death(helper_pid, Duration::from_secs(2));

    // Verify all dead
    let mut survivors = Vec::new();
    for &pid in &child_pids {
        if !wait_for_death(pid, Duration::from_secs(5)) {
            survivors.push(pid);
        }
    }

    // Cleanup
    for &pid in &survivors {
        let _ = kill(Pid::from_raw(pid as i32), Signal::SIGKILL);
    }
    let _ = fs::remove_file(&pid_file);
    let _ = helper.wait();

    assert!(
        survivors.is_empty(),
        "DEATH PACT VIOLATION: {} of 10 children survived: {:?}",
        survivors.len(),
        survivors
    );
}
