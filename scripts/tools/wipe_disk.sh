#!/bin/bash
# wipe_disk.sh - Safely wipe a disk with appropriate method for device type
#
# ENVIRONMENT CONTRACT:
#   CONFIRM_WIPE_DISK=yes   Required. Script refuses to run without this.
#
# This script is NON-INTERACTIVE. All confirmation must come from environment.
#
# STORAGE SAFETY:
#   - SSDs: Uses blkdiscard (TRIM/UNMAP) - fast and preserves SSD lifespan
#   - HDDs: Uses dd with /dev/zero - appropriate for magnetic storage
#   - NEVER uses /dev/urandom (wastes entropy, slow, no security benefit on SSDs)
#
# METHODS:
#   quick  - Remove partition table and filesystem signatures only (wipefs)
#   secure - Full device wipe: blkdiscard for SSD, zeros for HDD
#   auto   - Auto-detect device type and use appropriate secure wipe
#
# References:
#   - https://wiki.archlinux.org/title/Solid_state_drive/Memory_cell_clearing
#   - https://wiki.archlinux.org/title/Securely_wipe_disk

set -euo pipefail

# --- Signal Handling for Destructive Operations ---
cleanup_and_exit() {
    local sig="$1"
    echo "WIPE_DISK: Received $sig, aborting..." >&2
    exit 130
}
trap 'cleanup_and_exit SIGTERM' SIGTERM
trap 'cleanup_and_exit SIGINT' SIGINT

# Source common utilities
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source_or_die() {
    local script_path="$1"
    local error_msg="${2:-Failed to source required script: $script_path}"
    if [[ ! -f "$script_path" ]]; then
        echo "FATAL: $error_msg (file not found)" >&2
        exit 1
    fi
    # shellcheck source=/dev/null
    if ! source "$script_path"; then
        echo "FATAL: $error_msg (source failed)" >&2
        exit 1
    fi
}
source_or_die "$SCRIPT_DIR/../utils.sh"

# --- Environment Contract Enforcement ---
if [[ "${CONFIRM_WIPE_DISK:-}" != "yes" ]]; then
    error_exit "CONFIRM_WIPE_DISK=yes is required. This script refuses to run without explicit environment confirmation."
fi

# --- Disk Type Detection ---

# Get the base device name (strip partition numbers, handle nvme)
# /dev/sda1 -> sda, /dev/nvme0n1p1 -> nvme0n1
get_base_device_name() {
    local device="$1"
    local base_name

    # Remove /dev/ prefix
    base_name="${device#/dev/}"

    # Handle NVMe devices: nvme0n1p1 -> nvme0n1
    if [[ "$base_name" =~ ^(nvme[0-9]+n[0-9]+) ]]; then
        echo "${BASH_REMATCH[1]}"
        return
    fi

    # Handle standard devices: sda1 -> sda, vda2 -> vda
    # Strip trailing numbers (partition numbers)
    echo "${base_name%%[0-9]*}"
}

# Check if a disk is rotational (HDD) or non-rotational (SSD/NVMe)
# Returns: 0 if SSD, 1 if HDD, 2 if unknown
is_ssd() {
    local device="$1"
    local base_name
    local rotational_file

    base_name=$(get_base_device_name "$device")
    rotational_file="/sys/block/${base_name}/queue/rotational"

    if [[ -f "$rotational_file" ]]; then
        local rotational
        rotational=$(cat "$rotational_file")
        if [[ "$rotational" == "0" ]]; then
            return 0  # SSD (non-rotational)
        else
            return 1  # HDD (rotational)
        fi
    fi

    # Fallback: NVMe devices are always SSDs
    if [[ "$base_name" =~ ^nvme ]]; then
        return 0
    fi

    # Unknown - treat as HDD for safety (zeros won't hurt)
    return 2
}

