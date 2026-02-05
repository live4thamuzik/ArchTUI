# ArchTUI Deep Dive Audit - Complete Roadmap

**Audit Date:** 2026-02-04
**Auditor:** Claude Code Deep Dive
**Codebase:** ~26,565 LOC Rust + ~5,240 LOC Bash
**Last Updated:** 2026-02-04 (Post-Fix)

---

## Executive Summary

| Category | Health | Critical Issues | Status |
|----------|--------|-----------------|--------|
| Rust Architecture | 9/10 | 0 | **FIXED** - Mutex poisoning, script validation |
| Bash Scripts Safety | 9/10 | 0 | **FIXED** - eval() removed |
| Partition Strategies | 8/10 | 0 | **FIXED** - Missing functions added |
| Test Coverage | 5/10 | 6 | Major gaps (unchanged) |
| Documentation | 7/10 | 4 | Missing contributor guide |
| Security | 9/10 | 0 | **FIXED** - eval() injection patched |
| Bootloader Config | 9/10 | 0 | Excellent |
| mkinitcpio | 9/10 | 0 | Excellent |

**Overall Project Health: 8.5/10** - Critical blocking bugs fixed, strategies now functional.

---

## COMPLETED FIXES (2026-02-04)

### P0 Critical Blockers - ALL FIXED

| Issue | Fix Applied | File |
|-------|-------------|------|
| `generate_crypttab()` undefined | Added function | `disk_utils.sh` |
| `verify_essential_mounts()` undefined | Added function | `disk_utils.sh` |
| `sync_partitions()` undefined | Added function | `disk_utils.sh` |
| `log_partitioning_complete()` undefined | Added function | `disk_utils.sh` |
| `format_filesystem()` undefined | Added function | `disk_utils.sh` |
| `setup_luks_encryption()` no return | Now returns mapper path | `disk_utils.sh` |
| `setup_btrfs_subvolumes()` param mismatch | Fixed to mount device first | `disk_utils.sh` |
| `eval()` command injection | Replaced with array execution | `add_user.sh` |

### P1 High Priority - FIXED

| Issue | Fix Applied | File |
|-------|-------------|------|
| Cleanup trap missing | Added `setup_partitioning_trap` | All strategies |
| `cleanup_partitioning()` function | Added comprehensive cleanup | `disk_utils.sh` |
| Mutex poisoning panic | Changed to `unwrap_or_else` | `script_runner.rs` |
| Script validation missing | Added Path::exists check | `script_runner.rs` |
| ESP size in RAID (100M→512M) | Fixed partition size | `raid.sh` |
| UUID capture missing | Added ROOT_UUID export | `raid.sh`, strategies |
| mdadm.conf missing | Added save to /mnt/etc | `raid.sh` |

### Verification

```bash
# All tests pass
cargo check --no-default-features  # OK
cargo test --no-default-features   # 19 passed, 0 failed
bash -n scripts/disk_utils.sh      # OK
bash -n scripts/strategies/*.sh    # All OK
bash -n scripts/tools/add_user.sh  # OK
```

---

## CRITICAL BLOCKERS (Must Fix Before Any Installation) - RESOLVED

### BLOCKER 1: Undefined Functions in Bash Scripts

These functions are **CALLED but NEVER DEFINED**, causing immediate script failure:

| Function | Called In | Impact |
|----------|-----------|--------|
| `generate_crypttab()` | simple_luks.sh:179, lvm_luks.sh:131, raid_luks.sh:169 | **All LUKS strategies fail** |
| `verify_essential_mounts()` | manual.sh:43 | **Manual strategy fails** |
| `log_partitioning_complete()` | All 9 strategies | Cosmetic but breaks execution |
| `sync_partitions()` | simple_luks.sh:78,121 | Partition sync fails |

**Files to Fix:** `/home/l4tm/ArchTUI/scripts/disk_utils.sh`

### BLOCKER 2: setup_luks_encryption() Returns Nothing

**Location:** `scripts/disk_utils.sh:338-355`

```bash
# CURRENT (BROKEN):
setup_luks_encryption() {
    local device="$1"
    local mapper_name="$2"
    cryptsetup luksFormat "$device" --key-file "$KEY_FILE"
    cryptsetup open "$device" "$mapper_name" --key-file "$KEY_FILE"
    # MISSING: echo "/dev/mapper/$mapper_name"
}

# Called as:
encrypted_dev=$(setup_luks_encryption "$luks_dev" "cryptroot")
# $encrypted_dev is EMPTY - formatting wrong device!
```

