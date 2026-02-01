//! Test helper binary for death pact integration tests
//!
//! This binary simulates a "Rust TUI" process that spawns children with death pact.
//! The test harness spawns this helper, then kills it to verify children die.
//!
//! Usage:
//!   death_pact_test_helper --mode <mode> --pid-file <path>
//!
//! Modes:
//!   spawn-and-wait: Spawn children, write PIDs to file, wait forever
//!   spawn-and-panic: Spawn children, write PIDs to file, then panic
//!   spawn-nested: Spawn bash that spawns grandchildren, write all PIDs

use std::env;
use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use archinstall_tui::process_guard::CommandProcessGroup;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut mode = "spawn-and-wait";
    let mut pid_file = "/tmp/death_pact_pids.txt";
    let mut child_count = 3;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                mode = args.get(i + 1).map(|s| s.as_str()).unwrap_or(mode);
                i += 2;
            }
            "--pid-file" => {
                pid_file = args.get(i + 1).map(|s| s.as_str()).unwrap_or(pid_file);
                i += 2;
            }
            "--count" => {
                child_count = args
                    .get(i + 1)
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(child_count);
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }

    match mode {
        "spawn-and-wait" => spawn_and_wait(pid_file, child_count),
        "spawn-and-panic" => spawn_and_panic(pid_file, child_count),
        "spawn-nested" => spawn_nested(pid_file),
        "spawn-destructive-sim" => spawn_destructive_simulation(pid_file),
        _ => {
            eprintln!("Unknown mode: {}", mode);
            std::process::exit(1);
        }
    }
}

/// Spawn children with death pact, write PIDs, wait forever
fn spawn_and_wait(pid_file: &str, count: usize) {
    let pids = spawn_children(count);
    write_pids(pid_file, &pids);

    // Signal readiness
    println!("READY");

    // Wait forever (until killed)
    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}

/// Spawn children with death pact, write PIDs, then panic
fn spawn_and_panic(pid_file: &str, count: usize) {
    let pids = spawn_children(count);
    write_pids(pid_file, &pids);

    // Signal readiness
    println!("READY");

    // Small delay to ensure PIDs are written
    thread::sleep(Duration::from_millis(100));

    // Panic! This tests if children survive a Rust panic
    panic!("Intentional panic for death pact test");
}

/// Spawn bash that spawns grandchildren (nested tree)
/// Uses proper signal handling as required by lint_rules.md
fn spawn_nested(pid_file: &str) {
    // Spawn bash that creates a process tree WITH proper signal handling
    // This simulates our real scripts that must trap SIGTERM/SIGINT
    // The trap ensures children are killed when bash receives SIGTERM
    let nested_pid_file = format!("{}.nested", pid_file);

    let parent = Command::new("bash")
        .args([
            "-c",
            &format!(
                r#"
                set -euo pipefail

                # Track child PIDs for cleanup
                CHILD_PIDS=""

                # Signal handler - kill all children on TERM/INT
                cleanup() {{
                    if [[ -n "$CHILD_PIDS" ]]; then
                        kill $CHILD_PIDS 2>/dev/null || true
                    fi
                    exit 0
                }}
                trap cleanup TERM INT

                # Spawn children
                sleep 800 &
                PID1=$!
                CHILD_PIDS="$PID1"

                sleep 801 &
                PID2=$!
                CHILD_PIDS="$CHILD_PIDS $PID2"

                sleep 802 &
                PID3=$!
                CHILD_PIDS="$CHILD_PIDS $PID3"

                # Write PIDs for test verification
                echo "$PID1 $PID2 $PID3" > "{}"

                # Wait for children (will be interrupted by signal)
                wait
                "#,
                nested_pid_file
            ),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .in_new_process_group()
        .spawn()
        .expect("Failed to spawn bash parent");

    let parent_pid = parent.id();

    // Give bash time to spawn children and write PIDs
    thread::sleep(Duration::from_millis(500));

    // Read grandchild PIDs from the temp file
    let mut grandchild_pids = Vec::new();
    if let Ok(content) = std::fs::read_to_string(&nested_pid_file) {
        for word in content.trim().split_whitespace() {
            if let Ok(pid) = word.parse::<u32>() {
                grandchild_pids.push(pid);
            }
        }
    }

    // Clean up temp file
    let _ = std::fs::remove_file(&nested_pid_file);

    // Write all PIDs: parent first, then grandchildren
    let mut all_pids = vec![parent_pid];
    all_pids.extend(grandchild_pids);
    write_pids(pid_file, &all_pids);

    // Signal readiness
    println!("READY");

    // Wait forever
    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}

/// Simulate a destructive operation (like disk wiping)
fn spawn_destructive_simulation(pid_file: &str) {
    // Spawn a bash script that simulates a long-running destructive operation
    // This is what sgdisk or cryptsetup would look like
    let child = Command::new("bash")
        .args([
            "-c",
            r#"
            # Simulate destructive operation with signal handling
            trap 'echo "SIGTERM received, aborting"; exit 143' TERM
            trap 'echo "SIGINT received, aborting"; exit 130' INT

            # Simulate long operation
            for i in $(seq 1 1000); do
                sleep 1
            done
            "#,
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .stdin(Stdio::null())
        .in_new_process_group()
        .spawn()
        .expect("Failed to spawn destructive simulation");

    let pid = child.id();
    write_pids(pid_file, &[pid]);

    println!("READY");

    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}

/// Spawn simple sleep children with death pact
fn spawn_children(count: usize) -> Vec<u32> {
    let mut pids = Vec::new();

    for i in 0..count {
        let child = Command::new("sleep")
            .arg(format!("{}", 600 + i)) // sleep 600, 601, 602, ...
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null())
            .in_new_process_group()
            .spawn()
            .expect("Failed to spawn sleep child");

        pids.push(child.id());
    }

    // Allow children to start
    thread::sleep(Duration::from_millis(100));

    pids
}

/// Write PIDs to file, one per line
fn write_pids(path: &str, pids: &[u32]) {
    let mut file = File::create(path).expect("Failed to create PID file");
    for pid in pids {
        writeln!(file, "{}", pid).expect("Failed to write PID");
    }
    file.flush().expect("Failed to flush PID file");
}
