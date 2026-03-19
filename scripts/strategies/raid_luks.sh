#!/bin/bash
# raid_luks.sh - RAID + LUKS partitioning strategy
set -euo pipefail

# Source common utilities via source_or_die
_STRATEGY_SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../disk_utils.sh
source_or_die "$_STRATEGY_SCRIPT_DIR/../disk_utils.sh"

# Execute RAID + LUKS partitioning strategy
execute_raid_luks_partitioning() {
    echo "=== RAID + LUKS Partitioning ==="
    log_info "Starting RAID + LUKS partitioning strategy"

    # Setup cleanup trap for error recovery
    setup_partitioning_trap

    # Clean up stale RAID/LVM/LUKS state from any previous failed attempt
    cleanup_stale_raid

    # Validate that we have multiple disks
    if [[ ${#RAID_DEVICES[@]} -lt 2 ]]; then
        error_exit "RAID + LUKS requires at least 2 disks, but only ${#RAID_DEVICES[@]} provided"
    fi

    # --- Dual-boot detection ---
    if detect_other_os; then
        log_warn "Other OS detected - enabling os-prober for dual-boot"
        export OS_PROBER="yes"
    fi

    # detect_other_os already called detect_windows_installation internally
    if [[ "${WINDOWS_DETECTED:-}" == "yes" ]]; then
        log_warn "Windows installation detected on ${WINDOWS_ESP_DEVICE:-unknown}"
        export DUAL_BOOT_WINDOWS="yes"
        local win_esp_disk
        win_esp_disk=$(lsblk -ndo PKNAME "${WINDOWS_ESP_DEVICE}" 2>/dev/null || true)
        if [[ -n "$win_esp_disk" ]]; then
            for disk in "${RAID_DEVICES[@]}"; do
                if [[ "$disk" == "/dev/$win_esp_disk" ]]; then
                    log_error "Windows ESP ($WINDOWS_ESP_DEVICE) is on RAID member disk $disk!"
                    log_error "RAID partitioning would destroy Windows. Remove $disk from RAID devices."
                    return 1
                fi
            done
        fi
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
            log_cmd "sgdisk --zap-all $disk"
            sgdisk --zap-all "$disk" || error_exit "Failed to wipe $disk"
            if bootloader_needs_fat32_boot; then
                # 2-partition layout: larger ESP at /boot (no XBOOTLDR)
                log_cmd "sgdisk --new=1:0:+${RAID_ESP_BOOT_SIZE_MIB}MiB --typecode=1:$esp_type --change-name=1:ESP $disk"
                sgdisk --new=1:0:+${RAID_ESP_BOOT_SIZE_MIB}MiB --typecode=1:"$esp_type" --change-name=1:ESP "$disk" || error_exit "Failed to create ESP on $disk"
                log_cmd "sgdisk --new=2:0:0 --typecode=2:$LUKS_PARTITION_TYPE --change-name=2:RAID_MEMBER $disk"
                sgdisk --new=2:0:0 --typecode=2:"$LUKS_PARTITION_TYPE" --change-name=2:RAID_MEMBER "$disk" || error_exit "Failed to create RAID member on $disk"
            else
                # 3-partition layout: ESP + XBOOTLDR + RAID member
                log_cmd "sgdisk --new=1:0:+${DEFAULT_ESP_SIZE_MIB}MiB --typecode=1:$esp_type --change-name=1:ESP $disk"
                sgdisk --new=1:0:+${DEFAULT_ESP_SIZE_MIB}MiB --typecode=1:"$esp_type" --change-name=1:ESP "$disk" || error_exit "Failed to create ESP on $disk"
                log_cmd "sgdisk --new=2:0:+${BOOT_PART_SIZE_MIB}MiB --typecode=2:$xbootldr_type --change-name=2:XBOOTLDR $disk"
                sgdisk --new=2:0:+${BOOT_PART_SIZE_MIB}MiB --typecode=2:"$xbootldr_type" --change-name=2:XBOOTLDR "$disk" || error_exit "Failed to create XBOOTLDR on $disk"
                log_cmd "sgdisk --new=3:0:0 --typecode=3:$LUKS_PARTITION_TYPE --change-name=3:RAID_MEMBER $disk"
                sgdisk --new=3:0:0 --typecode=3:"$LUKS_PARTITION_TYPE" --change-name=3:RAID_MEMBER "$disk" || error_exit "Failed to create RAID member on $disk"
            fi
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
        if bootloader_needs_fat32_boot; then
            # 2-partition layout: no XBOOTLDR, DATA from partition 2
            DATA_PARTS=()

            for disk in "${RAID_DEVICES[@]}"; do
                DATA_PARTS+=("$(get_partition_path "$disk" 2)")
            done

            # Zero stale superblocks and stop auto-assembled arrays
            prepare_raid_partitions "${DATA_PARTS[@]}"

            # Create data RAID array
            log_info "Creating data RAID array"
            log_cmd "mdadm --create --run --verbose --level=$raid_level --raid-devices=${#RAID_DEVICES[@]} /dev/md/DATA ${DATA_PARTS[*]}"
            mdadm --create --run --verbose --level="$raid_level" --raid-devices=${#RAID_DEVICES[@]} /dev/md/DATA "${DATA_PARTS[@]}" || error_exit "Failed to create DATA RAID array"

            # Resume udev event processing now that all arrays are created
            resume_udev

            # Wait for RAID array to be ready
            wait_for_raid_array /dev/md/DATA
        else
            # 3-partition layout: XBOOTLDR RAID1 + DATA RAID
            XBOOTLDR_PARTS=()
            DATA_PARTS=()

            for disk in "${RAID_DEVICES[@]}"; do
                XBOOTLDR_PARTS+=("$(get_partition_path "$disk" 2)")
                DATA_PARTS+=("$(get_partition_path "$disk" 3)")
            done

            # Zero stale superblocks and stop auto-assembled arrays
            prepare_raid_partitions "${XBOOTLDR_PARTS[@]}" "${DATA_PARTS[@]}"

            # Create XBOOTLDR RAID1 array (always RAID1 for boot)
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
        fi

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

        # Create boot RAID1 array (always RAID1 for boot)
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
    encrypted_dev=$(setup_luks_encryption "/dev/md/DATA" "cryptroot")

    # FIDO2 enrollment (if configured)
    if [[ "${ENCRYPTION_KEY_TYPE:-Password}" == *"FIDO2"* ]]; then
        enroll_fido2 "/dev/md/DATA" || log_warn "FIDO2 enrollment failed — password-only fallback"
    fi

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
        if bootloader_needs_fat32_boot; then
            # 2-partition layout: ESP from first disk mounted at /boot (no /efi)
            local efi_part
            efi_part=$(get_partition_path "${RAID_DEVICES[0]}" 1)
            format_filesystem "$efi_part" "vfat"
            safe_mount "$efi_part" "/mnt/boot"

            # Capture UUIDs for configuration
            capture_device_info "boot" "$efi_part"
            capture_device_info "root" "/dev/mapper/cryptroot"
            capture_device_info "luks" "/dev/md/DATA"
        else
            # 3-partition layout: XBOOTLDR RAID1 at /boot, ESP at /efi
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
        fi
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
        capture_device_info "swap" "/mnt/swapfile"
    fi

    # Capture UUIDs for bootloader config
    ROOT_UUID=$(get_device_uuid "/dev/mapper/cryptroot") || error_exit "Cannot determine ROOT_UUID"
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
    generate_crypttab "/dev/md/DATA" "cryptroot"

    log_partitioning_complete "RAID + LUKS (ESP + boot RAID1 + encrypted data RAID)"
}

# Export the function
export -f execute_raid_luks_partitioning