#!/bin/bash
# lvm.sh - LVM partitioning strategy (ESP + boot + LVM root/home)
set -euo pipefail

# Source common utilities via source_or_die
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../disk_utils.sh
source_or_die "$SCRIPT_DIR/../disk_utils.sh"

# Execute LVM partitioning strategy
execute_lvm_partitioning() {
    echo "=== PHASE 1: LVM Partitioning ==="
    log_info "Starting LVM partitioning for $INSTALL_DISK..."

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

        # Boot partition - 1GB ext4 at /boot
        boot_part_num=$part_num
        create_boot_partition "$INSTALL_DISK" "$part_num" "1024"
        current_start_mib=$((current_start_mib + 1024))
        part_num=$((part_num + 1))
    else
        # BIOS: Boot partition at /boot
        boot_part_num=$part_num
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
    
    # LVM partition
    log_info "Creating LVM partition..."
    if [ "$BOOT_MODE" = "UEFI" ]; then
        sgdisk -n "$part_num:0:0" -t "$part_num:$LVM_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create LVM partition."
    else
        printf "n\np\n$part_num\n\n\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create LVM partition."
    fi
    sync_partitions "$INSTALL_DISK"
    local lvm_part
    lvm_part=$(get_partition_path "$INSTALL_DISK" "$part_num")
    
    # Create LVM setup
    log_info "Setting up LVM..."
    pvcreate "$lvm_part" || error_exit "Failed to create physical volume."
    vgcreate arch "$lvm_part" || error_exit "Failed to create volume group."
    
    # Create logical volumes
    log_info "Creating logical volumes..."
    local root_size_gb=50
    lvcreate -L "${root_size_gb}G" -n root arch || error_exit "Failed to create root logical volume."
    
    if [ "$WANT_HOME_PARTITION" = "yes" ]; then
        lvcreate -l 100%FREE -n home arch || error_exit "Failed to create home logical volume."
    fi
    
    # Format logical volumes
    log_info "Formatting logical volumes..."
    format_filesystem "/dev/arch/root" "$ROOT_FILESYSTEM_TYPE"
    capture_device_info "root" "/dev/arch/root"
    safe_mount "/dev/arch/root" "/mnt"

    if [ "$WANT_HOME_PARTITION" = "yes" ]; then
        format_filesystem "/dev/arch/home" "$HOME_FILESYSTEM_TYPE"
        capture_device_info "home" "/dev/arch/home"
        mkdir -p /mnt/home
        safe_mount "/dev/arch/home" "/mnt/home"
    fi

    # Mount boot and ESP partitions
    mkdir -p /mnt/boot /mnt/efi

    local boot_device
    boot_device=$(get_partition_path "$INSTALL_DISK" "$boot_part_num")
    safe_mount "$boot_device" "/mnt/boot"

    if [ "$BOOT_MODE" = "UEFI" ]; then
        local esp_device
        esp_device=$(get_partition_path "$INSTALL_DISK" "$esp_part_num")
        safe_mount "$esp_device" "/mnt/efi"
        export EFI_DEVICE="$esp_device"
    fi

    # Capture root UUID
    ROOT_UUID=$(get_device_uuid "/dev/arch/root")
    export ROOT_UUID

    log_partitioning_complete "LVM (ESP + boot + LVM)"
}
