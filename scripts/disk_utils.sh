#!/bin/bash
# disk_utils.sh - Common utilities and constants for disk partitioning strategies

set -euo pipefail

# Source utility functions
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$SCRIPT_DIR/utils.sh"

# --- Partition Configuration Constants ---
# These constants replace magic strings throughout the partitioning functions
# for better readability and maintainability

# Partition Types (GPT codes)
readonly EFI_PARTITION_TYPE="EF00"
readonly BIOS_BOOT_PARTITION_TYPE="EF02"
# shellcheck disable=SC2034
readonly LINUX_PARTITION_TYPE="8300"
# shellcheck disable=SC2034
readonly LVM_PARTITION_TYPE="8E00"
# shellcheck disable=SC2034
readonly LUKS_PARTITION_TYPE="8309"
readonly SWAP_PARTITION_TYPE="8200"
readonly XBOOTLDR_PARTITION_TYPE="EA00"

# Partition Names (for identification)
# shellcheck disable=SC2034
readonly EFI_PARTITION_NAME="EFI System"
# shellcheck disable=SC2034
readonly BIOS_BOOT_PARTITION_NAME="BIOS Boot"
# shellcheck disable=SC2034
readonly LINUX_PARTITION_NAME="Linux filesystem"
# shellcheck disable=SC2034
readonly LVM_PARTITION_NAME="Linux LVM"
# shellcheck disable=SC2034
readonly LUKS_PARTITION_NAME="Linux LUKS"
# shellcheck disable=SC2034
readonly SWAP_PARTITION_NAME="Linux swap"

# Default Partition Sizes (in MiB)
readonly BIOS_BOOT_PART_SIZE_MIB=1
readonly BOOT_PART_SIZE_MIB=1024  # Same as XBOOTLDR for consistency
readonly DEFAULT_SWAP_SIZE_MIB=2048
readonly DEFAULT_ROOT_SIZE_MIB=102400

# Filesystem Types
# shellcheck disable=SC2034
readonly DEFAULT_ROOT_FILESYSTEM="ext4"
# shellcheck disable=SC2034
readonly DEFAULT_HOME_FILESYSTEM="ext4"
readonly EFI_FILESYSTEM="vfat"
readonly BOOT_FILESYSTEM="ext4"

# --- Common Partitioning Functions ---

# Get partition path based on disk type (NVMe vs regular)
get_partition_path() {
    local disk="$1"
    local part_num="$2"
    
    if [[ "$disk" =~ nvme ]]; then
        echo "${disk}p${part_num}"
    else
        echo "${disk}${part_num}"
    fi
}

# Wipe disk clean
wipe_disk() {
    local disk="$1"
    log_info "Wiping disk: $disk"
    wipefs -a "$disk" || error_exit "Failed to wipe disk $disk"
    partprobe "$disk"
}

# Create partition table (GPT for UEFI, MBR for BIOS)
create_partition_table() {
    local disk="$1"
    
    if [ "$BOOT_MODE" = "UEFI" ]; then
        log_info "Creating GPT partition table on $disk"
        sgdisk -Z "$disk" || error_exit "Failed to create GPT label on $disk."
    else
        log_info "Creating MBR partition table on $disk"
        printf "o\nw\n" | fdisk "$disk" || error_exit "Failed to create MBR label on $disk."
    fi
    partprobe "$disk"
}

# Create ESP partition (UEFI only)
create_esp_partition() {
    local disk="$1"
    local part_num="$2"
    local size_mib="${3:-100}"
    
    log_info "Creating ESP partition (${size_mib}MiB) for /efi..."
    local size_mb="${size_mib}M"
    sgdisk -n "$part_num:0:+$size_mb" -t "$part_num:$EFI_PARTITION_TYPE" "$disk" || error_exit "Failed to create ESP partition."
    partprobe "$disk"
    local part_dev=$(get_partition_path "$disk" "$part_num")
    
    format_filesystem "$part_dev" "$EFI_FILESYSTEM"
    capture_device_info "efi" "$part_dev" "UUID"
    capture_device_info "efi" "$part_dev" "PARTUUID"
    safe_mount "$part_dev" "/mnt/efi"
    
    echo "$part_dev"
}

