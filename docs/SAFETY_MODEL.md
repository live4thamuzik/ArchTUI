# ArchTUI Safety Model

This document provides a unified overview of the safety guarantees in ArchTUI.

## Overview

ArchTUI is designed with **defense in depth**. Multiple independent safety mechanisms ensure that:

1. **No orphaned processes** can continue after the TUI exits
2. **No unauthorized destruction** occurs without explicit confirmation
3. **No silent failures** - all errors propagate and are visible
4. **Dry-run mode** allows safe preview of operations

## The Three Pillars

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        ARCHINSTALL SAFETY MODEL                          │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│   ┌──────────────────┐  ┌──────────────────┐  ┌──────────────────┐     │
│   │   DEATH PACT     │  │  TYPED ARGUMENTS │  │    REFUSALS      │     │
│   │                  │  │                  │  │                  │     │
│   │  All children    │  │  Type-safe args  │  │  Dry-run skips   │     │
│   │  die with parent │  │  with validation │  │  destructive ops │     │
│   │                  │  │                  │  │                  │     │
│   │  PR_SET_PDEATHSIG│  │  ScriptArgs      │  │  is_destructive()│     │
│   │  Process Groups  │  │  trait system    │  │  env gating      │     │
│   └──────────────────┘  └──────────────────┘  └──────────────────┘     │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Pillar 1: Death Pact

**Goal**: No child process survives parent termination.

**Mechanisms**:
- `PR_SET_PDEATHSIG`: Kernel sends SIGTERM to child when parent dies
- Process Groups: All children in same group for coordinated signaling
- Signal Handlers: Bash scripts forward signals to grandchildren

**Guarantee**: Even if the TUI is SIGKILL'd, all spawned processes terminate.

```rust
// Every Command uses the death pact extension
Command::new("bash")
    .arg(script)
    .in_new_process_group()  // Sets PR_SET_PDEATHSIG + process group
    .spawn()
```

**Full details**: See [process-safety.md](process-safety.md)

---

## Pillar 2: Typed Arguments

**Goal**: Scripts receive validated, type-safe arguments.

**Mechanism**: The `ScriptArgs` trait provides a contract between Rust and Bash:

```rust
pub trait ScriptArgs {
    /// Convert to CLI arguments for the script
    fn to_cli_args(&self) -> Vec<String>;

    /// Environment variables to set
    fn get_env_vars(&self) -> Vec<(String, String)>;

    /// Script filename
    fn script_name(&self) -> &'static str;

    /// Whether this operation destroys data (default: true for safety)
    fn is_destructive(&self) -> bool { true }
}
```

**Example**: Wipe disk arguments are fully typed:

```rust
pub struct WipeDiskArgs {
    pub device: PathBuf,     // Must be valid path
    pub method: WipeMethod,  // Enum: Zero, Random, Secure
    pub confirm: bool,       // Required for execution
}

impl ScriptArgs for WipeDiskArgs {
    fn to_cli_args(&self) -> Vec<String> {
        vec![
            "--device".to_string(),
            self.device.display().to_string(),
            "--method".to_string(),
            self.method.to_string(),
        ]
    }

    fn is_destructive(&self) -> bool {
        true  // Wiping disk destroys data
    }
}
```

**Benefits**:
- Compile-time verification of argument structure
- No string parsing errors at runtime
- Clear documentation of what each script needs
- Destructive classification for dry-run support

---

## Pillar 3: Refusals

**Goal**: Operations that shouldn't run, don't run.

### 3.1 Dry-Run Mode

When `--dry-run` is passed, destructive operations are skipped:

```rust
pub fn run_script_safe<T: ScriptArgs>(args: &T) -> Result<ScriptOutput> {
    if is_dry_run() && args.is_destructive() {
        info!("[DRY RUN] Would execute: {}", args.script_name());
        return Ok(ScriptOutput {
            stdout: format!("[DRY RUN] Skipped: {}\n", args.script_name()),
            dry_run: true,
            success: true,
            ..Default::default()
        });
    }
    // Actually execute...
}
```

**Destructive Classification**:

| Operation | is_destructive() | Runs in Dry-Run? |
|-----------|------------------|------------------|
| Wipe Disk | `true` | No (skipped) |
| Format Partition | `true` | No (skipped) |
| Add User | `true` | No (skipped) |
| System Info | `false` | Yes (read-only) |
| Check Disk Health | `false` | Yes (read-only) |
| Network Test | `false` | Yes (read-only) |