# Get human-readable disk type
get_disk_type() {
    local device="$1"

    if is_ssd "$device"; then
        echo "SSD"
    else
        local ret=$?
        if [[ $ret -eq 1 ]]; then
            echo "HDD"
        else
            echo "Unknown"
        fi
    fi
}

# Check if blkdiscard supports secure erase
supports_secure_erase() {
    local device="$1"

    # Check if blkdiscard is available
    if ! command -v blkdiscard >/dev/null 2>&1; then
        return 1
    fi

    # Check if device supports discard
    local base_name
    base_name=$(get_base_device_name "$device")
    local discard_file="/sys/block/${base_name}/queue/discard_max_bytes"

    if [[ -f "$discard_file" ]]; then
        local discard_max
        discard_max=$(cat "$discard_file")
        if [[ "$discard_max" -gt 0 ]]; then
            return 0
        fi
    fi

    return 1
}

# --- Wipe Functions ---

# Quick wipe: just remove signatures (fastest, for re-partitioning)
wipe_quick() {
    local disk="$1"

    log_info "Quick wipe: Removing partition table and filesystem signatures..."
    wipefs -a "$disk"

    # Zero the first and last MB to clear GPT backup
    log_info "Clearing GPT structures..."
    dd if=/dev/zero of="$disk" bs=1M count=1 status=none 2>/dev/null || true

    # Get disk size and zero last MB (GPT backup header)
    local disk_size_bytes
    disk_size_bytes=$(blockdev --getsize64 "$disk")
    local last_mb_offset=$(( (disk_size_bytes / 1048576) - 1 ))
    if [[ $last_mb_offset -gt 0 ]]; then
        dd if=/dev/zero of="$disk" bs=1M count=1 seek="$last_mb_offset" status=none 2>/dev/null || true
    fi

    # Inform kernel of partition table changes
    partprobe "$disk" 2>/dev/null || true
}

# Secure wipe for SSD: use blkdiscard (TRIM)
wipe_ssd_secure() {
    local disk="$1"

    if ! supports_secure_erase "$disk"; then
        log_warning "Device does not support TRIM/discard, falling back to zero fill..."
        wipe_hdd_secure "$disk"
        return
    fi

    log_info "SSD secure wipe: Issuing TRIM/UNMAP commands via blkdiscard..."
    log_info "This is fast and safe for SSDs (no unnecessary writes)"

    # First, quick wipe to remove signatures
    wipefs -a "$disk"

    # Issue TRIM to entire device
    if blkdiscard "$disk"; then
        log_success "blkdiscard completed successfully"
    else
        log_warning "blkdiscard failed, falling back to zero fill..."
        wipe_hdd_secure "$disk"
        return
    fi

    # Inform kernel of changes
    partprobe "$disk" 2>/dev/null || true
}

# Secure wipe for HDD: overwrite with zeros
wipe_hdd_secure() {
    local disk="$1"
    local disk_size
    disk_size=$(lsblk -b -d -n -o SIZE "$disk" | numfmt --to=iec)

    log_info "HDD secure wipe: Overwriting with zeros ($disk_size)..."
    log_warning "This will take a long time for large disks"
    log_info "Using /dev/zero (NOT /dev/urandom - that provides no additional security)"

    # First, quick wipe
    wipefs -a "$disk"

    # Overwrite with zeros
    # Note: We do NOT use /dev/urandom because:
    # 1. It's slow (limited by CPU entropy generation)
    # 2. For SSDs, it's pointless (wear leveling makes it ineffective)
    # 3. For HDDs, zeros are sufficient for non-forensic purposes
    # 4. Using urandom would waste system entropy
    dd if=/dev/zero of="$disk" bs=4M status=progress conv=fsync

    # Inform kernel of changes
    partprobe "$disk" 2>/dev/null || true
}

# Auto wipe: detect device type and use appropriate method
wipe_auto() {
    local disk="$1"
    local disk_type
    disk_type=$(get_disk_type "$disk")

    log_info "Auto-detected disk type: $disk_type"

    case "$disk_type" in
        SSD)
            wipe_ssd_secure "$disk"
            ;;
        HDD|Unknown)
            wipe_hdd_secure "$disk"
            ;;
    esac
}

