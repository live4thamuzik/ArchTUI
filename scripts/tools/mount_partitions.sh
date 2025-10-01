#!/bin/bash
# mount_partitions.sh - Mount/unmount partitions
# Usage: ./mount_partitions.sh --action mount --device /dev/sda1 --mountpoint /mnt

set -euo pipefail

# Source common utilities
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$SCRIPT_DIR/../utils.sh"

# Default values
ACTION=""
DEVICE=""
MOUNTPOINT=""
FILESYSTEM=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --action)
            ACTION="$2"
            shift 2
            ;;
        --device)
            DEVICE="$2"
            shift 2
            ;;
        --mountpoint)
            MOUNTPOINT="$2"
            shift 2
            ;;
        --filesystem)
            FILESYSTEM="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 --action <mount|unmount|list> [options]"
            echo "  --device <device>        Device to mount/unmount"
            echo "  --mountpoint <path>      Mount point for mounting"
            echo "  --filesystem <type>      Filesystem type (optional)"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate required arguments
if [[ -z "$ACTION" ]]; then
    error_exit "Action is required (--action mount|unmount|list)"
fi

case "$ACTION" in
    list)
        log_info "Currently mounted filesystems:"
        mount | grep -E "^/dev/" | sort
        exit 0
        ;;
    mount)
        if [[ -z "$DEVICE" ]]; then
            error_exit "Device is required for mounting (--device /dev/sda1)"
        fi
        if [[ -z "$MOUNTPOINT" ]]; then
            error_exit "Mountpoint is required for mounting (--mountpoint /mnt)"
        fi
        
        # Check if device exists
        if [[ ! -b "$DEVICE" ]]; then
            error_exit "Device does not exist: $DEVICE"
        fi
        
        # Check if device is already mounted
        if mountpoint -q "$DEVICE" 2>/dev/null; then
            log_warning "Device $DEVICE is already mounted"
            mount | grep "$DEVICE"
            exit 0
        fi
        
        # Create mountpoint if it doesn't exist
        if [[ ! -d "$MOUNTPOINT" ]]; then
            log_info "Creating mountpoint: $MOUNTPOINT"
            mkdir -p "$MOUNTPOINT"
        fi
        
        # Mount the device
        log_info "Mounting $DEVICE to $MOUNTPOINT..."
        if [[ -n "$FILESYSTEM" ]]; then
            mount -t "$FILESYSTEM" "$DEVICE" "$MOUNTPOINT"
        else
            mount "$DEVICE" "$MOUNTPOINT"
        fi
        
        log_success "Successfully mounted $DEVICE to $MOUNTPOINT"
        ;;
    unmount)
        if [[ -z "$DEVICE" ]]; then
            error_exit "Device is required for unmounting (--device /dev/sda1)"
        fi
        
        # Check if device is mounted
        if ! mountpoint -q "$DEVICE" 2>/dev/null; then
            log_warning "Device $DEVICE is not mounted"
            exit 0
        fi
        
        # Unmount the device
        log_info "Unmounting $DEVICE..."
        umount "$DEVICE"
        
        log_success "Successfully unmounted $DEVICE"
        ;;
    *)
        error_exit "Invalid action: $ACTION. Use mount, unmount, or list"
        ;;
esac
