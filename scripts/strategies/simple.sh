#!/bin/bash
# simple.sh - Simple partitioning strategy with ESP + XBOOTLDR (UEFI) or boot partition (BIOS)

# Source common utilities
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$SCRIPT_DIR/../disk_utils.sh"

# Execute simple partitioning strategy
execute_simple_partitioning() {
    echo "=== PHASE 1: Simple Partitioning (ESP + XBOOTLDR) ==="
    log_info "Starting simple partitioning with ESP + XBOOTLDR for $INSTALL_DISK..."
    
    # Validate requirements
    validate_partitioning_requirements
    
    # Wipe disk
    wipe_disk "$INSTALL_DISK"
    
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
    
    # Root partition
    create_root_partition "$INSTALL_DISK" "$part_num" "$ROOT_FILESYSTEM_TYPE"
    
    # Separate home partition (if requested)
    if [ "$WANT_HOME_PARTITION" = "yes" ]; then
        part_num=$((part_num + 1))
        create_home_partition "$INSTALL_DISK" "$part_num" "$HOME_FILESYSTEM_TYPE"
    fi
    
    log_partitioning_complete "Simple ESP + XBOOTLDR"
}
