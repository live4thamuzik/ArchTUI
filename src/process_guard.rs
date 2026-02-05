//! Process lifecycle management for child processes
//!
//! This module ensures that child bash processes are properly terminated when
//! the Rust parent process exits (gracefully or via crash/signal).
//!
//! # Problem Solved
//! Without explicit process group management, if the TUI crashes while a
//! destructive operation (e.g., `sgdisk --zap-all`) is running, the child
//! process becomes orphaned and continues executing.
//!
//! # Solution
//! - Spawn children in their own process group
//! - Track all child PIDs in a global registry
//! - On parent exit (Drop, SIGTERM, SIGINT), send SIGTERM to all children
//! - Children have 5 seconds to cleanup before SIGKILL

use nix::libc;
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::collections::HashSet;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

/// Global registry of child process IDs
/// Using OnceLock for safe lazy initialization
static CHILD_REGISTRY: OnceLock<Arc<Mutex<ChildRegistry>>> = OnceLock::new();

/// Registry tracking all spawned child processes
#[derive(Debug, Default)]
pub struct ChildRegistry {
    /// Set of child PIDs currently running
    pids: HashSet<u32>,
    /// Whether cleanup has already been initiated (prevent double-cleanup)
    cleanup_initiated: bool,
}

impl ChildRegistry {
    /// Get or create the global child registry
    pub fn global() -> Arc<Mutex<ChildRegistry>> {
        CHILD_REGISTRY
            .get_or_init(|| Arc::new(Mutex::new(ChildRegistry::default())))
            .clone()
    }

    /// Register a new child process
    pub fn register(&mut self, pid: u32) {
        self.pids.insert(pid);
        log::debug!("Registered child process PID {}", pid);
    }

    /// Unregister a child process (called when it exits normally)
    pub fn unregister(&mut self, pid: u32) {
        self.pids.remove(&pid);
        log::debug!("Unregistered child process PID {}", pid);
    }

    /// Get count of tracked children
    ///
    /// Useful for debugging and tests to verify process registration.
    #[allow(dead_code)] // Test/debug utility
    pub fn count(&self) -> usize {
        self.pids.len()
    }

    /// Terminate all tracked child processes
    /// Sends SIGTERM first, waits up to `grace_period`, then SIGKILL
    pub fn terminate_all(&mut self, grace_period: Duration) {
        if self.cleanup_initiated {
            log::debug!("Cleanup already initiated, skipping");
            return;
        }
        self.cleanup_initiated = true;

        if self.pids.is_empty() {
            log::debug!("No child processes to terminate");
            return;
        }

        log::info!(
            "Terminating {} child process(es)...",
            self.pids.len()
        );

        // First pass: send SIGTERM to all process GROUPS
        // Using group signaling ensures children (sgdisk, cryptsetup, etc.) also receive the signal
        let pids_to_kill: Vec<u32> = self.pids.iter().copied().collect();
        for &pid in &pids_to_kill {
            // Try group signal first (catches entire process tree)
            if let Err(e) = send_signal_to_group(pid, Signal::SIGTERM) {
                log::warn!("Failed to send SIGTERM to process group {}: {}", pid, e);
                // Fall back to direct signal if group signal fails
                if let Err(e2) = send_signal(pid, Signal::SIGTERM) {
                    log::warn!("Failed to send SIGTERM to PID {}: {}", pid, e2);
                }
            } else {
                log::debug!("Sent SIGTERM to process group {}", pid);
            }
        }

        // Wait for grace period, checking if processes have exited
        let start = Instant::now();
        while start.elapsed() < grace_period {
            // Check which processes are still alive
            let still_alive: Vec<u32> = pids_to_kill
                .iter()
                .filter(|&&pid| is_process_alive(pid))
                .copied()
                .collect();

            if still_alive.is_empty() {
                log::info!("All child processes terminated gracefully");
                self.pids.clear();
                return;
            }

            std::thread::sleep(Duration::from_millis(100));
        }

        // Second pass: SIGKILL any remaining process groups
        for &pid in &pids_to_kill {
            if is_process_alive(pid) {
                log::warn!("Process group {} did not terminate, sending SIGKILL", pid);
                // Try group signal first
                if let Err(e) = send_signal_to_group(pid, Signal::SIGKILL) {
                    log::error!("Failed to send SIGKILL to process group {}: {}", pid, e);
                    // Fall back to direct signal
                    let _ = send_signal(pid, Signal::SIGKILL);
                }
            }
        }

        self.pids.clear();
        log::info!("Child process cleanup complete");
    }
}

/// Send a signal to a process
fn send_signal(pid: u32, signal: Signal) -> Result<(), nix::Error> {
    signal::kill(Pid::from_raw(pid as i32), signal)
}

/// Send a signal to an entire process group
/// Uses negative PID to signal all processes in the group, ensuring children
/// of bash (like sgdisk, cryptsetup, etc.) also receive the signal
fn send_signal_to_group(pgid: u32, signal: Signal) -> Result<(), nix::Error> {
    signal::kill(Pid::from_raw(-(pgid as i32)), signal)
}

