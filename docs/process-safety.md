# Process Safety Model

This document describes how ArchInstall TUI guarantees that no child process survives the parent's termination.

## The Problem

Traditional installers have a critical safety flaw: if the installer crashes while a destructive operation is running, the child process becomes orphaned and continues executing.

**Dangerous scenario:**
```
1. User launches installer
2. Installer spawns `sgdisk --zap-all /dev/sda`
3. Installer crashes (segfault, killed, panic)
4. sgdisk continues running (orphaned to init)
5. User thinks installation stopped, but disk is still being wiped
```

This is unacceptable for an installer that performs destructive disk operations.

## The Solution: Death Pact

ArchInstall TUI implements a "Death Pact" using two complementary mechanisms:

1. **PR_SET_PDEATHSIG**: Kernel-level automatic child termination
2. **Process Group Signaling**: Application-level coordinated shutdown

### Mechanism 1: PR_SET_PDEATHSIG

`prctl(PR_SET_PDEATHSIG, SIGTERM)` is a Linux kernel feature that delivers a signal to a child process when its parent dies.

**How we use it:**

```rust
// src/process_guard.rs
impl CommandProcessGroup for std::process::Command {
    fn in_new_process_group(&mut self) -> &mut Self {
        unsafe {
            self.pre_exec(|| {
                // Set death signal so child dies if parent dies
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        self
    }
}
```

**Behavior:**
- Set in child process before exec
- When parent process exits (any reason), kernel sends SIGTERM to child
- Cannot be overridden by child
- Works even if parent is SIGKILL'd

### Mechanism 2: Process Group Signaling

Children are spawned in their own process group (PGID = child PID), allowing us to signal the entire tree.

**How we use it:**

```rust
// In pre_exec hook
nix::unistd::setpgid(Pid::from_raw(0), Pid::from_raw(0))
```

**Behavior:**
- Child becomes leader of new process group
- All grandchildren inherit the process group
- Single signal to `-PGID` kills entire tree

### Combined Protection

```
┌─────────────────────────────────────────────────────────────────┐
│                     TUI Process (Rust)                          │
│                                                                 │
│  ┌─────────────────┐    ┌─────────────────┐                    │
│  │ ProcessGuard    │    │ ChildRegistry   │                    │
│  │ (RAII cleanup)  │────│ (tracks PIDs)   │                    │
│  └─────────────────┘    └─────────────────┘                    │
│           │                      │                              │
│           ▼                      ▼                              │
│  On Drop: terminate_all()  →  Send SIGTERM to all groups       │
└─────────────────────────────────────────────────────────────────┘
                              │
        Spawns with           │
        in_new_process_group()│
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│  Process Group (PGID = bash PID)                                │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │ Bash Script (PR_SET_PDEATHSIG = SIGTERM)                │   │
│  │                                                          │   │
│  │    ┌──────────────┐  ┌──────────────┐                   │   │
│  │    │ sgdisk       │  │ cryptsetup   │  (grandchildren)  │   │
│  │    │ (same group) │  │ (same group) │                   │   │
│  │    └──────────────┘  └──────────────┘                   │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘

Termination Paths:
─────────────────

1. Normal exit (Drop):
   ProcessGuard::drop() → terminate_all() → SIGTERM to -PGID → all die

2. Signal (SIGTERM/SIGINT):
   Signal handler → terminate_all() → SIGTERM to -PGID → all die

3. Crash (SIGKILL/segfault):
   Kernel detects parent death → PR_SET_PDEATHSIG fires → bash gets SIGTERM
   Bash signal handler → kills grandchildren → all die
```

## Signal Handling in Bash Scripts

For the death pact to work with nested process trees, bash scripts must forward signals to their children. This is enforced by `ci/lint_rules.md`.

**Required pattern:**

```bash
#!/bin/bash
set -euo pipefail

# Track children for cleanup
CHILD_PIDS=""

cleanup() {
    if [[ -n "$CHILD_PIDS" ]]; then
        kill $CHILD_PIDS 2>/dev/null || true
    fi
    exit 0
}
trap cleanup TERM INT

# Spawn and track children
some_command &
CHILD_PIDS="$CHILD_PIDS $!"

wait
```

## Termination Sequence

When termination is triggered:

### Step 1: SIGTERM to Process Groups

```rust
for &pid in &pids_to_kill {
    // Negative PID signals entire process group
    send_signal_to_group(pid, Signal::SIGTERM)
}
```

