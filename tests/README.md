# ArchTUI Test Suite

This directory contains integration and unit tests for ArchTUI.

## Test Files

### app_state_tests.rs

**Purpose:** Tests for Application State Management (P3.1)

**Tests included (31 tests):**
- `test_app_state_default_*` - AppState default initialization (9 tests)
- `test_app_mode_*` - AppMode enum behavior (5 tests)
- `test_tool_parameter_*` - ToolParameter variants (6 tests)
- `test_tool_param_*` - ToolParam fields (2 tests)
- `test_tool_dialog_state_*` - ToolDialogState behavior (2 tests)
- `test_app_state_*` - AppState mutation and cloning (7 tests)

### installer_tests.rs

**Purpose:** Tests for Installation Orchestration (P3.1)

**Tests included (14 tests):**
- `test_configuration_*` - Configuration validation and env vars (4 tests)
- `test_installer_*` - Installer creation with shared state (2 tests)
- `test_progress_*` - Progress tracking patterns (2 tests)
- `test_output_line_*` - Output accumulation pattern (1 test)
- `test_mode_transition_*` - Mode state transitions (2 tests)
- `test_error_*` - Error handling patterns (1 test)
- `test_concurrent_*` - Thread safety (1 test)
- `test_state_mutex_*` - Mutex behavior (1 test)

### script_execution_tests.rs

**Purpose:** Tests for Script Execution and Error Handling (P3.1)

**Tests included (27 tests):**
- `test_script_output_*` - ScriptOutput struct and methods (7 tests)
- `test_wipe_disk_args_*` - WipeDiskArgs ScriptArgs impl (4 tests)
- `test_format_partition_args_*` - FormatPartitionArgs ScriptArgs impl (3 tests)
- `test_mount_partition_args_*` - MountPartitionArgs ScriptArgs impl (3 tests)
- `test_filesystem_*` - Filesystem type conversions (2 tests)
- `test_wipe_method_*` - WipeMethod enum (2 tests)
- `test_cli_args_*` - CLI argument building (2 tests)
- `test_error_*` - Error path handling (2 tests)
- `test_dry_run_*` - Dry-run mode patterns (2 tests)

### property_tests.rs

**Purpose:** Property-Based Tests using proptest (P4.3)

**Tests included (17 tests):**
- `filesystem_*` - Filesystem enum round-trips and parsing (4 tests)
- `wipe_method_*` - WipeMethod enum properties (3 tests)
- `app_mode_*` - AppMode clone/equality/debug (3 tests)
- `tool_parameter_*` - ToolParameter clone preserves data (1 test)
- `config_*` - Configuration invariants (3 tests)
- `script_output_*` - ScriptOutput contracts (3 tests)

### integration_tests.rs

**Purpose:** Core integration tests for the application

**Tests included:**
- `test_binary_exists` - Verifies the compiled binary exists
- `test_binary_executable` - Verifies the binary has executable permissions
- `test_required_scripts_exist` - Validates all required bash scripts are present
- `test_plymouth_themes_exist` - Verifies Plymouth theme directories exist
- `test_binary_runs_without_crashing` - Smoke test for binary startup
- `test_config_structure` - Tests Configuration struct initialization
- `test_async_tool_execution_with_output_capture` - Validates async tool execution with stdout/stderr streaming
- `test_async_tool_execution_with_stdin_piping` - Tests secure password passing via stdin

**Death Pact Tests (Sprint 1.2):**
- `test_death_pact_registry_kills_processes` - ChildRegistry terminate_all works
- `test_process_group_setup` - Processes spawn in their own process group
- `test_process_group_kills_tree` - Group signal kills parent and children
- `test_sigterm_causes_graceful_exit` - SIGTERM causes clean exit
- `test_sigkill_after_grace_period` - Stubborn processes get SIGKILL
- `test_exit_code_propagation` - Exit codes are correctly captured
- `test_script_failure_exit_codes` - Failed scripts return non-zero
- `test_installer_pattern_exit_code_handling` - Exit code handling in App pattern
- `test_signal_termination_exit_status` - Signal-killed processes report correctly
- `test_process_groups_are_isolated` - Separate groups don't affect each other
- `test_pdeathsig_set` - PR_SET_PDEATHSIG is configured