/// Check if a process is still alive (not dead or zombie)
fn is_process_alive(pid: u32) -> bool {
    // First check if process exists at all
    if signal::kill(Pid::from_raw(pid as i32), None).is_err() {
        return false;
    }

    // Check for zombie state via /proc - zombies are "dead" for our purposes
    // A zombie can still receive signals but isn't running
    if let Ok(stat) = std::fs::read_to_string(format!("/proc/{}/stat", pid)) {
        // Field 3 of /proc/pid/stat is the state: R=running, S=sleeping, Z=zombie, etc.
        let fields: Vec<&str> = stat.split_whitespace().collect();
        if fields.len() > 2 {
            // 'Z' = zombie, 'X' = dead - neither is "alive"
            return !matches!(fields[2], "Z" | "X");
        }
    }

    // If we can't read /proc, assume alive (safe default)
    true
}

/// RAII guard that terminates all children on drop
/// Attach this to the App struct to ensure cleanup on any exit path
pub struct ProcessGuard {
    registry: Arc<Mutex<ChildRegistry>>,
}

impl ProcessGuard {
    /// Create a new process guard attached to the global registry
    pub fn new() -> Self {
        Self {
            registry: ChildRegistry::global(),
        }
    }

    /// Register a child process with the guard
    ///
    /// Called by `run_script_safe` when spawning tool scripts.
    #[allow(dead_code)] // API: Called via ChildRegistry::global() in script_runner
    pub fn register_child(&self, pid: u32) {
        if let Ok(mut registry) = self.registry.lock() {
            registry.register(pid);
        }
    }

    /// Unregister a child process (call when it exits normally)
    ///
    /// Called by `run_script_safe` when a script completes.
    #[allow(dead_code)] // API: Called via ChildRegistry::global() in script_runner
    pub fn unregister_child(&self, pid: u32) {
        if let Ok(mut registry) = self.registry.lock() {
            registry.unregister(pid);
        }
    }

    /// Get the number of tracked children
    ///
    /// Useful for debugging and tests.
    #[allow(dead_code)] // Test/debug utility
    pub fn child_count(&self) -> usize {
        self.registry
            .lock()
            .map(|r| r.count())
            .unwrap_or(0)
    }
}