**Impact:** All 5 LUKS strategies format empty device path.

### BLOCKER 3: setup_btrfs_subvolumes() Parameter Mismatch

**Location:** `scripts/disk_utils.sh:357-371`

Function expects mounted path but receives device path:
```bash
# simple_luks.sh:105 passes device:
setup_btrfs_subvolumes "/dev/mapper/cryptroot" "$include_home"

# Function tries:
btrfs subvolume create "/dev/mapper/cryptroot/@"  # FAILS
```

### BLOCKER 4: Security - eval() Command Injection

**Location:** `scripts/tools/add_user.sh:217,278,282`

```bash
# VULNERABLE:
if eval "$USERADD_CMD $USERNAME"; then  # Line 217
```

**Risk:** Username like `test$(whoami)` executes arbitrary commands as root.

---

## Priority 0: Critical Fixes (Blocking Installation)

### P0.1 Define Missing Functions

Add to `scripts/disk_utils.sh`:

```bash
# Generate crypttab entry for LUKS device
generate_crypttab() {
    local device="$1"
    local mapper_name="$2"
    local uuid
    uuid=$(blkid -s UUID -o value "$device")
    echo "$mapper_name UUID=$uuid none luks" >> /mnt/etc/crypttab
    log_info "Added crypttab entry: $mapper_name -> $uuid"
}

# Verify essential mounts exist
verify_essential_mounts() {
    local failed=0
    if ! mountpoint -q /mnt; then
        log_error "Root not mounted at /mnt"
        failed=1
    fi
    if ! mountpoint -q /mnt/boot; then
        log_error "Boot not mounted at /mnt/boot"
        failed=1
    fi
    [[ $failed -eq 1 ]] && error_exit "Essential mounts missing"
}

# Sync partitions after creation
sync_partitions() {
    local disk="$1"
    sync
    partprobe "$disk" 2>/dev/null || true
    sleep 1
}

# Log partitioning complete
log_partitioning_complete() {
    local strategy="$1"
    log_success "Partitioning complete: $strategy"
    log_info "Mounted partitions:"
    mount | grep /mnt
}
```

### P0.2 Fix setup_luks_encryption() Return Value

```bash
setup_luks_encryption() {
    local device="$1"
    local mapper_name="$2"

    # ... existing LUKS setup code ...

    # ADD THIS LINE:
    echo "/dev/mapper/$mapper_name"
}
```

### P0.3 Fix setup_btrfs_subvolumes() Flow

```bash
setup_btrfs_subvolumes() {
    local device="$1"  # Device to mount
    local include_home="${2:-no}"

    # Mount first
    mount "$device" /mnt

    # Create subvolumes
    btrfs subvolume create /mnt/@
    btrfs subvolume create /mnt/@var
    btrfs subvolume create /mnt/@tmp
    btrfs subvolume create /mnt/@snapshots
    [[ "$include_home" == "yes" ]] && btrfs subvolume create /mnt/@home

    # Unmount and remount with subvol
    umount /mnt
    mount -o compress=zstd,noatime,subvol=@ "$device" /mnt
    mkdir -p /mnt/{var,tmp,.snapshots,boot,efi}
    mount -o compress=zstd,noatime,subvol=@var "$device" /mnt/var
    mount -o compress=zstd,noatime,subvol=@tmp "$device" /mnt/tmp
    mount -o compress=zstd,noatime,subvol=@snapshots "$device" /mnt/.snapshots
    [[ "$include_home" == "yes" ]] && {
        mkdir -p /mnt/home
        mount -o compress=zstd,noatime,subvol=@home "$device" /mnt/home
    }
}
```

### P0.4 Fix eval() Security Vulnerability

Replace `scripts/tools/add_user.sh:217`:

```bash
# BEFORE (vulnerable):
if eval "$USERADD_CMD $USERNAME"; then

# AFTER (safe):
USERADD_ARGS=()
[[ "$SYSTEM_USER" == "true" ]] && USERADD_ARGS+=(--system)
[[ -n "$SHELL" ]] && USERADD_ARGS+=(--shell "$SHELL")
[[ -n "$HOME_DIR" ]] && USERADD_ARGS+=(--home "$HOME_DIR")
[[ "$CREATE_HOME" == "true" ]] && USERADD_ARGS+=(--create-home)
USERADD_ARGS+=("$USERNAME")

if useradd "${USERADD_ARGS[@]}"; then
```

---

## Priority 1: High Priority Fixes

