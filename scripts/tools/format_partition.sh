#!/bin/bash
# format_partition.sh - Format a partition with specified filesystem
# Usage: ./format_partition.sh --device /dev/sda1 --filesystem ext4

set -euo pipefail

# --- Signal Handling ---
cleanup_and_exit() {
    local sig="$1"
    echo "$(basename "$0"): Received $sig, aborting..." >&2
    exit 130
}
trap 'cleanup_and_exit SIGTERM' SIGTERM
trap 'cleanup_and_exit SIGINT' SIGINT

# Source common utilities via source_or_die
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

# Default values
DEVICE=""
FILESYSTEM=""
LABEL=""
FORCE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --device)
            DEVICE="$2"
            shift 2
            ;;
        --filesystem)
            FILESYSTEM="$2"
            shift 2
            ;;
        --label)
            LABEL="$2"
            shift 2
            ;;
        --force)
            FORCE=true
            shift
            ;;
        --help)
            echo "Usage: $0 --device <partition> --filesystem <fs_type> [--label <label>] [--force]"
            echo "Supported filesystems: ext4, xfs, btrfs, fat32, ntfs"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate required arguments
if [[ -z "$DEVICE" ]]; then
    error_exit "Device is required (--device /dev/sda1)"
fi

if [[ -z "$FILESYSTEM" ]]; then
    error_exit "Filesystem type is required (--filesystem ext4)"
fi

# Check if device exists
if [[ ! -b "$DEVICE" ]]; then
    error_exit "Device does not exist: $DEVICE"
fi

# Safety check - warn if device is mounted
if mountpoint -q "$DEVICE" 2>/dev/null; then
    if [[ "$FORCE" != true ]]; then
        error_exit "Device $DEVICE is currently mounted. Use --force to override."
    else
        log_warning "Device $DEVICE is mounted, but proceeding with --force"
    fi
fi

# Check if device is in use
if lsof "$DEVICE" >/dev/null 2>&1; then
    if [[ "$FORCE" != true ]]; then
        error_exit "Device $DEVICE is in use. Use --force to override."
    else
        log_warning "Device $DEVICE is in use, but proceeding with --force"
    fi
fi

log_info "Formatting $DEVICE with $FILESYSTEM filesystem..."

# Format based on filesystem type
case "$FILESYSTEM" in
    ext4)
        if [[ -n "$LABEL" ]]; then
            mkfs.ext4 -L "$LABEL" "$DEVICE"
        else
            mkfs.ext4 "$DEVICE"
        fi
        ;;
    xfs)
        if [[ -n "$LABEL" ]]; then
            mkfs.xfs -L "$LABEL" "$DEVICE"
        else
            mkfs.xfs "$DEVICE"
        fi
        ;;
    btrfs)
        if [[ -n "$LABEL" ]]; then
            mkfs.btrfs -L "$LABEL" "$DEVICE"
        else
            mkfs.btrfs "$DEVICE"
        fi
        ;;
    fat32)
        if [[ -n "$LABEL" ]]; then
            mkfs.fat -F 32 -n "$LABEL" "$DEVICE"
        else
            mkfs.fat -F 32 "$DEVICE"
        fi
        ;;
    ntfs)
        if [[ -n "$LABEL" ]]; then
            mkfs.ntfs -L "$LABEL" "$DEVICE"
        else
            mkfs.ntfs "$DEVICE"
        fi
        ;;
    *)
        error_exit "Unsupported filesystem type: $FILESYSTEM"
        ;;
esac

log_success "Partition $DEVICE formatted successfully with $FILESYSTEM filesystem!"

# Show filesystem information
log_info "Filesystem information:"
blkid "$DEVICE"
