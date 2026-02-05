#!/bin/bash
# disk_utils.sh - Disk partitioning and formatting utilities

set -euo pipefail

# --- Source-Once Guard ---
if [[ -n "${_DISK_UTILS_SH_SOURCED:-}" ]]; then
    # shellcheck disable=SC2317
    return 0 2>/dev/null || true
fi
readonly _DISK_UTILS_SH_SOURCED=1

# Source utilities (required dependency)
_SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -z "${_UTILS_SH_SOURCED:-}" ]]; then
    # Use source_or_die pattern inline since utils.sh defines it
    if [[ ! -f "${_SCRIPT_DIR}/utils.sh" ]]; then
        echo "FATAL: Required script not found: ${_SCRIPT_DIR}/utils.sh" >&2
        exit 1
    fi
    # shellcheck source=/dev/null
    if ! source "${_SCRIPT_DIR}/utils.sh"; then
        echo "FATAL: Failed to source: ${_SCRIPT_DIR}/utils.sh" >&2
        exit 1
    fi
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
    # shellcheck disable=SC2120
    local ram_gb="${1:-${RAM_GB:-}}"

    # Handle both "16" and "16G" formats
    local ram_val="${ram_gb%[Gg]*}"

    # Fallback if not a number or empty
    if [[ -z "$ram_val" || ! "$ram_val" =~ ^[0-9]+$ ]]; then
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

# --- Disk Type Detection ---
# Functions for detecting SSD vs HDD for appropriate wipe/optimization strategies
# Reference: https://wiki.archlinux.org/title/Solid_state_drive

# Get the base device name (strip partition numbers, handle nvme)
# /dev/sda1 -> sda, /dev/nvme0n1p1 -> nvme0n1
get_base_device_name() {
    local device="$1"
    local base_name

    # Remove /dev/ prefix
    base_name="${device#/dev/}"

    # Handle NVMe devices: nvme0n1p1 -> nvme0n1
    if [[ "$base_name" =~ ^(nvme[0-9]+n[0-9]+) ]]; then
        echo "${BASH_REMATCH[1]}"
        return
    fi

    # Handle standard devices: sda1 -> sda, vda2 -> vda
    # Strip trailing numbers (partition numbers)
    echo "${base_name%%[0-9]*}"
}

# Check if a disk is rotational (HDD) or non-rotational (SSD/NVMe)
# Returns: 0 if SSD, 1 if HDD, 2 if unknown
is_ssd() {
    local device="$1"
    local base_name
    local rotational_file

    base_name=$(get_base_device_name "$device")
    rotational_file="/sys/block/${base_name}/queue/rotational"

    if [[ -f "$rotational_file" ]]; then
        local rotational
        rotational=$(cat "$rotational_file")
        if [[ "$rotational" == "0" ]]; then
            return 0  # SSD (non-rotational)
        else
            return 1  # HDD (rotational)
        fi
    fi

    # Fallback: NVMe devices are always SSDs
    if [[ "$base_name" =~ ^nvme ]]; then
        return 0
    fi

    # Unknown - treat as HDD for safety (zeros won't hurt)
    return 2
}

# Get human-readable disk type
get_disk_type() {
    local device="$1"

    if is_ssd "$device"; then
        echo "SSD"
    else
        local ret=$?
        if [[ $ret -eq 1 ]]; then
            echo "HDD"
        else
            echo "Unknown"
        fi
    fi
}

# Check if device supports TRIM/discard (for blkdiscard)
supports_discard() {
    local device="$1"
    local base_name
    local discard_file

    # Check if blkdiscard is available
    if ! command -v blkdiscard >/dev/null 2>&1; then
        return 1
    fi

    base_name=$(get_base_device_name "$device")
    discard_file="/sys/block/${base_name}/queue/discard_max_bytes"

    if [[ -f "$discard_file" ]]; then
        local discard_max
        discard_max=$(cat "$discard_file")
        if [[ "$discard_max" -gt 0 ]]; then
            return 0
        fi
    fi

    return 1
}

# --- Partitioning Functions ---

