#!/bin/bash
# disk_strategies.sh - Contains specific partitioning/storage layout functions

# --- Partition Configuration Constants ---
# These constants replace magic strings throughout the partitioning functions
# for better readability and maintainability

# Partition Types (GPT codes)
readonly EFI_PARTITION_TYPE="EF00"
# shellcheck disable=SC2034
readonly LINUX_PARTITION_TYPE="8300"
# shellcheck disable=SC2034
readonly LVM_PARTITION_TYPE="8E00"
# shellcheck disable=SC2034
readonly LUKS_PARTITION_TYPE="8309"
readonly SWAP_PARTITION_TYPE="8200"

# Partition Names
# shellcheck disable=SC2034
readonly EFI_PARTITION_NAME="EFI System Partition"
# shellcheck disable=SC2034
readonly LINUX_PARTITION_NAME="Linux filesystem"
# shellcheck disable=SC2034
readonly LVM_PARTITION_NAME="Linux LVM"
# shellcheck disable=SC2034
readonly LUKS_PARTITION_NAME="Linux LUKS"
# shellcheck disable=SC2034
readonly SWAP_PARTITION_NAME="Linux swap"

# Default Partition Sizes (in MiB)
readonly DEFAULT_SWAP_SIZE_MIB=2048
readonly DEFAULT_ROOT_SIZE_MIB=102400

# Filesystem Types
# shellcheck disable=SC2034
readonly DEFAULT_ROOT_FILESYSTEM="ext4"
# shellcheck disable=SC2034
readonly DEFAULT_HOME_FILESYSTEM="ext4"
# shellcheck disable=SC2034
readonly EFI_FILESYSTEM="vfat"
# shellcheck disable=SC2034
readonly BOOT_FILESYSTEM="ext4"

# RAID Device Names
# shellcheck disable=SC2034
readonly RAID_BOOT_DEVICE="/dev/md0"
# shellcheck disable=SC2034
readonly RAID_ROOT_DEVICE="/dev/md1"
# shellcheck disable=SC2034
readonly RAID_HOME_DEVICE="/dev/md2"
# shellcheck disable=SC2034
readonly RAID_LVM_DEVICE="/dev/md1"
# shellcheck disable=SC2034
readonly RAID_LUKS_DEVICE="/dev/md1"

# --- Main Dispatcher for Disk Strategy ---
execute_disk_strategy() {
    log_info "Executing disk strategy: $PARTITION_SCHEME"
    
    # Parse the partition scheme to extract strategy and RAID level
    local base_strategy="$PARTITION_SCHEME"
    local raid_level=""
    
    # Check if this is a RAID strategy with level (e.g., auto_raid_lvm_raid1)
    if [[ "$PARTITION_SCHEME" =~ ^(.+)_(raid[0-9]+)$ ]]; then
        base_strategy="${BASH_REMATCH[1]}"
        raid_level="${BASH_REMATCH[2]}"
        log_info "Detected RAID strategy: $base_strategy with RAID level: $raid_level"
    fi
    
    # Auto-populate RAID_DEVICES for RAID strategies in TUI mode
    if [[ "$TUI_MODE" == "true" && "$base_strategy" =~ ^auto_raid ]]; then
        auto_populate_raid_devices || error_exit "Failed to populate RAID devices"
    fi
    
    # Find the corresponding function name
    local strategy_func=""
    local i=0
    while [ "$i" -lt "${#PARTITION_STRATEGY_FUNCTIONS[@]}" ]; do
        if [ "${PARTITION_STRATEGY_FUNCTIONS[$i]}" == "$base_strategy" ]; then
            strategy_func="${PARTITION_STRATEGY_FUNCTIONS[$((i+1))]}"
            break
        fi
        i=$((i+2))
    done

    if [[ -n "$strategy_func" ]]; then
        # Export RAID level for the partitioning function to use
        if [[ -n "$raid_level" ]]; then
            export RAID_LEVEL="$raid_level"
            log_info "RAID level set to: $RAID_LEVEL"
        fi
        "$strategy_func" || error_exit "Disk strategy '$PARTITION_SCHEME' failed."
    else
        error_exit "Unknown partitioning scheme: $PARTITION_SCHEME."
    fi
    log_info "Disk strategy execution complete."
}

