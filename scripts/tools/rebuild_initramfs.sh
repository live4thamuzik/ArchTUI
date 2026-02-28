#!/bin/bash
# rebuild_initramfs.sh - Rebuild the initramfs on an installed Arch system
# Usage: ./rebuild_initramfs.sh [--root /mnt]

set -euo pipefail

# --- Signal Handling ---
cleanup_and_exit() {
    local sig="$1"
    echo "$(basename "$0"): Received $sig, aborting..." >&2
    [[ "$sig" == "SIGTERM" ]] && exit 143 || exit 130
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
ROOT="/mnt"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --root)
            ROOT="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 [--root /mnt]"
            echo ""
            echo "Rebuild the initramfs (mkinitcpio -P) on an installed Arch system."
            echo ""
            echo "Options:"
            echo "  --root PATH   Root of the installed system (default: /mnt)"
            echo "  --help        Show this help message"
            echo ""
            echo "This tool is useful for recovering from a failed mkinitcpio run"
            echo "during installation, or after modifying /etc/mkinitcpio.conf."
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate root exists
if [[ ! -d "$ROOT" ]]; then
    log_error "Root directory does not exist: $ROOT"
    exit 1
fi

# Validate that the target looks like an Arch install
if [[ ! -f "$ROOT/etc/mkinitcpio.conf" ]]; then
    log_error "No mkinitcpio.conf found at $ROOT/etc/mkinitcpio.conf"
    log_error "Is $ROOT an Arch Linux installation?"
    exit 1
fi

log_info "Rebuilding initramfs for system at $ROOT"
log_info "Using mkinitcpio.conf from $ROOT/etc/mkinitcpio.conf"

# Ensure required bind mounts exist for chroot
for mp in proc sys dev dev/pts; do
    if ! mountpoint -q "$ROOT/$mp" 2>/dev/null; then
        log_info "Bind-mounting /$mp to $ROOT/$mp"
        mkdir -p "$ROOT/$mp"
        log_cmd "mount --bind /$mp $ROOT/$mp"
        if ! mount --bind "/$mp" "$ROOT/$mp"; then
            log_error "Failed to bind-mount /$mp"
            exit 1
        fi
    fi
done

# Run mkinitcpio -P inside the chroot
log_cmd "arch-chroot $ROOT mkinitcpio -P"
if arch-chroot "$ROOT" mkinitcpio -P; then
    log_success "Initramfs rebuilt successfully"
else
    log_error "mkinitcpio -P failed inside chroot"
    log_error "Check /etc/mkinitcpio.conf for errors"
    exit 1
fi