wipe_disk() {
    local disk="$1"
    local confirmation="${2:-}"

    # ENVIRONMENT CONTRACT: Require explicit confirmation
    # Either passed as argument or via CONFIRM_WIPE_DISK env var
    if [[ "$confirmation" != "CONFIRMED" && "${CONFIRM_WIPE_DISK:-}" != "yes" ]]; then
        log_error "wipe_disk() requires explicit confirmation"
        log_error "Pass 'CONFIRMED' as second arg or set CONFIRM_WIPE_DISK=yes"
        return 1
    fi

    log_warn "DESTRUCTIVE: Wiping all data on $disk..."

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

create_boot_partition() {
    local disk="$1"
    local part_num="$2"
    local size_mib="${3:-$BOOT_PART_SIZE_MIB}"

    local part_device
    part_device=$(get_partition_path "$disk" "$part_num")

    log_info "Creating boot partition: $part_device (${size_mib}MiB, ext4)"

    # Use standard Linux partition type (8300), NOT XBOOTLDR (EA00)
    sgdisk -n "${part_num}:0:+${size_mib}M" -t "${part_num}:${LINUX_PARTITION_TYPE}" -c "${part_num}:BOOT" "$disk"

    # Wait for device node
    sleep 1

    # Format as ext4 (standard for /boot)
    mkfs.ext4 -L BOOT "$part_device"
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
    local mapper_name="${2:-cryptroot}"

    # Password comes from environment (secure - not in command line)
    local password="${ENCRYPTION_PASSWORD:-}"

    if [[ -z "$password" ]]; then
        log_error "LUKS password is empty (set ENCRYPTION_PASSWORD environment variable)"
        return 1
    fi

    log_info "Encrypting $partition with LUKS2 (mapper: $mapper_name)"

    # Format with password from stdin (LUKS2 with argon2id)
    echo -n "$password" | cryptsetup luksFormat \
        --type luks2 \
        --cipher aes-xts-plain64 \
        --key-size 512 \
        --hash sha256 \
        --pbkdf argon2id \
        --batch-mode \
        "$partition" -

    # Open mapping
    echo -n "$password" | cryptsetup open "$partition" "$mapper_name" -

    # Return the mapper device path (CRITICAL for callers)
    echo "/dev/mapper/$mapper_name"
}

setup_btrfs_subvolumes() {
    local device="$1"
    local include_home="${2:-no}"

    log_info "Setting up Btrfs subvolumes on $device"

    # Mount the device first to create subvolumes
    mount "$device" /mnt

    # Create standard subvolume layout
    btrfs subvolume create /mnt/@
    btrfs subvolume create /mnt/@var
    btrfs subvolume create /mnt/@tmp
    btrfs subvolume create /mnt/@snapshots
    btrfs subvolume create /mnt/@cache
    btrfs subvolume create /mnt/@log

    if [[ "$include_home" == "yes" ]]; then
        btrfs subvolume create /mnt/@home
    fi

    # Unmount to remount with proper subvolume
    umount /mnt

    # Mount root subvolume with compression and noatime
    mount -o compress=zstd,noatime,space_cache=v2,subvol=@ "$device" /mnt

    # Create mount point directories
    mkdir -p /mnt/{var,tmp,.snapshots,boot,efi,var/cache,var/log}

    # Mount other subvolumes
    mount -o compress=zstd,noatime,space_cache=v2,subvol=@var "$device" /mnt/var
    mount -o compress=zstd,noatime,space_cache=v2,subvol=@tmp "$device" /mnt/tmp
    mount -o compress=zstd,noatime,space_cache=v2,subvol=@snapshots "$device" /mnt/.snapshots
    mount -o compress=zstd,noatime,space_cache=v2,subvol=@cache "$device" /mnt/var/cache
    mount -o compress=zstd,noatime,space_cache=v2,subvol=@log "$device" /mnt/var/log

    if [[ "$include_home" == "yes" ]]; then
        mkdir -p /mnt/home
        mount -o compress=zstd,noatime,space_cache=v2,subvol=@home "$device" /mnt/home
    fi

    log_success "Btrfs subvolumes created and mounted"
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
    # shellcheck disable=SC2120
    local config_file="${1:-}"
    # Basic check stub
    log_info "Validating partitioning requirements..."
}

# =============================================================================
# DUAL-BOOT DETECTION FUNCTIONS
# =============================================================================

# Detect existing EFI System Partition on any disk
# Returns: ESP device path if found, empty if not
detect_existing_esp() {
    local target_disk="${1:-}"

    log_info "Scanning for existing EFI System Partition..."

    # Find all ESP partitions (type EF00 / C12A7328-F81F-11D2-BA4B-00A0C93EC93B)
    local esp_devices
    esp_devices=$(lsblk -rno NAME,PARTTYPE 2>/dev/null | grep -i "c12a7328-f81f-11d2-ba4b-00a0c93ec93b" | awk '{print "/dev/"$1}')

    if [[ -z "$esp_devices" ]]; then
        log_info "No existing ESP found"
        echo ""
        return 1
    fi

    # If target disk specified, prefer ESP on that disk
    if [[ -n "$target_disk" ]]; then
        local disk_esp
        disk_esp=$(echo "$esp_devices" | grep "^${target_disk}" | head -1)
        if [[ -n "$disk_esp" ]]; then
            log_info "Found ESP on target disk: $disk_esp"
            echo "$disk_esp"
            return 0
        fi
    fi

    # Return first ESP found
    local first_esp
    first_esp=$(echo "$esp_devices" | head -1)
    log_info "Found existing ESP: $first_esp"
    echo "$first_esp"
    return 0
}

# Detect if Windows is installed (check for Windows Boot Manager in ESP)
# Returns: 0 if Windows found, 1 if not
detect_windows_installation() {
    local esp_device="${1:-}"

    # If no ESP provided, try to find one
    if [[ -z "$esp_device" ]]; then
        esp_device=$(detect_existing_esp)
    fi

    if [[ -z "$esp_device" || ! -b "$esp_device" ]]; then
        return 1
    fi

    # Mount ESP temporarily to check contents
    local temp_mount="/tmp/esp_check_$$"
    mkdir -p "$temp_mount"

    if ! mount -o ro "$esp_device" "$temp_mount" 2>/dev/null; then
        rmdir "$temp_mount" 2>/dev/null
        return 1
    fi

    local windows_found=1

    # Check for Windows Boot Manager
    if [[ -d "$temp_mount/EFI/Microsoft/Boot" ]] && [[ -f "$temp_mount/EFI/Microsoft/Boot/bootmgfw.efi" ]]; then
        log_info "Windows Boot Manager detected in ESP"
        windows_found=0
        export WINDOWS_DETECTED="yes"
        export WINDOWS_EFI_PATH="/EFI/Microsoft/Boot/bootmgfw.efi"
    fi

    umount "$temp_mount" 2>/dev/null
    rmdir "$temp_mount" 2>/dev/null

    return $windows_found
}

# Detect any other operating systems (for os-prober recommendation)
# Returns: 0 if other OS found, 1 if not
detect_other_os() {
    log_info "Scanning for other operating systems..."

    local other_os_found=1

    # Check for Windows
    if detect_windows_installation; then
        other_os_found=0
    fi

    # Check for other Linux installations (look for /etc/os-release in mounted partitions)
    local linux_roots
    linux_roots=$(lsblk -rno NAME,FSTYPE 2>/dev/null | grep -E "ext4|btrfs|xfs" | awk '{print "/dev/"$1}')

    for part in $linux_roots; do
        # Skip if it's our target disk
        if [[ "$part" == "${INSTALL_DISK}"* ]]; then
            continue
        fi

        local temp_mount="/tmp/linux_check_$$"
        mkdir -p "$temp_mount"

        if mount -o ro "$part" "$temp_mount" 2>/dev/null; then
            if [[ -f "$temp_mount/etc/os-release" ]]; then
                local os_name
                os_name=$(grep "^NAME=" "$temp_mount/etc/os-release" 2>/dev/null | cut -d= -f2 | tr -d '"')
                if [[ -n "$os_name" ]]; then
                    log_info "Found other Linux installation: $os_name on $part"
                    other_os_found=0
                    export OTHER_LINUX_DETECTED="yes"
                fi
            fi
            umount "$temp_mount" 2>/dev/null
        fi
        rmdir "$temp_mount" 2>/dev/null
    done

    if [[ $other_os_found -eq 0 ]]; then
        export OTHER_OS_DETECTED="yes"
        log_warn "Other OS detected - will enable os-prober for dual-boot"
    fi

    return $other_os_found
}

# Check if disk has existing partitions that should be preserved
# Returns: 0 if partitions found, 1 if disk is empty/safe to wipe
check_disk_has_data() {
    local disk="$1"

    local part_count
    part_count=$(lsblk -rno NAME "$disk" 2>/dev/null | wc -l)

    # Subtract 1 for the disk itself
    part_count=$((part_count - 1))

    if [[ $part_count -gt 0 ]]; then
        log_warn "Disk $disk has $part_count existing partition(s)"
        return 0
    fi

    return 1
}

# Mount standard partition layout (ESP + boot + root + optional home)
# Call this after partitioning to set up /mnt for pacstrap
mount_standard_partitions() {
    local disk="$1"
    local esp_part_num="${2:-1}"
    local boot_part_num="${3:-2}"
    local root_part_num="${4:-3}"
    local home_part_num="${5:-}"
    local use_existing_esp="${6:-no}"
    local existing_esp_device="${7:-}"

    log_info "Mounting partitions for installation..."

    # Get partition paths
    local root_device boot_device esp_device home_device
    root_device=$(get_partition_path "$disk" "$root_part_num")
    boot_device=$(get_partition_path "$disk" "$boot_part_num")

    if [[ "$use_existing_esp" == "yes" && -n "$existing_esp_device" ]]; then
        esp_device="$existing_esp_device"
        log_info "Using existing ESP: $esp_device"
    else
        esp_device=$(get_partition_path "$disk" "$esp_part_num")
    fi

    # Mount root first
    log_info "Mounting root: $root_device -> /mnt"
    safe_mount "$root_device" "/mnt"

    # Capture root UUID for bootloader config
    ROOT_UUID=$(get_device_uuid "$root_device")
    export ROOT_UUID

    # Create mount points
    mkdir -p /mnt/boot /mnt/efi

    # Mount boot
    log_info "Mounting boot: $boot_device -> /mnt/boot"
    safe_mount "$boot_device" "/mnt/boot"

    # Mount ESP
    log_info "Mounting ESP: $esp_device -> /mnt/efi"
    safe_mount "$esp_device" "/mnt/efi"

    # Capture ESP device for later
    export EFI_DEVICE="$esp_device"

    # Mount home if specified
    if [[ -n "$home_part_num" ]]; then
        home_device=$(get_partition_path "$disk" "$home_part_num")
        mkdir -p /mnt/home
        log_info "Mounting home: $home_device -> /mnt/home"
        safe_mount "$home_device" "/mnt/home"
    fi

    log_success "All partitions mounted"
}

# =============================================================================
# FILESYSTEM FORMATTING
# =============================================================================

format_filesystem() {
    local device="$1"
    local fs_type="${2:-ext4}"

    log_info "Formatting $device as $fs_type"

    case "$fs_type" in
        ext4)
            mkfs.ext4 -F "$device"
            ;;
        btrfs)
            mkfs.btrfs -f "$device"
            ;;
        xfs)
            mkfs.xfs -f "$device"
            ;;
        f2fs)
            mkfs.f2fs -f "$device"
            ;;
        vfat|fat32)
            mkfs.fat -F32 "$device"
            ;;
        *)
            log_error "Unknown filesystem type: $fs_type"
            return 1
            ;;
    esac

    log_success "Formatted $device as $fs_type"
}

