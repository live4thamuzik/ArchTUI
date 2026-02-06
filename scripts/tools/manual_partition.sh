#!/bin/bash
# manual_partition.sh - Manual disk partitioning using cfdisk
# Usage: ./manual_partition.sh --device /dev/sda

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

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --device)
            DEVICE="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 --device <device>"
            echo "Launch cfdisk for manual disk partitioning"
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
    error_exit "Device is required (--device /dev/sda)"
fi

# Check if device exists
if [[ ! -b "$DEVICE" ]]; then
    error_exit "Device does not exist: $DEVICE"
fi

# Check if device is mounted
if mountpoint -q "$DEVICE" 2>/dev/null; then
    log_warning "Device $DEVICE is currently mounted"
    log_info "Unmounting all partitions on this device..."
    
    # Find and unmount all partitions on this device
    for partition in "${DEVICE}"*; do
        if [[ -b "$partition" ]] && mountpoint -q "$partition" 2>/dev/null; then
            log_info "Unmounting $partition"
            umount "$partition" || log_warning "Failed to unmount $partition"
        fi
    done
fi

log_info "Launching cfdisk for manual partitioning of $DEVICE"
log_warning "WARNING: This will modify the partition table of $DEVICE"

# ENVIRONMENT CONTRACT: Require explicit confirmation for destructive operation
if [[ "${CONFIRM_MANUAL_PARTITION:-}" != "yes" ]]; then
    error_exit "CONFIRM_MANUAL_PARTITION=yes is required. This script refuses to run without explicit environment confirmation."
fi

# Check if cfdisk is available
if ! command -v cfdisk >/dev/null 2>&1; then
    log_info "Installing util-linux (contains cfdisk)..."
    pacman -Sy --noconfirm util-linux
fi

# Launch cfdisk
log_info "Starting cfdisk..."
if cfdisk "$DEVICE"; then
    log_success "Partitioning completed successfully"
    
    # Show the new partition table
    echo
    log_info "New partition table for $DEVICE:"
    fdisk -l "$DEVICE" | grep -E "^/dev/"
    
    log_info "Partition UUIDs:"
    blkid "${DEVICE}"* 2>/dev/null || log_info "No partitions found or not formatted"
else
    log_error "Partitioning failed or was cancelled"
    exit 1
fi