### P1.1 Add UUID Capture to RAID Strategies

**Files:** `raid.sh`, `raid_luks.sh`, `raid_lvm.sh`, `raid_lvm_luks.sh`

Add before completion:
```bash
# Capture UUIDs for bootloader
ROOT_UUID=$(get_device_uuid "$root_device")
export ROOT_UUID
[[ -n "${luks_dev:-}" ]] && {
    LUKS_UUID=$(get_device_uuid "$luks_dev")
    export LUKS_UUID
}
```

### P1.2 Fix Rust Mutex Poisoning

**File:** `src/process_guard.rs:146`

```rust
// BEFORE:
registry.lock().expect("ChildRegistry mutex poisoned")

// AFTER:
registry.lock().unwrap_or_else(|poisoned| {
    log::warn!("ChildRegistry mutex was poisoned, recovering");
    poisoned.into_inner()
})
```

### P1.3 Add Error Cleanup Trap to Strategies

Add to all strategy scripts after `set -euo pipefail`:

```bash
cleanup_on_error() {
    local exit_code=$?
    log_error "Strategy failed with exit code $exit_code"

    # Unmount in reverse order
    for mount in /mnt/home /mnt/boot /mnt/efi /mnt; do
        umount -R "$mount" 2>/dev/null || true
    done

    # Close LUKS mappings
    for mapper in /dev/mapper/crypt*; do
        [[ -e "$mapper" ]] && cryptsetup close "$mapper" 2>/dev/null || true
    done

    # Deactivate LVM
    vgchange -an 2>/dev/null || true

    exit $exit_code
}
trap cleanup_on_error ERR
```

### P1.4 Validate Script Existence Before Spawn

**File:** `src/script_runner.rs:138`

```rust
// ADD before spawn:
let script_path = PathBuf::from(format!("scripts/tools/{}", script_name));
if !script_path.exists() {
    return Err(ArchTuiError::Script(
        format!("Script not found: {}", script_name)
    ));
}
```

### P1.5 Migrate installer.rs to ArchTuiError

**File:** `src/installer.rs`

Replace `anyhow::Result` with `Result<T, ArchTuiError>` for consistency.

---

## Priority 2: Medium Priority Improvements

### P2.1 Add Network Input Validation

**File:** `src/scripts/network.rs`

```rust
fn validate_ip(ip: &str) -> bool {
    ip.parse::<std::net::IpAddr>().is_ok()
}

fn validate_interface(iface: &str) -> bool {
    iface.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        && !iface.is_empty()
        && iface.len() <= 15
}
```

### P2.2 Create Missing Script Manifests - **COMPLETED**

**Status:** ✅ 22 new manifests created (27 total)

All scripts in `scripts/tools/` now have manifests in `scripts/manifests/`.

### P2.3 Add Bluetooth/Avahi Services - **COMPLETED**

**Status:** ✅ Implemented

- Added `bluez`, `bluez-utils`, `avahi`, `nss-mdns` to essential packages in `install.sh`
- Added `bluetooth.service` and `avahi-daemon.service` enabling in `chroot_config.sh`

### P2.4 Add Microcode Package Installation - **VERIFIED**

**Status:** ✅ Already implemented

CPU microcode detection and installation already exists in `install.sh:557-563`:
- Intel: `intel-ucode`
- AMD: `amd-ucode`

---

## Priority 3: Testing & Documentation

### P3.1 Add Tests for Critical Paths - **COMPLETED**

**Status:** ✅ Created 3 test files with 72 new tests

**Files created:**

1. `tests/app_state_tests.rs` - 31 tests for AppMode, AppState, ToolParameter, ToolDialogState
2. `tests/installer_tests.rs` - 14 tests for Configuration, Installer creation, progress tracking
3. `tests/script_execution_tests.rs` - 27 tests for ScriptOutput, ScriptArgs, WipeDiskArgs, FormatPartitionArgs

**Test count:** Total test suite now has 264+ unit tests + 72 new integration tests

### P3.2 Create CONTRIBUTING.md - **COMPLETED**

**Status:** ✅ Created `/CONTRIBUTING.md`

Includes guidelines for:
- Adding tool scripts with manifests
- Adding partition strategies
- Code style for Bash and Rust
- Testing procedures
- PR checklist

### P3.3 Document app/mod.rs State Machine - **COMPLETED**

**Status:** ✅ Added section comments

