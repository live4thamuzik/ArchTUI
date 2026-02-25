#!/bin/bash
# raid_lvm_luks.sh - RAID + LVM + LUKS partitioning strategy
set -euo pipefail

# Source common utilities via source_or_die
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../disk_utils.sh
source_or_die "$SCRIPT_DIR/../disk_utils.sh"

# Execute RAID + LVM + LUKS partitioning strategy
execute_raid_lvm_luks_partitioning() {
    echo "=== RAID + LVM + LUKS Partitioning ==="
    log_info "Starting RAID + LVM + LUKS partitioning strategy"

    # Setup cleanup trap for error recovery
    setup_partitioning_trap

    # Validate that we have multiple disks
    if [[ ${#RAID_DEVICES[@]} -lt 2 ]]; then
        error_exit "RAID + LVM + LUKS requires at least 2 disks, but only ${#RAID_DEVICES[@]} provided"
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
    
    # Set up LVM on encrypted RAID array
    log_info "Setting up LVM on encrypted RAID array"

    # Create physical volume
    pvcreate /dev/mapper/cryptdata || error_exit "Failed to create physical volume on encrypted RAID."

    # Create volume group
    vgcreate archvg /dev/mapper/cryptdata || error_exit "Failed to create volume group on encrypted RAID."

    # Create swap logical volume FIRST (fixed size, before root/home)
    if [[ "$WANT_SWAP" == "yes" ]]; then
        log_info "Creating swap logical volume"
        lvcreate -L "$(get_swap_size_mib)M" -n swap archvg || error_exit "Failed to create swap logical volume."
        mkswap /dev/archvg/swap || error_exit "Failed to create swap filesystem."
        swapon /dev/archvg/swap || log_warn "Failed to activate swap"
        SWAP_UUID=$(get_device_uuid "/dev/archvg/swap")
        export SWAP_UUID
    fi

    # Create root and home logical volumes
    log_info "Creating root logical volume"
    local root_size_mib
    root_size_mib=$(get_root_size_mib)

    if [[ "$WANT_HOME_PARTITION" == "yes" ]]; then
        if [[ "$root_size_mib" == "REMAINING" ]]; then
            log_warn "Root=Remaining with separate home: falling back to ${DEFAULT_ROOT_SIZE_MIB}MiB root"
            root_size_mib="$DEFAULT_ROOT_SIZE_MIB"
        fi
        lvcreate -L "${root_size_mib}M" -n root archvg || error_exit "Failed to create root logical volume."

        log_info "Creating home logical volume"
        local home_size_mib
        home_size_mib=$(get_home_size_mib)
        if [[ "$home_size_mib" == "REMAINING" ]]; then
            lvcreate -l 100%FREE -n home archvg || error_exit "Failed to create home logical volume."
        else
            lvcreate -L "${home_size_mib}M" -n home archvg || error_exit "Failed to create home logical volume."
        fi
        format_filesystem "/dev/archvg/home" "$HOME_FILESYSTEM_TYPE"
    else
        if [[ "$root_size_mib" == "REMAINING" ]]; then
            lvcreate -l 100%FREE -n root archvg || error_exit "Failed to create root logical volume."
        else
            lvcreate -L "${root_size_mib}M" -n root archvg || error_exit "Failed to create root logical volume."
        fi
    fi

    # Format root
    format_filesystem "/dev/archvg/root" "$ROOT_FILESYSTEM_TYPE"

    # Mount filesystems
    log_info "Mounting filesystems"
    if [[ "$ROOT_FILESYSTEM_TYPE" == "btrfs" ]]; then
        local include_home="yes"
        [[ "$WANT_HOME_PARTITION" == "yes" ]] && include_home="no"
        setup_btrfs_subvolumes "/dev/archvg/root" "$include_home"
    else
        safe_mount "/dev/archvg/root" "/mnt"
    fi

    if [[ "$PARTITION_TABLE" == "gpt" ]]; then
        # UEFI: Mount ESP and XBOOTLDR
        safe_mount "/dev/md/XBOOTLDR" "/mnt/boot"
        safe_mount "${RAID_DEVICES[0]}1" "/mnt/efi"

        # Capture UUIDs for configuration
        capture_device_info "boot" "/dev/md/XBOOTLDR"
        capture_device_info "efi" "${RAID_DEVICES[0]}1"
        capture_device_info "root" "/dev/archvg/root"
        capture_device_info "luks" "/dev/md/DATA"
    else
        # BIOS: Mount boot
        safe_mount "/dev/md/BOOT" "/mnt/boot"

        # Capture UUIDs for configuration
        capture_device_info "boot" "/dev/md/BOOT"
        capture_device_info "root" "/dev/archvg/root"
        capture_device_info "luks" "/dev/md/DATA"
    fi

    # Mount home if created
    if [[ "$WANT_HOME_PARTITION" == "yes" ]]; then
        safe_mount "/dev/archvg/home" "/mnt/home"
        capture_device_info "home" "/dev/archvg/home"
    fi
    
    # Capture UUIDs for bootloader config
    ROOT_UUID=$(get_device_uuid "/dev/archvg/root")
    LUKS_UUID=$(get_device_uuid "/dev/md/DATA")
    export ROOT_UUID LUKS_UUID

    # Save RAID configuration
    log_info "Saving RAID configuration"
    mkdir -p /mnt/etc
    mdadm --detail --scan > /mnt/etc/mdadm.conf

    # Generate crypttab entry for boot-time unlocking
    log_info "Generating crypttab entry..."
    mkdir -p /mnt/etc
    generate_crypttab "/dev/md/DATA" "cryptdata"

    log_info "RAID + LVM + LUKS partitioning completed successfully"
}

# Export the function
export -f execute_raid_lvm_luks_partitioning