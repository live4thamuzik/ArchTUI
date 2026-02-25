#!/bin/bash
# raid_luks.sh - RAID + LUKS partitioning strategy
set -euo pipefail

# Source common utilities via source_or_die
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../disk_utils.sh
source_or_die "$SCRIPT_DIR/../disk_utils.sh"

# Execute RAID + LUKS partitioning strategy
execute_raid_luks_partitioning() {
    echo "=== RAID + LUKS Partitioning ==="
    log_info "Starting RAID + LUKS partitioning strategy"

    # Setup cleanup trap for error recovery
    setup_partitioning_trap

    # Validate that we have multiple disks
    if [[ ${#RAID_DEVICES[@]} -lt 2 ]]; then
        error_exit "RAID + LUKS requires at least 2 disks, but only ${#RAID_DEVICES[@]} provided"
    fi
    
    # Detect boot mode
    if [[ "$BOOT_MODE" == "UEFI" ]]; then
        log_info "UEFI boot mode detected - using GPT partition tables"
        PARTITION_TABLE="gpt"
        # shellcheck disable=SC2153  # EFI_PARTITION_TYPE is defined in disk_utils.sh
        ESP_PARTITION_TYPE="$EFI_PARTITION_TYPE"
        XBOOTLDR_PARTITION_TYPE="$XBOOTLDR_PARTITION_TYPE"
    else
        log_info "BIOS boot mode detected - using MBR partition tables"
        PARTITION_TABLE="mbr"
        ESP_PARTITION_TYPE=""
        XBOOTLDR_PARTITION_TYPE=""
    fi
    
    # Create partitions on all disks
    log_info "Creating partitions on ${#RAID_DEVICES[@]} disks"
    for disk in "${RAID_DEVICES[@]}"; do
        log_info "Partitioning disk: $disk"
        
        if [[ "$PARTITION_TABLE" == "gpt" ]]; then
            # UEFI: ESP + XBOOTLDR + RAID member
            sgdisk --zap-all "$disk" || error_exit "Failed to wipe $disk"
            sgdisk --new=1:0:+${DEFAULT_ESP_SIZE_MIB}MiB --typecode=1:"$ESP_PARTITION_TYPE" --change-name=1:ESP "$disk" || error_exit "Failed to create ESP on $disk"
            sgdisk --new=2:0:+${BOOT_PART_SIZE_MIB}MiB --typecode=2:"$XBOOTLDR_PARTITION_TYPE" --change-name=2:XBOOTLDR "$disk" || error_exit "Failed to create XBOOTLDR on $disk"
            sgdisk --new=3:0:0 --typecode=3:"$LUKS_PARTITION_TYPE" --change-name=3:RAID_MEMBER "$disk" || error_exit "Failed to create RAID member on $disk"
        else
            # BIOS: BIOS boot (EF02) + boot + RAID member
            sgdisk --zap-all "$disk" || error_exit "Failed to wipe $disk"
            sgdisk --new=1:0:+${BIOS_BOOT_PART_SIZE_MIB}MiB --typecode=1:"$BIOS_BOOT_PARTITION_TYPE" --change-name=1:BIOSBOOT "$disk" || error_exit "Failed to create BIOS boot partition on $disk"
            sgdisk --new=2:0:+${BOOT_PART_SIZE_MIB}MiB --typecode=2:8300 --change-name=2:BOOT "$disk" || error_exit "Failed to create boot partition on $disk"
            sgdisk --new=3:0:0 --typecode=3:"$LUKS_PARTITION_TYPE" --change-name=3:RAID_MEMBER "$disk" || error_exit "Failed to create RAID member partition on $disk"
        fi
        
        sgdisk --print "$disk"
    done
    
    # Wait for partitions to be available
    sleep 2
    for disk in "${RAID_DEVICES[@]}"; do
        partprobe "$disk" || true
    done
    
    # Create RAID arrays
    local raid_level="${RAID_LEVEL:-raid1}"
    log_info "Creating RAID arrays (level: $raid_level)"

    if [[ "$PARTITION_TABLE" == "gpt" ]]; then
        # UEFI: Create RAID arrays for XBOOTLDR and data
        XBOOTLDR_PARTS=()
        DATA_PARTS=()

        for disk in "${RAID_DEVICES[@]}"; do
            XBOOTLDR_PARTS+=("$(get_partition_path "$disk" 2)")
            DATA_PARTS+=("$(get_partition_path "$disk" 3)")
        done

        # Create XBOOTLDR RAID1 array (always RAID1 for boot)
        log_info "Creating XBOOTLDR RAID1 array"
        mdadm --create --run --verbose --level=1 --raid-devices=${#RAID_DEVICES[@]} /dev/md/XBOOTLDR "${XBOOTLDR_PARTS[@]}" || error_exit "Failed to create XBOOTLDR RAID array"

        # Create data RAID array
        log_info "Creating data RAID array"
        mdadm --create --run --verbose --level="$raid_level" --raid-devices=${#RAID_DEVICES[@]} /dev/md/DATA "${DATA_PARTS[@]}" || error_exit "Failed to create DATA RAID array"

        # Wait for RAID arrays to be ready
        mdadm --wait /dev/md/DATA || error_exit "DATA RAID array not ready"

        # Format XBOOTLDR
        format_filesystem "/dev/md/XBOOTLDR" "ext4"

    else
        # BIOS: Create RAID arrays for boot and data
        BOOT_PARTS=()
        DATA_PARTS=()

        for disk in "${RAID_DEVICES[@]}"; do
            BOOT_PARTS+=("$(get_partition_path "$disk" 2)")
            DATA_PARTS+=("$(get_partition_path "$disk" 3)")
        done

        # Create boot RAID1 array (always RAID1 for boot)
        log_info "Creating boot RAID1 array"
        mdadm --create --run --verbose --level=1 --raid-devices=${#RAID_DEVICES[@]} /dev/md/BOOT "${BOOT_PARTS[@]}" || error_exit "Failed to create BOOT RAID array"

        # Create data RAID array
        log_info "Creating data RAID array"
        mdadm --create --run --verbose --level="$raid_level" --raid-devices=${#RAID_DEVICES[@]} /dev/md/DATA "${DATA_PARTS[@]}" || error_exit "Failed to create DATA RAID array"

        # Wait for RAID arrays to be ready
        mdadm --wait /dev/md/DATA || error_exit "DATA RAID array not ready"

        # Format boot
        format_filesystem "/dev/md/BOOT" "ext4"
    fi

    # Set up LUKS encryption on data RAID array using helper function (non-interactive)
    local encrypted_dev
    encrypted_dev=$(setup_luks_encryption "/dev/md/DATA" "cryptroot")
    
    # Format encrypted array
    log_info "Formatting encrypted RAID array"
    format_filesystem "/dev/mapper/cryptroot" "$ROOT_FILESYSTEM_TYPE"

    # Mount filesystems
    log_info "Mounting filesystems"
    if [[ "$ROOT_FILESYSTEM_TYPE" == "btrfs" ]]; then
        local include_home="yes"
        [[ "$WANT_HOME_PARTITION" == "yes" ]] && include_home="no"
        setup_btrfs_subvolumes "/dev/mapper/cryptroot" "$include_home"
    else
        safe_mount "/dev/mapper/cryptroot" "/mnt"
    fi

    if [[ "$PARTITION_TABLE" == "gpt" ]]; then
        # UEFI: Mount ESP and XBOOTLDR
        safe_mount "/dev/md/XBOOTLDR" "/mnt/boot"
        local efi_part
        efi_part=$(get_partition_path "${RAID_DEVICES[0]}" 1)
        format_filesystem "$efi_part" "vfat"
        safe_mount "$efi_part" "/mnt/efi"

        # Capture UUIDs for configuration
        capture_device_info "boot" "/dev/md/XBOOTLDR"
        capture_device_info "efi" "$efi_part"
        capture_device_info "root" "/dev/mapper/cryptroot"
        capture_device_info "luks" "/dev/md/DATA"
    else
        # BIOS: Mount boot
        safe_mount "/dev/md/BOOT" "/mnt/boot"

        # Capture UUIDs for configuration
        capture_device_info "boot" "/dev/md/BOOT"
        capture_device_info "root" "/dev/mapper/cryptroot"
        capture_device_info "luks" "/dev/md/DATA"
    fi
    
    # Create swap file if requested (non-LVM RAID+LUKS uses swapfile since array is a single device)
    if [[ "$WANT_SWAP" == "yes" ]]; then
        create_swapfile "$(get_swap_size_mib)"
    fi

    # Capture UUIDs for bootloader config
    ROOT_UUID=$(get_device_uuid "/dev/mapper/cryptroot")
    LUKS_UUID=$(get_device_uuid "/dev/md/DATA")
    export ROOT_UUID LUKS_UUID

    # Save RAID configuration
    log_info "Saving RAID configuration"
    mkdir -p /mnt/etc
    mdadm --detail --scan > /mnt/etc/mdadm.conf

    # Generate crypttab entry for boot-time unlocking
    log_info "Generating crypttab entry..."
    mkdir -p /mnt/etc
    generate_crypttab "/dev/md/DATA" "cryptroot"

    log_info "RAID + LUKS partitioning completed successfully"
}

# Export the function
export -f execute_raid_luks_partitioning