# =============================================================================
# CRYPTTAB GENERATION
# =============================================================================

# Generate crypttab entry for LUKS device
# Called from LUKS strategies to enable boot-time unlocking
generate_crypttab() {
    local luks_device="$1"
    local mapper_name="$2"
    local options="${3:-luks}"

    if [[ -z "$luks_device" || -z "$mapper_name" ]]; then
        log_error "generate_crypttab: requires device and mapper_name"
        return 1
    fi

    # Get UUID of the LUKS container
    local luks_uuid
    luks_uuid=$(blkid -s UUID -o value "$luks_device" 2>/dev/null)

    if [[ -z "$luks_uuid" ]]; then
        log_error "Could not get UUID for $luks_device"
        return 1
    fi

    # Ensure /mnt/etc exists
    mkdir -p /mnt/etc

    # Append crypttab entry
    # Format: <name> <device> <keyfile> <options>
    # Using 'none' for keyfile means password prompt at boot
    echo "$mapper_name UUID=$luks_uuid none $options" >> /mnt/etc/crypttab

    log_info "Added crypttab entry: $mapper_name -> UUID=$luks_uuid"
}

# =============================================================================
# PARTITION SYNC AND VERIFICATION
# =============================================================================

# Sync partition table to kernel
sync_partitions() {
    local disk="$1"

    log_info "Syncing partition table for $disk"

    # Flush filesystem buffers
    sync

    # Inform kernel of partition table changes
    partprobe "$disk" 2>/dev/null || true

    # Give kernel time to create device nodes
    sleep 1

    # Alternative method if partprobe fails
    if ! lsblk "$disk" &>/dev/null; then
        blockdev --rereadpt "$disk" 2>/dev/null || true
        sleep 1
    fi

    log_info "Partition table synced"
}

