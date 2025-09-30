#!/bin/bash
# simple_luks.sh - Simple LUKS partitioning strategy with ESP + XBOOTLDR (UEFI) or boot partition (BIOS)
set -euo pipefail

# Source common utilities
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$SCRIPT_DIR/../disk_utils.sh"

# Execute simple LUKS partitioning strategy
execute_simple_luks_partitioning() {
    echo "=== PHASE 1: Simple LUKS Partitioning (ESP + XBOOTLDR) ==="
    log_info "Starting simple LUKS partitioning with ESP + XBOOTLDR for $INSTALL_DISK..."
    
    # Validate requirements
    validate_partitioning_requirements
    
    # Wipe disk
    wipe_disk "$INSTALL_DISK"
    
    local current_start_mib=1
    local part_num=1
    
    # Create partition table
    create_partition_table "$INSTALL_DISK"
    
    # ESP Partition (for UEFI only) - mounted to /efi
    if [ "$BOOT_MODE" = "UEFI" ]; then
        create_esp_partition "$INSTALL_DISK" "$part_num" "100"
        current_start_mib=$((current_start_mib + 100))
        part_num=$((part_num + 1))

        # XBOOTLDR Partition - mounted to /boot
        create_xbootldr_partition "$INSTALL_DISK" "$part_num" "1024"
        current_start_mib=$((current_start_mib + 1024))
        part_num=$((part_num + 1))
    else
        # BIOS: Boot partition - mounted to /boot
        create_boot_partition "$INSTALL_DISK" "$part_num" "1024"
        current_start_mib=$((current_start_mib + 1024))
        part_num=$((part_num + 1))
    fi
    
    # Swap partition (if requested)
    if [ "$WANT_SWAP" = "yes" ]; then
        local swap_size_mib=$(get_swap_size_mib)
        create_swap_partition "$INSTALL_DISK" "$part_num" "$swap_size_mib"
        current_start_mib=$((current_start_mib + swap_size_mib))
        part_num=$((part_num + 1))
    fi
    
    # LUKS partition (for root and optionally home)
    log_info "Creating LUKS partition..."
    if [ "$BOOT_MODE" = "UEFI" ]; then
        sgdisk -n "$part_num:0:0" -t "$part_num:$LUKS_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create LUKS partition."
    else
        printf "n\np\n$part_num\n\n\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create LUKS partition."
    fi
    partprobe "$INSTALL_DISK"
    local luks_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
    
    # Set up LUKS encryption
    log_info "Setting up LUKS encryption..."
    cryptsetup luksFormat --type luks2 "$luks_dev" || error_exit "Failed to format LUKS partition."
    cryptsetup open "$luks_dev" cryptroot || error_exit "Failed to open LUKS partition."
    
    # Format root filesystem
    log_info "Creating $ROOT_FILESYSTEM_TYPE filesystem on /dev/mapper/cryptroot..."
    format_filesystem "/dev/mapper/cryptroot" "$ROOT_FILESYSTEM_TYPE"
    capture_device_info "root" "/dev/mapper/cryptroot" "UUID"
    safe_mount "/dev/mapper/cryptroot" "/mnt"
    
    # Handle Btrfs subvolumes if needed
    if [ "$ROOT_FILESYSTEM_TYPE" = "btrfs" ]; then
        log_info "Creating Btrfs subvolumes..."
        btrfs subvolume create /mnt/@
        btrfs subvolume create /mnt/@home
        btrfs subvolume create /mnt/@var
        btrfs subvolume create /mnt/@tmp
    fi
    
    # Separate home partition (if requested)
    if [ "$WANT_HOME_PARTITION" = "yes" ]; then
        part_num=$((part_num + 1))
        log_info "Creating separate LUKS home partition..."
        
        if [ "$BOOT_MODE" = "UEFI" ]; then
            sgdisk -n "$part_num:0:0" -t "$part_num:$LUKS_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create LUKS home partition."
        else
            printf "n\np\n$part_num\n\n\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create LUKS home partition."
        fi
        partprobe "$INSTALL_DISK"
        local luks_home_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        
        # Set up LUKS encryption for home
        cryptsetup luksFormat --type luks2 "$luks_home_dev" || error_exit "Failed to format LUKS home partition."
        cryptsetup open "$luks_home_dev" crypthome || error_exit "Failed to open LUKS home partition."
        
        # Format home filesystem
        format_filesystem "/dev/mapper/crypthome" "$HOME_FILESYSTEM_TYPE"
        capture_device_info "home" "/dev/mapper/crypthome" "UUID"
        mkdir -p /mnt/home
        safe_mount "/dev/mapper/crypthome" "/mnt/home"
        
        # Handle Btrfs subvolumes for home if needed
        if [ "$HOME_FILESYSTEM_TYPE" = "btrfs" ]; then
            log_info "Creating Btrfs subvolumes for home..."
            btrfs subvolume create /mnt/home/@
        fi
    fi
    
    log_partitioning_complete "Simple LUKS ESP + XBOOTLDR"
}
