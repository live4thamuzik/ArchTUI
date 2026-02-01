# Destructive Operations Policy

This document defines the safety policy for operations that destroy user data.

## Definition: Destructive Operation

An operation is **destructive** if it:
- Modifies partition tables
- Formats filesystems
- Writes directly to block devices
- Encrypts or decrypts volumes
- Modifies bootloader configuration on disk

## Core Principle: Defense in Depth

Destructive operations require **multiple layers of confirmation** to execute:

```
┌─────────────────────────────────────────────────────────────────┐
│                    LAYER 1: State Machine                        │
│  InstallStage must be PartitioningDisk (destructive flag set)   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    LAYER 2: Manifest Validation                  │
│  Script manifest declares destructive=true                       │
│  Rust verifies required_confirmation env var is "yes"           │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    LAYER 3: Environment Gate                     │
│  Environment must contain CONFIRM_WIPE_DISK=yes (or similar)    │
│  Bash script refuses to run without this                        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    LAYER 4: Logged Warning                       │
│  Script logs "DESTRUCTIVE: Wiping all data on $disk..."        │
│  Before executing the dangerous command                          │
└─────────────────────────────────────────────────────────────────┘
```

## Confirmation Model

### Environment-Based Confirmation

Interactive prompts are forbidden (see `ci/lint_rules.md`). Instead, confirmation is environment-based:

**Why environment variables?**

1. **Auditability**: Environment can be logged, prompts cannot
2. **Automation**: Allows scripted installations
3. **Reproducibility**: Same environment = same behavior
4. **Testing**: Easy to test both paths

**Confirmation variables:**

| Variable | Script | Purpose |
|----------|--------|---------|
| `CONFIRM_WIPE_DISK` | wipe_disk.sh | Wipe entire disk |
| `CONFIRM_FORMAT` | format_partition.sh | Format partition |
| `CONFIRM_ENCRYPT` | (future) | LUKS encryption |

### Setting Confirmation

Rust sets confirmation based on user's explicit action in the TUI:

```rust
// Only set after user confirms in dialog
if user_confirmed_wipe {
    env::set_var("CONFIRM_WIPE_DISK", "yes");
}
```

The TUI displays a clear warning before setting this:

```
┌─────────────────────────────────────────────────────────┐
│           ⚠️  WARNING: DESTRUCTIVE OPERATION            │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  This will PERMANENTLY DESTROY all data on:            │
│                                                         │
│    /dev/sda (Samsung SSD 970 EVO 500GB)                │
│                                                         │
│  This action CANNOT be undone.                         │
│                                                         │
│  Type "CONFIRM" to proceed:  [__________]              │
│                                                         │
│         [ Cancel ]              [ Proceed ]             │
└─────────────────────────────────────────────────────────┘
```

## Script Requirements

### Manifest Declaration

Destructive scripts must declare their nature:

```json
{
  "script": "scripts/tools/wipe_disk.sh",
  "destructive": true,
  "required_confirmation": "CONFIRM_WIPE_DISK"
}
```

### Environment Contract

Scripts must validate at startup:

```bash
#!/bin/bash
set -euo pipefail

# ENVIRONMENT CONTRACT: Require explicit confirmation
if [[ "${CONFIRM_WIPE_DISK:-}" != "yes" ]]; then
    error_exit "CONFIRM_WIPE_DISK=yes is required. This script refuses to run without explicit environment confirmation."
fi
```

### Signal Handling

Destructive scripts must handle termination:

```bash
cleanup_and_exit() {
    local sig="$1"
    echo "WIPE_DISK: Received $sig, aborting..." >&2
    # Any cleanup needed
    exit 130
}
trap 'cleanup_and_exit SIGTERM' SIGTERM
trap 'cleanup_and_exit SIGINT' SIGINT
```

### Logging

Before destructive commands:

```bash
log_warn "DESTRUCTIVE: Wiping all data on $disk..."
```

## Storage-Specific Safety

### SSD Handling

SSDs require special handling to preserve lifespan:

```bash
# CORRECT: Use TRIM/discard for SSDs
blkdiscard "$disk"

# INCORRECT: Writing zeros wastes SSD write cycles
dd if=/dev/zero of="$disk" bs=1M  # Don't do this on SSDs
```

### HDD Handling

