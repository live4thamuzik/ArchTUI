#!/bin/bash
# simple.sh - Simple partitioning strategy (ESP + boot + root + optional home)
# Standard Arch Linux partition layout per wiki
set -euo pipefail

# Source common utilities via source_or_die
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../disk_utils.sh
source_or_die "$SCRIPT_DIR/../disk_utils.sh"

# Execute simple partitioning strategy
execute_simple_partitioning() {
    echo "=== PHASE 1: Simple Partitioning ==="
    log_info "Starting simple partitioning for $INSTALL_DISK..."

    # Setup cleanup trap for error recovery
    setup_partitioning_trap

    # Validate requirements
    validate_partitioning_requirements

    # --- Dual-boot detection ---
    local use_existing_esp="no"
    local existing_esp=""

    # Check for other operating systems
    if detect_other_os; then
        log_warn "Other OS detected - enabling os-prober for dual-boot"
        export OS_PROBER="yes"
    fi

    # Check for existing ESP (dual-boot scenario)
    existing_esp=$(detect_existing_esp "$INSTALL_DISK")
    if [[ -n "$existing_esp" ]]; then
        # Check if Windows is on this ESP
        if detect_windows_installation "$existing_esp"; then
            log_warn "Windows installation detected on $existing_esp"
            log_warn "Will preserve existing ESP for dual-boot chainloading"
            use_existing_esp="yes"
            export DUAL_BOOT_WINDOWS="yes"
        fi
    fi

    # Same-disk dual-boot check
    # If Windows ESP is on the same disk we're installing to, we cannot use
    # automatic partitioning as it would destroy Windows.
    #
    # Options for users:
    # 1. Use a different disk for Arch Linux
    # 2. Use manual partitioning (shrink Windows partition first from Windows)
    # 3. Use the manual strategy with pre-configured partitions
    if [[ "$use_existing_esp" == "yes" && "$existing_esp" == "${INSTALL_DISK}"* ]]; then
        log_error "Same-disk dual-boot detected: Windows ESP is on target disk!"
        log_error ""
        log_error "Automatic partitioning would destroy Windows. Options:"
        log_error "  1. Select a different disk for Arch Linux installation"
        log_error "  2. Shrink Windows partition from within Windows (Disk Management)"
        log_error "     Then use 'manual' partitioning strategy to install in free space"
        log_error "  3. Boot Windows and use 'diskpart' or Disk Management to create"
        log_error "     partitions for Linux, then use 'manual' strategy"
        log_error ""
        log_error "See: https://wiki.archlinux.org/title/Dual_boot_with_Windows"
        return 1
    fi

    wipe_disk "$INSTALL_DISK" "CONFIRMED"

    local part_num=1
    local esp_part_num=0
    local boot_part_num=0
    local swap_part_num=0
    local root_part_num=0
    local home_part_num=0

    # Create partition table
    create_partition_table "$INSTALL_DISK"

    # --- Partition Layout ---
    # UEFI: ESP (512M) + Boot (1G) + [Swap] + Root + [Home]
    # BIOS: BIOS Boot (1M) + Boot (1G) + [Swap] + Root + [Home]

    if [ "$BOOT_MODE" = "UEFI" ]; then
        # ESP Partition - 512MB FAT32 at /efi
        if [[ "$use_existing_esp" != "yes" ]]; then
            esp_part_num=$part_num
            create_esp_partition "$INSTALL_DISK" "$part_num" "512"
            part_num=$((part_num + 1))
        fi

        # Boot Partition - 1GB ext4 at /boot
        boot_part_num=$part_num
        create_boot_partition "$INSTALL_DISK" "$part_num" "1024"
        part_num=$((part_num + 1))
    else
        # BIOS with GPT: Need BIOS boot partition for GRUB (no filesystem, just raw)
        create_bios_boot_partition "$INSTALL_DISK" "$part_num"
        part_num=$((part_num + 1))

        # Boot partition - 1GB ext4 at /boot
        boot_part_num=$part_num
        create_boot_partition "$INSTALL_DISK" "$part_num" "1024"
        part_num=$((part_num + 1))
    fi

    # Swap partition (if requested)
    if [ "$WANT_SWAP" = "yes" ]; then
        swap_part_num=$part_num
        local swap_size_mib
        swap_size_mib=$(get_swap_size_mib)
        create_swap_partition "$INSTALL_DISK" "$part_num" "$swap_size_mib"

        # Capture swap UUID for hibernation
        local swap_device
        swap_device=$(get_partition_path "$INSTALL_DISK" "$part_num")
        SWAP_UUID=$(get_device_uuid "$swap_device")
        export SWAP_UUID

        part_num=$((part_num + 1))
    fi

    # Root partition - takes remaining space (or split with home)
    root_part_num=$part_num
    if [ "$WANT_HOME_PARTITION" = "yes" ]; then
        # If separate home, root gets fixed size (let user configure or use sensible default)
        local root_size_mib="${ROOT_SIZE_MIB:-51200}"  # Default 50GB
        sgdisk -n "${part_num}:0:+${root_size_mib}M" -t "${part_num}:${LINUX_PARTITION_TYPE}" -c "${part_num}:ROOT" "$INSTALL_DISK"
        sleep 1
        local root_device
        root_device=$(get_partition_path "$INSTALL_DISK" "$part_num")
        format_filesystem "$root_device" "$ROOT_FILESYSTEM_TYPE"
        part_num=$((part_num + 1))

        # Home partition - remainder of disk
        home_part_num=$part_num
        create_home_partition "$INSTALL_DISK" "$part_num" "$HOME_FILESYSTEM_TYPE"
    else
        # No separate home - root takes all remaining space
        create_root_partition "$INSTALL_DISK" "$part_num" "$ROOT_FILESYSTEM_TYPE"
    fi

    # --- Mount all partitions ---
    log_info "Mounting partitions..."

    local root_device boot_device esp_device
    root_device=$(get_partition_path "$INSTALL_DISK" "$root_part_num")
    boot_device=$(get_partition_path "$INSTALL_DISK" "$boot_part_num")

    # Handle Btrfs subvolumes if needed
    if [ "$ROOT_FILESYSTEM_TYPE" = "btrfs" ]; then
        log_info "Setting up Btrfs subvolumes..."
        # Temporarily mount to create subvolumes
        mount "$root_device" /mnt
        btrfs subvolume create /mnt/@
        btrfs subvolume create /mnt/@var
        btrfs subvolume create /mnt/@tmp
        btrfs subvolume create /mnt/@snapshots
        if [ "$WANT_HOME_PARTITION" != "yes" ]; then
            btrfs subvolume create /mnt/@home
        fi
        umount /mnt

        # Remount with subvolume
        mount -o compress=zstd,noatime,subvol=@ "$root_device" /mnt
        mkdir -p /mnt/{var,tmp,.snapshots,boot,efi}
        mount -o compress=zstd,noatime,subvol=@var "$root_device" /mnt/var
        mount -o compress=zstd,noatime,subvol=@tmp "$root_device" /mnt/tmp
        mount -o compress=zstd,noatime,subvol=@snapshots "$root_device" /mnt/.snapshots
        if [ "$WANT_HOME_PARTITION" != "yes" ]; then
            mkdir -p /mnt/home
            mount -o compress=zstd,noatime,subvol=@home "$root_device" /mnt/home
        fi
    else
        # Standard mount for ext4/xfs
        safe_mount "$root_device" "/mnt"
        mkdir -p /mnt/boot /mnt/efi
    fi

    # Mount boot
    safe_mount "$boot_device" "/mnt/boot"

    # Mount ESP
    if [[ "$use_existing_esp" == "yes" ]]; then
        esp_device="$existing_esp"
    else
        esp_device=$(get_partition_path "$INSTALL_DISK" "$esp_part_num")
    fi

    if [[ "$BOOT_MODE" == "UEFI" ]]; then
        mkdir -p /mnt/efi
        safe_mount "$esp_device" "/mnt/efi"
        export EFI_DEVICE="$esp_device"
    fi

    # Mount home if separate partition
    if [ "$WANT_HOME_PARTITION" = "yes" ]; then
        local home_device
        home_device=$(get_partition_path "$INSTALL_DISK" "$home_part_num")
        mkdir -p /mnt/home
        safe_mount "$home_device" "/mnt/home"
    fi

    # Capture UUIDs for bootloader config
    ROOT_UUID=$(get_device_uuid "$root_device")
    export ROOT_UUID

    log_partitioning_complete "Simple (ESP + boot + root)"
}
