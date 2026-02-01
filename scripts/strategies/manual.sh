#!/bin/bash
# manual.sh - Manual partitioning strategy with guided setup
set -euo pipefail

# Source common utilities via source_or_die
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../disk_utils.sh
source_or_die "$SCRIPT_DIR/../disk_utils.sh"

# Execute manual partitioning strategy
execute_manual_partitioning() {
    echo "=== PHASE 1: Manual Partitioning (Guided) ==="
    log_info "Starting manual partitioning for $INSTALL_DISK..."
    
    echo ""
    echo "Manual partitioning requires you to:"
    echo "1. Create partitions manually using fdisk, cfdisk, or gparted"
    echo "2. Format the partitions with your chosen filesystems"
    echo "3. Mount the root partition to /mnt"
    echo ""
    
    if [ "$BOOT_MODE" = "UEFI" ]; then
        echo "For UEFI systems, you need:"
        echo "- ESP partition (100MB, FAT32, mounted to /mnt/efi)"
        echo "- XBOOTLDR partition (1GB, ext4, mounted to /mnt/boot)"
        echo "- Root partition (your chosen size and filesystem, mounted to /mnt)"
        echo "- Optional: Home partition (your chosen size and filesystem, mounted to /mnt/home)"
        echo "- Optional: Swap partition (your chosen size, formatted as swap)"
    else
        echo "For BIOS systems, you need:"
        echo "- Boot partition (1GB, ext4, mounted to /mnt/boot)"
        echo "- Root partition (your chosen size and filesystem, mounted to /mnt)"
        echo "- Optional: Home partition (your chosen size and filesystem, mounted to /mnt/home)"
        echo "- Optional: Swap partition (your chosen size, formatted as swap)"
    fi
    
    echo ""
    # NOTE: In TUI mode, partitions should already be mounted by the user
    # This script verifies mounts exist rather than prompting interactively
    log_info "Verifying partition mounts..."

    # Verify essential mounts
    verify_essential_mounts
    
    # Capture device information for mounted partitions
    log_info "Capturing device information for mounted partitions..."

    # Capture root device
    local root_dev=$(findmnt -n -o SOURCE /mnt)
    if [ -n "$root_dev" ]; then
        capture_device_info "root" "$root_dev"
        log_info "Captured root device: $root_dev"
    fi

    # Capture EFI device if mounted
    if mountpoint -q /mnt/efi; then
        local efi_dev=$(findmnt -n -o SOURCE /mnt/efi)
        if [ -n "$efi_dev" ]; then
            capture_device_info "efi" "$efi_dev"
            log_info "Captured EFI device: $efi_dev"
        fi
    fi

    # Capture boot device if mounted
    if mountpoint -q /mnt/boot; then
        local boot_dev=$(findmnt -n -o SOURCE /mnt/boot)
        if [ -n "$boot_dev" ]; then
            capture_device_info "boot" "$boot_dev"
            log_info "Captured boot device: $boot_dev"
        fi
    fi

    # Capture home device if mounted
    if mountpoint -q /mnt/home; then
        local home_dev=$(findmnt -n -o SOURCE /mnt/home)
        if [ -n "$home_dev" ]; then
            capture_device_info "home" "$home_dev"
            log_info "Captured home device: $home_dev"
        fi
    fi

    # Capture swap device if active
    if [ -n "$(swapon --show --noheadings --output=NAME 2>/dev/null)" ]; then
        local swap_dev=$(swapon --show --noheadings --output=NAME | head -1)
        if [ -n "$swap_dev" ]; then
            capture_device_info "swap" "$swap_dev"
            log_info "Captured swap device: $swap_dev"
        fi
    fi
    
    log_partitioning_complete "Manual guided"
}