### Step 2: Grace Period

```rust
while start.elapsed() < grace_period {
    let still_alive = pids_to_kill
        .iter()
        .filter(|&&pid| is_process_alive(pid))
        .collect();

    if still_alive.is_empty() {
        return; // All dead
    }

    thread::sleep(Duration::from_millis(100));
}
```

### Step 3: SIGKILL (if needed)

```rust
for &pid in &pids_to_kill {
    if is_process_alive(pid) {
        send_signal_to_group(pid, Signal::SIGKILL)
    }
}
```

## Testing the Death Pact

We have comprehensive integration tests proving the mechanism works:

### Test: Forced SIGKILL

```rust
// tests/death_pact_forced_crash.rs

#[test]
fn test_forced_crash_sigkill_kills_all_children() {
    // 1. Spawn helper that creates children with death pact
    // 2. SIGKILL the helper (cannot be caught)
    // 3. Verify ALL children die via PR_SET_PDEATHSIG

    // ... spawns children ...

    kill(Pid::from_raw(helper_pid as i32), Signal::SIGKILL);

    // Verify children died
    for &pid in &child_pids {
        assert!(
            wait_for_death(pid, Duration::from_secs(3)),
            "DEATH PACT VIOLATION: Child survived"
        );
    }
}
```

### Test: Nested Tree

```rust
#[test]
fn test_forced_crash_nested_tree_all_die() {
    // Verifies: TUI crash → bash death → grandchildren death
    // Uses bash with proper signal handlers (lint requirement)
}
```

### Test: Destructive Operation Simulation

```rust
#[test]
fn test_forced_crash_destructive_operation_stops() {
    // Simulates sgdisk/cryptsetup - long-running destructive op
    // Crashes parent, verifies operation stops
}
```

## Why Orphaned Processes Are Impossible

Given the implementation:

| Scenario | Protection |
|----------|------------|
| TUI exits normally | ProcessGuard::drop() terminates all |
| TUI receives SIGTERM | Signal handler terminates all |
| TUI receives SIGINT | Signal handler terminates all |
| TUI SIGKILL'd | PR_SET_PDEATHSIG fires, bash traps forward |
| TUI segfaults | PR_SET_PDEATHSIG fires (parent died) |
| TUI panics | Panic → Drop → terminates all |
| OOM killer | PR_SET_PDEATHSIG fires (parent died) |

**The only way a child survives is if:**
1. It was not spawned with `in_new_process_group()` - CI lint catches this
2. Bash script lacks signal handler - CI lint catches this
3. Kernel PR_SET_PDEATHSIG is broken - kernel bug, not our fault

## Implementation Details

### Global Registry

```rust
static CHILD_REGISTRY: OnceLock<Arc<Mutex<ChildRegistry>>> = OnceLock::new();
```

**Why global?** Signal handlers cannot access local state. The registry must be globally accessible.

**Why OnceLock?** Safe lazy initialization without `static mut`.

### Idempotent Cleanup

```rust
pub fn terminate_all(&mut self, grace_period: Duration) {
    if self.cleanup_initiated {
        return; // Prevent double-cleanup
    }
    self.cleanup_initiated = true;
    // ... cleanup ...
}
```

**Why?** Multiple exit paths (Drop + signal handler) might try to cleanup. Flag prevents redundant work.

### Zombie Detection

```rust
fn is_process_alive(pid: u32) -> bool {
    // Check /proc/pid/stat for state
    if let Ok(stat) = std::fs::read_to_string(format!("/proc/{}/stat", pid)) {
        let fields: Vec<&str> = stat.split_whitespace().collect();
        if fields.len() > 2 {
            return !matches!(fields[2], "Z" | "X"); // Not zombie or dead
        }
    }
    true // Assume alive if can't read
}
```

**Why?** A zombie can receive signals but isn't running. We need to detect this.

## Guarantees

1. **No orphaned destructive processes**: Mathematically impossible given the implementation
2. **Bounded cleanup time**: SIGKILL after grace period ensures termination
3. **No double-cleanup**: Idempotent flag prevents issues
4. **Works with nested trees**: Process groups + bash signal forwarding
5. **Tested under forced crashes**: Integration tests prove the mechanism

## Limitations

1. **Requires Linux**: PR_SET_PDEATHSIG is Linux-specific
2. **Requires bash signal handlers**: Scripts must cooperate (enforced by lint)
3. **5-second grace period**: Processes have 5s to cleanup before SIGKILL
