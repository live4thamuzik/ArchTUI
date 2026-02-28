#!/bin/bash
# lvm_luks.sh - LVM + LUKS partitioning strategy (ESP + boot + encrypted LVM)
set -euo pipefail

# Source common utilities via source_or_die
_STRATEGY_SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../disk_utils.sh
source_or_die "$_STRATEGY_SCRIPT_DIR/../disk_utils.sh"

# Execute LVM + LUKS partitioning strategy
execute_lvm_luks_partitioning() {
    echo "=== PHASE 1: LVM + LUKS Partitioning ==="
    log_info "Starting LVM + LUKS partitioning for $INSTALL_DISK..."

    # Setup cleanup trap for error recovery
    setup_partitioning_trap

    # Validate requirements
    validate_partitioning_requirements

    # Dual-boot detection
    if detect_other_os; then
        export OS_PROBER="yes"
    fi

    # Wipe disk with explicit confirmation
    wipe_disk "$INSTALL_DISK" "CONFIRMED"

    local current_start_mib=1
    local part_num=1
    local esp_part_num=0
    local boot_part_num=0

    # Create partition table
    create_partition_table "$INSTALL_DISK"

    # ESP Partition (for UEFI only) - 512MB FAT32 at /efi
    if [ "$BOOT_MODE" = "UEFI" ]; then
        esp_part_num=$part_num
        create_esp_partition "$INSTALL_DISK" "$part_num" "512"
        current_start_mib=$((current_start_mib + 512))
        part_num=$((part_num + 1))

        # Boot partition - 1GB ext4 at /boot (unencrypted for GRUB)
        boot_part_num=$part_num
        create_boot_partition "$INSTALL_DISK" "$part_num" "1024"
        current_start_mib=$((current_start_mib + 1024))
        part_num=$((part_num + 1))
    else
        # BIOS with GPT: Need BIOS boot partition for GRUB (no filesystem, just raw)
        create_bios_boot_partition "$INSTALL_DISK" "$part_num"
        part_num=$((part_num + 1))

        # Boot partition - 1GB ext4 at /boot (unencrypted for GRUB)
        boot_part_num=$part_num
        create_boot_partition "$INSTALL_DISK" "$part_num" "1024"
        current_start_mib=$((current_start_mib + 1024))
        part_num=$((part_num + 1))
    fi
    
    # Swap partition (if requested)
    if [ "$WANT_SWAP" = "yes" ]; then
        local swap_size_mib=$(get_swap_size_mib)
        create_swap_partition "$INSTALL_DISK" "$part_num" "$swap_size_mib"
        local swap_device
        swap_device=$(get_partition_path "$INSTALL_DISK" "$part_num")
        capture_device_info "swap" "$swap_device"
        SWAP_UUID=$(get_device_uuid "$swap_device") || log_warn "Cannot determine SWAP_UUID"
        export SWAP_UUID
        current_start_mib=$((current_start_mib + swap_size_mib))
        part_num=$((part_num + 1))
    fi
    
    # LUKS partition for LVM (uses remaining space)
    log_info "Creating LUKS partition for LVM..."
    log_cmd "sgdisk -n $part_num:0:0 -t $part_num:$LUKS_PARTITION_TYPE $INSTALL_DISK"
    sgdisk -n "$part_num:0:0" -t "$part_num:$LUKS_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create LUKS partition."
    sync_partitions "$INSTALL_DISK"
    local luks_dev
    luks_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
    
    # Set up LUKS encryption using helper function (non-interactive)
    local encrypted_dev
    encrypted_dev=$(setup_luks_encryption "$luks_dev" "cryptlvm")
    
    # Create LVM setup on encrypted device
    log_info "Setting up LVM on encrypted device..."
    log_cmd "pvcreate /dev/mapper/cryptlvm"
    pvcreate /dev/mapper/cryptlvm || error_exit "Failed to create physical volume."
    log_cmd "vgcreate archvg /dev/mapper/cryptlvm"
    vgcreate archvg /dev/mapper/cryptlvm || error_exit "Failed to create volume group."
    
    # Create logical volumes
    log_info "Creating logical volumes..."
    local root_size_mib
    root_size_mib=$(get_root_size_mib)

    if [ "$WANT_HOME_PARTITION" = "yes" ]; then
        if [[ "$root_size_mib" == "REMAINING" ]]; then
            log_warn "Root=Remaining with separate home: falling back to ${DEFAULT_ROOT_SIZE_MIB}MiB root"
            root_size_mib="$DEFAULT_ROOT_SIZE_MIB"
        fi
        log_cmd "lvcreate -L ${root_size_mib}M -n root archvg"
        lvcreate -L "${root_size_mib}M" -n root archvg || error_exit "Failed to create root logical volume."

        local home_size_mib
        home_size_mib=$(get_home_size_mib)
        if [[ "$home_size_mib" == "REMAINING" ]]; then
            log_cmd "lvcreate -l 100%FREE -n home archvg"
            lvcreate -l 100%FREE -n home archvg || error_exit "Failed to create home logical volume."
        else
            log_cmd "lvcreate -L ${home_size_mib}M -n home archvg"
            lvcreate -L "${home_size_mib}M" -n home archvg || error_exit "Failed to create home logical volume."
        fi
    else
        if [[ "$root_size_mib" == "REMAINING" ]]; then
            log_cmd "lvcreate -l 100%FREE -n root archvg"
            lvcreate -l 100%FREE -n root archvg || error_exit "Failed to create root logical volume."
        else
            log_cmd "lvcreate -L ${root_size_mib}M -n root archvg"
            lvcreate -L "${root_size_mib}M" -n root archvg || error_exit "Failed to create root logical volume."
        fi
    fi
    
    # Format logical volumes
    log_info "Formatting logical volumes..."
    format_filesystem "/dev/archvg/root" "$ROOT_FILESYSTEM_TYPE"
    capture_device_info "root" "/dev/archvg/root"

    # Handle Btrfs subvolumes if needed
    if [ "$ROOT_FILESYSTEM_TYPE" = "btrfs" ]; then
        local include_home="yes"
        [ "$WANT_HOME_PARTITION" = "yes" ] && include_home="no"
        setup_btrfs_subvolumes "/dev/archvg/root" "$include_home"
    else
        safe_mount "/dev/archvg/root" "/mnt"
    fi

    if [ "$WANT_HOME_PARTITION" = "yes" ]; then
        format_filesystem "/dev/archvg/home" "$HOME_FILESYSTEM_TYPE"
        capture_device_info "home" "/dev/archvg/home"
        mkdir -p /mnt/home
        safe_mount "/dev/archvg/home" "/mnt/home"
    fi

    # Mount boot and ESP partitions
    mkdir -p /mnt/boot /mnt/efi

    local boot_device
    boot_device=$(get_partition_path "$INSTALL_DISK" "$boot_part_num")
    safe_mount "$boot_device" "/mnt/boot"
    capture_device_info "boot" "$boot_device"

    if [ "$BOOT_MODE" = "UEFI" ]; then
        local esp_device
        esp_device=$(get_partition_path "$INSTALL_DISK" "$esp_part_num")
        safe_mount "$esp_device" "/mnt/efi"
        capture_device_info "efi" "$esp_device"
        export EFI_DEVICE="$esp_device"
    fi

    # Capture UUIDs for bootloader config
    ROOT_UUID=$(get_device_uuid "/dev/archvg/root") || error_exit "Cannot determine ROOT_UUID"
    LUKS_UUID=$(get_device_uuid "$luks_dev") || error_exit "Cannot determine LUKS_UUID"
    export ROOT_UUID LUKS_UUID

    # Generate crypttab entry for boot-time unlocking
    log_info "Generating crypttab entry..."
    mkdir -p /mnt/etc
    generate_crypttab "$luks_dev" "cryptlvm"

    log_partitioning_complete "LVM + LUKS (ESP + boot + encrypted LVM)"
}
