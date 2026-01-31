#!/bin/bash
# disk_utils.sh - Disk partitioning and formatting utilities

set -euo pipefail

# --- Source-Once Guard ---
if [[ -n "${_DISK_UTILS_SH_SOURCED:-}" ]]; then
    return 0 2>/dev/null || exit 0
fi
readonly _DISK_UTILS_SH_SOURCED=1

# Source utilities (required dependency)
_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -z "${_UTILS_SH_SOURCED:-}" ]]; then
    source "${_SCRIPT_DIR}/utils.sh"
fi

# --- Partition Configuration Constants ---
# Guard against re-definition to allow multiple sourcing during tests
if [[ -z "${EFI_PARTITION_TYPE:-}" ]]; then
    # Partition Types (GPT codes)
    readonly EFI_PARTITION_TYPE="EF00"
    readonly BIOS_BOOT_PARTITION_TYPE="EF02"
    readonly LINUX_PARTITION_TYPE="8300"
    readonly LVM_PARTITION_TYPE="8E00"
    readonly LUKS_PARTITION_TYPE="8309"
    readonly SWAP_PARTITION_TYPE="8200"
    readonly XBOOTLDR_PARTITION_TYPE="EA00"

    # Default Partition Sizes (in MiB)
    readonly BIOS_BOOT_PART_SIZE_MIB=1
    readonly BOOT_PART_SIZE_MIB=1024
    readonly DEFAULT_SWAP_SIZE_MIB=2048
    readonly DEFAULT_ROOT_SIZE_MIB=102400
    readonly DEFAULT_ESP_SIZE_MIB=512

    # Filesystem Types
    readonly DEFAULT_ROOT_FILESYSTEM="ext4"
    readonly DEFAULT_HOME_FILESYSTEM="ext4"
    readonly EFI_FILESYSTEM="vfat"
    readonly BOOT_FILESYSTEM="ext4"
fi

# --- Device Discovery ---

get_partition_path() {
    local disk="$1"
    local part_num="$2"
    
    # Handle NVMe/MMC/Loop devices (e.g., /dev/nvme0n1 -> /dev/nvme0n1p1)
    if [[ "$disk" =~ (nvme|mmcblk|loop) ]]; then
        echo "${disk}p${part_num}"
    else
        # Handle SATA/SCSI/VirtIO (e.g., /dev/sda -> /dev/sda1)
        echo "${disk}${part_num}"
    fi
}

get_swap_size_mib() {
    local ram_gb="$1"
    
    # Handle both "16" and "16G" formats
    local ram_val="${ram_gb%[Gg]*}"
    
    # Fallback if not a number
    if [[ ! "$ram_val" =~ ^[0-9]+$ ]]; then
        echo "$DEFAULT_SWAP_SIZE_MIB"
        return
    fi
    
    # Calculate swap size: Ram <= 4GB ? 2x RAM : 1x RAM (Capped at 8GB usually, but simplified here)
    if (( ram_val <= 4 )); then
        echo $(( ram_val * 1024 * 2 ))
    elif (( ram_val <= 16 )); then
         echo $(( ram_val * 1024 ))
    else
        echo "16384" # Cap at 16GB swap for large RAM
    fi
}

# --- Partitioning Functions ---

wipe_disk() {
    local disk="$1"
    log_warn "Wiping all data on $disk..."
    
    # Wipe filesystem signatures
    wipefs --all --force "$disk"
    
    # Zero out the beginning of the disk to kill MBR/GPT tables
    dd if=/dev/zero of="$disk" bs=1M count=10 status=none
    
    # Reload partition table
    partprobe "$disk" || true
    
    return 0
}

create_partition_table() {
    local disk="$1"
    local label="${2:-gpt}"
    
    log_info "Creating $label partition table on $disk"
    
    # Use sgdisk for scripting (non-interactive)
    # --zap-all clears table, -o creates new GPT
    sgdisk --zap-all "$disk"
    sgdisk -o "$disk"
    
    partprobe "$disk" || true
}

create_esp_partition() {
    local disk="$1"
    local part_num="$2"
    local size_mib="${3:-$DEFAULT_ESP_SIZE_MIB}"
    
    local part_device
    part_device=$(get_partition_path "$disk" "$part_num")
    
    log_info "Creating ESP partition: $part_device (${size_mib}MiB)"
    
    sgdisk -n "${part_num}:0:+${size_mib}M" -t "${part_num}:${EFI_PARTITION_TYPE}" -c "${part_num}:EFI" "$disk"
    
    # Wait for device node
    sleep 1
    
    mkfs.fat -F32 "$part_device"
}

