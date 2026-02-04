# ArchTUI Architecture

This document describes the architecture of ArchTUI, a terminal-based Arch Linux installer designed with determinism, safety, and recoverability as core principles.

## 1. Design Goals

### 1.1 Determinism

Every installation produces identical results given identical inputs. The installer:

- Uses a structured configuration file as the single source of truth
- Executes scripts in a deterministic order defined by a state machine
- Validates all inputs before beginning destructive operations
- Logs every action for reproducibility

### 1.2 Safety

The installer is designed to fail safely rather than cause partial damage:

- **Fail Fast**: Validation occurs before any destructive operation
- **Death Pact**: All child processes terminate if the TUI crashes
- **Environment Gating**: Destructive operations require explicit environment confirmation
- **No Interactive Prompts**: Scripts cannot request user input mid-execution

### 1.3 Recoverability

When failures occur, the system state is predictable:

- State transitions are logged to disk
- Failed stages are recorded with context
- Partial operations can be identified via logs
- No silent failures or swallowed errors

## 2. Rust/Bash Separation

The architecture enforces a strict separation between **control plane** (Rust) and **execution plane** (Bash).

### 2.1 Control Plane (Rust)

Rust owns all decision-making:

| Responsibility | Implementation |
|----------------|----------------|
| State management | `InstallStage` enum with validated transitions |
| Configuration | `InstallerConfig` parsed and validated at startup |
| Sequencing | State machine determines script execution order |
| Policy enforcement | Destructive operations gated by environment vars |
| Process lifecycle | `ProcessGuard` ensures child termination |
| Error handling | All errors bubble up with context |

### 2.2 Execution Plane (Bash)

Bash scripts are intentionally "dumb"—they execute commands and report status:

| Allowed | Forbidden |
|---------|-----------|
| Execute system commands | Make decisions |
| Report exit codes | Prompt for input |
| Log progress | Change execution order |
| Validate preconditions | Catch and hide errors |

**Why Bash is intentionally limited:**

1. **Predictability**: A script that cannot make decisions always behaves the same way
2. **Testability**: Scripts with no branching logic are easier to verify
3. **Safety**: Scripts cannot override Rust's safety decisions
4. **Auditability**: Reviewers can trace all control flow in Rust

### 2.3 Communication Protocol

Rust and Bash communicate through:

```
┌──────────────────────────────────────────────────────────────────┐
│                         RUST (Control)                           │
├──────────────────────────────────────────────────────────────────┤
│  1. Validates configuration                                      │
│  2. Sets environment variables (INSTALL_DISK, CONFIRM_*, etc.)   │
│  3. Spawns bash script in process group                         │
│  4. Waits for exit code                                          │
│  5. Advances or fails state based on result                      │
└──────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌──────────────────────────────────────────────────────────────────┐
│                         BASH (Execution)                         │
├──────────────────────────────────────────────────────────────────┤
│  1. Validates environment contract (refuses without CONFIRM_*)   │
│  2. Executes commands                                            │
│  3. Logs to stdout/stderr                                        │
│  4. Returns exit code (0=success, non-zero=failure)             │
└──────────────────────────────────────────────────────────────────┘
```

## 3. Install State Machine

Installation proceeds through a linear sequence of stages. The state machine is defined in `src/install_state.rs`.

### 3.1 All Stages

```
NotStarted (0)
    │
    ▼
ValidatingConfig (1)      ← Verify user configuration is valid
    │
    ▼
PreparingSystem (2)       ← Sync clock, update mirrors
    │
    ▼
InstallingDependencies (3)← Install required packages on live system
    │
    ▼
PartitioningDisk (4)      ← [DESTRUCTIVE] Partition and format disk
    │
    ▼
InstallingBaseSystem (5)  ← pacstrap base system
    │
    ▼
GeneratingFstab (6)       ← Generate /etc/fstab
    │
    ▼
ConfiguringChroot (7)     ← Configure locale, users, bootloader, DE
    │
    ▼
Finalizing (8)            ← Cleanup and verification
    │
    ▼
Completed (9)             ← Terminal state: success

    ┌─────────────────────┐
    │ Failed (255)        │ ← Terminal state: any stage can fail
    │ (records stage)     │
    └─────────────────────┘
```

