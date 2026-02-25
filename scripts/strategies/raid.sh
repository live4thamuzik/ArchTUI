#!/bin/bash
# raid.sh - RAID partitioning strategy with ESP + XBOOTLDR (UEFI) or boot partition (BIOS)
set -euo pipefail

# Source common utilities via source_or_die
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../disk_utils.sh
source_or_die "$SCRIPT_DIR/../disk_utils.sh"

# Execute RAID partitioning strategy
execute_raid_partitioning() {
    echo "=== PHASE 1: RAID Partitioning (ESP + boot + RAID) ==="
    log_info "Starting RAID partitioning for multiple disks (RAID Level: ${RAID_LEVEL:-raid1})..."

    # Setup cleanup trap for error recovery
    setup_partitioning_trap

    # Validate RAID requirements
    if [ ${#RAID_DEVICES[@]} -lt 2 ]; then
        error_exit "RAID requires at least 2 disks. Current disks: ${RAID_DEVICES[*]}"
    fi

    # Validate requirements
    validate_partitioning_requirements
    
    local raid_level="${RAID_LEVEL:-raid1}"
    log_info "RAID level: $raid_level"
    log_info "RAID devices: ${RAID_DEVICES[*]}"
    
    local efi_part_num=1
    local xbootldr_part_num=2
    local data_part_num=3

    # --- Phase 1: Partition all RAID member disks identically ---
    for disk in "${RAID_DEVICES[@]}"; do
        log_info "Partitioning RAID member disk: $disk"
        wipe_disk "$disk" "CONFIRMED"

        local current_start_mib=1

        # Create partition table (GPT for UEFI, MBR for BIOS)
        create_partition_table "$disk"

        # 1. ESP partition on each disk (if UEFI) - NOT in RAID, 512MB
        if [ "$BOOT_MODE" = "UEFI" ]; then
            sgdisk -n "$efi_part_num:0:+512M" -t "$efi_part_num:$EFI_PARTITION_TYPE" -c "$efi_part_num:EFI" "$disk" || error_exit "Failed to create ESP partition on $disk."
            current_start_mib=$((current_start_mib + 512))
        fi

        # 2. Boot partition on each disk - NOT in RAID, 1GB ext4
        if [ "$BOOT_MODE" = "BIOS" ]; then
            # BIOS boot partition first
            sgdisk -n "1:0:+1M" -t "1:$BIOS_BOOT_PARTITION_TYPE" -c "1:BIOSBOOT" "$disk" || error_exit "Failed to create BIOS boot partition on $disk."
            current_start_mib=$((current_start_mib + 1))
        fi

        # 3. Boot partition (for kernels) - NOT in RAID, standard Linux type
        local boot_part_num=$xbootldr_part_num
        sgdisk -n "$boot_part_num:0:+1024M" -t "$boot_part_num:$LINUX_PARTITION_TYPE" -c "$boot_part_num:BOOT" "$disk" || error_exit "Failed to create boot partition on $disk."
        current_start_mib=$((current_start_mib + 1024))
        
        # 4. Data partition on each disk (takes rest of disk) - IN RAID
        if [ "$BOOT_MODE" = "UEFI" ]; then
            sgdisk -n "$data_part_num:0:0" -t "$data_part_num:$LINUX_PARTITION_TYPE" "$disk" || error_exit "Failed to create data partition on $disk."
        else
            sgdisk -n "$data_part_num:0:0" -t "$data_part_num:$LINUX_PARTITION_TYPE" "$disk" || error_exit "Failed to create data partition on $disk."
        fi
        
        sync_partitions "$disk"
    done

    # --- Phase 2: Create RAID array ---
    log_info "Creating RAID array..."
    
    # Get data partition paths for RAID array
    local data_partitions=()
    for disk in "${RAID_DEVICES[@]}"; do
        local data_part=$(get_partition_path "$disk" "$data_part_num")
        data_partitions+=("$data_part")
    done
    
    # Create RAID array
    mdadm --create --run --verbose --level="$raid_level" --raid-devices="${#RAID_DEVICES[@]}" /dev/md0 "${data_partitions[@]}" || error_exit "Failed to create RAID array."
    
    # Wait for RAID to be ready
    sleep 5
    mdadm --wait /dev/md0 || error_exit "RAID array not ready."
    
    # --- Phase 3: Format and mount root FIRST ---
    format_filesystem "/dev/md0" "$ROOT_FILESYSTEM_TYPE"
    capture_device_info "root" "/dev/md0"

    # Mount root (setup_btrfs_subvolumes handles its own mount/remount cycle)
    if [ "$ROOT_FILESYSTEM_TYPE" = "btrfs" ]; then
        local include_home="yes"
        [ "$WANT_HOME_PARTITION" = "yes" ] && include_home="no"
        setup_btrfs_subvolumes "/dev/md0" "$include_home"
    else
        safe_mount "/dev/md0" "/mnt"
    fi

    # --- Phase 4: Handle boot partitions (after root is mounted) ---
    if [ "$BOOT_MODE" = "UEFI" ]; then
        # Format and mount ESP (not in RAID)
        local first_disk="${RAID_DEVICES[0]}"
        local efi_part=$(get_partition_path "$first_disk" "$efi_part_num")
        format_filesystem "$efi_part" "$EFI_FILESYSTEM"
        capture_device_info "efi" "$efi_part"
        safe_mount "$efi_part" "/mnt/efi"

        # Format and mount XBOOTLDR (not in RAID)
        local xbootldr_part=$(get_partition_path "$first_disk" "$xbootldr_part_num")
        format_filesystem "$xbootldr_part" "$BOOT_FILESYSTEM"
        capture_device_info "boot" "$xbootldr_part"
        safe_mount "$xbootldr_part" "/mnt/boot"
    else
        # BIOS: Format and mount boot partition (not in RAID)
        local first_disk="${RAID_DEVICES[0]}"
        local boot_part=$(get_partition_path "$first_disk" "$xbootldr_part_num")
        format_filesystem "$boot_part" "$BOOT_FILESYSTEM"
        capture_device_info "boot" "$boot_part"
        safe_mount "$boot_part" "/mnt/boot"
    fi
    
    # Create swap file if requested (non-LVM RAID uses swapfile since array is a single device)
    if [ "$WANT_SWAP" = "yes" ]; then
        create_swapfile "$(get_swap_size_mib)"
    fi

    # Capture UUIDs for bootloader config
    ROOT_UUID=$(get_device_uuid "/dev/md0") || error_exit "Cannot determine ROOT_UUID"
    export ROOT_UUID

    # Save RAID configuration for boot
    mkdir -p /mnt/etc
    mdadm --detail --scan > /mnt/etc/mdadm.conf
    log_info "Saved RAID configuration to /mnt/etc/mdadm.conf"

    log_partitioning_complete "RAID (ESP + boot + RAID array)"
}