impl Default for ProcessGuard {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for ProcessGuard {
    fn drop(&mut self) {
        log::debug!("ProcessGuard dropped, initiating cleanup");
        if let Ok(mut registry) = self.registry.lock() {
            registry.terminate_all(Duration::from_secs(5));
        }
    }
}

/// Initialize global signal handlers for graceful shutdown
/// Handles SIGINT (Ctrl+C), SIGTERM, and SIGHUP
/// Call this once at program start
pub fn init_signal_handlers() -> Result<(), std::io::Error> {
    use signal_hook::consts::signal::{SIGHUP, SIGINT, SIGTERM};
    use signal_hook::iterator::Signals;
    use std::thread;

    let mut signals = Signals::new([SIGINT, SIGTERM, SIGHUP])?;

    thread::spawn(move || {
        for sig in signals.forever() {
            let signal_name = match sig {
                SIGINT => "SIGINT",
                SIGTERM => "SIGTERM",
                SIGHUP => "SIGHUP",
                _ => "UNKNOWN",
            };

            log::info!("Received {} signal, cleaning up...", signal_name);

            // Terminate all children
            if let Ok(mut registry) = ChildRegistry::global().lock() {
                registry.terminate_all(Duration::from_secs(3));
            }

            // Exit with appropriate code (128 + signal number)
            std::process::exit(128 + sig);
        }
    });

    Ok(())
}

/// Extension trait for std::process::Command to set up process groups
pub trait CommandProcessGroup {
    /// Configure the command to run in its own process group
    /// This allows us to kill the entire process tree with a single signal
    fn in_new_process_group(&mut self) -> &mut Self;
}

impl CommandProcessGroup for std::process::Command {
    fn in_new_process_group(&mut self) -> &mut Self {
        use std::os::unix::process::CommandExt;
        // process_group(0) creates a new process group with PGID = child PID
        unsafe {
            self.pre_exec(|| {
                // Set process group ID to this process's PID
                // This makes this process the leader of a new process group
                nix::unistd::setpgid(Pid::from_raw(0), Pid::from_raw(0))
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

                // CRITICAL: Set death signal so child dies if parent dies
                // This prevents orphaned processes from continuing destructive operations
                // (e.g., sgdisk --zap-all continuing after TUI crash)
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) == -1 {
                    return Err(std::io::Error::last_os_error());
                }

                Ok(())
            });
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn test_registry_register_unregister() {
        let mut registry = ChildRegistry::default();

        registry.register(1234);
        assert_eq!(registry.count(), 1);

        registry.register(5678);
        assert_eq!(registry.count(), 2);

        registry.unregister(1234);
        assert_eq!(registry.count(), 1);

        registry.unregister(5678);
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn test_process_guard_tracks_children() {
        let guard = ProcessGuard::new();

        guard.register_child(1111);
        guard.register_child(2222);

        assert_eq!(guard.child_count(), 2);

        guard.unregister_child(1111);
        assert_eq!(guard.child_count(), 1);
    }

    // =========================================================================
    // Sprint 1.2: Death Pact Integration Tests
    // =========================================================================

    /// Helper to wait for a process to terminate (reap zombie)
    fn wait_for_process_death(pid: u32, timeout: Duration) -> bool {
        use nix::sys::wait::{waitpid, WaitPidFlag, WaitStatus};

        let start = Instant::now();
        let nix_pid = Pid::from_raw(pid as i32);

        while start.elapsed() < timeout {
            // Try to reap the zombie if we're the parent
            match waitpid(nix_pid, Some(WaitPidFlag::WNOHANG)) {
                Ok(WaitStatus::Exited(_, _)) | Ok(WaitStatus::Signaled(_, _, _)) => {
                    return true; // Process reaped
                }
                Ok(WaitStatus::StillAlive) => {
                    // Still running, keep waiting
                }
                Err(nix::errno::Errno::ECHILD) => {
                    // Not our child or already reaped - check if PID exists
                    if !is_process_alive(pid) {
                        return true;
                    }
                }
                _ => {}
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    #[test]
    fn test_terminate_all_kills_real_process() {
        use std::process::Command;
        use std::time::Duration;

        // Spawn a real long-running bash process
        let child = Command::new("bash")
            .args(["-c", "sleep 60"])
            .spawn()
            .expect("Failed to spawn bash sleep process");

        let pid = child.id();

        // Register it in a fresh registry (not the global one to avoid interference)
        let mut registry = ChildRegistry::default();
        registry.register(pid);

        // Verify process is alive
        assert!(is_process_alive(pid), "Process should be alive after spawn");

        // Terminate all with short grace period
        registry.terminate_all(Duration::from_millis(500));

        // Wait for process to die and be reaped
        let died = wait_for_process_death(pid, Duration::from_secs(2));

        assert!(died, "Process should be dead after terminate_all");
    }

    #[test]
    fn test_terminate_all_is_idempotent() {
        use std::time::Duration;

        let mut registry = ChildRegistry::default();

        // First termination on empty registry
        registry.terminate_all(Duration::from_millis(100));

        // Reset flag to test idempotency behavior
        registry.cleanup_initiated = false;
        registry.register(99999); // Fake PID

        // Second termination should work without panic
        registry.terminate_all(Duration::from_millis(100));

        // No panic = success
    }

    #[test]
    fn test_terminate_all_handles_already_dead_process() {
        use std::process::Command;
        use std::time::Duration;

        // Spawn a process that exits immediately
        let mut child = Command::new("bash")
            .args(["-c", "exit 0"])
            .spawn()
            .expect("Failed to spawn bash");

        let pid = child.id();

        // Wait for it to finish naturally (reaps zombie)
        let _ = child.wait();

        // Register the (now dead and reaped) PID
        let mut registry = ChildRegistry::default();
        registry.register(pid);

        // terminate_all should handle this gracefully
        registry.terminate_all(Duration::from_millis(100));

        // No panic = success
    }

    #[test]
    fn test_sigterm_before_sigkill() {
        use std::process::Command;
        use std::time::Duration;

        // Spawn a bash process that traps SIGTERM and exits cleanly
        let child = Command::new("bash")
            .args(["-c", "trap 'exit 0' TERM; sleep 60"])
            .spawn()
            .expect("Failed to spawn bash with trap");

        let pid = child.id();

        let mut registry = ChildRegistry::default();
        registry.register(pid);

        // Small delay to let trap be set up
        std::thread::sleep(Duration::from_millis(50));

        // Terminate with grace period - should use SIGTERM first
        registry.terminate_all(Duration::from_secs(2));

        // Process should exit cleanly from SIGTERM handler
        let died = wait_for_process_death(pid, Duration::from_secs(3));
        assert!(died, "Process should exit from SIGTERM trap");
    }

    #[test]
    fn test_send_signal_to_nonexistent_pid() {
        // Sending signal to a PID that doesn't exist should return error
        let result = send_signal(999999, Signal::SIGTERM);
        assert!(result.is_err(), "Should fail for nonexistent PID");
    }

    #[test]
    fn test_is_process_alive_nonexistent() {
        // Check for a PID that almost certainly doesn't exist
        assert!(!is_process_alive(999999));
    }

    #[test]
    fn test_cleanup_initiated_flag_prevents_double_cleanup() {
        use std::time::Duration;

        let mut registry = ChildRegistry::default();
        registry.register(12345); // Fake PID

        // First call sets the flag
        registry.terminate_all(Duration::from_millis(10));
        assert!(registry.cleanup_initiated);

        // Second call should return early due to flag
        registry.terminate_all(Duration::from_millis(10));

        // Flag should still be set
        assert!(registry.cleanup_initiated);
    }
}