# Create XBOOTLDR partition (UEFI only)
create_xbootldr_partition() {
    local disk="$1"
    local part_num="$2"
    local size_mib="${3:-1024}"
    
    log_info "Creating XBOOTLDR partition (${size_mib}MiB) for /boot..."
    local size_mb="${size_mib}M"
    sgdisk -n "$part_num:0:+$size_mb" -t "$part_num:$XBOOTLDR_PARTITION_TYPE" "$disk" || error_exit "Failed to create XBOOTLDR partition."
    partprobe "$disk"
    local part_dev=$(get_partition_path "$disk" "$part_num")
    
    format_filesystem "$part_dev" "$BOOT_FILESYSTEM"
    capture_device_info "boot" "$part_dev" "UUID"
    safe_mount "$part_dev" "/mnt/boot"
    
    echo "$part_dev"
}

# Create boot partition (BIOS only)
create_boot_partition() {
    local disk="$1"
    local part_num="$2"
    local size_mib="${3:-1024}"
    
    log_info "Creating boot partition (${size_mib}MiB) for /boot..."
    printf "n\np\n$part_num\n\n+${size_mib}M\nw\n" | fdisk "$disk" || error_exit "Failed to create boot partition."
    partprobe "$disk"
    local part_dev=$(get_partition_path "$disk" "$part_num")
    
    format_filesystem "$part_dev" "$BOOT_FILESYSTEM"
    capture_device_info "boot" "$part_dev" "UUID"
    safe_mount "$part_dev" "/mnt/boot"
    
    echo "$part_dev"
}

# Create swap partition
create_swap_partition() {
    local disk="$1"
    local part_num="$2"
    local size_mib="$3"
    
    if [ "$WANT_SWAP" != "yes" ]; then
        return 0
    fi
    
    log_info "Creating swap partition (${size_mib}MiB)..."
    local size_mb="${size_mib}M"
    
    if [ "$BOOT_MODE" = "UEFI" ]; then
        sgdisk -n "$part_num:0:+$size_mb" -t "$part_num:$SWAP_PARTITION_TYPE" "$disk" || error_exit "Failed to create swap partition."
    else
        printf "n\np\n$part_num\n\n+${size_mib}M\nw\n" | fdisk "$disk" || error_exit "Failed to create swap partition."
    fi
    partprobe "$disk"
    local part_dev=$(get_partition_path "$disk" "$part_num")
    
    format_filesystem "$part_dev" "swap"
    capture_device_info "swap" "$part_dev" "UUID"
    swapon "$part_dev"
    
    echo "$part_dev"
}

# Create root partition
create_root_partition() {
    local disk="$1"
    local part_num="$2"
    local filesystem="${3:-ext4}"
    
    log_info "Creating root partition with $filesystem filesystem..."
    
    if [ "$BOOT_MODE" = "UEFI" ]; then
        sgdisk -n "$part_num:0:0" -t "$part_num:$LINUX_PARTITION_TYPE" "$disk" || error_exit "Failed to create root partition."
    else
        printf "n\np\n$part_num\n\n\nw\n" | fdisk "$disk" || error_exit "Failed to create root partition."
    fi
    partprobe "$disk"
    local part_dev=$(get_partition_path "$disk" "$part_num")
    
    format_filesystem "$part_dev" "$filesystem"
    capture_device_info "root" "$part_dev" "UUID"
    safe_mount "$part_dev" "/mnt"
    
    echo "$part_dev"
}

# Create home partition
create_home_partition() {
    local disk="$1"
    local part_num="$2"
    local filesystem="${3:-ext4}"
    
    if [ "$WANT_HOME_PARTITION" != "yes" ]; then
        return 0
    fi
    
    log_info "Creating home partition with $filesystem filesystem..."
    
    if [ "$BOOT_MODE" = "UEFI" ]; then
        sgdisk -n "$part_num:0:0" -t "$part_num:$LINUX_PARTITION_TYPE" "$disk" || error_exit "Failed to create home partition."
    else
        printf "n\np\n$part_num\n\n\nw\n" | fdisk "$disk" || error_exit "Failed to create home partition."
    fi
    partprobe "$disk"
    local part_dev=$(get_partition_path "$disk" "$part_num")
    
    format_filesystem "$part_dev" "$filesystem"
    capture_device_info "home" "$part_dev" "UUID"
    mkdir -p /mnt/home
    safe_mount "$part_dev" "/mnt/home"
    
    echo "$part_dev"
}

