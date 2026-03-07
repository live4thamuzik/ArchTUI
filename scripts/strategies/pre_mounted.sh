#!/bin/bash
# pre_mounted.sh — Detect and validate user-mounted partitions at /mnt
# Called by: disk_strategies.sh dispatcher
# Environment contract: BOOT_MODE (required)
set -euo pipefail

# Source common utilities via source_or_die
_STRATEGY_SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../disk_utils.sh
source_or_die "$_STRATEGY_SCRIPT_DIR/../disk_utils.sh"

# Signal handling (ROE §3.1) — detection only, no children to kill
# shellcheck disable=SC2317
cleanup() { :; }
trap cleanup TERM INT EXIT

execute_pre_mounted_partitioning() {
    echo "=== PHASE 1: Pre-Mounted Partition Detection ==="
    log_info "Detecting pre-mounted partitions at /mnt..."

    # Validate /mnt is mounted (ROE §7.3)
    if ! mountpoint -q /mnt; then
        error_exit "/mnt is not mounted — mount your root partition to /mnt first"
    fi

    # Detect root mount
    local root_source root_fstype
    root_source=$(findmnt -n -o SOURCE /mnt 2>/dev/null || echo "")
    root_fstype=$(findmnt -n -o FSTYPE /mnt 2>/dev/null || echo "")

    if [[ -z "$root_source" ]]; then
        error_exit "Cannot determine root device mounted at /mnt"
    fi
    log_info "Root: $root_source ($root_fstype)"

    # Export root filesystem type
    export ROOT_FILESYSTEM_TYPE="$root_fstype"

    # Get root UUID (ROE §8.3: validated)
    local root_uuid
    root_uuid=$(get_device_uuid "$root_source") || error_exit "Cannot get UUID for root device $root_source"
    export ROOT_UUID="$root_uuid"
    log_info "ROOT_UUID=$root_uuid"
    capture_device_info "root" "$root_source"

    # Detect LUKS (if root is a dm-crypt mapper device)
    if [[ "$root_source" == /dev/mapper/* ]]; then
        local luks_device
        luks_device=$(cryptsetup status "${root_source##*/}" 2>/dev/null | grep "device:" | awk '{print $2}' || echo "")
        if [[ -n "$luks_device" && -b "$luks_device" ]]; then
            local luks_uuid
            luks_uuid=$(get_device_uuid "$luks_device") || log_warn "Cannot get LUKS UUID for $luks_device"
            if [[ -n "${luks_uuid:-}" ]]; then
                export LUKS_UUID="$luks_uuid"
                log_info "LUKS_UUID=$luks_uuid (backing device: $luks_device)"
                capture_device_info "luks" "$luks_device"
            fi
        fi
    fi

    # Detect boot mount
    for boot_path in /mnt/boot /mnt/boot/efi /mnt/efi; do
        if mountpoint -q "$boot_path" 2>/dev/null; then
            local boot_source boot_fstype_boot
            boot_source=$(findmnt -n -o SOURCE "$boot_path" 2>/dev/null || echo "")
            boot_fstype_boot=$(findmnt -n -o FSTYPE "$boot_path" 2>/dev/null || echo "")
            log_info "Boot: $boot_source ($boot_fstype_boot) at $boot_path"
            if [[ "$boot_fstype_boot" == "vfat" ]]; then
                capture_device_info "efi" "$boot_source"
            else
                capture_device_info "boot" "$boot_source"
            fi
        fi
    done

    # UEFI validation (ROE §7.3)
    if [[ "${BOOT_MODE:-Auto}" == "UEFI" ]]; then
        local has_esp="no"
        for esp_check in /mnt/efi /mnt/boot/efi /mnt/boot; do
            if mountpoint -q "$esp_check" 2>/dev/null; then
                local esp_fstype
                esp_fstype=$(findmnt -n -o FSTYPE "$esp_check" 2>/dev/null || echo "")
                if [[ "$esp_fstype" == "vfat" ]]; then
                    has_esp="yes"
                    break
                fi
            fi
        done
        if [[ "$has_esp" == "no" ]]; then
            log_warn "UEFI mode but no FAT32 ESP detected — bootloader installation may fail"
        fi
    fi

    # Detect home mount
    if mountpoint -q /mnt/home 2>/dev/null; then
        local home_source home_fstype_home
        home_source=$(findmnt -n -o SOURCE /mnt/home 2>/dev/null || echo "")
        home_fstype_home=$(findmnt -n -o FSTYPE /mnt/home 2>/dev/null || echo "")
        log_info "Home: $home_source ($home_fstype_home)"
        capture_device_info "home" "$home_source"
    fi

    # Detect active swap
    local swap_device
    swap_device=$(swapon --show=NAME --noheadings 2>/dev/null | head -1) || log_warn "swapon --show failed"
    if [[ -n "$swap_device" ]]; then
        log_info "Swap: $swap_device"
        local swap_uuid
        swap_uuid=$(get_device_uuid "$swap_device") || log_warn "Cannot get SWAP_UUID for $swap_device"
        if [[ -n "${swap_uuid:-}" ]]; then
            export SWAP_UUID="$swap_uuid"
            log_info "SWAP_UUID=$swap_uuid"
        fi
        capture_device_info "swap" "$swap_device"
    fi

    log_success "Pre-mounted partition detection complete"
    log_info "Proceeding with existing mount layout — no partitioning will be performed"
}
