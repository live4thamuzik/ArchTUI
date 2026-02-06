#!/bin/bash
# chroot_system.sh - Chroot into a mounted system
# Usage: ./chroot_system.sh --root /mnt

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
ROOT_PATH="/mnt"
MOUNT_SYSTEMS=true

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --root)
            ROOT_PATH="$2"
            shift 2
            ;;
        --no-mount)
            MOUNT_SYSTEMS=false
            shift
            ;;
        --help)
            echo "Usage: $0 [--root <path>] [--no-mount]"
            echo "  --root <path>     Root directory to chroot into (default: /mnt)"
            echo "  --no-mount        Skip mounting /proc, /sys, /dev"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Check if root path exists
if [[ ! -d "$ROOT_PATH" ]]; then
    error_exit "Root path does not exist: $ROOT_PATH"
fi

# Check if it looks like a Linux system
if [[ ! -f "$ROOT_PATH/etc/passwd" ]]; then
    error_exit "Path $ROOT_PATH does not appear to be a Linux system root"
fi

log_info "Chrooting into $ROOT_PATH..."

# Mount system directories if requested
if [[ "$MOUNT_SYSTEMS" == true ]]; then
    log_info "Mounting system directories..."
    
    # Mount /proc
    if ! mountpoint -q "$ROOT_PATH/proc" 2>/dev/null; then
        mount --bind /proc "$ROOT_PATH/proc"
        log_info "Mounted /proc"
    fi
    
    # Mount /sys
    if ! mountpoint -q "$ROOT_PATH/sys" 2>/dev/null; then
        mount --bind /sys "$ROOT_PATH/sys"
        log_info "Mounted /sys"
    fi
    
    # Mount /dev
    if ! mountpoint -q "$ROOT_PATH/dev" 2>/dev/null; then
        mount --bind /dev "$ROOT_PATH/dev"
        log_info "Mounted /dev"
    fi
    
    # Mount /dev/pts
    if ! mountpoint -q "$ROOT_PATH/dev/pts" 2>/dev/null; then
        mount --bind /dev/pts "$ROOT_PATH/dev/pts"
        log_info "Mounted /dev/pts"
    fi
fi

# Change to the root directory
cd "$ROOT_PATH"

log_info "Starting chroot session..."
log_warning "Type 'exit' to leave the chroot environment"

# Start the chroot session
arch-chroot "$ROOT_PATH" /bin/bash

log_success "Chroot session ended"