### 3.2 Environment Gating

Destructive operations require explicit environment confirmation:

```bash
# Script refuses without confirmation
if [[ "${CONFIRM_WIPE_DISK:-}" != "yes" ]]; then
    error_exit "CONFIRM_WIPE_DISK=yes is required"
fi
```

Rust sets these only after user confirms in TUI:

```rust
if user_confirmed_wipe {
    env::set_var("CONFIRM_WIPE_DISK", "yes");
}
```

**Full details**: See [destructive-ops-policy.md](destructive-ops-policy.md)

---

## Controller/Worker Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         RUST CONTROLLER                                  │
│                                                                          │
│  Responsibilities:                                                       │
│  ✓ All decision-making                                                  │
│  ✓ State machine transitions                                            │
│  ✓ Configuration validation                                             │
│  ✓ Process lifecycle (death pact)                                       │
│  ✓ Dry-run enforcement                                                  │
│  ✓ Type-safe argument construction                                      │
│                                                                          │
│  Rules:                                                                  │
│  • Never executes destructive commands directly                         │
│  • Always spawns children with in_new_process_group()                   │
│  • Validates before invoking any script                                  │
└────────────────────────────────┬────────────────────────────────────────┘
                                 │
                    Environment vars + CLI args
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                          BASH WORKER                                     │
│                                                                          │
│  Responsibilities:                                                       │
│  ✓ Execute system commands                                              │
│  ✓ Report exit codes                                                    │
│  ✓ Log progress to stdout/stderr                                        │
│  ✓ Validate environment contract                                        │
│                                                                          │
│  Rules:                                                                  │
│  • Never makes decisions                                                │
│  • Never prompts for input (forbidden by lint)                          │
│  • Never catches errors silently                                        │
│  • Always uses set -euo pipefail                                        │
│  • Always has signal handlers for cleanup                               │
└─────────────────────────────────────────────────────────────────────────┘
```

**Full details**: See [architecture.md](architecture.md)

---

## Verification Checklist

Use this checklist when reviewing code:

### Rust Code

- [ ] All `Command::new()` uses `.in_new_process_group()`
- [ ] All script invocations use typed `ScriptArgs` structs
- [ ] Destructive operations have `is_destructive() -> true`
- [ ] Configuration validation happens before any execution
- [ ] Errors propagate (no silent swallowing)

### Bash Scripts

- [ ] Starts with `set -euo pipefail`
- [ ] Has signal handlers (`trap ... TERM INT`)
- [ ] Validates environment contract at startup
- [ ] Does not use `read` for input (enforced by lint)
- [ ] Does not use `source` (must use `source_or_die`)
- [ ] Logs before destructive operations

### New Script Checklist

When adding a new script:

1. Create typed arguments struct implementing `ScriptArgs`
2. Set `is_destructive()` correctly
3. Add environment gating if destructive
4. Add signal handlers to bash script
5. Update tests

---

## Testing the Safety Model

### Death Pact Test

```bash
# Start TUI, begin an operation
# Kill TUI with SIGKILL (uncatchable)
kill -9 $(pgrep archinstall-tui)

# Verify all children died
pgrep -f "install.sh"  # Should return nothing
```

### Dry-Run Test

```bash
# Run with dry-run flag
./archinstall-tui --dry-run tools disk wipe -d /dev/sda -m zero

# Output shows skipped, disk unchanged:
# [DRY RUN] Skipped: wipe_disk.sh
```

### Environment Gate Test

```bash
# Try to run script directly without confirmation
INSTALL_DISK=/dev/sda ./scripts/tools/wipe_disk.sh
# Output: "CONFIRM_WIPE_DISK=yes is required"
# Exit code: non-zero
```

---

## Summary

| Threat | Protection | Layer |
|--------|------------|-------|
| Orphaned destructive process | Death Pact (PR_SET_PDEATHSIG) | Kernel |
| Accidental data destruction | Environment gating + dry-run | Application |
| Script injection | Typed arguments | Compile-time |
| Silent failures | set -euo pipefail + Result types | Runtime |
| Unauthorized execution | Manifest validation | Pre-execution |

ArchTUI is designed so that **accidental data loss is structurally impossible** when using the TUI interface correctly.

---

## Related Documentation

- [architecture.md](architecture.md) - Full system architecture
- [process-safety.md](process-safety.md) - Death pact implementation
- [destructive-ops-policy.md](destructive-ops-policy.md) - Destructive operation policy
- [roe](roe) - Rules of Engagement for development
