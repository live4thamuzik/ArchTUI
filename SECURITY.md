# Security Features & Improvements

This document outlines the security features and improvements implemented in this Arch Linux installer.

## Security Enhancements

### Password Protection

**Issue:** Passwords visible in process list (`ps aux`) and `/proc/*/environ`, or written to disk in config files.

**Implementation:**
- Passwords are stored in a `SecretFile` on tmpfs with 0600 permissions and RAII wipe (overwritten on drop)
- Passwords are passed to `arch-chroot` as inline environment variables on the command line — never written to the installed system's disk
- `printf %q` is used for all user-supplied strings in shell contexts to prevent shell injection from passwords containing `$`, backticks, or other special characters
- User and root passwords use `chpasswd` via environment variable (not interactive `passwd`, which hangs under `Stdio::null`)

### Destructive Operation Confirmation

**Issue:** No safety checks before wiping disks.

**Fix:** Require explicit "CONFIRMED" parameter.
```bash
wipe_disk "$INSTALL_DISK" "CONFIRMED"  # Required
```

### Device Path Validation

**Issue:** No protection against symlink/path traversal attacks.

**Fix:** Path canonicalization + whitelist validation.
```bash
canonical_disk=$(readlink -f "$disk")
case "$canonical_disk" in
    /dev/sd[a-z]|/dev/nvme*|/dev/vd[a-z]) ;;  # Valid
    *) error_exit "Invalid disk path" ;;
esac
```

### NVMe Partition Race Conditions

**Issue:** Hardcoded `sleep` after partition operations caused race conditions on fast NVMe drives.

**Fix:** Replaced all `sleep 1`/`sleep 2` after partition operations with `udevadm settle --timeout=10` (fallback `sleep 2`) across `disk_utils.sh` and all RAID strategy files. Partition functions (`sync_partitions`, `create_esp`, `create_boot_partition`, `create_bios_boot_partition`, etc.) and `get_device_uuid` (with retry) all use proper device readiness detection.

### NVMe Partition Path Handling

**Issue:** RAID strategies used `${disk}1` partition syntax, which is incorrect for NVMe devices (`/dev/nvme0n1` needs `/dev/nvme0n1p1`).

**Fix:** All partition references use `get_partition_path()` helper which detects NVMe naming conventions.

### mkfs Force Flags

**Issue:** `mkfs` commands could prompt interactively when reformatting, hanging under `Stdio::null`.

**Fix:** Added force flags to all format calls: `-F` (ext4), `-f` (xfs/btrfs), `--force` (ntfs). Applied to `format_partition.sh` tool and `create_boot_partition` in `disk_utils.sh`.

### Sudoers Cleanup

**Issue:** AUR helper installation temporarily adds NOPASSWD to sudoers. If `set -e` caused early exit, NOPASSWD persisted.

**Fix:** `trap RETURN` ensures sudoers cleanup in `chroot_config.sh` — NOPASSWD is removed even on unexpected function exit.

### Shell Injection Prevention

- `printf %q` for all user-supplied values in shell contexts (passwords, working directories)
- `grep -F` for literal device paths in `mount_partitions.sh` (prevents regex injection)
- Hostname validation: RFC 1123 (1-63 chars, lowercase + hyphens + underscores)
- Username validation: 3-32 chars, lowercase + digits + underscore
- RAID disk validation: comma-separated install_disk must have 2+ entries for RAID strategies

### LUKS Mapper Name Alignment

**Issue:** Inconsistent LUKS mapper names across strategies caused bootloader to reference wrong device.

**Fix:** Aligned to `cryptroot` for non-LVM encrypted strategies, `cryptlvm` for LVM-on-LUKS strategies, matching what `chroot_config.sh` expects for bootloader `root=` parameter.

### Input Sanitization

- Usernames: `^[a-z_][a-z0-9_-]{0,31}$`
- Hostnames: RFC 1123 compliant
- Reserved names blocked
- Special characters rejected
- Empty git URL validation
- N/A sentinel sanitization in `to_env_vars()` (gated fields stripped before export)

---

## Additional Features

### Dry-Run Mode
Preview destructive operations without executing them.

**Usage:**
```bash
export DRY_RUN=true
./archtui install
```

**What's Previewed:**
- Disk wiping operations
- Partition table creation
- Filesystem formatting
- All destructive changes

### SMART Health Checks
Automatic disk health validation before installation.

- Checks SMART status on all disks
- Validates RAID member disk health
- Blocks installation on failing disks (unless overridden)

### Device Readiness Detection
Replaced hardcoded `sleep` timers with proper device polling using `udevadm settle`.

### Comprehensive Audit Logging
Master log file, per-script verbose traces, and configuration dump. Enabled via `--verbose` flag or `ARCHTUI_LOG_LEVEL` environment variable.

---

## Security Best Practices

### For Developers
1. Never pass passwords via environment variables to disk — use SecretFile or inline env vars
2. Always validate user input with strict regex
3. Use `readlink -f` for path canonicalization
4. Require explicit confirmation for destructive operations
5. Check device health before RAID creation
6. Use `printf %q` for user-supplied strings in shell contexts
7. Add `// SAFETY:` comments on all `.unwrap()` calls

### For Users
1. Use dry-run mode to preview changes
2. Verify disk paths before installation
3. Check SMART health status manually if concerned
4. Never override safety checks without understanding risks
5. Test in VM before using on production hardware

---

## Vulnerability Disclosure

If you discover a security vulnerability, please email the maintainer directly rather than creating a public issue.

**Do NOT report security issues on GitHub Issues.**

---

## Security Audit History

| Period | Type | Scope | Key Findings |
|--------|------|-------|--------------|
| DD#1-5 | Code audit | Process safety, input validation | SecretFile leak fix, shell-injection sanitization, PID casts, process::exit bypass, Death Pact implementation |
| DD#6-10 | Code audit | Disk strategies, config pipeline | sed delimiter hardening, lsof guard, LUKS password wiring, encryption handling, RAID fixes |
| DD#11-15 | Code audit | Bootloader, LVM, RAID | LUKS mapper alignment, heredoc→printf %q (shell injection), SWAP_UUID capture, GPU driver expansion |
| DD#16-20 | Code audit | Config cascading, KISS defaults | Encryption password gating, Btrfs snapshot cascading, mdadm.conf path fixes, format_filesystem error checking, safe_mount helper |
| DD#21-25 | Code audit | RAID wiring, error handling | RAID_DEVICES array init (critical), grub-mkconfig failure handling, NVMe partition paths, LUKS mapper name alignment, swapoff cleanup, dotfiles timeout |
| DD#26-30 | Code audit | UI, config pipeline, process safety | FloatingOutput scroll fix, Complete-mode rendering, get_device_uuid error handling, dialog underflow protection, printf %q for WORKDIR, sudoers trap cleanup |
| DD#31-34 | Code audit | Mount order, readonly crash, logging | Btrfs mount order fix, readonly variable crash, LOG_COLORS unbound fix, enable_services param order, comprehensive logging overhaul |
| Gemini Review | External review | Password exposure, NVMe races | Password removed from disk (inline env vars), udevadm settle (all partition ops), mkfs force flags |

---

## Remaining Recommendations

### Low Priority (Not Implemented)
- Encryption benchmark warnings (performance impact notification)
- Rollback mechanism for failed installations

These are enhancements that don't affect security directly but would improve robustness.
