#!/bin/bash
# raid_lvm_luks.sh - RAID + LVM + LUKS partitioning strategy
set -euo pipefail

# Source common utilities via source_or_die
_STRATEGY_SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../disk_utils.sh
source_or_die "$_STRATEGY_SCRIPT_DIR/../disk_utils.sh"

# Execute RAID + LVM + LUKS partitioning strategy
execute_raid_lvm_luks_partitioning() {
    echo "=== RAID + LVM + LUKS Partitioning ==="
    log_info "Starting RAID + LVM + LUKS partitioning strategy"

    # Setup cleanup trap for error recovery
    setup_partitioning_trap

    # Clean up stale RAID/LVM/LUKS state from any previous failed attempt
    cleanup_stale_raid

    # Validate that we have multiple disks
    if [[ ${#RAID_DEVICES[@]} -lt 2 ]]; then
        error_exit "RAID + LVM + LUKS requires at least 2 disks, but only ${#RAID_DEVICES[@]} provided"
    fi
    
    # Detect boot mode — use local vars to avoid writing to readonly constants from disk_utils.sh
    local esp_type xbootldr_type
    if [[ "$BOOT_MODE" == "UEFI" ]]; then
        log_info "UEFI boot mode detected - using GPT partition tables"
        PARTITION_TABLE="gpt"
        esp_type="$EFI_PARTITION_TYPE"
        xbootldr_type="$XBOOTLDR_PARTITION_TYPE"
    else
        log_info "BIOS boot mode detected - using MBR partition tables"
        PARTITION_TABLE="mbr"
        esp_type="$BIOS_BOOT_PARTITION_TYPE"
        xbootldr_type="8300"
    fi
    
    # Create partitions on all disks
    log_info "Creating partitions on ${#RAID_DEVICES[@]} disks"
    for disk in "${RAID_DEVICES[@]}"; do
        log_info "Partitioning disk: $disk"
        
        if [[ "$PARTITION_TABLE" == "gpt" ]]; then
            # UEFI: ESP + XBOOTLDR + RAID member
            log_cmd "sgdisk --zap-all $disk"
            sgdisk --zap-all "$disk" || error_exit "Failed to wipe $disk"
            log_cmd "sgdisk --new=1:0:+${DEFAULT_ESP_SIZE_MIB}MiB --typecode=1:$esp_type --change-name=1:ESP $disk"
            sgdisk --new=1:0:+${DEFAULT_ESP_SIZE_MIB}MiB --typecode=1:"$esp_type" --change-name=1:ESP "$disk" || error_exit "Failed to create ESP on $disk"
            log_cmd "sgdisk --new=2:0:+${BOOT_PART_SIZE_MIB}MiB --typecode=2:$xbootldr_type --change-name=2:XBOOTLDR $disk"
            sgdisk --new=2:0:+${BOOT_PART_SIZE_MIB}MiB --typecode=2:"$xbootldr_type" --change-name=2:XBOOTLDR "$disk" || error_exit "Failed to create XBOOTLDR on $disk"
            log_cmd "sgdisk --new=3:0:0 --typecode=3:$LUKS_PARTITION_TYPE --change-name=3:RAID_MEMBER $disk"
            sgdisk --new=3:0:0 --typecode=3:"$LUKS_PARTITION_TYPE" --change-name=3:RAID_MEMBER "$disk" || error_exit "Failed to create RAID member on $disk"
        else
            # BIOS: BIOS boot (EF02) + boot + RAID member
            log_cmd "sgdisk --zap-all $disk"
            sgdisk --zap-all "$disk" || error_exit "Failed to wipe $disk"
            log_cmd "sgdisk --new=1:0:+${BIOS_BOOT_PART_SIZE_MIB}MiB --typecode=1:$esp_type --change-name=1:BIOSBOOT $disk"
            sgdisk --new=1:0:+${BIOS_BOOT_PART_SIZE_MIB}MiB --typecode=1:"$esp_type" --change-name=1:BIOSBOOT "$disk" || error_exit "Failed to create BIOS boot partition on $disk"
            log_cmd "sgdisk --new=2:0:+${BOOT_PART_SIZE_MIB}MiB --typecode=2:$xbootldr_type --change-name=2:BOOT $disk"
            sgdisk --new=2:0:+${BOOT_PART_SIZE_MIB}MiB --typecode=2:"$xbootldr_type" --change-name=2:BOOT "$disk" || error_exit "Failed to create boot partition on $disk"
            log_cmd "sgdisk --new=3:0:0 --typecode=3:$LUKS_PARTITION_TYPE --change-name=3:RAID_MEMBER $disk"
            sgdisk --new=3:0:0 --typecode=3:"$LUKS_PARTITION_TYPE" --change-name=3:RAID_MEMBER "$disk" || error_exit "Failed to create RAID member partition on $disk"
        fi
        
        sgdisk --print "$disk"
    done
    
    # Inform kernel of partition table changes, then wait for udev
    for disk in "${RAID_DEVICES[@]}"; do
        partprobe "$disk" || true
    done
    udevadm settle --timeout=10 2>/dev/null || { log_warn "udevadm settle timed out, falling back to sleep"; sleep 2; }
    
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

        # Zero stale superblocks and stop auto-assembled arrays
        prepare_raid_partitions "${XBOOTLDR_PARTS[@]}" "${DATA_PARTS[@]}"

        # Create XBOOTLDR RAID1 array
        log_info "Creating XBOOTLDR RAID1 array"
        log_cmd "mdadm --create --run --verbose --level=1 --raid-devices=${#RAID_DEVICES[@]} /dev/md/XBOOTLDR ${XBOOTLDR_PARTS[*]}"
        mdadm --create --run --verbose --level=1 --raid-devices=${#RAID_DEVICES[@]} /dev/md/XBOOTLDR "${XBOOTLDR_PARTS[@]}" || error_exit "Failed to create XBOOTLDR RAID array"

        # Create data RAID array
        log_info "Creating data RAID array"
        log_cmd "mdadm --create --run --verbose --level=$raid_level --raid-devices=${#RAID_DEVICES[@]} /dev/md/DATA ${DATA_PARTS[*]}"
        mdadm --create --run --verbose --level="$raid_level" --raid-devices=${#RAID_DEVICES[@]} /dev/md/DATA "${DATA_PARTS[@]}" || error_exit "Failed to create DATA RAID array"

        # Resume udev event processing now that all arrays are created
        resume_udev

        # Wait for RAID arrays to be ready
        wait_for_raid_array /dev/md/XBOOTLDR
        wait_for_raid_array /dev/md/DATA

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

        # Zero stale superblocks and stop auto-assembled arrays
        prepare_raid_partitions "${BOOT_PARTS[@]}" "${DATA_PARTS[@]}"

        # Create boot RAID1 array
        log_info "Creating boot RAID1 array"
        log_cmd "mdadm --create --run --verbose --level=1 --raid-devices=${#RAID_DEVICES[@]} /dev/md/BOOT ${BOOT_PARTS[*]}"
        mdadm --create --run --verbose --level=1 --raid-devices=${#RAID_DEVICES[@]} /dev/md/BOOT "${BOOT_PARTS[@]}" || error_exit "Failed to create BOOT RAID array"

        # Create data RAID array
        log_info "Creating data RAID array"
        log_cmd "mdadm --create --run --verbose --level=$raid_level --raid-devices=${#RAID_DEVICES[@]} /dev/md/DATA ${DATA_PARTS[*]}"
        mdadm --create --run --verbose --level="$raid_level" --raid-devices=${#RAID_DEVICES[@]} /dev/md/DATA "${DATA_PARTS[@]}" || error_exit "Failed to create DATA RAID array"

        # Resume udev event processing now that all arrays are created
        resume_udev

        # Wait for RAID arrays to be ready
        wait_for_raid_array /dev/md/BOOT
        wait_for_raid_array /dev/md/DATA

        # Format boot
        format_filesystem "/dev/md/BOOT" "ext4"
    fi
    
    # Set up LUKS encryption on data RAID array using helper function (non-interactive)
    local encrypted_dev
    encrypted_dev=$(setup_luks_encryption "/dev/md/DATA" "cryptlvm")

    # FIDO2 enrollment (if configured)
    if [[ "${ENCRYPTION_KEY_TYPE:-Password}" == *"FIDO2"* ]]; then
        enroll_fido2 "/dev/md/DATA" || log_warn "FIDO2 enrollment failed — password-only fallback"
    fi

    # Set up LVM on encrypted RAID array
    log_info "Setting up LVM on encrypted RAID array"

    # Create physical volume
    log_cmd "pvcreate /dev/mapper/cryptlvm"
    pvcreate /dev/mapper/cryptlvm || error_exit "Failed to create physical volume on encrypted RAID."

    # Create volume group
    log_cmd "vgcreate archvg /dev/mapper/cryptlvm"
    vgcreate archvg /dev/mapper/cryptlvm || error_exit "Failed to create volume group on encrypted RAID."

    # Create swap logical volume FIRST (fixed size, before root/home)
    if [[ "$WANT_SWAP" == "yes" ]]; then
        log_info "Creating swap logical volume"
        log_cmd "lvcreate -L $(get_swap_size_mib)M -n swap archvg"
        lvcreate -L "$(get_swap_size_mib)M" -n swap archvg || error_exit "Failed to create swap logical volume."
        log_cmd "mkswap /dev/archvg/swap"
        mkswap /dev/archvg/swap || error_exit "Failed to create swap filesystem."
        log_cmd "swapon /dev/archvg/swap"
        swapon /dev/archvg/swap || log_warn "Failed to activate swap"
        capture_device_info "swap" "/dev/archvg/swap"
        SWAP_UUID=$(get_device_uuid "/dev/archvg/swap") || log_warn "Cannot determine SWAP_UUID"
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
        log_cmd "lvcreate -L ${root_size_mib}M -n root archvg"
        lvcreate -L "${root_size_mib}M" -n root archvg || error_exit "Failed to create root logical volume."

        log_info "Creating home logical volume"
        local home_size_mib
        home_size_mib=$(get_home_size_mib)
        if [[ "$home_size_mib" == "REMAINING" ]]; then
            log_cmd "lvcreate -l 100%FREE -n home archvg"
            lvcreate -l 100%FREE -n home archvg || error_exit "Failed to create home logical volume."
        else
            log_cmd "lvcreate -L ${home_size_mib}M -n home archvg"
            lvcreate -L "${home_size_mib}M" -n home archvg || error_exit "Failed to create home logical volume."
        fi
        format_filesystem "/dev/archvg/home" "$HOME_FILESYSTEM_TYPE"
    else
        if [[ "$root_size_mib" == "REMAINING" ]]; then
            log_cmd "lvcreate -l 100%FREE -n root archvg"
            lvcreate -l 100%FREE -n root archvg || error_exit "Failed to create root logical volume."
        else
            log_cmd "lvcreate -L ${root_size_mib}M -n root archvg"
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
        local efi_part
        efi_part=$(get_partition_path "${RAID_DEVICES[0]}" 1)
        format_filesystem "$efi_part" "vfat"
        safe_mount "$efi_part" "/mnt/efi"

        # Capture UUIDs for configuration
        capture_device_info "boot" "/dev/md/XBOOTLDR"
        capture_device_info "efi" "$efi_part"
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
    ROOT_UUID=$(get_device_uuid "/dev/archvg/root") || error_exit "Cannot determine ROOT_UUID"
    LUKS_UUID=$(get_device_uuid "/dev/md/DATA") || error_exit "Cannot determine LUKS_UUID"
    export ROOT_UUID LUKS_UUID

    # Save RAID configuration
    log_info "Saving RAID configuration"
    mkdir -p /mnt/etc
    log_cmd "mdadm --detail --scan > /mnt/etc/mdadm.conf"
    mdadm --detail --scan > /mnt/etc/mdadm.conf || log_warn "Failed to write mdadm.conf"

    # Generate crypttab entry for boot-time unlocking
    log_info "Generating crypttab entry..."
    mkdir -p /mnt/etc
    generate_crypttab "/dev/md/DATA" "cryptlvm"

    log_info "RAID + LVM + LUKS partitioning completed successfully"
}

# Export the function
export -f execute_raid_lvm_luks_partitioning