HDDs use traditional zeroing:

```bash
# For HDDs: zero-fill is appropriate
dd if=/dev/zero of="$disk" bs=1M status=progress
```

### Auto-Detection

The `wipe_disk.sh` script auto-detects device type:

```bash
is_ssd() {
    local rotational="/sys/block/${device}/queue/rotational"
    if [[ -f "$rotational" ]] && [[ "$(cat "$rotational")" == "0" ]]; then
        return 0  # SSD
    fi
    return 1  # HDD or unknown
}
```

## Wipe Methods

Three wipe methods are supported:

| Method | Operation | Use Case |
|--------|-----------|----------|
| `quick` | wipefs + GPT clear | Reinstallation, same owner |
| `secure` | Full device wipe (SSD: TRIM, HDD: zeros) | Decommissioning |
| `auto` | Detect device type, use appropriate secure method | Default safe choice |

### Never Used: /dev/urandom

We explicitly do NOT use `/dev/urandom` for wiping:

**Reasons:**
1. **Wastes entropy**: System entropy pool is finite
2. **No security benefit on SSDs**: TRIM achieves same result faster
3. **Slow**: Orders of magnitude slower than zeros
4. **Not required**: Modern analysis cannot recover zeroed data

This is enforced by `ci/lint_rules.md`:
```
❌ /dev/urandom forbidden
```

## Validation Before Execution

Rust validates before spawning any destructive script:

```rust
impl ScriptManifest {
    pub fn validate_for_execution(&self) -> Result<(), ManifestError> {
        // 1. Check required environment variables
        for req in &self.required_env {
            self.validate_env_requirement(req)?;
        }

        // 2. For destructive scripts, verify confirmation
        if self.destructive {
            if let Some(ref confirmation) = self.required_confirmation {
                let value = std::env::var(confirmation).unwrap_or_default();
                if value != "yes" {
                    return Err(ManifestError::MissingConfirmation {
                        script: self.script.clone(),
                        confirmation: confirmation.clone(),
                    });
                }
            }
        }

        Ok(())
    }
}
```

## State Machine Integration

The state machine tracks destructive stages:

```rust
impl InstallStage {
    pub const fn is_destructive(self) -> bool {
        matches!(self, Self::PartitioningDisk)
    }
}
```

Rust can add additional checks:

```rust
if context.current_stage().is_destructive() {
    // Extra validation for destructive stages
    ensure_backup_prompt_shown()?;
    log_destructive_operation_start()?;
}
```

## Logging Requirements

All destructive operations must be logged:

1. **Before execution**: What will be destroyed
2. **Device identification**: Full path and model if available
3. **Method used**: quick/secure/auto
4. **Timestamp**: When the operation started
5. **Result**: Success or failure with error

Example log output:
```
[2024-01-15T10:30:45Z] WARN  wipe_disk: DESTRUCTIVE: Wiping all data on /dev/sda
[2024-01-15T10:30:45Z] INFO  wipe_disk: Device: Samsung SSD 970 EVO 500GB (SSD)
[2024-01-15T10:30:45Z] INFO  wipe_disk: Method: secure (blkdiscard)
[2024-01-15T10:30:47Z] INFO  wipe_disk: Wipe completed successfully
```

## Recovery Considerations

Because destructive operations cannot be undone, the system:

1. **Validates config completely** before any destruction
2. **Records state to disk** at each stage transition
3. **Preserves error context** for debugging
4. **Does not auto-retry** destructive operations on failure

## Summary of Safeguards

| Safeguard | Enforced By | Bypass Possible? |
|-----------|-------------|------------------|
| State machine ordering | Rust compiler | No |
| Manifest validation | Rust runtime | No |
| Environment confirmation | Rust + Bash | No |
| Signal handling | lint_rules.md CI | No |
| Logged warnings | Script implementation | Policy only |
| SSD/HDD detection | Script implementation | User can force method |

## Policy Violations

A destructive operation policy violation would be:

1. ❌ Script that doesn't check CONFIRM_* variable
2. ❌ Script that reads confirmation from stdin
3. ❌ Script that swallows errors and continues
4. ❌ Script without signal handlers
5. ❌ Script that uses /dev/urandom
6. ❌ Rust code that spawns without in_new_process_group()

All of these are caught by CI lint rules and code review.
