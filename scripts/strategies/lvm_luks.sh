#!/bin/bash
# lvm_luks.sh - LVM + LUKS partitioning strategy with ESP + XBOOTLDR (UEFI) or boot partition (BIOS)
set -euo pipefail

# Source common utilities via source_or_die
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../disk_utils.sh
source_or_die "$SCRIPT_DIR/../disk_utils.sh"

# Execute LVM + LUKS partitioning strategy
execute_lvm_luks_partitioning() {
    echo "=== PHASE 1: LVM + LUKS Partitioning (ESP + XBOOTLDR) ==="
    log_info "Starting LVM + LUKS partitioning with ESP + XBOOTLDR for $INSTALL_DISK..."
    
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
    
    # LUKS partition (for LVM)
    log_info "Creating LUKS partition for LVM..."
    if [ "$BOOT_MODE" = "UEFI" ]; then
        sgdisk -n "$part_num:0:0" -t "$part_num:$LUKS_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create LUKS partition."
    else
        printf "n\np\n$part_num\n\n\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create LUKS partition."
    fi
    partprobe "$INSTALL_DISK"
    local luks_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
    
    # Set up LUKS encryption using helper function (non-interactive)
    local encrypted_dev
    encrypted_dev=$(setup_luks_encryption "$luks_dev" "cryptlvm")
    
    # Create LVM setup on encrypted device
    log_info "Setting up LVM on encrypted device..."
    pvcreate /dev/mapper/cryptlvm || error_exit "Failed to create physical volume."
    vgcreate arch /dev/mapper/cryptlvm || error_exit "Failed to create volume group."
    
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
    
    # Store LVM device mapping
    LVM_DEVICES_MAP["arch_root"]="/dev/arch/root"
    if [ "$WANT_HOME_PARTITION" = "yes" ]; then
        LVM_DEVICES_MAP["arch_home"]="/dev/arch/home"
    fi
    
    # Generate crypttab entry for boot-time unlocking
    log_info "Generating crypttab entry..."
    mkdir -p /mnt/etc
    generate_crypttab "$luks_dev" "cryptlvm"

    log_partitioning_complete "LVM + LUKS ESP + XBOOTLDR"
}