# Auto-populate RAID_DEVICES for TUI mode
auto_populate_raid_devices() {
    log_info "Auto-populating RAID devices for TUI mode..."
    
    # Get all available disks
    local available_disks=()
    while IFS= read -r line; do
        available_disks+=("/dev/$line")
    done < <(lsblk -dno NAME,TYPE | awk '$2=="disk"{print $1}' | grep -v 'loop' | grep -v 'ram')
    
    if [ ${#available_disks[@]} -lt 2 ]; then
        error_exit "RAID requires at least 2 disks, but only ${#available_disks[@]} disk(s) found: ${available_disks[*]}"
    fi
    
    # Initialize RAID_DEVICES with the primary install disk
    RAID_DEVICES=("$INSTALL_DISK")
    
    # Add additional disks (exclude the primary install disk)
    for disk in "${available_disks[@]}"; do
        if [ "$disk" != "$INSTALL_DISK" ]; then
            RAID_DEVICES+=("$disk")
        fi
    done
    
    log_info "RAID devices populated: ${RAID_DEVICES[*]}"
    export RAID_DEVICES
    
    return 0
}

# --- Specific Partitioning Strategy Implementations ---

do_auto_simple_partitioning() {
    echo "=== PHASE 1: Disk Partitioning ==="
    log_info "Starting auto simple partitioning for $INSTALL_DISK (Boot Mode: $BOOT_MODE)..."

    wipe_disk "$INSTALL_DISK"

    local current_start_mib=1 # Always start at 1MiB for the first partition
    local part_num=1 # Keep track of partition numbers
    local part_dev=""

    # Create partition table (GPT for UEFI, MBR for BIOS)
    if [ "$BOOT_MODE" == "uefi" ]; then
        sgdisk -Z "$INSTALL_DISK" || error_exit "Failed to create GPT label on $INSTALL_DISK."
    else
        # For BIOS, we'll use fdisk to create MBR
        printf "o\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create MBR label on $INSTALL_DISK."
    fi
    partprobe "$INSTALL_DISK"

    # EFI Partition (for UEFI)
    if [ "$BOOT_MODE" == "uefi" ]; then
        log_info "Creating EFI partition (${EFI_PART_SIZE_MIB}MiB)..."
        # Create EFI partition with sgdisk: -n 1:0:+size -t 1:EF00
        local efi_size_mb="${EFI_PART_SIZE_MIB}M"
        sgdisk -n "$part_num:0:+$efi_size_mb" -t "$part_num:$EFI_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create EFI partition."
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        
        # Debug: Show partition info before formatting
        log_info "EFI partition created at: $part_dev"
        sgdisk -p "$INSTALL_DISK" || log_warn "Failed to print partition table"
        
        format_filesystem "$part_dev" "vfat"
        capture_id_for_config "efi" "$part_dev" "UUID"
        capture_id_for_config "efi" "$part_dev" "PARTUUID"
        safe_mount "$part_dev" "/mnt/boot/efi"
        current_start_mib=$((current_start_mib + EFI_PART_SIZE_MIB))
        part_num=$((part_num + 1))
    fi

    # Swap Partition (if desired)
    if [ "$WANT_SWAP" == "yes" ]; then
        log_info "Creating Swap partition..."
        # Use an appropriate size for swap
        local swap_size_mib=$DEFAULT_SWAP_SIZE_MIB # Defaulting to 2 GiB for a reasonable swap partition
        local swap_size_mb="${swap_size_mib}M"
        if [ "$BOOT_MODE" == "uefi" ]; then
            sgdisk -n "$part_num:0:+$swap_size_mb" -t "$part_num:$SWAP_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create swap partition."
        else
            # For BIOS, use fdisk for swap partition
            printf "n\np\n$part_num\n\n+${swap_size_mib}M\nt\n$part_num\n82\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create swap partition."
        fi
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        format_filesystem "$part_dev" "swap"
        capture_id_for_config "swap" "$part_dev" "UUID"
        swapon "$part_dev" || error_exit "Failed to activate swap on $part_dev."
        current_start_mib=$((current_start_mib + swap_size_mib))
        part_num=$((part_num + 1))
    fi

    # Root Partition and Optional Home Partition
    local root_size_mib=$DEFAULT_ROOT_SIZE_MIB # Defaulting to 100 GiB for a reasonable root partition
    
    # Set default filesystem types if not specified
    ROOT_FILESYSTEM_TYPE="${ROOT_FILESYSTEM_TYPE:-ext4}"
    HOME_FILESYSTEM_TYPE="${HOME_FILESYSTEM_TYPE:-ext4}"

    if [ "$WANT_HOME_PARTITION" == "yes" ]; then
        log_info "Creating Root partition and separate Home partition (rest of disk)..."
        # Root partition (fixed size)
        local root_size_mb="${root_size_mib}M"
        if [ "$BOOT_MODE" == "uefi" ]; then
            sgdisk -n "$part_num:0:+$root_size_mb" -t "$part_num:8300" "$INSTALL_DISK" || error_exit "Failed to create root partition."
        else
            # For BIOS, use fdisk for root partition
            printf "n\np\n$part_num\n\n+${root_size_mib}M\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create root partition."
        fi
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        format_filesystem "$part_dev" "$ROOT_FILESYSTEM_TYPE"
        capture_id_for_config "root" "$part_dev" "UUID"
        safe_mount "$part_dev" "/mnt"
        
        # Create Btrfs subvolumes if using Btrfs
        if [ "$ROOT_FILESYSTEM_TYPE" == "btrfs" ]; then
            create_btrfs_subvolumes "/mnt"
        fi
        current_start_mib=$((current_start_mib + root_size_mib))
        part_num=$((part_num + 1))

        # Home partition (takes remaining space)
        log_info "Creating Home partition (rest of disk)..."
        if [ "$BOOT_MODE" == "uefi" ]; then
            sgdisk -n "$part_num:0:0" -t "$part_num:8300" "$INSTALL_DISK" || error_exit "Failed to create home partition."
        else
            # For BIOS, use fdisk for home partition (rest of disk)
            printf "n\np\n$part_num\n\n\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create home partition."
        fi
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        format_filesystem "$part_dev" "$HOME_FILESYSTEM_TYPE"
        capture_id_for_config "home" "$part_dev" "UUID"
        mkdir -p /mnt/home || error_exit "Failed to create /mnt/home."
        safe_mount "$part_dev" "/mnt/home"
    else
        # Root takes all remaining space
        log_info "Creating Root partition (rest of disk)..."
        if [ "$BOOT_MODE" == "uefi" ]; then
            sgdisk -n "$part_num:0:0" -t "$part_num:8300" "$INSTALL_DISK" || error_exit "Failed to create root partition."
        else
            # For BIOS, use fdisk for root partition (rest of disk)
            printf "n\np\n$part_num\n\n\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create root partition."
        fi
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        format_filesystem "$part_dev" "$ROOT_FILESYSTEM_TYPE"
        capture_id_for_config "root" "$part_dev" "UUID"
        safe_mount "$part_dev" "/mnt"
        
        # Create Btrfs subvolumes if using Btrfs
        if [ "$ROOT_FILESYSTEM_TYPE" == "btrfs" ]; then
            create_btrfs_subvolumes "/mnt"
        fi
    fi

    log_info "Simple auto partitioning complete. Filesystems formatted and mounted."
}

do_auto_luks_lvm_partitioning() {
    echo "=== PHASE 1: Disk Partitioning (LUKS+LVM) ==="
    log_info "Starting auto LUKS+LVM partitioning for $INSTALL_DISK (Boot Mode: $BOOT_MODE)..."

    # Set default filesystem types if not specified
    ROOT_FILESYSTEM_TYPE="${ROOT_FILESYSTEM_TYPE:-ext4}"
    HOME_FILESYSTEM_TYPE="${HOME_FILESYSTEM_TYPE:-ext4}"

    wipe_disk "$INSTALL_DISK"

    local current_start_mib=1
    local part_num=1
    local part_dev=""

    # 1. Create partition table (GPT for UEFI, MBR for BIOS)
    if [ "$BOOT_MODE" == "uefi" ]; then
        sgdisk -Z "$INSTALL_DISK" || error_exit "Failed to create GPT label on $INSTALL_DISK."
    else
        # For BIOS, we'll use fdisk to create MBR
        printf "o\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create MBR label on $INSTALL_DISK."
    fi
    partprobe "$INSTALL_DISK"

    # 2. Dedicated /boot Partition - 2GiB (mount this FIRST)
    log_info "Creating dedicated /boot partition (${BOOT_PART_SIZE_MIB}MiB)..."
    local boot_size_mb="${BOOT_PART_SIZE_MIB}M"
    if [ "$BOOT_MODE" == "uefi" ]; then
        sgdisk -n "$part_num:0:+$boot_size_mb" -t "$part_num:8300" "$INSTALL_DISK" || error_exit "Failed to create /boot partition."
    else
        # For BIOS, use fdisk for /boot partition
        printf "n\np\n$part_num\n\n+${BOOT_PART_SIZE_MIB}M\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create /boot partition."
    fi
    partprobe "$INSTALL_DISK"
    part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
    format_filesystem "$part_dev" "ext4"
    capture_id_for_config "boot" "$part_dev" "UUID"
    mkdir -p /mnt/boot || error_exit "Failed to create /mnt/boot."
    safe_mount "$part_dev" "/mnt/boot"
    current_start_mib=$((current_start_mib + BOOT_PART_SIZE_MIB))
    part_num=$((part_num + 1))

    # 3. EFI Partition (for UEFI) - 1024MiB (mounted AFTER /boot partition)
    if [ "$BOOT_MODE" == "uefi" ]; then
        log_info "Creating EFI partition (${EFI_PART_SIZE_MIB}MiB)..."
        # Create EFI partition with sgdisk: -n 1:0:+size -t 1:EF00
        local efi_size_mb="${EFI_PART_SIZE_MIB}M"
        sgdisk -n "$part_num:0:+$efi_size_mb" -t "$part_num:$EFI_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create EFI partition."
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        
        # Debug: Show partition info before formatting
        log_info "EFI partition created at: $part_dev"
        sgdisk -p "$INSTALL_DISK" || log_warn "Failed to print partition table"
        
        format_filesystem "$part_dev" "vfat"
        capture_id_for_config "efi" "$part_dev" "UUID"
        capture_id_for_config "efi" "$part_dev" "PARTUUID"
        
        # Create efi directory inside the mounted /boot partition
        mkdir -p /mnt/boot/efi || error_exit "Failed to create /mnt/boot/efi directory."
        safe_mount "$part_dev" "/mnt/boot/efi"
        current_start_mib=$((current_start_mib + EFI_PART_SIZE_MIB))
        part_num=$((part_num + 1))
    fi

    # 4. Main LUKS Container Partition (takes rest of disk)
    log_info "Creating LUKS container partition (rest of disk)..."
    if [ "$BOOT_MODE" == "uefi" ]; then
        sgdisk -n "$part_num:0:0" -t "$part_num:8300" "$INSTALL_DISK" || error_exit "Failed to create LUKS container partition."
    else
        # For BIOS, use fdisk for LUKS container partition (rest of disk)
        printf "n\np\n$part_num\n\n\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create LUKS container partition."
    fi
    partprobe "$INSTALL_DISK"
    part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")

    # Perform LUKS encryption on this partition
    # Use the volume group name as the LUKS device name for consistency
    local luks_name="$VG_NAME"
    encrypt_device "$part_dev" "$luks_name"

    # 5. Setup LVM on the encrypted device
    setup_lvm "/dev/mapper/$luks_name" "$VG_NAME"
    # LVs (lv_root, lv_swap, lv_home) are created, formatted, and mounted inside setup_lvm.

    log_info "LUKS+LVM partitioning complete. Filesystems formatted and mounted."
}

do_auto_raid_luks_lvm_partitioning() {
    echo "=== PHASE 1: Disk Partitioning (RAID+LUKS+LVM) ==="
    log_info "Starting auto RAID+LUKS+LVM partitioning with disks: ${RAID_DEVICES[*]} (Boot Mode: $BOOT_MODE)..."

    # Set default filesystem types if not specified
    ROOT_FILESYSTEM_TYPE="${ROOT_FILESYSTEM_TYPE:-ext4}"
    HOME_FILESYSTEM_TYPE="${HOME_FILESYSTEM_TYPE:-ext4}"

    if [ ${#RAID_DEVICES[@]} -lt 2 ]; then error_exit "RAID requires at least 2 disks."; fi

    local efi_part_num=1
    local boot_part_num=2
    local luks_part_num=3

    # --- Phase 1: Partition all RAID member disks identically ---
    for disk in "${RAID_DEVICES[@]}"; do
        log_info "Partitioning RAID member disk: $disk"
        wipe_disk "$disk"

        local current_start_mib=1

        sgdisk -Z "$disk" || error_exit "Failed to create GPT label on $disk."
        partprobe "$disk"

        # 1. EFI partition on each disk (if UEFI)
        if [ "$BOOT_MODE" == "uefi" ]; then
            local efi_size_mb="${EFI_PART_SIZE_MIB}M"
            sgdisk -n "$efi_part_num:0:+$efi_size_mb" -t "$efi_part_num:EF00" "$disk" || error_exit "Failed to create EFI partition on $disk."
            current_start_mib=$((current_start_mib + EFI_PART_SIZE_MIB))
        fi

        # 2. /boot partition on each disk (will be part of RAID1 for /boot)
        local boot_size_mb="${BOOT_PART_SIZE_MIB}M"
        sgdisk -n "$boot_part_num:0:+$boot_size_mb" -t "$boot_part_num:8300" "$disk" || error_exit "Failed to create /boot partition on $disk."
        current_start_mib=$((current_start_mib + BOOT_PART_SIZE_MIB))
        
        # 3. Main LUKS Container Partition (takes rest of disk, will be part of RAID1 for LUKS)
        sgdisk -n "$luks_part_num:0:0" -t "$luks_part_num:8300" "$disk" || error_exit "Failed to create LUKS container partition on $disk."
        partprobe "$disk"
    done

    # --- Phase 2: Assemble RAID Arrays ---
    # Create RAID1 for /boot
    local md_boot_dev="/dev/md0"
    local boot_component_devices=()
    for disk in "${RAID_DEVICES[@]}"; do
mapfile -t         boot_component_devices+ < <(get_partition_path "$disk" "$boot_part_num")
    done
    setup_raid "$RAID_LEVEL" "$md_boot_dev" "${boot_component_devices[@]}" || error_exit "RAID setup for /boot failed."
    format_filesystem "$md_boot_dev" "ext4"
    capture_id_for_config "boot" "$md_boot_dev" "UUID"
    mkdir -p /mnt/boot || error_exit "Failed to create /mnt/boot."
    safe_mount "$md_boot_dev" "/mnt/boot"


    # Create RAID1 for LUKS container
    local md_luks_container="/dev/md1"
    local luks_component_devices=()
    for disk in "${RAID_DEVICES[@]}"; do
mapfile -t         luks_component_devices+ < <(get_partition_path "$disk" "$luks_part_num")
    done
    setup_raid "$RAID_LEVEL" "$md_luks_container" "${luks_component_devices[@]}" || error_exit "RAID setup for LUKS container failed."


    # --- Phase 3: Encrypt the RAID device (md_luks_container) ---
    # Use the volume group name as the LUKS device name for consistency
    local luks_name="$VG_NAME"
    encrypt_device "$md_luks_container" "$luks_name"


    # --- Phase 4: Setup LVM on the encrypted RAID device ---
    setup_lvm "/dev/mapper/$luks_name" "$VG_NAME"


    # --- Phase 5: Mount EFI(s) for initial install ---
    # For UEFI, mount the EFI partition from the *first* RAID disk.
    if [ "$BOOT_MODE" == "uefi" ]; then
        local first_efi_dev=$(get_partition_path "${RAID_DEVICES[0]}" "$efi_part_num")
        format_filesystem "$first_efi_dev" "vfat"
        capture_id_for_config "efi" "$first_efi_dev" "UUID"
        capture_id_for_config "efi" "$first_efi_dev" "PARTUUID"
        mkdir -p /mnt/boot/efi || error_exit "Failed to create /mnt/boot/efi."
        safe_mount "$first_efi_dev" "/mnt/boot/efi"
    fi

    log_info "RAID+LUKS+LVM partitioning complete."
}

do_auto_simple_luks_partitioning() {
    echo "=== PHASE 1: Disk Partitioning with LUKS Encryption ==="
    log_info "Starting auto simple LUKS partitioning for $INSTALL_DISK (Boot Mode: $BOOT_MODE)..."

    wipe_disk "$INSTALL_DISK"

    local current_start_mib=1
    local part_num=1

    # Create partition table (GPT for UEFI, MBR for BIOS)
    if [ "$BOOT_MODE" == "uefi" ]; then
        sgdisk -Z "$INSTALL_DISK" || error_exit "Failed to create GPT label on $INSTALL_DISK."
    else
        printf "o\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create MBR label on $INSTALL_DISK."
    fi
    partprobe "$INSTALL_DISK"

    # EFI Partition (for UEFI)
    if [ "$BOOT_MODE" == "uefi" ]; then
        log_info "Creating EFI partition (${EFI_PART_SIZE_MIB}MiB)..."
        local efi_size_mb="${EFI_PART_SIZE_MIB}M"
        sgdisk -n "$part_num:0:+$efi_size_mb" -t "$part_num:$EFI_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create EFI partition."
        partprobe "$INSTALL_DISK"
        local efi_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        mkfs.fat -F32 "$efi_dev" || error_exit "Failed to format EFI partition."
        PARTITION_UUIDS_EFI_UUID=$(blkid -s UUID -o value "$efi_dev")
        PARTITION_UUIDS_EFI_PARTUUID=$(blkid -s PARTUUID -o value "$efi_dev")
        log_info "EFI partition UUID: $PARTITION_UUIDS_EFI_UUID"
        part_num=$((part_num + 1))
    fi

    # Boot Partition (for BIOS or additional boot partition)
    if [ "$BOOT_MODE" == "bios" ] || [ "$WANT_SEPARATE_BOOT" == "yes" ]; then
        log_info "Creating Boot partition (${BOOT_PART_SIZE_MIB}MiB)..."
        local boot_size_mb="${BOOT_PART_SIZE_MIB}M"
        if [ "$BOOT_MODE" == "uefi" ]; then
            sgdisk -n "$part_num:0:+$boot_size_mb" -t "$part_num:8300" "$INSTALL_DISK" || error_exit "Failed to create boot partition."
        else
            printf "n\np\n$part_num\n\n+${BOOT_PART_SIZE_MIB}M\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create boot partition."
        fi
        partprobe "$INSTALL_DISK"
        local boot_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        mkfs.ext4 "$boot_dev" || error_exit "Failed to format boot partition."
        PARTITION_UUIDS_BOOT_UUID=$(blkid -s UUID -o value "$boot_dev")
        log_info "Boot partition UUID: $PARTITION_UUIDS_BOOT_UUID"
        part_num=$((part_num + 1))
    fi

    # Create LUKS container partition (rest of disk)
    log_info "Creating LUKS container partition (rest of disk)..."
    if [ "$BOOT_MODE" == "uefi" ]; then
        sgdisk -n "$part_num:0:0" -t "$part_num:8309" "$INSTALL_DISK" || error_exit "Failed to create LUKS container partition."
    else
        printf "n\np\n$part_num\n\n\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create LUKS container partition."
    fi
    partprobe "$INSTALL_DISK"
    local luks_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
    
    # Set up LUKS encryption
    log_info "Setting up LUKS encryption on $luks_dev..."
    cryptsetup luksFormat --type luks2 "$luks_dev" || error_exit "Failed to create LUKS container."
    cryptsetup open "$luks_dev" cryptroot || error_exit "Failed to open LUKS container."
    
    # Get LUKS container UUID
    PARTITION_UUIDS_LUKS_CONTAINER_UUID=$(blkid -s UUID -o value "$luks_dev")
    log_info "LUKS container UUID: $PARTITION_UUIDS_LUKS_CONTAINER_UUID"
    
    # Create filesystem on decrypted device
    local root_fs="${ROOT_FILESYSTEM:-ext4}"
    log_info "Creating $root_fs filesystem on /dev/mapper/cryptroot..."
    case "$root_fs" in
        "ext4") mkfs.ext4 /dev/mapper/cryptroot || error_exit "Failed to create ext4 filesystem." ;;
        "btrfs") mkfs.btrfs /dev/mapper/cryptroot || error_exit "Failed to create btrfs filesystem." ;;
        "xfs") mkfs.xfs /dev/mapper/cryptroot || error_exit "Failed to create xfs filesystem." ;;
        *) error_exit "Unsupported filesystem: $root_fs" ;;
    esac
    
    PARTITION_UUIDS_ROOT_UUID=$(blkid -s UUID -o value /dev/mapper/cryptroot)
    log_info "Root filesystem UUID: $PARTITION_UUIDS_ROOT_UUID"
    
    log_info "Auto simple LUKS partitioning completed successfully."
}

