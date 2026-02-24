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
            sgdisk --zap-all "$disk"
            sgdisk --new=1:0:+${DEFAULT_ESP_SIZE_MIB}MiB --typecode=1:"$ESP_PARTITION_TYPE" --change-name=1:ESP "$disk"
            sgdisk --new=2:0:+${BOOT_PART_SIZE_MIB}MiB --typecode=2:"$XBOOTLDR_PARTITION_TYPE" --change-name=2:XBOOTLDR "$disk"
            sgdisk --new=3:0:0 --typecode=3:"$LUKS_PARTITION_TYPE" --change-name=3:RAID_MEMBER "$disk"
        else
            # BIOS: MBR + RAID member
            sgdisk --zap-all "$disk"
            sgdisk --new=1:0:+${BOOT_PART_SIZE_MIB}MiB --typecode=1:8300 --change-name=1:BOOT "$disk"
            sgdisk --new=2:0:0 --typecode=2:"$LUKS_PARTITION_TYPE" --change-name=2:RAID_MEMBER "$disk"
        fi
        
        sgdisk --print "$disk"
    done
    
    # Wait for partitions to be available
    sleep 2
    partprobe
    
    # Create RAID arrays
    log_info "Creating RAID arrays"
    
    if [[ "$PARTITION_TABLE" == "gpt" ]]; then
        # UEFI: Create RAID arrays for XBOOTLDR and data
        XBOOTLDR_PARTS=()
        DATA_PARTS=()
        
        for disk in "${RAID_DEVICES[@]}"; do
            XBOOTLDR_PARTS+=("${disk}2")
            DATA_PARTS+=("${disk}3")
        done
        
        # Create XBOOTLDR RAID1 array
        log_info "Creating XBOOTLDR RAID1 array"
        mdadm --create --verbose --level=1 --raid-devices=${#RAID_DEVICES[@]} /dev/md/XBOOTLDR "${XBOOTLDR_PARTS[@]}"
        
        # Create data RAID array
        log_info "Creating data RAID array"
        if [[ ${#RAID_DEVICES[@]} -eq 2 ]]; then
            mdadm --create --verbose --level=1 --raid-devices=2 /dev/md/DATA "${DATA_PARTS[@]}"
        else
            mdadm --create --verbose --level=5 --raid-devices=${#RAID_DEVICES[@]} /dev/md/DATA "${DATA_PARTS[@]}"
        fi
        
        # Format XBOOTLDR
        format_filesystem "/dev/md/XBOOTLDR" "ext4"
        
    else
        # BIOS: Create RAID arrays for boot and data
        BOOT_PARTS=()
        DATA_PARTS=()
        
        for disk in "${RAID_DEVICES[@]}"; do
            BOOT_PARTS+=("${disk}1")
            DATA_PARTS+=("${disk}2")
        done
        
        # Create boot RAID1 array
        log_info "Creating boot RAID1 array"
        mdadm --create --verbose --level=1 --raid-devices=${#RAID_DEVICES[@]} /dev/md/BOOT "${BOOT_PARTS[@]}"
        
        # Create data RAID array
        log_info "Creating data RAID array"
        if [[ ${#RAID_DEVICES[@]} -eq 2 ]]; then
            mdadm --create --verbose --level=1 --raid-devices=2 /dev/md/DATA "${DATA_PARTS[@]}"
        else
            mdadm --create --verbose --level=5 --raid-devices=${#RAID_DEVICES[@]} /dev/md/DATA "${DATA_PARTS[@]}"
        fi
        
        # Format boot
        format_filesystem "/dev/md/BOOT" "ext4"
    fi
    
    # Set up LUKS encryption on data RAID array using helper function (non-interactive)
    local encrypted_dev
    encrypted_dev=$(setup_luks_encryption "/dev/md/DATA" "cryptdata")
    
    # Format encrypted array
    log_info "Formatting encrypted RAID array"
    format_filesystem "/dev/mapper/cryptdata" "$ROOT_FILESYSTEM_TYPE"
    
    # Create swap if requested
    if [[ "$WANT_SWAP" == "yes" ]]; then
        log_info "Creating swap on encrypted RAID array"
        if [[ "$ROOT_FILESYSTEM_TYPE" == "btrfs" ]]; then
            # Create swap subvolume for Btrfs
            mount /dev/mapper/cryptdata /mnt
            btrfs subvolume create /mnt/@swap
            umount /mnt
        fi
        # Note: For non-btrfs, swap will be created as a file or using LVM on the encrypted volume
        # Creating a separate LUKS device for swap on the same RAID array is not practical
    fi
    
    # Mount filesystems
    log_info "Mounting filesystems"
    mount /dev/mapper/cryptdata /mnt
    
    if [[ "$PARTITION_TABLE" == "gpt" ]]; then
        # UEFI: Mount ESP and XBOOTLDR
        mkdir -p /mnt/efi /mnt/boot
        mount /dev/md/XBOOTLDR /mnt/boot
        
        # Mount ESP on first disk
        mount "${RAID_DEVICES[0]}1" /mnt/efi
        
        # Capture UUIDs for configuration
        capture_device_info "boot" "/dev/md/XBOOTLDR"
        capture_device_info "root" "/dev/mapper/cryptdata"
        capture_device_info "luks" "/dev/md/DATA"
    else
        # BIOS: Mount boot
        mkdir -p /mnt/boot
        mount /dev/md/BOOT /mnt/boot
        
        # Capture UUIDs for configuration
        capture_device_info "boot" "/dev/md/BOOT"
        capture_device_info "root" "/dev/mapper/cryptdata"
        capture_device_info "luks" "/dev/md/DATA"
    fi
    
    # Capture UUIDs for bootloader config
    ROOT_UUID=$(get_device_uuid "/dev/mapper/cryptdata")
    LUKS_UUID=$(get_device_uuid "/dev/md/DATA")
    export ROOT_UUID LUKS_UUID

    # Save RAID configuration
    log_info "Saving RAID configuration"
    mkdir -p /mnt/etc/mdadm
    mdadm --detail --scan > /mnt/etc/mdadm/mdadm.conf

    # Generate crypttab entry for boot-time unlocking
    log_info "Generating crypttab entry..."
    mkdir -p /mnt/etc
    generate_crypttab "/dev/md/DATA" "cryptdata"

    log_info "RAID + LUKS partitioning completed successfully"
}

# Export the function
export -f execute_raid_luks_partitioning