# --- Main Script ---

# Default values (environment variables as fallback for manifest compatibility)
DISK="${INSTALL_DISK:-}"
METHOD="${WIPE_METHOD:-quick}"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --disk)
            DISK="$2"
            shift 2
            ;;
        --method)
            METHOD="$2"
            shift 2
            ;;
        --help)
            cat << 'EOF'
Usage: CONFIRM_WIPE_DISK=yes ./wipe_disk.sh --disk <device> [--method <METHOD>]

ENVIRONMENT CONTRACT:
  CONFIRM_WIPE_DISK=yes   Required for script to execute

METHODS:
  quick   - Remove partition table and filesystem signatures only (default)
            Fast, suitable for re-partitioning. Uses wipefs.

  secure  - Full device wipe appropriate for device type:
            SSD: Uses blkdiscard (TRIM) - fast, preserves SSD lifespan
            HDD: Overwrites with zeros - thorough for magnetic storage

  auto    - Auto-detect device type (SSD/HDD) and use appropriate secure wipe

STORAGE SAFETY NOTES:
  - This script NEVER uses /dev/urandom for disk wiping because:
    1. It's extremely slow (limited by entropy generation)
    2. It provides no security benefit over zeros for SSDs (wear leveling)
    3. It wastes system entropy needed for cryptographic operations
    4. For HDDs, zeros are sufficient for non-forensic purposes

  - For true secure erasure of SSDs, use manufacturer tools or ATA Secure Erase
  - For forensic-level HDD wiping, use specialized tools (DBAN, nwipe)

EXAMPLES:
  # Quick wipe for re-partitioning
  CONFIRM_WIPE_DISK=yes ./wipe_disk.sh --disk /dev/sda --method quick

  # Auto-detect and securely wipe
  CONFIRM_WIPE_DISK=yes ./wipe_disk.sh --disk /dev/nvme0n1 --method auto
EOF
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate required arguments
if [[ -z "$DISK" ]]; then
    error_exit "Disk is required (--disk /dev/sda)"
fi

# Validate device path format (injection prevention)
if ! validate_device_path "$DISK"; then
    error_exit "Invalid device path format: $DISK"
fi

# Check if device exists
if [[ ! -b "$DISK" ]]; then
    error_exit "Device does not exist: $DISK"
fi

# Log operation details
log_warning "DESTRUCTIVE OPERATION: Wiping $DISK"
log_warning "Environment confirmation: CONFIRM_WIPE_DISK=$CONFIRM_WIPE_DISK"

# Get disk information
DISK_SIZE=$(lsblk -b -d -n -o SIZE "$DISK" | numfmt --to=iec)
DISK_TYPE=$(get_disk_type "$DISK")
log_info "Target disk: $DISK ($DISK_SIZE, $DISK_TYPE)"

# Check if any partitions are mounted
MOUNTED_PARTS=$(lsblk -n -o MOUNTPOINT "$DISK" 2>/dev/null | grep -v "^$" | wc -l || echo "0")
if [[ "$MOUNTED_PARTS" -gt 0 ]]; then
    error_exit "Disk $DISK has mounted partitions. Unmount before wiping."
fi

# Execute wipe method
log_info "Wipe method: $METHOD"

case "$METHOD" in
    quick)
        wipe_quick "$DISK"
        ;;
    secure)
        # Secure uses auto-detection internally
        wipe_auto "$DISK"
        ;;
    auto)
        wipe_auto "$DISK"
        ;;
    *)
        error_exit "Unsupported wipe method: $METHOD (valid: quick, secure, auto)"
        ;;
esac

log_success "Disk $DISK wiped successfully using '$METHOD' method!"

# Show disk status
log_info "Disk status after wiping:"
lsblk "$DISK" 2>/dev/null || true