### 3.2 Valid Transitions

| From | To | Condition |
|------|----|-----------|
| Any stage | Next stage | Previous stage completed successfully |
| Any stage | Failed | Error occurred |
| NotStarted | ValidatingConfig | Installation started |

**Invalid transitions are compile-time errors.** The `advance()` method returns `Result<(), InstallTransitionError>`, and invalid transitions (e.g., skipping stages) return errors.

### 3.3 Failure Handling

When a stage fails:

1. The `InstallerContext` records which stage failed
2. The error context is preserved
3. State transitions to `Failed(at_stage)`
4. All child processes are terminated
5. User sees exactly which stage failed and why

## 4. Script Manifest System

Every bash script has a corresponding JSON manifest that declares its contract.

### 4.1 Manifest Structure

```json
{
  "script": "scripts/tools/wipe_disk.sh",
  "description": "Securely wipe a disk",
  "destructive": true,
  "required_confirmation": "CONFIRM_WIPE_DISK",
  "version": "2.0",
  "needs_stdin": false,
  "valid_exit_codes": [0],
  "required_env": [
    {
      "name": "INSTALL_DISK",
      "description": "Target disk device path",
      "pattern": "^/dev/"
    }
  ],
  "optional_env": [
    {
      "name": "WIPE_METHOD",
      "description": "quick, secure, or auto",
      "default": "quick"
    }
  ]
}
```

### 4.2 Validation Flow

```
1. Rust loads manifest for script
2. Rust validates all required_env are set
3. Rust validates patterns match (e.g., disk path starts with /dev/)
4. For destructive scripts: Rust verifies confirmation env var is "yes"
5. Only then: Rust spawns the script
6. Script ALSO validates (defense in depth)
```

### 4.3 Defense in Depth

Both Rust AND Bash validate requirements:

**Rust (pre-execution):**
```rust
manifest.validate_environment()?;
```

**Bash (at script start):**
```bash
if [[ "${CONFIRM_WIPE_DISK:-}" != "yes" ]]; then
    error_exit "CONFIRM_WIPE_DISK=yes is required"
fi
```

This ensures scripts cannot be executed incorrectly even if called directly.

## 5. Directory Structure

```
archinstall-tui/
├── src/
│   ├── main.rs              # Entry point, TUI setup
│   ├── lib.rs               # Library exports
│   ├── install_state.rs     # State machine
│   ├── process_guard.rs     # Death pact implementation
│   ├── script_manifest.rs   # Manifest validation
│   ├── config.rs            # Configuration types
│   ├── config_file.rs       # Config file parsing
│   └── ui/                  # TUI rendering
│
├── scripts/
│   ├── tools/               # Individual tool scripts
│   │   ├── wipe_disk.sh
│   │   ├── install_bootloader.sh
│   │   └── ...
│   ├── strategies/          # Partitioning strategies
│   │   ├── simple.sh
│   │   ├── lvm.sh
│   │   └── ...
│   ├── manifests/           # Script contracts (JSON)
│   └── utils.sh             # Common functions
│
├── tests/
│   ├── integration_tests.rs
│   ├── integration_death_pact.rs
│   └── death_pact_forced_crash.rs
│
├── docs/
│   ├── architecture.md      # This file
│   ├── process-safety.md    # Process safety guarantees
│   └── destructive-ops-policy.md
│
└── ci/
    └── lint_rules.md        # CI enforcement rules
```

## 6. Package Management: ALPM vs Bash

The installer uses a hybrid approach: ALPM (Arch Linux Package Manager) bindings for
package operations, and Bash scripts for disk/system operations.

### 6.1 Why ALPM for Packages?

Direct ALPM bindings (via `alpm-rs`) provide:

| Benefit | Explanation |
|---------|-------------|
| **Type Safety** | Package names, versions, and dependencies are typed |
| **Progress Callbacks** | Native `log_cb` provides real-time install progress |
| **Error Handling** | ALPM errors map to Rust Result types |
| **No Parsing** | No parsing pacman stdout for progress percentage |

```rust
// Example: Type-safe package installation
let pkg = db.pkg("base")?;  // Returns Result, not string
alpm.trans_add_pkg(pkg)?;   // Compile-time checked
```

