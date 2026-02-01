#!/bin/bash
# raid.sh - RAID partitioning strategy with ESP + XBOOTLDR (UEFI) or boot partition (BIOS)
set -euo pipefail

# Source common utilities via source_or_die
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../disk_utils.sh
source_or_die "$SCRIPT_DIR/../disk_utils.sh"

# Execute RAID partitioning strategy
execute_raid_partitioning() {
    echo "=== PHASE 1: RAID Partitioning (ESP + XBOOTLDR) ==="
    log_info "Starting auto RAID partitioning with ESP + XBOOTLDR for multiple disks (RAID Level: ${RAID_LEVEL:-raid1})..."
    
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

        # 1. ESP partition on each disk (if UEFI) - NOT in RAID
        if [ "$BOOT_MODE" = "UEFI" ]; then
            sgdisk -n "$efi_part_num:0:+100M" -t "$efi_part_num:$EFI_PARTITION_TYPE" "$disk" || error_exit "Failed to create ESP partition on $disk."
            current_start_mib=$((current_start_mib + 100))
        fi

        # 2. Boot partition on each disk (if BIOS) - NOT in RAID
        if [ "$BOOT_MODE" = "BIOS" ]; then
            printf "n\np\n$xbootldr_part_num\n\n+1024M\nw\n" | fdisk "$disk" || error_exit "Failed to create boot partition on $disk."
            current_start_mib=$((current_start_mib + 1024))
        fi

        # 3. XBOOTLDR partition on each disk (UEFI only) - NOT in RAID
        if [ "$BOOT_MODE" = "UEFI" ]; then
            sgdisk -n "$xbootldr_part_num:0:+1024M" -t "$xbootldr_part_num:$XBOOTLDR_PARTITION_TYPE" "$disk" || error_exit "Failed to create XBOOTLDR partition on $disk."
            current_start_mib=$((current_start_mib + 1024))
        fi
        
        # 4. Data partition on each disk (takes rest of disk) - IN RAID
        if [ "$BOOT_MODE" = "UEFI" ]; then
            sgdisk -n "$data_part_num:0:0" -t "$data_part_num:$LINUX_PARTITION_TYPE" "$disk" || error_exit "Failed to create data partition on $disk."
        else
            printf "n\np\n$data_part_num\n\n\nw\n" | fdisk "$disk" || error_exit "Failed to create data partition on $disk."
        fi
        
        partprobe "$disk"
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
    mdadm --create --verbose --level="$raid_level" --raid-devices="${#RAID_DEVICES[@]}" /dev/md0 "${data_partitions[@]}" || error_exit "Failed to create RAID array."
    
    # Wait for RAID to be ready
    sleep 5
    mdadm --wait /dev/md0 || error_exit "RAID array not ready."
    
    # --- Phase 3: Handle boot partitions ---
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

    # --- Phase 4: Handle data partition ---
    # Format RAID array
    format_filesystem "/dev/md0" "$ROOT_FILESYSTEM_TYPE"
    capture_device_info "root" "/dev/md0"
    safe_mount "/dev/md0" "/mnt"
    
    # Handle Btrfs subvolumes if needed
    if [ "$ROOT_FILESYSTEM_TYPE" = "btrfs" ]; then
        log_info "Creating Btrfs subvolumes..."
        btrfs subvolume create /mnt/@
        btrfs subvolume create /mnt/@home
        btrfs subvolume create /mnt/@var
        btrfs subvolume create /mnt/@tmp
    fi
    
    # Separate home partition (if requested)
    if [ "$WANT_HOME_PARTITION" = "yes" ]; then
        log_info "Creating separate home RAID array..."
        
        # Create second RAID array for home
        local home_partitions=()
        for disk in "${RAID_DEVICES[@]}"; do
            local home_part=$(get_partition_path "$disk" "$((data_part_num + 1))")
            home_partitions+=("$home_part")
        done
        
        mdadm --create --verbose --level="$raid_level" --raid-devices="${#RAID_DEVICES[@]}" /dev/md1 "${home_partitions[@]}" || error_exit "Failed to create home RAID array."
        
        # Wait for RAID to be ready
        sleep 5
        mdadm --wait /dev/md1 || error_exit "Home RAID array not ready."
        
        # Format and mount home
        format_filesystem "/dev/md1" "$HOME_FILESYSTEM_TYPE"
        capture_device_info "home" "/dev/md1"
        mkdir -p /mnt/home
        safe_mount "/dev/md1" "/mnt/home"
        
        # Handle Btrfs subvolumes for home if needed
        if [ "$HOME_FILESYSTEM_TYPE" = "btrfs" ]; then
            log_info "Creating Btrfs subvolumes for home..."
            btrfs subvolume create /mnt/home/@
        fi
    fi
    
    log_partitioning_complete "RAID ESP + XBOOTLDR"
}