do_auto_lvm_partitioning() {
    echo "=== PHASE 1: Disk Partitioning with LVM ==="
    log_info "Starting auto LVM partitioning for $INSTALL_DISK (Boot Mode: $BOOT_MODE)..."

    wipe_disk "$INSTALL_DISK"

    local current_start_mib=1
    local part_num=1

    # Create partition table (GPT for UEFI, MBR for BIOS)
    if [ "$BOOT_MODE" == "uefi" ]; then
        sgdisk -Z "$INSTALL_DISK" || error_exit "Failed to create GPT label on $INSTALL_DISK."
    else
        printf "o\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create MBR label on $INSTALL_DISK."
    fi
    partprobe "$INSTALL_DISK"

    # EFI Partition (for UEFI)
    if [ "$BOOT_MODE" == "uefi" ]; then
        log_info "Creating EFI partition (${EFI_PART_SIZE_MIB}MiB)..."
        local efi_size_mb="${EFI_PART_SIZE_MIB}M"
        sgdisk -n "$part_num:0:+$efi_size_mb" -t "$part_num:$EFI_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create EFI partition."
        partprobe "$INSTALL_DISK"
        local efi_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        mkfs.fat -F32 "$efi_dev" || error_exit "Failed to format EFI partition."
        PARTITION_UUIDS_EFI_UUID=$(blkid -s UUID -o value "$efi_dev")
        PARTITION_UUIDS_EFI_PARTUUID=$(blkid -s PARTUUID -o value "$efi_dev")
        log_info "EFI partition UUID: $PARTITION_UUIDS_EFI_UUID"
        part_num=$((part_num + 1))
    fi

    # Boot Partition (for BIOS or additional boot partition)
    if [ "$BOOT_MODE" == "bios" ] || [ "$WANT_SEPARATE_BOOT" == "yes" ]; then
        log_info "Creating Boot partition (${BOOT_PART_SIZE_MIB}MiB)..."
        local boot_size_mb="${BOOT_PART_SIZE_MIB}M"
        if [ "$BOOT_MODE" == "uefi" ]; then
            sgdisk -n "$part_num:0:+$boot_size_mb" -t "$part_num:8300" "$INSTALL_DISK" || error_exit "Failed to create boot partition."
        else
            printf "n\np\n$part_num\n\n+${BOOT_PART_SIZE_MIB}M\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create boot partition."
        fi
        partprobe "$INSTALL_DISK"
        local boot_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        mkfs.ext4 "$boot_dev" || error_exit "Failed to format boot partition."
        PARTITION_UUIDS_BOOT_UUID=$(blkid -s UUID -o value "$boot_dev")
        log_info "Boot partition UUID: $PARTITION_UUIDS_BOOT_UUID"
        part_num=$((part_num + 1))
    fi

    # Create LVM physical volume partition (rest of disk)
    log_info "Creating LVM physical volume partition (rest of disk)..."
    if [ "$BOOT_MODE" == "uefi" ]; then
        sgdisk -n "$part_num:0:0" -t "$part_num:8E00" "$INSTALL_DISK" || error_exit "Failed to create LVM partition."
    else
        printf "n\np\n$part_num\n\n\nt\n$part_num\n8e\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create LVM partition."
    fi
    partprobe "$INSTALL_DISK"
    local pv_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
    
    # Set up LVM
    log_info "Setting up LVM on $pv_dev..."
    pvcreate "$pv_dev" || error_exit "Failed to create physical volume."
    vgcreate "$VG_NAME" "$pv_dev" || error_exit "Failed to create volume group."
    
    # Create logical volumes
    local root_size="${ROOT_SIZE_MIB:-102400}M"
    log_info "Creating root logical volume (${root_size})..."
    lvcreate -L "$root_size" -n root "$VG_NAME" || error_exit "Failed to create root logical volume."
    
    if [ "$WANT_SWAP" == "yes" ]; then
        local swap_size="${SWAP_SIZE_MIB:-2048}M"
        log_info "Creating swap logical volume (${swap_size})..."
        lvcreate -L "$swap_size" -n swap "$VG_NAME" || error_exit "Failed to create swap logical volume."
        mkswap "/dev/$VG_NAME/swap" || error_exit "Failed to create swap."
        PARTITION_UUIDS_SWAP_UUID=$(blkid -s UUID -o value "/dev/$VG_NAME/swap")
        log_info "Swap logical volume UUID: $PARTITION_UUIDS_SWAP_UUID"
    fi
    
    if [ "$WANT_HOME_PARTITION" == "yes" ]; then
        log_info "Creating home logical volume (rest of space)..."
        lvcreate -l 100%FREE -n home "$VG_NAME" || error_exit "Failed to create home logical volume."
    fi
    
    # Create filesystem on root logical volume
    local root_fs="${ROOT_FILESYSTEM:-ext4}"
    log_info "Creating $root_fs filesystem on /dev/$VG_NAME/root..."
    case "$root_fs" in
        "ext4") mkfs.ext4 "/dev/$VG_NAME/root" || error_exit "Failed to create ext4 filesystem." ;;
        "btrfs") mkfs.btrfs "/dev/$VG_NAME/root" || error_exit "Failed to create btrfs filesystem." ;;
        "xfs") mkfs.xfs "/dev/$VG_NAME/root" || error_exit "Failed to create xfs filesystem." ;;
        *) error_exit "Unsupported filesystem: $root_fs" ;;
    esac
    
    PARTITION_UUIDS_LV_ROOT_UUID=$(blkid -s UUID -o value "/dev/$VG_NAME/root")
    log_info "Root logical volume UUID: $PARTITION_UUIDS_LV_ROOT_UUID"
    
    if [ "$WANT_HOME_PARTITION" == "yes" ]; then
        local home_fs="${HOME_FILESYSTEM:-ext4}"
        log_info "Creating $home_fs filesystem on /dev/$VG_NAME/home..."
        case "$home_fs" in
            "ext4") mkfs.ext4 "/dev/$VG_NAME/home" || error_exit "Failed to create ext4 filesystem." ;;
            "btrfs") mkfs.btrfs "/dev/$VG_NAME/home" || error_exit "Failed to create btrfs filesystem." ;;
            "xfs") mkfs.xfs "/dev/$VG_NAME/home" || error_exit "Failed to create xfs filesystem." ;;
            *) error_exit "Unsupported filesystem: $home_fs" ;;
        esac
        
        PARTITION_UUIDS_LV_HOME_UUID=$(blkid -s UUID -o value "/dev/$VG_NAME/home")
        log_info "Home logical volume UUID: $PARTITION_UUIDS_LV_HOME_UUID"
    fi
    
    log_info "Auto LVM partitioning completed successfully."
}

