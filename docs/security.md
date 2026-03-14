# Security Features

Security features and hardening implemented in ArchTUI.

## Password Protection

**Problem:** Passwords visible in process list (`ps aux`), `/proc/*/environ`, or written to disk in config files.

**Implementation:**
- Passwords stored in `SecretFile` on tmpfs with 0600 permissions and RAII wipe (overwritten on drop)
- Passed to `arch-chroot` as inline environment variables — never written to the installed system's disk
- `printf %q` for all user-supplied strings in shell contexts to prevent injection from passwords containing `$`, backticks, or other special characters
- User and root passwords use `chpasswd` via environment variable (not interactive `passwd`, which hangs under `Stdio::null`)

## Destructive Operation Confirmation

**Problem:** No safety checks before wiping disks.

**Fix:** Require explicit `CONFIRM_*` environment variable.
```bash
# Rust sets CONFIRM_WIPE_DISK=yes only after user confirms in dialog
if [[ "${CONFIRM_WIPE_DISK:-}" != "yes" ]]; then
    error_exit "CONFIRM_WIPE_DISK=yes is required"
fi
```

See `docs/destructive-ops-policy.md` for the full confirmation model.

## Device Path Validation

**Problem:** No protection against symlink/path traversal attacks.

**Fix:** Path canonicalization + whitelist validation.
```bash
canonical_disk=$(readlink -f "$disk")
case "$canonical_disk" in
    /dev/sd[a-z]|/dev/sd[a-z][a-z]|/dev/nvme[0-9]*n[0-9]*|/dev/vd[a-z]|/dev/xvd[a-z]) ;;
    *) error_exit "Invalid or unsupported disk path: ${canonical_disk}" ;;
esac
```

## NVMe Race Conditions

**Problem:** Hardcoded `sleep` after partition operations caused race conditions on fast NVMe drives.

**Fix:** All `sleep` after partition operations replaced with `udevadm settle --timeout=10` (fallback `sleep 2`) across `disk_utils.sh` and all RAID strategy files. Partition helpers (`sync_partitions`, `create_esp`, `create_boot_partition`, etc.) and `get_device_uuid` (with retry) all use proper device readiness detection.

## NVMe Partition Path Handling

**Problem:** RAID strategies used `${disk}1` partition syntax, which is incorrect for NVMe (`/dev/nvme0n1` needs `/dev/nvme0n1p1`).

**Fix:** All partition references use `get_partition_path()` helper which detects NVMe naming conventions.

## mkfs Force Flags

**Problem:** `mkfs` commands prompt interactively when reformatting, hanging under `Stdio::null`.

**Fix:** Force flags on all format calls: `-F` (ext4), `-f` (xfs/btrfs), `--force` (ntfs).

## Sudoers Cleanup

**Problem:** AUR helper installation temporarily adds NOPASSWD to sudoers. If `set -e` causes early exit, NOPASSWD persists.

**Fix:** `trap RETURN` ensures sudoers cleanup — NOPASSWD is removed even on unexpected function exit.

## Shell Injection Prevention

- `printf %q` for all user-supplied values in shell contexts (passwords, working directories)
- `grep -F` for literal device paths (prevents regex injection)
- Hostname validation: RFC 1123 (1-63 chars, lowercase alphanumeric + hyphens)
- Username validation: 1-32 chars, lowercase + digits + underscore
- RAID disk validation: comma-separated disk list must have 2+ entries
- Git URL validation: restricted to `https://` only
- AUR package names: alphanumeric + hyphens + underscores + dots only

## LUKS Mapper Name Alignment

**Problem:** Inconsistent LUKS mapper names across strategies caused bootloader to reference wrong device.

**Fix:** `cryptroot` for non-LVM encrypted strategies, `cryptlvm` for LVM-on-LUKS strategies, matching what `chroot_config.sh` expects for the bootloader `root=` parameter.

## Secure Boot

GRUB's internal `shim_lock` verifier is disabled unconditionally (`GRUB_DISABLE_SHIM_LOCK=y`). ArchTUI uses sbctl for Secure Boot signing, not shim. Some firmware (VirtualBox, certain UEFI) reports Secure Boot active even when unmanaged, triggering the verifier and dropping to grub rescue.

When Secure Boot is enabled:
- All EFI binaries signed with `sbctl sign-all`
- Pacman hook re-signs on package updates (grub, systemd, refind, limine, fwupd)
- `sbctl verify` runs after signing to catch missed files

---

## Additional Features

### Dry-Run Mode
Preview destructive operations without executing them.
```bash
export DRY_RUN=true
./archtui install
```

### SMART Health Checks
Automatic disk health validation before installation. Checks SMART status on all disks, validates RAID member health, blocks installation on failing disks (unless overridden).

### Audit Logging
Master log file, per-script verbose traces, and configuration dump. Enabled via `--verbose` flag or `ARCHTUI_LOG_LEVEL` environment variable.

---

## Best Practices

### For Developers
1. Never pass passwords via environment variables to disk — use SecretFile or inline env vars
2. Always validate user input with strict regex
3. Use `readlink -f` for path canonicalization
4. Require explicit confirmation for destructive operations
5. Use `printf %q` for user-supplied strings in shell contexts
6. Add `// SAFETY:` comments on all `.unwrap()` calls

### For Users
1. Use dry-run mode to preview changes
2. Verify disk paths before installation
3. Check SMART health status if concerned about disk reliability
4. Never override safety checks without understanding risks
5. Test in VM before using on production hardware

---

## Vulnerability Disclosure

If you discover a security vulnerability, please email the maintainer directly rather than creating a public issue.

**Do NOT report security issues on GitHub Issues.**
