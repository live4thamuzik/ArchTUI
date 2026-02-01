# Death Pact Test Documentation

Sprint 4: Process Death Guarantees

## Overview

This document describes the integration tests that PROVE no child process survives when the Rust parent crashes. These tests verify the Death Pact mechanism works under forced termination scenarios.

## Test Architecture

### Two-Level Test Design

To test what happens when the parent process dies unexpectedly, we use a helper binary:

```
Test Process (cargo test)
    └── death_pact_test_helper (simulates TUI)
            └── Child processes (sleep, bash scripts)
```

The test process spawns the helper, then SIGKILL's it (simulating a crash), and verifies all children die.

### Death Pact Mechanisms

The death pact uses two complementary mechanisms:

1. **PR_SET_PDEATHSIG**: Kernel mechanism where children receive SIGTERM when their parent dies
2. **Process Group Signaling**: All children in a process group can be killed with a single signal

## Test Files

### tests/death_pact_forced_crash.rs

Primary Sprint 4 tests proving children die under forced crashes.

| Test | Description | Mechanism Tested |
|------|-------------|-----------------|
| `test_forced_crash_sigkill_kills_all_children` | SIGKILL parent, verify children die | PR_SET_PDEATHSIG |
| `test_forced_crash_panic_kills_all_children` | Rust panic, verify children die | Panic → exit → PDEATHSIG |
| `test_forced_crash_nested_tree_all_die` | Kill parent, nested bash tree dies | PDEATHSIG + bash signal handlers |
| `test_forced_crash_rapid_no_orphans` | Race condition: kill during spawn | No orphans possible |
| `test_forced_crash_destructive_operation_stops` | Simulated sgdisk/cryptsetup stops on crash | Critical safety test |
| `test_stress_many_children_rapid_crash` | 10 children, rapid crash | Scalability |
| `test_verify_pdeathsig_is_set` | Verify process setup | Mechanism verification |
| `test_verify_process_group_is_new` | Verify PGID = PID | Process group setup |

### tests/integration_death_pact.rs

Existing tests for explicit termination (non-crash scenarios).

| Test | Description |
|------|-------------|
| `test_death_pact_spawn_and_kill_via_group` | Group signal kills process |
| `test_death_pact_group_signal_kills_children` | Group signal kills entire tree |
| `test_death_pact_entire_tree_killed` | Nested tree killed via group |
| `test_death_pact_registry_terminate_all` | Registry-based cleanup (Drop) |
| `test_death_pact_group_isolation` | Different groups are isolated |
| `test_death_pact_rapid_spawn_kill` | Rapid spawn/kill works |
| `test_death_pact_handles_already_dead` | Dead processes handled gracefully |

## Test Helper Binary

`src/bin/death_pact_test_helper.rs` provides test scenarios:

| Mode | Description |
|------|-------------|
| `spawn-and-wait` | Spawn N children, write PIDs, wait forever |
| `spawn-and-panic` | Spawn children, then Rust panic |
| `spawn-nested` | Spawn bash with grandchildren (proper signal handlers) |
| `spawn-destructive-sim` | Simulate destructive operation |

## Acceptance Criteria Verification

### Criterion 1: No running child processes after forced crash

Verified by all `test_forced_crash_*` tests. Each test:
1. Spawns children via helper
2. SIGKILL's the helper
3. Asserts all child PIDs are dead
4. Fails with "DEATH PACT VIOLATION" if any survive

### Criterion 2: Tests fail if any process survives

All tests include explicit survivor detection:
```rust
assert!(
    survivors.is_empty(),
    "DEATH PACT VIOLATION: {} child process(es) survived parent crash: {:?}",
    survivors.len(),
    survivors
);
```

## Why Bash Scripts Need Signal Handlers

The `test_forced_crash_nested_tree_all_die` test demonstrates why our lint rules require `trap` in bash scripts:

**Without signal handlers:**
- Helper dies → Bash receives SIGTERM (via PDEATHSIG)
- Bash exits, but grandchildren become orphans (reparented to init)
- DEATH PACT VIOLATION

**With signal handlers (lint requirement):**
- Helper dies → Bash receives SIGTERM (via PDEATHSIG)
- Bash trap runs, kills all children
- Entire tree dies cleanly

This is why `ci/lint_rules.md` requires:
```
✅ trap required in destructive scripts
```

## Running the Tests

```bash
# Run all death pact tests
cargo test --test death_pact_forced_crash -- --test-threads=1
cargo test --test integration_death_pact -- --test-threads=1

# Run specific test
cargo test --test death_pact_forced_crash test_forced_crash_sigkill
```

Note: `--test-threads=1` recommended to avoid resource contention.

## Failure Modes

If tests fail with "DEATH PACT VIOLATION":
1. Check if `in_new_process_group()` is being used for child spawning
2. Verify bash scripts have proper `trap` handlers
3. Check for processes escaping to new sessions (setsid)

## Future Work

- Container/namespace isolation tests (QEMU)
- Test under memory pressure
- Test with disk I/O in progress