# Verify essential mounts exist for installation
verify_essential_mounts() {
    local errors=0

    log_info "Verifying essential mounts..."

    # Root must be mounted
    if ! mountpoint -q /mnt; then
        log_error "Root filesystem not mounted at /mnt"
        errors=$((errors + 1))
    fi

    # Boot must be mounted (for kernels and initramfs)
    if ! mountpoint -q /mnt/boot; then
        log_error "Boot partition not mounted at /mnt/boot"
        errors=$((errors + 1))
    fi

    # ESP should be mounted for UEFI systems
    if [[ "${BOOT_MODE:-}" == "UEFI" ]]; then
        if ! mountpoint -q /mnt/efi; then
            log_error "EFI System Partition not mounted at /mnt/efi"
            errors=$((errors + 1))
        fi
    fi

    if [[ $errors -gt 0 ]]; then
        log_error "Essential mount verification failed with $errors error(s)"
        return 1
    fi

    log_success "All essential mounts verified"
    return 0
}

# Log partitioning completion with summary
log_partitioning_complete() {
    local strategy_name="$1"

    log_success "=== Partitioning Complete: $strategy_name ==="
    log_info "Mounted filesystems:"

    # Show mounted partitions under /mnt
    mount | grep " /mnt" | while read -r line; do
        log_info "  $line"
    done

    # Show exported UUIDs
    [[ -n "${ROOT_UUID:-}" ]] && log_info "ROOT_UUID: $ROOT_UUID"
    [[ -n "${LUKS_UUID:-}" ]] && log_info "LUKS_UUID: $LUKS_UUID"
    [[ -n "${SWAP_UUID:-}" ]] && log_info "SWAP_UUID: $SWAP_UUID"
    [[ -n "${EFI_DEVICE:-}" ]] && log_info "EFI_DEVICE: $EFI_DEVICE"
}

