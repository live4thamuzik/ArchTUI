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

        # Capture swap device info and UUID for hibernation
        local swap_device
        swap_device=$(get_partition_path "$INSTALL_DISK" "$part_num")
        capture_device_info "swap" "$swap_device"
        SWAP_UUID=$(get_device_uuid "$swap_device")
        export SWAP_UUID

        part_num=$((part_num + 1))
    fi

    # Root partition - takes remaining space (or split with home)
    root_part_num=$part_num
    if [ "$WANT_HOME_PARTITION" = "yes" ]; then
        # Root gets user-configured size; home gets remainder or fixed size
        local root_size_mib
        root_size_mib=$(get_root_size_mib)

        if [[ "$root_size_mib" == "REMAINING" ]]; then
            # Can't both take remaining — fall back to default
            log_warn "Root=Remaining with separate home: falling back to ${DEFAULT_ROOT_SIZE_MIB}MiB root"
            root_size_mib="$DEFAULT_ROOT_SIZE_MIB"
        fi

        sgdisk -n "${part_num}:0:+${root_size_mib}M" -t "${part_num}:${LINUX_PARTITION_TYPE}" -c "${part_num}:ROOT" "$INSTALL_DISK" || error_exit "Failed to create root partition."
        sync_partitions "$INSTALL_DISK"
        local root_device
        root_device=$(get_partition_path "$INSTALL_DISK" "$part_num")
        format_filesystem "$root_device" "$ROOT_FILESYSTEM_TYPE"
        part_num=$((part_num + 1))

        # Home partition
        home_part_num=$part_num
        local home_size_mib
        home_size_mib=$(get_home_size_mib)

        if [[ "$home_size_mib" == "REMAINING" ]]; then
            # Use all remaining space
            create_home_partition "$INSTALL_DISK" "$part_num" "$HOME_FILESYSTEM_TYPE"
        else
            # Fixed size home
            local home_device
            sgdisk -n "${part_num}:0:+${home_size_mib}M" -t "${part_num}:${LINUX_PARTITION_TYPE}" -c "${part_num}:HOME" "$INSTALL_DISK" || error_exit "Failed to create home partition."
            sync_partitions "$INSTALL_DISK"
            home_device=$(get_partition_path "$INSTALL_DISK" "$part_num")
            format_filesystem "$home_device" "$HOME_FILESYSTEM_TYPE"
        fi
    else
        # No separate home - root takes all remaining space
        create_root_partition "$INSTALL_DISK" "$part_num" "$ROOT_FILESYSTEM_TYPE"
    fi

    # --- Mount all partitions ---
    log_info "Mounting partitions..."

    local root_device boot_device esp_device
    root_device=$(get_partition_path "$INSTALL_DISK" "$root_part_num")
    boot_device=$(get_partition_path "$INSTALL_DISK" "$boot_part_num")

    # Mount root (setup_btrfs_subvolumes handles its own mount/remount cycle)
    if [ "$ROOT_FILESYSTEM_TYPE" = "btrfs" ]; then
        local include_home="yes"
        [ "$WANT_HOME_PARTITION" = "yes" ] && include_home="no"
        setup_btrfs_subvolumes "$root_device" "$include_home"
    else
        safe_mount "$root_device" "/mnt"
    fi
    mkdir -p /mnt/boot /mnt/efi

    # Mount boot
    safe_mount "$boot_device" "/mnt/boot"
    capture_device_info "boot" "$boot_device"

    # Mount ESP
    if [[ "$use_existing_esp" == "yes" ]]; then
        esp_device="$existing_esp"
    else
        esp_device=$(get_partition_path "$INSTALL_DISK" "$esp_part_num")
    fi

    if [[ "$BOOT_MODE" == "UEFI" ]]; then
        safe_mount "$esp_device" "/mnt/efi"
        capture_device_info "efi" "$esp_device"
        export EFI_DEVICE="$esp_device"
    fi

    # Mount home if separate partition
    if [ "$WANT_HOME_PARTITION" = "yes" ]; then
        local home_device
        home_device=$(get_partition_path "$INSTALL_DISK" "$home_part_num")
        safe_mount "$home_device" "/mnt/home"
        capture_device_info "home" "$home_device"
    fi

    # Capture UUIDs for bootloader config
    capture_device_info "root" "$root_device"
    ROOT_UUID=$(get_device_uuid "$root_device")
    export ROOT_UUID

    log_partitioning_complete "Simple (ESP + boot + root)"
}