# Safe mount function
safe_mount() {
    local device="$1"
    local mountpoint="$2"
    
    # Create mountpoint if it doesn't exist
    mkdir -p "$mountpoint"
    
    # Check if already mounted
    if mountpoint -q "$mountpoint"; then
        log_info "$mountpoint is already mounted"
        return 0
    fi
    
    # Mount the device
    mount "$device" "$mountpoint" || error_exit "Failed to mount $device to $mountpoint"
    log_info "Mounted $device to $mountpoint"
}

# Validate partitioning requirements
validate_partitioning_requirements() {
    # Verify filesystem types are set by user
    if [ -z "$ROOT_FILESYSTEM_TYPE" ]; then
        error_exit "ROOT_FILESYSTEM_TYPE must be set in TUI configuration"
    fi
    if [ -z "$HOME_FILESYSTEM_TYPE" ]; then
        error_exit "HOME_FILESYSTEM_TYPE must be set in TUI configuration"
    fi
    
    log_info "Partitioning requirements validated"
}

# Get swap size in MiB
get_swap_size_mib() {
    case "$SWAP_SIZE" in
        "2GB") echo "2048" ;;
        "4GB") echo "4096" ;;
        "8GB") echo "8192" ;;
        "16GB") echo "16384" ;;
        *) echo "$DEFAULT_SWAP_SIZE_MIB" ;;
    esac
}

# Verify essential mounts for installation
verify_essential_mounts() {
    # Verify root is mounted
    if ! mountpoint -q /mnt; then
        error_exit "/mnt is not mounted. Please ensure your root partition is mounted correctly."
    fi
    
    # Verify UEFI mounts if in UEFI mode
    if [ "$BOOT_MODE" = "UEFI" ]; then
        if ! mountpoint -q /mnt/efi; then
            log_warn "/mnt/efi is not mounted. This is required for UEFI installations."
            read -rp "Press Enter to continue after mounting /mnt/efi: "
            if ! mountpoint -q /mnt/efi; then
                error_exit "/mnt/efi is still not mounted. Cannot proceed with UEFI installation."
            fi
        fi
        
        if ! mountpoint -q /mnt/boot; then
            log_warn "/mnt/boot is not mounted. This is required for UEFI installations."
            read -rp "Press Enter to continue after mounting /mnt/boot: "
            if ! mountpoint -q /mnt/boot; then
                error_exit "/mnt/boot is still not mounted. Cannot proceed with UEFI installation."
            fi
        fi
    else
        # BIOS mode - verify boot is mounted
        if ! mountpoint -q /mnt/boot; then
            log_warn "/mnt/boot is not mounted. This is required for BIOS installations."
            read -rp "Press Enter to continue after mounting /mnt/boot: "
            if ! mountpoint -q /mnt/boot; then
                error_exit "/mnt/boot is still not mounted. Cannot proceed with BIOS installation."
            fi
        fi
    fi
    
    log_info "Essential mounts verified successfully"
}

# Auto-populate RAID devices for TUI mode
auto_populate_raid_devices() {
    log_info "Auto-populating RAID devices for TUI mode..."
    
    # Get all available disks
    local available_disks=()
    for disk in /dev/sd[a-z] /dev/nvme[0-9]n[0-9]; do
        if [ -b "$disk" ] && [ "$disk" != "$INSTALL_DISK" ]; then
            available_disks+=("$disk")
        fi
    done
    
    # Check minimum requirements
    if [ ${#available_disks[@]} -lt 1 ]; then 
        error_exit "RAID requires at least 2 disks, but only ${#available_disks[@]} disk(s) found: ${available_disks[*]}"
    fi
    
    # Initialize RAID_DEVICES with the primary install disk
    RAID_DEVICES=("$INSTALL_DISK")
    
    # Add additional disks
    for disk in "${available_disks[@]}"; do
        if [ "$disk" != "$INSTALL_DISK" ]; then
            RAID_DEVICES+=("$disk")
        fi
    done
    
    log_info "RAID devices populated: ${RAID_DEVICES[*]}"
    export RAID_DEVICES
}

# Log completion message
log_partitioning_complete() {
    local strategy="$1"
    log_info "$strategy partitioning complete. Filesystems formatted and mounted."
}
