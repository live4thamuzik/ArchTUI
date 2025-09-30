#!/bin/bash
# wipe_disk.sh - Securely wipe a disk
# Usage: ./wipe_disk.sh --disk /dev/sda [--force]

set -euo pipefail

# Source common utilities
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$SCRIPT_DIR/../utils.sh"

# Default values
DISK=""
FORCE=false
METHOD="quick"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --disk)
            DISK="$2"
            shift 2
            ;;
        --force)
            FORCE=true
            shift
            ;;
        --method)
            METHOD="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 --disk <device> [--force] [--method <quick|secure>]"
            echo "Methods:"
            echo "  quick   - Remove partition table and filesystem signatures (default)"
            echo "  secure  - Overwrite with random data (slower but more secure)"
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

# Check if device exists
if [[ ! -b "$DISK" ]]; then
    error_exit "Device does not exist: $DISK"
fi

# Safety check - warn about destructive operation
log_warning "⚠️  WARNING: This will PERMANENTLY DESTROY all data on $DISK"
log_warning "This operation cannot be undone!"

# Get disk information
DISK_SIZE=$(lsblk -b -d -n -o SIZE "$DISK" | numfmt --to=iec)
log_info "Target disk: $DISK ($DISK_SIZE)"

# Check if any partitions are mounted
MOUNTED_PARTS=$(lsblk -n -o MOUNTPOINT "$DISK" | grep -v "^$" | wc -l)
if [[ "$MOUNTED_PARTS" -gt 0 ]]; then
    if [[ "$FORCE" != true ]]; then
        error_exit "Disk $DISK has mounted partitions. Use --force to override."
    else
        log_warning "Disk $DISK has mounted partitions, but proceeding with --force"
    fi
fi

# Final confirmation
if [[ "$FORCE" != true ]]; then
    log_warning "Are you absolutely sure you want to wipe $DISK? (yes/no)"
    read -r confirmation
    if [[ "$confirmation" != "yes" ]]; then
        log_info "Operation cancelled."
        exit 0
    fi
fi

log_info "Wiping disk $DISK using $METHOD method..."

case "$METHOD" in
    quick)
        log_info "Removing partition table and filesystem signatures..."
        wipefs -a "$DISK"
        ;;
    secure)
        log_warning "This will take a long time for large disks..."
        log_info "Overwriting disk with random data..."
        dd if=/dev/urandom of="$DISK" bs=1M status=progress
        ;;
    *)
        error_exit "Unsupported wipe method: $METHOD"
        ;;
esac

log_success "Disk $DISK wiped successfully!"

# Show disk status
log_info "Disk status after wiping:"
lsblk "$DISK"
