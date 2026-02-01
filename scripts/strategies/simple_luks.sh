#!/bin/bash
# simple_luks.sh - Simple LUKS partitioning strategy with ESP + XBOOTLDR (UEFI) or boot partition (BIOS)
set -euo pipefail

# Source common utilities via source_or_die
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../disk_utils.sh
source_or_die "$SCRIPT_DIR/../disk_utils.sh"

# Execute simple LUKS partitioning strategy
execute_simple_luks_partitioning() {
    echo "=== PHASE 1: Simple LUKS Partitioning (ESP + XBOOTLDR) ==="
    log_info "Starting simple LUKS partitioning with ESP + XBOOTLDR for $INSTALL_DISK..."
    
    # Validate requirements
    validate_partitioning_requirements

    # Wipe disk with explicit confirmation
    wipe_disk "$INSTALL_DISK" "CONFIRMED"

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
        # BIOS with GPT: Need BIOS boot partition for GRUB
        create_bios_boot_partition "$INSTALL_DISK" "$part_num"
        current_start_mib=$((current_start_mib + BIOS_BOOT_PART_SIZE_MIB))
        part_num=$((part_num + 1))

        # Boot partition - mounted to /boot
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

    # Use sgdisk for both UEFI and BIOS (GPT works with both)
    sgdisk -n "${part_num}:0:0" \
           -t "${part_num}:${LUKS_PARTITION_TYPE}" \
           -c "${part_num}:LUKS" \
           "$INSTALL_DISK" || error_exit "Failed to create LUKS partition"

    sync_partitions "$INSTALL_DISK"

    local luks_dev
    luks_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")

    # Verify partition exists before proceeding
    if [[ ! -b "$luks_dev" ]]; then
        error_exit "LUKS partition $luks_dev not found after creation"
    fi
    
    # Set up LUKS encryption using helper function (non-interactive)
    local encrypted_dev
    encrypted_dev=$(setup_luks_encryption "$luks_dev" "cryptroot")

    # Format root filesystem
    log_info "Creating $ROOT_FILESYSTEM_TYPE filesystem on $encrypted_dev..."
    format_filesystem "$encrypted_dev" "$ROOT_FILESYSTEM_TYPE"
    capture_device_info "root" "$encrypted_dev"

    # Handle Btrfs subvolumes if needed, otherwise simple mount
    if [ "$ROOT_FILESYSTEM_TYPE" = "btrfs" ]; then
        # Use helper function for proper Btrfs subvolume setup
        # Include @home subvolume only if not using separate home partition
        local include_home="no"
        if [ "$WANT_HOME_PARTITION" != "yes" ]; then
            include_home="yes"
        fi
        setup_btrfs_subvolumes "/dev/mapper/cryptroot" "$include_home"
    else
        safe_mount "/dev/mapper/cryptroot" "/mnt"
    fi

    # Separate home partition (if requested)
    if [ "$WANT_HOME_PARTITION" = "yes" ]; then
        part_num=$((part_num + 1))
        log_info "Creating separate LUKS home partition..."

        # Use sgdisk for both UEFI and BIOS (GPT works with both)
        sgdisk -n "${part_num}:0:0" \
               -t "${part_num}:${LUKS_PARTITION_TYPE}" \
               -c "${part_num}:LUKS_HOME" \
               "$INSTALL_DISK" || error_exit "Failed to create LUKS home partition"

        sync_partitions "$INSTALL_DISK"

        local luks_home_dev
        luks_home_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")

        # Verify partition exists before proceeding
        if [[ ! -b "$luks_home_dev" ]]; then
            error_exit "LUKS home partition $luks_home_dev not found after creation"
        fi
        
        # Set up LUKS encryption for home using helper function (non-interactive)
        local encrypted_home_dev
        encrypted_home_dev=$(setup_luks_encryption "$luks_home_dev" "crypthome")

        # Capture LUKS home UUID separately
        local luks_home_uuid
        luks_home_uuid=$(blkid -s UUID -o value "$luks_home_dev" 2>/dev/null) || true

        # Format home filesystem
        format_filesystem "$encrypted_home_dev" "$HOME_FILESYSTEM_TYPE"
        capture_device_info "home" "$encrypted_home_dev"
        mkdir -p /mnt/home
        safe_mount "/dev/mapper/crypthome" "/mnt/home"
    fi
    
    # Generate crypttab entries for boot-time unlocking
    log_info "Generating crypttab entries..."
    mkdir -p /mnt/etc
    generate_crypttab "$luks_dev" "cryptroot"
    if [ "$WANT_HOME_PARTITION" = "yes" ]; then
        generate_crypttab "$luks_home_dev" "crypthome"
    fi

    log_partitioning_complete "Simple LUKS ESP + XBOOTLDR"
}