# =============================================================================
# ERROR RECOVERY / CLEANUP
# =============================================================================

# Cleanup function for partition strategy failures
# Call this in trap handler or on error
cleanup_partitioning() {
    log_warn "Cleaning up partitioning state..."

    # Unmount in reverse order (most nested first)
    local mount_points=(
        "/mnt/var/log"
        "/mnt/var/cache"
        "/mnt/.snapshots"
        "/mnt/tmp"
        "/mnt/var"
        "/mnt/home"
        "/mnt/boot"
        "/mnt/efi"
        "/mnt"
    )

    for mp in "${mount_points[@]}"; do
        if mountpoint -q "$mp" 2>/dev/null; then
            log_info "Unmounting $mp"
            umount -R "$mp" 2>/dev/null || umount -l "$mp" 2>/dev/null || true
        fi
    done

    # Close LUKS mappings
    for mapper in /dev/mapper/crypt*; do
        if [[ -e "$mapper" ]]; then
            local name
            name=$(basename "$mapper")
            log_info "Closing LUKS mapping: $name"
            cryptsetup close "$name" 2>/dev/null || true
        fi
    done

    # Deactivate LVM volume groups
    if command -v vgchange &>/dev/null; then
        vgchange -an 2>/dev/null || true
    fi

    # Stop any RAID arrays we may have created
    if command -v mdadm &>/dev/null; then
        for md in /dev/md*; do
            if [[ -b "$md" ]]; then
                log_info "Stopping RAID array: $md"
                mdadm --stop "$md" 2>/dev/null || true
            fi
        done
    fi

    # Turn off swap
    swapoff -a 2>/dev/null || true

    log_warn "Partitioning cleanup complete"
}

# Setup error trap for partition strategies
# Call this at the start of each strategy
setup_partitioning_trap() {
    trap 'cleanup_partitioning; exit 1' ERR
    trap 'cleanup_partitioning; exit 130' INT TERM
}