do_auto_raid_simple_partitioning() {
    echo "=== PHASE 1: Disk Partitioning with Software RAID ==="
    log_info "Starting auto RAID simple partitioning for multiple disks (RAID Level: ${RAID_LEVEL:-raid1})..."

    # Validate RAID requirements
    if [ ${#RAID_DEVICES[@]} -lt 2 ]; then 
        error_exit "RAID requires at least 2 disks. Current disks: ${RAID_DEVICES[*]}"
    fi
    
    local raid_level="${RAID_LEVEL:-raid1}"
    log_info "RAID level: $raid_level"
    log_info "RAID devices: ${RAID_DEVICES[*]}"
    
    # Set default filesystem types if not specified
    ROOT_FILESYSTEM_TYPE="${ROOT_FILESYSTEM_TYPE:-ext4}"
    HOME_FILESYSTEM_TYPE="${HOME_FILESYSTEM_TYPE:-ext4}"

    local efi_part_num=1
    local boot_part_num=2
    local root_part_num=3
    local home_part_num=4

    # --- Phase 1: Partition all RAID member disks identically ---
    for disk in "${RAID_DEVICES[@]}"; do
        log_info "Partitioning RAID member disk: $disk"
        wipe_disk "$disk"

        local current_start_mib=1

        sgdisk -Z "$disk" || error_exit "Failed to create GPT label on $disk."
        partprobe "$disk"

        # 1. EFI partition on each disk (if UEFI)
        if [ "$BOOT_MODE" == "uefi" ]; then
            local efi_size_mb="${EFI_PART_SIZE_MIB}M"
            sgdisk -n "$efi_part_num:0:+$efi_size_mb" -t "$efi_part_num:EF00" "$disk" || error_exit "Failed to create EFI partition on $disk."
            current_start_mib=$((current_start_mib + EFI_PART_SIZE_MIB))
        fi

        # 2. /boot partition on each disk (will be part of RAID)
        local boot_size_mb="${BOOT_PART_SIZE_MIB}M"
        sgdisk -n "$boot_part_num:0:+$boot_size_mb" -t "$boot_part_num:8300" "$disk" || error_exit "Failed to create /boot partition on $disk."
        current_start_mib=$((current_start_mib + BOOT_PART_SIZE_MIB))
        
        # 3. Root partition on each disk (will be part of RAID)
        local root_size_mib=102400  # 100GB
        sgdisk -n "$root_part_num:0:+${root_size_mib}M" -t "$root_part_num:8300" "$disk" || error_exit "Failed to create root partition on $disk."
        current_start_mib=$((current_start_mib + root_size_mib))
        
        # 4. Home partition on each disk (will be part of RAID, takes rest of disk)
        if [ "$WANT_HOME_PARTITION" == "yes" ]; then
            sgdisk -n "$home_part_num:0:0" -t "$home_part_num:8300" "$disk" || error_exit "Failed to create home partition on $disk."
        fi
        
        partprobe "$disk"
    done

    # --- Phase 2: Assemble RAID Arrays ---
    # Create RAID for /boot
    local md_boot_dev="/dev/md0"
    local boot_component_devices=()
    for disk in "${RAID_DEVICES[@]}"; do
mapfile -t         boot_component_devices+ < <(get_partition_path "$disk" "$boot_part_num")
    done
    setup_raid "$raid_level" "$md_boot_dev" "${boot_component_devices[@]}" || error_exit "RAID setup for /boot failed."
    format_filesystem "$md_boot_dev" "ext4"
    capture_id_for_config "boot" "$md_boot_dev" "UUID"
    mkdir -p /mnt/boot || error_exit "Failed to create /mnt/boot."
    safe_mount "$md_boot_dev" "/mnt/boot"

    # Create RAID for root
    local md_root_dev="/dev/md1"
    local root_component_devices=()
    for disk in "${RAID_DEVICES[@]}"; do
mapfile -t         root_component_devices+ < <(get_partition_path "$disk" "$root_part_num")
    done
    setup_raid "$raid_level" "$md_root_dev" "${root_component_devices[@]}" || error_exit "RAID setup for root failed."
    format_filesystem "$md_root_dev" "$ROOT_FILESYSTEM_TYPE"
    capture_id_for_config "root" "$md_root_dev" "UUID"
    safe_mount "$md_root_dev" "/mnt"

    # Create RAID for home (if requested)
    if [ "$WANT_HOME_PARTITION" == "yes" ]; then
        local md_home_dev="/dev/md2"
        local home_component_devices=()
        for disk in "${RAID_DEVICES[@]}"; do
mapfile -t             home_component_devices+ < <(get_partition_path "$disk" "$home_part_num")
        done
        setup_raid "$raid_level" "$md_home_dev" "${home_component_devices[@]}" || error_exit "RAID setup for home failed."
        format_filesystem "$md_home_dev" "$HOME_FILESYSTEM_TYPE"
        capture_id_for_config "home" "$md_home_dev" "UUID"
        mkdir -p /mnt/home || error_exit "Failed to create /mnt/home."
        safe_mount "$md_home_dev" "/mnt/home"
    fi

    # --- Phase 3: Mount EFI for initial install ---
    if [ "$BOOT_MODE" == "uefi" ]; then
        local first_efi_dev=$(get_partition_path "${RAID_DEVICES[0]}" "$efi_part_num")
        format_filesystem "$first_efi_dev" "vfat"
        capture_id_for_config "efi" "$first_efi_dev" "UUID"
        capture_id_for_config "efi" "$first_efi_dev" "PARTUUID"
        mkdir -p /mnt/boot/efi || error_exit "Failed to create /mnt/boot/efi."
        safe_mount "$first_efi_dev" "/mnt/boot/efi"
    fi

    log_info "Auto RAID simple partitioning completed successfully."
}

do_auto_raid_lvm_partitioning() {
    echo "=== PHASE 1: Disk Partitioning with Software RAID and LVM ==="
    log_info "Starting auto RAID LVM partitioning for multiple disks (RAID Level: ${RAID_LEVEL:-raid1})..."

    # Validate RAID requirements
    if [ ${#RAID_DEVICES[@]} -lt 2 ]; then 
        error_exit "RAID requires at least 2 disks. Current disks: ${RAID_DEVICES[*]}"
    fi
    
    local raid_level="${RAID_LEVEL:-raid1}"
    log_info "RAID level: $raid_level"
    log_info "RAID devices: ${RAID_DEVICES[*]}"
    
    # Set default filesystem types if not specified
    ROOT_FILESYSTEM_TYPE="${ROOT_FILESYSTEM_TYPE:-ext4}"
    HOME_FILESYSTEM_TYPE="${HOME_FILESYSTEM_TYPE:-ext4}"

    local efi_part_num=1
    local boot_part_num=2
    local lvm_part_num=3

    # --- Phase 1: Partition all RAID member disks identically ---
    for disk in "${RAID_DEVICES[@]}"; do
        log_info "Partitioning RAID member disk: $disk"
        wipe_disk "$disk"

        local current_start_mib=1

        sgdisk -Z "$disk" || error_exit "Failed to create GPT label on $disk."
        partprobe "$disk"

        # 1. EFI partition on each disk (if UEFI)
        if [ "$BOOT_MODE" == "uefi" ]; then
            local efi_size_mb="${EFI_PART_SIZE_MIB}M"
            sgdisk -n "$efi_part_num:0:+$efi_size_mb" -t "$efi_part_num:EF00" "$disk" || error_exit "Failed to create EFI partition on $disk."
            current_start_mib=$((current_start_mib + EFI_PART_SIZE_MIB))
        fi

        # 2. /boot partition on each disk (will be part of RAID)
        local boot_size_mb="${BOOT_PART_SIZE_MIB}M"
        sgdisk -n "$boot_part_num:0:+$boot_size_mb" -t "$boot_part_num:8300" "$disk" || error_exit "Failed to create /boot partition on $disk."
        current_start_mib=$((current_start_mib + BOOT_PART_SIZE_MIB))
        
        # 3. LVM partition on each disk (will be part of RAID, takes rest of disk)
        sgdisk -n "$lvm_part_num:0:0" -t "$lvm_part_num:8E00" "$disk" || error_exit "Failed to create LVM partition on $disk."
        partprobe "$disk"
    done

    # --- Phase 2: Assemble RAID Arrays ---
    # Create RAID for /boot
    local md_boot_dev="/dev/md0"
    local boot_component_devices=()
    for disk in "${RAID_DEVICES[@]}"; do
mapfile -t         boot_component_devices+ < <(get_partition_path "$disk" "$boot_part_num")
    done
    setup_raid "$raid_level" "$md_boot_dev" "${boot_component_devices[@]}" || error_exit "RAID setup for /boot failed."
    format_filesystem "$md_boot_dev" "ext4"
    capture_id_for_config "boot" "$md_boot_dev" "UUID"
    mkdir -p /mnt/boot || error_exit "Failed to create /mnt/boot."
    safe_mount "$md_boot_dev" "/mnt/boot"

    # Create RAID for LVM container
    local md_lvm_dev="/dev/md1"
    local lvm_component_devices=()
    for disk in "${RAID_DEVICES[@]}"; do
mapfile -t         lvm_component_devices+ < <(get_partition_path "$disk" "$lvm_part_num")
    done
    setup_raid "$raid_level" "$md_lvm_dev" "${lvm_component_devices[@]}" || error_exit "RAID setup for LVM container failed."

    # --- Phase 3: Setup LVM on the RAID device ---
    setup_lvm "$md_lvm_dev" "$VG_NAME"

    # --- Phase 4: Mount EFI for initial install ---
    if [ "$BOOT_MODE" == "uefi" ]; then
        local first_efi_dev=$(get_partition_path "${RAID_DEVICES[0]}" "$efi_part_num")
        format_filesystem "$first_efi_dev" "vfat"
        capture_id_for_config "efi" "$first_efi_dev" "UUID"
        capture_id_for_config "efi" "$first_efi_dev" "PARTUUID"
        mkdir -p /mnt/boot/efi || error_exit "Failed to create /mnt/boot/efi."
        safe_mount "$first_efi_dev" "/mnt/boot/efi"
    fi

    log_info "Auto RAID LVM partitioning completed successfully."
}

do_manual_partitioning_guided() {
    log_info "Starting manual partitioning using fdisk for $INSTALL_DISK"
    
    # Check if fdisk is available
    if ! command -v fdisk &>/dev/null; then
        error_exit "fdisk is not available. Cannot proceed with manual partitioning."
    fi
    
    log_info "Launching fdisk for manual partitioning of $INSTALL_DISK"
    log_warn "Manual partitioning instructions:"
    log_warn "1. Create partitions as needed (root partition is required)"
    log_warn "2. If using UEFI, create an EFI System Partition (type EF00)"
    log_warn "3. Set the root partition type to Linux (type 8300)"
    log_warn "4. Write changes with 'w' and quit fdisk"
    log_warn "5. The script will then ask you to format and mount partitions"
    
    # Launch fdisk
    fdisk "$INSTALL_DISK" || error_exit "fdisk failed or was cancelled"
    
    log_info "fdisk completed. Now you need to format and mount partitions."
    log_warn "You must create and mount the root filesystem at '/mnt'."
    log_warn "If using UEFI, create and mount the EFI System Partition at '/mnt/boot/efi'."
    log_warn "If using LVM, LUKS, or RAID, ensure they are opened/assembled before mounting."

    read -rp "Press Enter when you have finished formatting and mounting to /mnt (and /mnt/boot/efi): "

    # Verify essential mounts
    if ! mountpoint -q /mnt; then
        error_exit "/mnt is not mounted. Please ensure your root partition is mounted correctly."
    fi
    if [ "$BOOT_MODE" == "uefi" ] && ! mountpoint -q /mnt/boot/efi; then
        log_warn "/mnt/boot/efi is not mounted. This is required for UEFI installations. Please mount it manually."
        read -rp "Press Enter to continue after mounting /mnt/boot/efi: "
        if ! mountpoint -q /mnt/boot/efi; then
            error_exit "/mnt/boot/efi is still not mounted. Cannot proceed with UEFI installation."
        fi
    fi

    log_info "Attempting to gather UUIDs from manually mounted partitions for fstab and bootloader..."
    local mounted_devs_info=$(findmnt -n --raw --output SOURCE,TARGET /mnt -R)

    # Process root
    local root_dev=$(echo "$mounted_devs_info" | awk '$2=="/mnt"{print $1}')
    if [ -n "$root_dev" ]; then
        capture_id_for_config "root" "$root_dev" "UUID"
        if [[ "$root_dev" =~ ^/dev/mapper/ ]]; then
            local lv_name=$(basename "$root_dev")
            local vg_name=$(basename "$(dirname "$root_dev")")
            LVM_DEVICES_MAP["${vg_name}_${lv_name}"]="$root_dev"
        fi
    else
        log_warn "Could not determine root device for UUID capture after manual partitioning."
    fi

    # Process EFI
    if [ "$BOOT_MODE" == "uefi" ]; then
        local efi_dev=$(echo "$mounted_devs_info" | awk '$2=="/mnt/boot/efi"{print $1}')
        if [ -n "$efi_dev" ]; then
            capture_id_for_config "efi" "$efi_dev" "UUID"
            capture_id_for_config "efi" "$efi_dev" "PARTUUID"
        else
            log_warn "Could not determine EFI device for UUID/PARTUUID capture after manual partitioning."
        fi
    fi

    # Process /boot (if separate)
    local boot_dev=$(echo "$mounted_devs_info" | awk '$2=="/mnt/boot" && $1!="/mnt"{print $1}')
    if [ -n "$boot_dev" ]; then
        capture_id_for_config "boot" "$boot_dev" "UUID"
    fi

    # Process /home (if separate)
    local home_dev=$(echo "$mounted_devs_info" | awk '$2=="/mnt/home"{print $1}')
    if [ -n "$home_dev" ]; then
        capture_id_for_config "home" "$home_dev" "UUID"
        if [[ "$home_dev" =~ ^/dev/mapper/ ]]; then
            local lv_name=$(basename "$home_dev")
            local vg_name=$(basename "$(dirname "$home_dev")")
            LVM_DEVICES_MAP["${vg_name}_${lv_name}"]="$home_dev"
        fi
    fi

    log_warn "Automatic LUKS/RAID/LVM detection for GRUB kernel parameters in manual mode is limited."
    log_warn "Please ensure your GRUB configuration (cryptdevice, rd.lvm.vg) is correct post-install if using complex setups."

    log_info "UUID capture for manual setup attempted. Please verify fstab and bootloader configs post-install."
}

# --- ESP + XBOOTLDR Partitioning Functions ---
# These functions implement the /efi + /boot approach recommended for dual-boot scenarios

do_auto_simple_partitioning_efi_xbootldr() {
    echo "=== PHASE 1: Disk Partitioning (ESP + XBOOTLDR) ==="
    log_info "Starting auto simple partitioning with ESP + XBOOTLDR for $INSTALL_DISK (Boot Mode: $BOOT_MODE)..."

    wipe_disk "$INSTALL_DISK"

    local current_start_mib=1 # Always start at 1MiB for the first partition
    local part_num=1 # Keep track of partition numbers
    local part_dev=""

    # Create partition table (GPT for UEFI, MBR for BIOS)
    if [ "$BOOT_MODE" == "uefi" ]; then
        sgdisk -Z "$INSTALL_DISK" || error_exit "Failed to create GPT label on $INSTALL_DISK."
    else
        # For BIOS, we'll use fdisk to create MBR
        printf "o\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create MBR label on $INSTALL_DISK."
    fi
    partprobe "$INSTALL_DISK"

    # ESP Partition (for UEFI) - mounted to /efi
    if [ "$BOOT_MODE" == "uefi" ]; then
        log_info "Creating ESP partition (${EFI_PART_SIZE_MIB}MiB) for /efi..."
        local efi_size_mb="${EFI_PART_SIZE_MIB}M"
        sgdisk -n "$part_num:0:+$efi_size_mb" -t "$part_num:$EFI_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create ESP partition."
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        
        format_filesystem "$part_dev" "$EFI_FILESYSTEM"  # vfat
        capture_id_for_config "efi" "$part_dev" "UUID"
        capture_id_for_config "efi" "$part_dev" "PARTUUID"
        safe_mount "$part_dev" "/mnt/efi"
        current_start_mib=$((current_start_mib + EFI_PART_SIZE_MIB))
        part_num=$((part_num + 1))

        # XBOOTLDR Partition - mounted to /boot
        log_info "Creating XBOOTLDR partition (${XBOOTLDR_PART_SIZE_MIB}MiB) for /boot..."
        local xbootldr_size_mb="${XBOOTLDR_PART_SIZE_MIB}M"
        sgdisk -n "$part_num:0:+$xbootldr_size_mb" -t "$part_num:$XBOOTLDR_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create XBOOTLDR partition."
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        
        format_filesystem "$part_dev" "$BOOT_FILESYSTEM"  # ext4
        capture_id_for_config "boot" "$part_dev" "UUID"
        safe_mount "$part_dev" "/mnt/boot"
        current_start_mib=$((current_start_mib + XBOOTLDR_PART_SIZE_MIB))
        part_num=$((part_num + 1))
    fi

    # Swap Partition (if desired)
    if [ "$WANT_SWAP" == "yes" ]; then
        log_info "Creating Swap partition..."
        local swap_size_mib=$DEFAULT_SWAP_SIZE_MIB
        local swap_size_mb="${swap_size_mib}M"
        if [ "$BOOT_MODE" == "uefi" ]; then
            sgdisk -n "$part_num:0:+$swap_size_mb" -t "$part_num:$SWAP_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create swap partition."
        else
            printf "n\np\n$part_num\n\n+${swap_size_mib}M\nt\n$part_num\n82\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create swap partition."
        fi
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        format_filesystem "$part_dev" "swap"
        capture_id_for_config "swap" "$part_dev" "UUID"
        swapon "$part_dev" || error_exit "Failed to activate swap on $part_dev."
        current_start_mib=$((current_start_mib + swap_size_mib))
        part_num=$((part_num + 1))
    fi

    # Root Partition and Optional Home Partition
    local root_size_mib=$DEFAULT_ROOT_SIZE_MIB
    
    # Set default filesystem types if not specified
    ROOT_FILESYSTEM_TYPE="${ROOT_FILESYSTEM_TYPE:-ext4}"
    HOME_FILESYSTEM_TYPE="${HOME_FILESYSTEM_TYPE:-ext4}"

    if [ "$WANT_HOME_PARTITION" == "yes" ]; then
        log_info "Creating Root partition and separate Home partition (rest of disk)..."
        # Root partition (fixed size)
        local root_size_mb="${root_size_mib}M"
        if [ "$BOOT_MODE" == "uefi" ]; then
            sgdisk -n "$part_num:0:+$root_size_mb" -t "$part_num:8300" "$INSTALL_DISK" || error_exit "Failed to create root partition."
        else
            printf "n\np\n$part_num\n\n+${root_size_mib}M\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create root partition."
        fi
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        format_filesystem "$part_dev" "$ROOT_FILESYSTEM_TYPE"
        capture_id_for_config "root" "$part_dev" "UUID"
        safe_mount "$part_dev" "/mnt"
        
        # Create Btrfs subvolumes if using Btrfs
        if [ "$ROOT_FILESYSTEM_TYPE" == "btrfs" ]; then
            create_btrfs_subvolumes "/mnt"
        fi
        current_start_mib=$((current_start_mib + root_size_mib))
        part_num=$((part_num + 1))

        # Home partition (takes remaining space)
        log_info "Creating Home partition (rest of disk)..."
        if [ "$BOOT_MODE" == "uefi" ]; then
            sgdisk -n "$part_num:0:0" -t "$part_num:8300" "$INSTALL_DISK" || error_exit "Failed to create home partition."
        else
            printf "n\np\n$part_num\n\n\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create home partition."
        fi
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        format_filesystem "$part_dev" "$HOME_FILESYSTEM_TYPE"
        capture_id_for_config "home" "$part_dev" "UUID"
        mkdir -p /mnt/home || error_exit "Failed to create /mnt/home."
        safe_mount "$part_dev" "/mnt/home"
    else
        # Root takes all remaining space
        log_info "Creating Root partition (rest of disk)..."
        if [ "$BOOT_MODE" == "uefi" ]; then
            sgdisk -n "$part_num:0:0" -t "$part_num:8300" "$INSTALL_DISK" || error_exit "Failed to create root partition."
        else
            printf "n\np\n$part_num\n\n\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create root partition."
        fi
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        format_filesystem "$part_dev" "$ROOT_FILESYSTEM_TYPE"
        capture_id_for_config "root" "$part_dev" "UUID"
        safe_mount "$part_dev" "/mnt"
        
        # Create Btrfs subvolumes if using Btrfs
        if [ "$ROOT_FILESYSTEM_TYPE" == "btrfs" ]; then
            create_btrfs_subvolumes "/mnt"
        fi
    fi

    log_info "Simple auto partitioning with ESP + XBOOTLDR complete. Filesystems formatted and mounted."
}

do_auto_lvm_partitioning_efi_xbootldr() {
    echo "=== PHASE 1: Disk Partitioning with LVM (ESP + XBOOTLDR) ==="
    log_info "Starting auto LVM partitioning with ESP + XBOOTLDR for $INSTALL_DISK (Boot Mode: $BOOT_MODE)..."

    wipe_disk "$INSTALL_DISK"

    local current_start_mib=1
    local part_num=1
    local part_dev=""

    # Create partition table (GPT for UEFI, MBR for BIOS)
    if [ "$BOOT_MODE" == "uefi" ]; then
        sgdisk -Z "$INSTALL_DISK" || error_exit "Failed to create GPT label on $INSTALL_DISK."
    else
        printf "o\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create MBR label on $INSTALL_DISK."
    fi
    partprobe "$INSTALL_DISK"

    # ESP Partition (for UEFI) - mounted to /efi
    if [ "$BOOT_MODE" == "uefi" ]; then
        log_info "Creating ESP partition (${EFI_PART_SIZE_MIB}MiB) for /efi..."
        local efi_size_mb="${EFI_PART_SIZE_MIB}M"
        sgdisk -n "$part_num:0:+$efi_size_mb" -t "$part_num:$EFI_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create ESP partition."
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        
        format_filesystem "$part_dev" "$EFI_FILESYSTEM"  # vfat
        capture_id_for_config "efi" "$part_dev" "UUID"
        capture_id_for_config "efi" "$part_dev" "PARTUUID"
        safe_mount "$part_dev" "/mnt/efi"
        current_start_mib=$((current_start_mib + EFI_PART_SIZE_MIB))
        part_num=$((part_num + 1))

        # XBOOTLDR Partition - mounted to /boot
        log_info "Creating XBOOTLDR partition (${XBOOTLDR_PART_SIZE_MIB}MiB) for /boot..."
        local xbootldr_size_mb="${XBOOTLDR_PART_SIZE_MIB}M"
        sgdisk -n "$part_num:0:+$xbootldr_size_mb" -t "$part_num:$XBOOTLDR_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create XBOOTLDR partition."
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        
        format_filesystem "$part_dev" "$BOOT_FILESYSTEM"  # ext4
        capture_id_for_config "boot" "$part_dev" "UUID"
        safe_mount "$part_dev" "/mnt/boot"
        current_start_mib=$((current_start_mib + XBOOTLDR_PART_SIZE_MIB))
        part_num=$((part_num + 1))
    fi

    # Swap Partition (if desired)
    if [ "$WANT_SWAP" == "yes" ]; then
        log_info "Creating Swap partition..."
        local swap_size_mib=$DEFAULT_SWAP_SIZE_MIB
        local swap_size_mb="${swap_size_mib}M"
        if [ "$BOOT_MODE" == "uefi" ]; then
            sgdisk -n "$part_num:0:+$swap_size_mb" -t "$part_num:$SWAP_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create swap partition."
        else
            printf "n\np\n$part_num\n\n+${swap_size_mib}M\nt\n$part_num\n82\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create swap partition."
        fi
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        format_filesystem "$part_dev" "swap"
        capture_id_for_config "swap" "$part_dev" "UUID"
        swapon "$part_dev" || error_exit "Failed to activate swap on $part_dev."
        current_start_mib=$((current_start_mib + swap_size_mib))
        part_num=$((part_num + 1))
    fi

    # Create LVM physical volume partition (rest of disk)
    log_info "Creating LVM physical volume partition (rest of disk)..."
    if [ "$BOOT_MODE" == "uefi" ]; then
        sgdisk -n "$part_num:0:0" -t "$part_num:$LVM_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create LVM partition."
    else
        printf "n\np\n$part_num\n\n\nt\n$part_num\n8e\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create LVM partition."
    fi
    partprobe "$INSTALL_DISK"
    part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
    
    # Create LVM physical volume
    pvcreate "$part_dev" || error_exit "Failed to create LVM physical volume."
    
    # Create volume group
    local vg_name="archvg"
    vgcreate "$vg_name" "$part_dev" || error_exit "Failed to create LVM volume group."
    
    # Set default filesystem types if not specified
    ROOT_FILESYSTEM_TYPE="${ROOT_FILESYSTEM_TYPE:-ext4}"
    HOME_FILESYSTEM_TYPE="${HOME_FILESYSTEM_TYPE:-ext4}"
    
    if [ "$WANT_HOME_PARTITION" == "yes" ]; then
        log_info "Creating Root and Home logical volumes..."
        
        # Root logical volume (100GB)
        local root_size_gb=100
        lvcreate -L "${root_size_gb}G" -n root "$vg_name" || error_exit "Failed to create root logical volume."
        
        # Home logical volume (rest of space)
        lvcreate -l 100%FREE -n home "$vg_name" || error_exit "Failed to create home logical volume."
        
        # Format and mount root
        local root_lv="/dev/$vg_name/root"
        format_filesystem "$root_lv" "$ROOT_FILESYSTEM_TYPE"
        capture_id_for_config "root" "$root_lv" "UUID"
        safe_mount "$root_lv" "/mnt"
        
        # Create Btrfs subvolumes if using Btrfs
        if [ "$ROOT_FILESYSTEM_TYPE" == "btrfs" ]; then
            create_btrfs_subvolumes "/mnt"
        fi
        
        # Format and mount home
        local home_lv="/dev/$vg_name/home"
        format_filesystem "$home_lv" "$HOME_FILESYSTEM_TYPE"
        capture_id_for_config "home" "$home_lv" "UUID"
        mkdir -p /mnt/home || error_exit "Failed to create /mnt/home."
        safe_mount "$home_lv" "/mnt/home"
        
        # Store LVM device mapping
        LVM_DEVICES_MAP["${vg_name}_root"]="$root_lv"
        LVM_DEVICES_MAP["${vg_name}_home"]="$home_lv"
    else
        log_info "Creating Root logical volume (rest of space)..."
        
        # Root logical volume (all remaining space)
        lvcreate -l 100%FREE -n root "$vg_name" || error_exit "Failed to create root logical volume."
        
        # Format and mount root
        local root_lv="/dev/$vg_name/root"
        format_filesystem "$root_lv" "$ROOT_FILESYSTEM_TYPE"
        capture_id_for_config "root" "$root_lv" "UUID"
        safe_mount "$root_lv" "/mnt"
        
        # Create Btrfs subvolumes if using Btrfs
        if [ "$ROOT_FILESYSTEM_TYPE" == "btrfs" ]; then
            create_btrfs_subvolumes "/mnt"
        fi
        
        # Store LVM device mapping
        LVM_DEVICES_MAP["${vg_name}_root"]="$root_lv"
    fi

    log_info "LVM auto partitioning with ESP + XBOOTLDR complete. Filesystems formatted and mounted."
}

do_auto_btrfs_partitioning_efi_xbootldr() {
    echo "=== PHASE 1: Disk Partitioning (ESP + XBOOTLDR) ==="
    log_info "Starting auto partitioning with ESP + XBOOTLDR for $INSTALL_DISK (Boot Mode: $BOOT_MODE)..."

    wipe_disk "$INSTALL_DISK"

    local current_start_mib=1
    local part_num=1
    local part_dev=""

    # Create partition table (GPT for UEFI, MBR for BIOS)
    if [ "$BOOT_MODE" == "uefi" ]; then
        sgdisk -Z "$INSTALL_DISK" || error_exit "Failed to create GPT label on $INSTALL_DISK."
    else
        printf "o\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create MBR label on $INSTALL_DISK."
    fi
    partprobe "$INSTALL_DISK"

    # ESP Partition (for UEFI) - mounted to /efi
    if [ "$BOOT_MODE" == "uefi" ]; then
        log_info "Creating ESP partition (${EFI_PART_SIZE_MIB}MiB) for /efi..."
        local efi_size_mb="${EFI_PART_SIZE_MIB}M"
        sgdisk -n "$part_num:0:+$efi_size_mb" -t "$part_num:$EFI_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create ESP partition."
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        
        format_filesystem "$part_dev" "$EFI_FILESYSTEM"  # vfat
        capture_id_for_config "efi" "$part_dev" "UUID"
        capture_id_for_config "efi" "$part_dev" "PARTUUID"
        safe_mount "$part_dev" "/mnt/efi"
        current_start_mib=$((current_start_mib + EFI_PART_SIZE_MIB))
        part_num=$((part_num + 1))

        # XBOOTLDR Partition - mounted to /boot
        log_info "Creating XBOOTLDR partition (${XBOOTLDR_PART_SIZE_MIB}MiB) for /boot..."
        local xbootldr_size_mb="${XBOOTLDR_PART_SIZE_MIB}M"
        sgdisk -n "$part_num:0:+$xbootldr_size_mb" -t "$part_num:$XBOOTLDR_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create XBOOTLDR partition."
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        
        format_filesystem "$part_dev" "$BOOT_FILESYSTEM"  # ext4
        capture_id_for_config "boot" "$part_dev" "UUID"
        safe_mount "$part_dev" "/mnt/boot"
        current_start_mib=$((current_start_mib + XBOOTLDR_PART_SIZE_MIB))
        part_num=$((part_num + 1))
    fi

    # Swap Partition (if desired)
    if [ "$WANT_SWAP" == "yes" ]; then
        log_info "Creating Swap partition..."
        local swap_size_mib=$DEFAULT_SWAP_SIZE_MIB
        local swap_size_mb="${swap_size_mib}M"
        if [ "$BOOT_MODE" == "uefi" ]; then
            sgdisk -n "$part_num:0:+$swap_size_mb" -t "$part_num:$SWAP_PARTITION_TYPE" "$INSTALL_DISK" || error_exit "Failed to create swap partition."
        else
            printf "n\np\n$part_num\n\n+${swap_size_mib}M\nt\n$part_num\n82\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create swap partition."
        fi
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        format_filesystem "$part_dev" "swap"
        capture_id_for_config "swap" "$part_dev" "UUID"
        swapon "$part_dev" || error_exit "Failed to activate swap on $part_dev."
        current_start_mib=$((current_start_mib + swap_size_mib))
        part_num=$((part_num + 1))
    fi

    # Root Partition and Optional Home Partition
    local root_size_mib=$DEFAULT_ROOT_SIZE_MIB
    
    # Set default filesystem types if not specified (respect user choice)
    ROOT_FILESYSTEM_TYPE="${ROOT_FILESYSTEM_TYPE:-ext4}"
    HOME_FILESYSTEM_TYPE="${HOME_FILESYSTEM_TYPE:-ext4}"

    if [ "$WANT_HOME_PARTITION" == "yes" ]; then
        log_info "Creating Root and Home partitions..."
        # Root partition (fixed size)
        local root_size_mb="${root_size_mib}M"
        if [ "$BOOT_MODE" == "uefi" ]; then
            sgdisk -n "$part_num:0:+$root_size_mb" -t "$part_num:8300" "$INSTALL_DISK" || error_exit "Failed to create root partition."
        else
            printf "n\np\n$part_num\n\n+${root_size_mib}M\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create root partition."
        fi
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        format_filesystem "$part_dev" "$ROOT_FILESYSTEM_TYPE"
        capture_id_for_config "root" "$part_dev" "UUID"
        safe_mount "$part_dev" "/mnt"
        
        # Create Btrfs subvolumes for root
        create_btrfs_subvolumes "/mnt"
        current_start_mib=$((current_start_mib + root_size_mib))
        part_num=$((part_num + 1))

        # Home partition (takes remaining space)
        log_info "Creating Home partition (rest of disk)..."
        if [ "$BOOT_MODE" == "uefi" ]; then
            sgdisk -n "$part_num:0:0" -t "$part_num:8300" "$INSTALL_DISK" || error_exit "Failed to create home partition."
        else
            printf "n\np\n$part_num\n\n\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create home partition."
        fi
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        format_filesystem "$part_dev" "$HOME_FILESYSTEM_TYPE"
        capture_id_for_config "home" "$part_dev" "UUID"
        mkdir -p /mnt/home || error_exit "Failed to create /mnt/home."
        safe_mount "$part_dev" "/mnt/home"
        
        # Create Btrfs subvolumes for home
        create_btrfs_subvolumes "/mnt/home"
    else
        # Root takes all remaining space
        log_info "Creating Root partition (rest of disk)..."
        if [ "$BOOT_MODE" == "uefi" ]; then
            sgdisk -n "$part_num:0:0" -t "$part_num:8300" "$INSTALL_DISK" || error_exit "Failed to create root partition."
        else
            printf "n\np\n$part_num\n\n\nw\n" | fdisk "$INSTALL_DISK" || error_exit "Failed to create root partition."
        fi
        partprobe "$INSTALL_DISK"
        part_dev=$(get_partition_path "$INSTALL_DISK" "$part_num")
        format_filesystem "$part_dev" "$ROOT_FILESYSTEM_TYPE"
        capture_id_for_config "root" "$part_dev" "UUID"
        safe_mount "$part_dev" "/mnt"
        
        # Create Btrfs subvolumes for root
        create_btrfs_subvolumes "/mnt"
    fi

    log_info "Auto partitioning with ESP + XBOOTLDR complete. Filesystems formatted and mounted."
}

# Export constants that might be used by other scripts
export LINUX_PARTITION_TYPE
export LVM_PARTITION_TYPE
export LUKS_PARTITION_TYPE
export EFI_PARTITION_NAME
export LINUX_PARTITION_NAME
export LVM_PARTITION_NAME
export LUKS_PARTITION_NAME
export SWAP_PARTITION_NAME
export DEFAULT_ROOT_FILESYSTEM
export DEFAULT_HOME_FILESYSTEM
export EFI_FILESYSTEM
export BOOT_FILESYSTEM
export RAID_BOOT_DEVICE
export RAID_ROOT_DEVICE
export RAID_HOME_DEVICE
export RAID_LVM_DEVICE
export RAID_LUKS_DEVICE