Section comments added to `src/app/mod.rs`:
- Application State Management (with ASCII state diagram)
- Main Event Loop
- Navigation Handlers
- Enter Key / Selection Handlers
- Confirmation Dialog Handlers
- Menu Selection Handlers
- Tool Parameter Definitions
- Tool Execution
- Installation

### P3.4 Create tests/README.md - **COMPLETED**

**Status:** ✅ Created `/tests/README.md`

Includes:
- Description of each test file
- List of all tests with their purpose
- Instructions for running tests
- Test helper binary documentation
- Coverage gaps identification

---

## Priority 4: Nice to Have

### P4.1 Remove Dead Code Annotations - **COMPLETED**

**Status:** ✅ Reviewed and documented 36 `#[allow(dead_code)]` instances

All annotations now have clear documentation explaining why they're kept:
- `// API:` - Library methods for external consumers
- `// WIP:` - Work in progress features (manual partitioning, wizard)
- `// Test utility:` - Methods used in tests

### P4.2 Add Cargo Doc Generation - **COMPLETED**

**Status:** ✅ Added documentation generation

- Added `[package.metadata.docs.rs]` to Cargo.toml
- Created `scripts/generate_docs.sh` helper script
- Updated CONTRIBUTING.md with documentation commands

### P4.3 Add Property-Based Tests - **COMPLETED**

**Status:** ✅ Created `tests/property_tests.rs` with 17 property tests

Tests cover:
- Filesystem enum round-trips and parsing
- WipeMethod enum round-trips
- AppMode clone/equality properties
- ToolParameter cloning
- Configuration invariants
- ScriptOutput contracts

### P4.4 Same-Disk Dual-Boot Support - **IMPROVED**

**Status:** ✅ Improved error messaging and documentation

- Enhanced error message in `simple.sh` with helpful options
- Added link to Arch Wiki dual-boot guide
- Recommends safe alternatives (different disk, manual partitioning)
- Full automatic same-disk dual-boot not implemented (too risky)

---

## Verification Checklist

After implementing fixes, verify:

```bash
# Syntax validation
bash -n scripts/disk_utils.sh
bash -n scripts/strategies/*.sh
bash -n scripts/tools/*.sh
bash -n scripts/chroot_config.sh
bash -n scripts/install.sh

# Rust compilation
cargo check --no-default-features
cargo test --no-default-features

# Security
grep -r "eval " scripts/  # Should only be in safe contexts
grep -r "read " scripts/ | grep -v "read -r"  # Should be empty

# Undefined functions
grep -rh "^[a-z_]*() {" scripts/ | sort | uniq > /tmp/defined.txt
# Compare with function calls
```

---

## Implementation Order

### Week 1: Critical Blockers
1. Define missing bash functions (P0.1)
2. Fix LUKS return value (P0.2)
3. Fix Btrfs subvolume flow (P0.3)
4. Fix eval() security (P0.4)

### Week 2: High Priority
5. Add UUID capture to RAID (P1.1)
6. Fix Rust mutex handling (P1.2)
7. Add cleanup traps (P1.3)
8. Validate script existence (P1.4)

### Week 3: Testing
9. Add app state tests (P3.1)
10. Create CONTRIBUTING.md (P3.2)
11. Document state machine (P3.3)

### Week 4: Polish
12. Create missing manifests (P2.2)
13. Clean up dead code (P4.1)
14. Add installer tests

---

## Files Modified Summary

| File | Changes | Priority |
|------|---------|----------|
| `scripts/disk_utils.sh` | Add 4 missing functions, fix LUKS return | P0 |
| `scripts/tools/add_user.sh` | Remove eval() | P0 |
| `scripts/strategies/*.sh` | Add UUID capture, cleanup traps | P1 |
| `src/process_guard.rs` | Fix mutex poisoning | P1 |
| `src/script_runner.rs` | Add script validation | P1 |
| `src/installer.rs` | Migrate to ArchTuiError | P1 |
| `scripts/manifests/*.json` | Create 20 new manifests | P2 |
| `tests/*.rs` | Add missing tests | P3 |
| `CONTRIBUTING.md` | Create new file | P3 |

---

## ROE Compliance Verification

All changes must follow Rules of Engagement:

- [ ] `set -euo pipefail` in all scripts
- [ ] `trap` handlers for cleanup
- [ ] No `read` from stdin (environment vars only)
- [ ] Use `source_or_die` not bare `source`
- [ ] All `Command::new` uses `.in_new_process_group()`
- [ ] Manifest validation before script execution
- [ ] Destructive ops require confirmation env var

---

**End of Roadmap**
