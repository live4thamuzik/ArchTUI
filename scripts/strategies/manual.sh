#!/bin/bash
# manual.sh - Manual partitioning strategy
# Formats and mounts user-assigned partitions (TUI assigns roles via MANUAL_* env vars)
set -euo pipefail

# Source common utilities via source_or_die
_STRATEGY_SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../disk_utils.sh
source_or_die "$_STRATEGY_SCRIPT_DIR/../disk_utils.sh"

# Execute manual partitioning strategy
# Env vars from TUI (injected by installer.rs):
#   MANUAL_ROOT_PARTITION  (required)  e.g. /dev/sda2
#   MANUAL_ROOT_FS         (required)  e.g. ext4
#   MANUAL_BOOT_PARTITION  (required)  e.g. /dev/sda1
#   MANUAL_EFI_PARTITION   (UEFI only) e.g. /dev/sda1
#   MANUAL_HOME_PARTITION  (optional)  e.g. /dev/sda3
#   MANUAL_HOME_FS         (optional)  e.g. ext4
#   MANUAL_SWAP_PARTITION  (optional)  e.g. /dev/sda4
execute_manual_partitioning() {
    setup_partitioning_trap
    echo "=== PHASE 1: Manual Partitioning ==="
    log_info "Starting manual partitioning (format + mount)..."

    # --- Validate required env vars ---
    if [[ -z "${MANUAL_ROOT_PARTITION:-}" ]]; then
        error_exit "MANUAL_ROOT_PARTITION is not set (TUI must assign root partition)"
    fi
    if [[ -z "${MANUAL_ROOT_FS:-}" ]]; then
        error_exit "MANUAL_ROOT_FS is not set (TUI must assign root filesystem)"
    fi
    if [[ -z "${MANUAL_BOOT_PARTITION:-}" ]]; then
        error_exit "MANUAL_BOOT_PARTITION is not set (TUI must assign boot partition)"
    fi

    # --- Validate all device paths exist ---
    if [[ ! -b "$MANUAL_ROOT_PARTITION" ]]; then
        error_exit "Root partition $MANUAL_ROOT_PARTITION is not a block device"
    fi
    if [[ ! -b "$MANUAL_BOOT_PARTITION" ]]; then
        error_exit "Boot partition $MANUAL_BOOT_PARTITION is not a block device"
    fi
    if [[ -n "${MANUAL_EFI_PARTITION:-}" && ! -b "$MANUAL_EFI_PARTITION" ]]; then
        error_exit "EFI partition $MANUAL_EFI_PARTITION is not a block device"
    fi
    if [[ -n "${MANUAL_HOME_PARTITION:-}" && ! -b "$MANUAL_HOME_PARTITION" ]]; then
        error_exit "Home partition $MANUAL_HOME_PARTITION is not a block device"
    fi
    if [[ -n "${MANUAL_SWAP_PARTITION:-}" && ! -b "$MANUAL_SWAP_PARTITION" ]]; then
        error_exit "Swap partition $MANUAL_SWAP_PARTITION is not a block device"
    fi

    # --- 1. Format + mount root ---
    log_info "Formatting root: $MANUAL_ROOT_PARTITION as $MANUAL_ROOT_FS"
    format_filesystem "$MANUAL_ROOT_PARTITION" "$MANUAL_ROOT_FS" || error_exit "Failed to format root partition"

    if [[ "$MANUAL_ROOT_FS" == "btrfs" ]]; then
        local include_home="yes"
        [[ -n "${MANUAL_HOME_PARTITION:-}" ]] && include_home="no"
        setup_btrfs_subvolumes "$MANUAL_ROOT_PARTITION" "$include_home" || error_exit "Failed to setup btrfs subvolumes"
    else
        safe_mount "$MANUAL_ROOT_PARTITION" "/mnt" || error_exit "Failed to mount root partition"
    fi
    capture_device_info "root" "$MANUAL_ROOT_PARTITION"

    # --- 2. Format + mount boot (always ext4) ---
    log_info "Formatting boot: $MANUAL_BOOT_PARTITION as ext4"
    format_filesystem "$MANUAL_BOOT_PARTITION" "ext4" || error_exit "Failed to format boot partition"
    mkdir -p /mnt/boot
    safe_mount "$MANUAL_BOOT_PARTITION" "/mnt/boot" || error_exit "Failed to mount boot partition"
    capture_device_info "boot" "$MANUAL_BOOT_PARTITION"

    # --- 3. Format + mount EFI (UEFI only, always vfat) ---
    if [[ -n "${MANUAL_EFI_PARTITION:-}" ]]; then
        log_info "Formatting EFI: $MANUAL_EFI_PARTITION as vfat"
        format_filesystem "$MANUAL_EFI_PARTITION" "vfat" || error_exit "Failed to format EFI partition"
        mkdir -p /mnt/efi
        safe_mount "$MANUAL_EFI_PARTITION" "/mnt/efi" || error_exit "Failed to mount EFI partition"
        capture_device_info "efi" "$MANUAL_EFI_PARTITION"
        export EFI_DEVICE="$MANUAL_EFI_PARTITION"
    fi

    # --- 4. Format + mount home (optional) ---
    if [[ -n "${MANUAL_HOME_PARTITION:-}" ]]; then
        local home_fs="${MANUAL_HOME_FS:-ext4}"
        log_info "Formatting home: $MANUAL_HOME_PARTITION as $home_fs"
        format_filesystem "$MANUAL_HOME_PARTITION" "$home_fs" || error_exit "Failed to format home partition"
        mkdir -p /mnt/home
        safe_mount "$MANUAL_HOME_PARTITION" "/mnt/home" || error_exit "Failed to mount home partition"
        capture_device_info "home" "$MANUAL_HOME_PARTITION"
    fi

    # --- 5. mkswap + swapon (optional) ---
    if [[ -n "${MANUAL_SWAP_PARTITION:-}" ]]; then
        log_info "Setting up swap: $MANUAL_SWAP_PARTITION"
        log_cmd "mkswap $MANUAL_SWAP_PARTITION"
        mkswap "$MANUAL_SWAP_PARTITION" || error_exit "Failed to format swap partition"
        log_cmd "swapon $MANUAL_SWAP_PARTITION"
        swapon "$MANUAL_SWAP_PARTITION" || log_warn "Failed to activate swap"
        capture_device_info "swap" "$MANUAL_SWAP_PARTITION"
        SWAP_UUID=$(get_device_uuid "$MANUAL_SWAP_PARTITION") || log_warn "Cannot determine SWAP_UUID"
        export SWAP_UUID
    fi

    # --- 6. Get ROOT_UUID ---
    ROOT_UUID=$(get_device_uuid "$MANUAL_ROOT_PARTITION") || error_exit "Cannot determine ROOT_UUID"
    export ROOT_UUID

    # --- 7. Verify essential mounts ---
    verify_essential_mounts || error_exit "Essential mount verification failed"

    log_partitioning_complete "Manual"
}