create_xbootldr_partition() {
    local disk="$1"
    local part_num="$2"
    local size_mib="${3:-$BOOT_PART_SIZE_MIB}"
    
    log_info "Creating XBOOTLDR partition: partition $part_num"
    sgdisk -n "${part_num}:0:+${size_mib}M" -t "${part_num}:${XBOOTLDR_PARTITION_TYPE}" -c "${part_num}:XBOOTLDR" "$disk"
}

create_bios_boot_partition() {
    local disk="$1"
    local part_num="$2"
    
    log_info "Creating BIOS Boot partition: partition $part_num"
    sgdisk -n "${part_num}:0:+${BIOS_BOOT_PART_SIZE_MIB}M" -t "${part_num}:${BIOS_BOOT_PARTITION_TYPE}" -c "${part_num}:BIOSBOOT" "$disk"
}

create_swap_partition() {
    local disk="$1"
    local part_num="$2"
    local size_mib="$3"
    
    local part_device
    part_device=$(get_partition_path "$disk" "$part_num")
    
    log_info "Creating Swap partition: $part_device (${size_mib}MiB)"
    
    sgdisk -n "${part_num}:0:+${size_mib}M" -t "${part_num}:${SWAP_PARTITION_TYPE}" -c "${part_num}:SWAP" "$disk"
    
    # Wait for device node
    sleep 1
    
    mkswap "$part_device"
    swapon "$part_device"
}

create_root_partition() {
    local disk="$1"
    local part_num="$2"
    local filesystem="${3:-$DEFAULT_ROOT_FILESYSTEM}"
    
    local part_device
    part_device=$(get_partition_path "$disk" "$part_num")
    
    log_info "Creating Root partition: $part_device ($filesystem)"
    
    # Use remaining space (0)
    sgdisk -n "${part_num}:0:0" -t "${part_num}:${LINUX_PARTITION_TYPE}" -c "${part_num}:ROOT" "$disk"
    
    sleep 1
    
    format_filesystem "$part_device" "$filesystem"
}

create_home_partition() {
    local disk="$1"
    local part_num="$2"
    local filesystem="${3:-$DEFAULT_HOME_FILESYSTEM}"
    
    local part_device
    part_device=$(get_partition_path "$disk" "$part_num")
    
    log_info "Creating Home partition: $part_device"
    
    # This implies root didn't take 100%. Logic for splitting root/home should be in strategy.
    # For now, assumes we are appending to disk.
    sgdisk -n "${part_num}:0:0" -t "${part_num}:${LINUX_PARTITION_TYPE}" -c "${part_num}:HOME" "$disk"
    
    sleep 1
    format_filesystem "$part_device" "$filesystem"
}

safe_mount() {
    local device="$1"
    local mountpoint="$2"
    local options="${3:-defaults}"
    
    mkdir -p "$mountpoint"
    mount -o "$options" "$device" "$mountpoint"
}

setup_luks_encryption() {
    local partition="$1"
    local password="$2"
    local mapper_name="${3:-cryptroot}"
    
    if [[ -z "$password" ]]; then
        log_error "LUKS password is empty"
        return 1
    fi
    
    log_info "Encrypting $partition with LUKS2"
    
    # Format with password from stdin
    echo -n "$password" | cryptsetup luksFormat --type luks2 --batch-mode "$partition" -
    
    # Open mapping
    echo -n "$password" | cryptsetup open "$partition" "$mapper_name" -
}

setup_btrfs_subvolumes() {
    local mountpoint="$1"
    local include_home="${2:-no}"
    
    log_info "Creating Btrfs subvolumes at $mountpoint"
    
    btrfs subvolume create "$mountpoint/@"
    btrfs subvolume create "$mountpoint/@var"
    btrfs subvolume create "$mountpoint/@tmp"
    btrfs subvolume create "$mountpoint/@snapshots"
    
    if [[ "$include_home" == "yes" ]]; then
        btrfs subvolume create "$mountpoint/@home"
    fi
}

capture_device_info() {
    local type="$1"
    local device="$2"
    
    if [[ -z "$device" ]]; then return 1; fi
    
    case "$type" in
        root) export ROOT_DEVICE="$device" ;;
        efi)  export EFI_DEVICE="$device" ;;
        swap) export SWAP_DEVICE="$device" ;;
    esac
    
    log_info "Captured $type device: $device"
}

get_device_uuid() {
    local device="$1"
    if [[ -z "$device" ]]; then
        return 1
    fi
    lsblk -n -o UUID "$device"
}

validate_partitioning_requirements() {
    local config_file="$1"
    # Basic check stub
    log_info "Validating partitioning requirements..."
}