### integration_death_pact.rs

**Purpose:** "Torture tests" proving the Death Pact mechanism works

These tests verify that child processes spawned with `in_new_process_group()` are properly killed when signaled:

- `test_death_pact_spawn_and_kill_via_group` - Basic group signal kills process
- `test_death_pact_group_signal_kills_children` - Group signal kills bash and its children
- `test_death_pact_entire_tree_killed` - Nested process trees die on group signal
- `test_death_pact_registry_terminate_all` - Registry-based termination (simulating Drop)
- `test_death_pact_group_isolation` - One group's death doesn't affect another
- `test_death_pact_rapid_spawn_kill` - Rapid spawn/kill doesn't cause issues
- `test_death_pact_handles_already_dead` - Already-dead processes don't cause errors

### death_pact_forced_crash.rs

**Purpose:** PROVE no child survives when Rust parent crashes (Sprint 4)

These are the critical acceptance tests for the Death Pact mechanism:

**Forced Crash Tests:**
- `test_forced_crash_sigkill_kills_all_children` - SIGKILL parent → children die via PR_SET_PDEATHSIG
- `test_forced_crash_panic_kills_all_children` - Rust panic → children die
- `test_forced_crash_nested_tree_all_die` - Nested tree (bash → grandchildren) all die
- `test_forced_crash_rapid_no_orphans` - Race condition test: no orphans during spawn
- `test_forced_crash_destructive_operation_stops` - Simulated sgdisk/cryptsetup stops on crash

**Verification Tests:**
- `test_verify_pdeathsig_is_set` - PR_SET_PDEATHSIG is actually configured
- `test_verify_process_group_is_new` - Child's PGID equals its PID (group leader)

**Documentation Tests:**
- `test_doc_why_signal_handlers_required` - Documents why lint rules require `trap` handlers

**Stress Tests:**
- `test_stress_many_children_rapid_crash` - 10 children, rapid crash, no survivors

## Running Tests

### Run all tests
```bash
cargo test --no-default-features
```

### Run specific test file
```bash
cargo test --no-default-features --test integration_tests
cargo test --no-default-features --test integration_death_pact
cargo test --no-default-features --test death_pact_forced_crash
```

### Run specific test
```bash
cargo test --no-default-features test_config_structure
```

### Run with output
```bash
cargo test --no-default-features -- --nocapture
```

## Test Helper Binary

The `death_pact_forced_crash.rs` tests require a helper binary that simulates the TUI spawning children. Build it with:

```bash
cargo build
```

The helper binary is at `target/debug/death_pact_test_helper`.

## Test Dependencies

- `nix` crate for signal handling
- Linux `/proc` filesystem for process inspection
- Process group support (POSIX)

## Writing New Tests

### For process lifecycle tests:
1. Use `CommandProcessGroup::in_new_process_group()` for spawning
2. Use `ChildRegistry` for tracking processes
3. Use `is_process_alive()` helper to check process state
4. Always clean up processes in test teardown

### For integration tests:
1. Use `archtui::` imports for public API
2. Use `env!("CARGO_BIN_EXE_archtui")` for binary path
3. Set reasonable timeouts for process operations

## Coverage Gaps

The following areas could benefit from additional coverage:

1. **App Event Handlers** (`src/app/mod.rs`)
   - Full key event simulation (requires TUI mocking)
   - Menu navigation integration tests

2. **Installer** (`src/installer.rs`)
   - Full installation flow (requires QEMU/mock environment)
   - Error recovery paths with real scripts

3. **Script Execution** (`src/script_runner.rs`)
   - Manifest validation integration (requires manifest wiring)
   - Real script execution in isolated environment

**Addressed by P3.1:**
- ✅ App state transitions (app_state_tests.rs)
- ✅ Configuration validation (installer_tests.rs)
- ✅ ScriptArgs trait implementations (script_execution_tests.rs)
- ✅ Progress tracking patterns (installer_tests.rs)
- ✅ Error handling patterns (script_execution_tests.rs)

See `ROADMAP.md` for the complete testing roadmap.