### 6.2 Why Bash for Disk Operations?

Disk operations use Bash because:

| Reason | Explanation |
|--------|-------------|
| **CLI-only tools** | `cryptsetup`, `sgdisk`, `mkfs.*` have no stable Rust bindings |
| **Auditability** | Security reviewers can inspect shell scripts directly |
| **Environment gating** | `CONFIRM_*` variables are shell-native patterns |
| **Existing ecosystem** | Leverage battle-tested Arch Wiki commands |

### 6.3 The Hybrid Approach

| Operation | Implementation | Reason |
|-----------|----------------|--------|
| Package install | ALPM (Rust) | Type-safe, progress callbacks, no stdout parsing |
| Package queries | ALPM (Rust) | Structured data, not string parsing |
| Disk partitioning | Bash script | sgdisk CLI only, auditable |
| Disk formatting | Bash script | mkfs.* CLI only, auditable |
| LUKS encryption | Bash script | cryptsetup CLI only, security-sensitive |
| Bootloader install | Bash script | grub-install CLI, arch-chroot needed |

### 6.4 Security Boundary

```
┌─────────────────────────────────────────────────────────────┐
│                    RUST (Type-Safe)                         │
│  ┌─────────────────┐  ┌─────────────────────────────────┐  │
│  │  ALPM Bindings  │  │  Process Guard + Death Pact     │  │
│  │  - pkg install  │  │  - Spawns bash in process group │  │
│  │  - pkg query    │  │  - Sets CONFIRM_* env vars      │  │
│  │  - progress cb  │  │  - Kills on parent death        │  │
│  └─────────────────┘  └─────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────┐
│                    BASH (Execution Only)                    │
│  - Refuses without CONFIRM_* variables                      │
│  - Executes CLI tools (sgdisk, cryptsetup, mkfs)           │
│  - Reports exit codes to Rust                               │
│  - No decision making, no prompts                           │
└─────────────────────────────────────────────────────────────┘
```

## 7. Why This Is Safer Than Traditional Installers

### 7.1 vs. Shell-Script Installers

| Traditional | ArchTUI |
|-------------|-----------------|
| Scripts make decisions | Rust makes all decisions |
| Interactive prompts | Environment-based confirmation |
| Errors may be swallowed | Fail fast on any error |
| Orphaned processes possible | Death pact prevents orphans |
| Implicit dependencies | Explicit manifest contracts |

### 7.2 vs. Python-Based Installers

| Python Installer | ArchTUI |
|------------------|-----------------|
| Runtime type errors | Compile-time type safety |
| GC pauses during I/O | Predictable performance |
| Exception handling varies | Explicit Result types |
| Process management complex | Built-in death pact |

### 7.3 Concrete Safety Guarantees

1. **No orphaned processes**: PR_SET_PDEATHSIG + process groups ensure all children die with parent
2. **No silent failures**: `set -euo pipefail` in all scripts, errors propagate to Rust
3. **No skipped validation**: State machine enforces stage ordering
4. **No unauthorized destruction**: Environment confirmation required before disk operations
5. **No implicit state**: All state owned by `InstallerContext`, not global variables

## 8. Testing Strategy

### 8.1 Unit Tests

- State machine transitions (`src/install_state.rs`)
- Manifest validation (`src/script_manifest.rs`)
- Configuration parsing (`src/config_file.rs`)

### 8.2 Integration Tests

- Process death pact (`tests/death_pact_forced_crash.rs`)
- Script execution contracts
- Full installation flow (QEMU)

### 8.3 CI Enforcement

The CI system enforces invariants via `ci/lint_rules.md`:

- No `source` in bash (must use `source_or_die`)
- No `read` in bash (no interactive prompts)
- No `unwrap()` without comment in Rust
- No `Command::new` without `.in_new_process_group()`

## 9. Contributing

When contributing:

1. **Read LLM_CHARACTER.md** - Understand the operating constraints
2. **Follow lint_rules.md** - CI will reject violations
3. **Update manifests** - New scripts need JSON manifests
4. **Add tests** - Especially for safety-critical changes
5. **Document invariants** - What does your code guarantee?
