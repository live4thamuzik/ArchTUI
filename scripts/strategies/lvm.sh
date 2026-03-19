#!/bin/bash
# lvm.sh - LVM partitioning strategy (ESP + boot + LVM root/home)
set -euo pipefail

# Source common utilities via source_or_die
_STRATEGY_SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../disk_utils.sh
source_or_die "$_STRATEGY_SCRIPT_DIR/../disk_utils.sh"

# Execute LVM partitioning strategy
execute_lvm_partitioning() {
    echo "=== PHASE 1: LVM Partitioning ==="
    log_info "Starting LVM partitioning for $INSTALL_DISK..."

    # Setup cleanup trap for error recovery
    setup_partitioning_trap

    # Validate requirements
    validate_partitioning_requirements

    # --- Dual-boot detection ---
    local use_existing_esp="no"
    local existing_esp=""

    if detect_other_os; then
        log_warn "Other OS detected - enabling os-prober for dual-boot"
        export OS_PROBER="yes"
    fi

    existing_esp=$(detect_existing_esp "$INSTALL_DISK")

    # detect_other_os already called detect_windows_installation internally
    if [[ "${WINDOWS_DETECTED:-}" == "yes" ]]; then
        log_warn "Windows installation detected on ${WINDOWS_ESP_DEVICE:-unknown}"
        if [[ -n "$existing_esp" && "${WINDOWS_ESP_DEVICE:-}" == "$existing_esp" ]]; then
            log_warn "Will preserve existing ESP for dual-boot chainloading"
            use_existing_esp="yes"
        fi
        export DUAL_BOOT_WINDOWS="yes"
    fi

    if [[ "$use_existing_esp" == "yes" && "$existing_esp" == "${INSTALL_DISK}"* ]]; then
        log_error "Same-disk dual-boot detected: Windows ESP is on target disk!"
        log_error "Automatic partitioning would destroy Windows. Use 'manual' strategy instead."
        log_error "See: https://wiki.archlinux.org/title/Dual_boot_with_Windows"
        return 1
    fi

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

        # Boot partition - 1GB ext4 at /boot
        boot_part_num=$part_num
        create_boot_partition "$INSTALL_DISK" "$part_num" "1024"
        current_start_mib=$((current_start_mib + 1024))
        part_num=$((part_num + 1))
    else
        # BIOS with GPT: Need BIOS boot partition for GRUB (no filesystem, just raw)
        create_bios_boot_partition "$INSTALL_DISK" "$part_num"
        part_num=$((part_num + 1))

        # Boot partition - 1GB ext4 at /boot
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
    
    # LVM partition (uses remaining space)
    log_info "Creating LVM partition..."
    log_cmd "sgdisk -n $part_num:0:0 -t $part_num:$LVM_PARTITION_TYPE $INSTALL_DISK"
    sgdisk -n "$part_num:0:0" -t "$part_num:$LVM_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create LVM partition."
    sync_partitions "$INSTALL_DISK"
    local lvm_part
    lvm_part=$(get_partition_path "$INSTALL_DISK" "$part_num")
    
    # Create LVM setup
    log_info "Setting up LVM..."
    log_cmd "pvcreate $lvm_part"
    pvcreate "$lvm_part" || error_exit "Failed to create physical volume."
    log_cmd "vgcreate archvg $lvm_part"
    vgcreate archvg "$lvm_part" || error_exit "Failed to create volume group."
    
    # Create logical volumes
    log_info "Creating logical volumes..."
    local root_size_mib
    root_size_mib=$(get_root_size_mib)

    if [ "$WANT_HOME_PARTITION" = "yes" ]; then
        if [[ "$root_size_mib" == "REMAINING" ]]; then
            # Can't both take remaining — fall back to default
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
        # No home: root takes all or user-specified size
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

    # Capture root UUID
    ROOT_UUID=$(get_device_uuid "/dev/archvg/root") || error_exit "Cannot determine ROOT_UUID"
    export ROOT_UUID

    log_partitioning_complete "LVM (ESP + boot + LVM)"
}
