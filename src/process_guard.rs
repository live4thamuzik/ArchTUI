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

        // First pass: send SIGTERM to all children
        let pids_to_kill: Vec<u32> = self.pids.iter().copied().collect();
        for &pid in &pids_to_kill {
            if let Err(e) = send_signal(pid, Signal::SIGTERM) {
                log::warn!("Failed to send SIGTERM to PID {}: {}", pid, e);
            } else {
                log::debug!("Sent SIGTERM to PID {}", pid);
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

        // Second pass: SIGKILL any remaining processes
        for &pid in &pids_to_kill {
            if is_process_alive(pid) {
                log::warn!("Process {} did not terminate, sending SIGKILL", pid);
                if let Err(e) = send_signal(pid, Signal::SIGKILL) {
                    log::error!("Failed to send SIGKILL to PID {}: {}", pid, e);
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

/// Check if a process is still alive
fn is_process_alive(pid: u32) -> bool {
    // Sending signal 0 checks if process exists without actually signaling
    signal::kill(Pid::from_raw(pid as i32), None).is_ok()
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
    pub fn register_child(&self, pid: u32) {
        if let Ok(mut registry) = self.registry.lock() {
            registry.register(pid);
        }
    }

    /// Unregister a child process (call when it exits normally)
    pub fn unregister_child(&self, pid: u32) {
        if let Ok(mut registry) = self.registry.lock() {
            registry.unregister(pid);
        }
    }

    /// Get the number of tracked children
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
/// Call this once at program start
pub fn init_signal_handlers() -> Result<(), ctrlc::Error> {
    ctrlc::set_handler(move || {
        log::info!("Received interrupt signal, cleaning up...");

        // Terminate all children
        if let Ok(mut registry) = ChildRegistry::global().lock() {
            registry.terminate_all(Duration::from_secs(3));
        }

        // Exit with appropriate code
        std::process::exit(130); // 128 + SIGINT(2)
    })
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
                Ok(())
            });
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
