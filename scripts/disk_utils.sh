#!/bin/bash
# disk_utils.sh - Common utilities and constants for disk partitioning strategies

set -euo pipefail

# Source utility functions
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
if [[ -f "$SCRIPT_DIR/utils.sh" ]]; then
    source "$SCRIPT_DIR/utils.sh"
fi

# --- Partition Configuration Constants ---
# These constants replace magic strings throughout the partitioning functions

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

# --- Global Variables for Device Information ---
declare -g ROOT_DEVICE=""
declare -g ROOT_UUID=""
declare -g EFI_DEVICE=""
declare -g EFI_UUID=""
declare -g BOOT_DEVICE=""
declare -g BOOT_UUID=""
declare -g XBOOTLDR_DEVICE=""
declare -g XBOOTLDR_UUID=""
declare -g SWAP_DEVICE=""
declare -g SWAP_UUID=""
declare -g HOME_DEVICE=""
declare -g HOME_UUID=""
declare -g LUKS_DEVICE=""
declare -g LUKS_UUID=""
declare -g -A LVM_DEVICES_MAP=()

# --- Common Partitioning Functions ---

# Get partition path based on disk type (NVMe, eMMC, vs regular)
get_partition_path() {
    local disk="$1"
    local part_num="$2"

    if [[ "$disk" =~ nvme|mmcblk|loop ]]; then
        # NVMe, eMMC, and loop devices: /dev/nvme0n1p1, /dev/mmcblk0p1, /dev/loop0p1
        echo "${disk}p${part_num}"
    else
        # Regular drives: /dev/sda1, /dev/sdb2, etc.
        echo "${disk}${part_num}"
    fi
}

# Wipe disk clean
# SECURITY: Requires explicit confirmation to prevent accidental data loss
wipe_disk() {
    local disk="$1"
    local confirmed="${2:-no}"

    # Safety check: require explicit confirmation parameter
    if [[ "$confirmed" != "CONFIRMED" ]]; then
        error_exit "CRITICAL: wipe_disk requires explicit 'CONFIRMED' parameter to prevent accidental data loss"
    fi

    log_warning "⚠️  DESTROYING ALL DATA ON $disk"
    log_info "Wiping disk: $disk"

    # Unmount any mounted partitions
    for part in "${disk}"*; do
        if mountpoint -q "$part" 2>/dev/null; then
            umount "$part" 2>/dev/null || true
        fi
    done

    # Wipe filesystem signatures
    wipefs -af "$disk" || error_exit "Failed to wipe disk $disk"

    # Zero out the beginning and end of disk for clean partition table
    dd if=/dev/zero of="$disk" bs=1M count=10 status=none 2>/dev/null || true
    dd if=/dev/zero of="$disk" bs=1M seek=$(($(blockdev --getsz "$disk") / 2048 - 10)) count=10 status=none 2>/dev/null || true

    # Inform kernel of partition changes
    partprobe "$disk" 2>/dev/null || true
    sleep 1

    log_success "Disk $disk wiped successfully"
}

# Create partition table (GPT for UEFI, MBR for BIOS)
create_partition_table() {
    local disk="$1"

    if [[ "${BOOT_MODE:-UEFI}" == "UEFI" ]]; then
        log_info "Creating GPT partition table on $disk"
        sgdisk -Z "$disk" || error_exit "Failed to create GPT label on $disk"
    else
        log_info "Creating MBR partition table on $disk"
        printf "o\nw\n" | fdisk "$disk" || error_exit "Failed to create MBR label on $disk"
    fi
    partprobe "$disk"
    sleep 1
}

# Create ESP partition (UEFI only)
create_esp_partition() {
    local disk="$1"
    local part_num="$2"
    local size_mib="${3:-$DEFAULT_ESP_SIZE_MIB}"

    log_info "Creating ESP partition (${size_mib}MiB) for /efi..."
    sgdisk -n "${part_num}:0:+${size_mib}M" -t "${part_num}:${EFI_PARTITION_TYPE}" -c "${part_num}:EFI" "$disk" || \
        error_exit "Failed to create ESP partition"
    partprobe "$disk"
    sleep 1

    local part_dev
    part_dev=$(get_partition_path "$disk" "$part_num")

    format_filesystem "$part_dev" "$EFI_FILESYSTEM"
    capture_device_info "efi" "$part_dev"

    # Mount ESP
    mkdir -p /mnt/efi
    safe_mount "$part_dev" "/mnt/efi"

    echo "$part_dev"
}

# Create XBOOTLDR partition (UEFI only) - for /boot
create_xbootldr_partition() {
    local disk="$1"
    local part_num="$2"
    local size_mib="${3:-1024}"

    log_info "Creating XBOOTLDR partition (${size_mib}MiB) for /boot..."
    sgdisk -n "${part_num}:0:+${size_mib}M" -t "${part_num}:${XBOOTLDR_PARTITION_TYPE}" -c "${part_num}:BOOT" "$disk" || \
        error_exit "Failed to create XBOOTLDR partition"
    partprobe "$disk"
    sleep 1

    local part_dev
    part_dev=$(get_partition_path "$disk" "$part_num")

    format_filesystem "$part_dev" "$BOOT_FILESYSTEM"
    capture_device_info "boot" "$part_dev"

    # Mount boot
    mkdir -p /mnt/boot
    safe_mount "$part_dev" "/mnt/boot"

    echo "$part_dev"
}

# Create boot partition (BIOS only)
create_boot_partition() {
    local disk="$1"
    local part_num="$2"
    local size_mib="${3:-1024}"

    log_info "Creating boot partition (${size_mib}MiB) for /boot..."

    if [[ "${BOOT_MODE:-UEFI}" == "UEFI" ]]; then
        # For UEFI, use GPT
        sgdisk -n "${part_num}:0:+${size_mib}M" -t "${part_num}:${LINUX_PARTITION_TYPE}" -c "${part_num}:BOOT" "$disk" || \
            error_exit "Failed to create boot partition"
    else
        # For BIOS, use fdisk
        printf "n\np\n%s\n\n+%sM\nw\n" "$part_num" "$size_mib" | fdisk "$disk" || \
            error_exit "Failed to create boot partition"
    fi
    partprobe "$disk"
    sleep 1

    local part_dev
    part_dev=$(get_partition_path "$disk" "$part_num")

    format_filesystem "$part_dev" "$BOOT_FILESYSTEM"
    capture_device_info "boot" "$part_dev"

    # Mount boot
    mkdir -p /mnt/boot
    safe_mount "$part_dev" "/mnt/boot"

    echo "$part_dev"
}

# Create BIOS boot partition (for GRUB on GPT disks in BIOS mode)
create_bios_boot_partition() {
    local disk="$1"
    local part_num="$2"

    log_info "Creating BIOS boot partition..."
    sgdisk -n "${part_num}:0:+${BIOS_BOOT_PART_SIZE_MIB}M" -t "${part_num}:${BIOS_BOOT_PARTITION_TYPE}" -c "${part_num}:BIOS" "$disk" || \
        error_exit "Failed to create BIOS boot partition"
    partprobe "$disk"
    sleep 1

    # BIOS boot partition is not formatted or mounted

    echo "$(get_partition_path "$disk" "$part_num")"
}

# Create swap partition
create_swap_partition() {
    local disk="$1"
    local part_num="$2"
    local size_mib="$3"

    if [[ "${WANT_SWAP:-no}" != "yes" ]]; then
        log_info "Swap partition not requested"
        return 0
    fi

    log_info "Creating swap partition (${size_mib}MiB)..."

    if [[ "${BOOT_MODE:-UEFI}" == "UEFI" ]]; then
        sgdisk -n "${part_num}:0:+${size_mib}M" -t "${part_num}:${SWAP_PARTITION_TYPE}" -c "${part_num}:SWAP" "$disk" || \
            error_exit "Failed to create swap partition"
    else
        printf "n\np\n%s\n\n+%sM\nt\n%s\n82\nw\n" "$part_num" "$size_mib" "$part_num" | fdisk "$disk" || \
            error_exit "Failed to create swap partition"
    fi
    partprobe "$disk"
    sleep 1

    local part_dev
    part_dev=$(get_partition_path "$disk" "$part_num")

    format_filesystem "$part_dev" "swap"
    capture_device_info "swap" "$part_dev"

    # Enable swap
    swapon "$part_dev"

    echo "$part_dev"
}

# Create root partition
create_root_partition() {
    local disk="$1"
    local part_num="$2"
    local filesystem="${3:-ext4}"

    log_info "Creating root partition with $filesystem filesystem..."

    if [[ "${BOOT_MODE:-UEFI}" == "UEFI" ]]; then
        sgdisk -n "${part_num}:0:0" -t "${part_num}:${LINUX_PARTITION_TYPE}" -c "${part_num}:ROOT" "$disk" || \
            error_exit "Failed to create root partition"
    else
        printf "n\np\n%s\n\n\nw\n" "$part_num" | fdisk "$disk" || \
            error_exit "Failed to create root partition"
    fi
    partprobe "$disk"
    sleep 1

    local part_dev
    part_dev=$(get_partition_path "$disk" "$part_num")

    format_filesystem "$part_dev" "$filesystem"
    capture_device_info "root" "$part_dev"

    # Mount root
    safe_mount "$part_dev" "/mnt"

    echo "$part_dev"
}

# Create home partition
create_home_partition() {
    local disk="$1"
    local part_num="$2"
    local filesystem="${3:-ext4}"

    if [[ "${WANT_HOME_PARTITION:-no}" != "yes" ]]; then
        log_info "Separate home partition not requested"
        return 0
    fi

    log_info "Creating home partition with $filesystem filesystem..."

    if [[ "${BOOT_MODE:-UEFI}" == "UEFI" ]]; then
        sgdisk -n "${part_num}:0:0" -t "${part_num}:${LINUX_PARTITION_TYPE}" -c "${part_num}:HOME" "$disk" || \
            error_exit "Failed to create home partition"
    else
        printf "n\np\n%s\n\n\nw\n" "$part_num" | fdisk "$disk" || \
            error_exit "Failed to create home partition"
    fi
    partprobe "$disk"
    sleep 1

    local part_dev
    part_dev=$(get_partition_path "$disk" "$part_num")

    format_filesystem "$part_dev" "$filesystem"
    capture_device_info "home" "$part_dev"

    # Mount home
    mkdir -p /mnt/home
    safe_mount "$part_dev" "/mnt/home"

    echo "$part_dev"
}

# Safe mount function
safe_mount() {
    local device="$1"
    local mountpoint="$2"
    local options="${3:-}"

    # Create mountpoint if it doesn't exist
    mkdir -p "$mountpoint"

    # Check if already mounted at this mountpoint
    if mountpoint -q "$mountpoint" 2>/dev/null; then
        log_info "$mountpoint is already mounted"
        return 0
    fi

    # Mount the device
    if [[ -n "$options" ]]; then
        mount -o "$options" "$device" "$mountpoint" || error_exit "Failed to mount $device to $mountpoint"
    else
        mount "$device" "$mountpoint" || error_exit "Failed to mount $device to $mountpoint"
    fi
    log_info "Mounted $device to $mountpoint"
}

# Capture device info (UUID, PARTUUID) for later use
capture_device_info() {
    local device_type="$1"
    local device_path="$2"

    if [[ -z "$device_type" || -z "$device_path" ]]; then
        log_warn "capture_device_info: missing device_type or device_path"
        return 1
    fi

    if [[ ! -b "$device_path" ]]; then
        log_warn "capture_device_info: $device_path is not a block device"
        return 1
    fi

    # Get UUID
    local uuid
    uuid=$(blkid -s UUID -o value "$device_path" 2>/dev/null) || true

    # Get PARTUUID (for GPT partitions)
    local partuuid
    partuuid=$(blkid -s PARTUUID -o value "$device_path" 2>/dev/null) || true

    case "$device_type" in
        "root")
            ROOT_DEVICE="$device_path"
            ROOT_UUID="$uuid"
            export ROOT_UUID ROOT_DEVICE
            log_info "Captured root device: $device_path (UUID: $uuid)"
            ;;
        "efi")
            EFI_DEVICE="$device_path"
            EFI_UUID="$uuid"
            export EFI_UUID EFI_DEVICE
            log_info "Captured EFI device: $device_path (UUID: $uuid)"
            ;;
        "boot"|"xbootldr")
            BOOT_DEVICE="$device_path"
            BOOT_UUID="$uuid"
            XBOOTLDR_DEVICE="$device_path"
            XBOOTLDR_UUID="$uuid"
            export BOOT_UUID BOOT_DEVICE XBOOTLDR_UUID XBOOTLDR_DEVICE
            log_info "Captured boot device: $device_path (UUID: $uuid)"
            ;;
        "swap")
            SWAP_DEVICE="$device_path"
            SWAP_UUID="$uuid"
            export SWAP_UUID SWAP_DEVICE
            log_info "Captured swap device: $device_path (UUID: $uuid)"
            ;;
        "home")
            HOME_DEVICE="$device_path"
            HOME_UUID="$uuid"
            export HOME_UUID HOME_DEVICE
            log_info "Captured home device: $device_path (UUID: $uuid)"
            ;;
        "luks")
            LUKS_DEVICE="$device_path"
            LUKS_UUID="$uuid"
            export LUKS_UUID LUKS_DEVICE
            log_info "Captured LUKS device: $device_path (UUID: $uuid)"
            ;;
        *)
            log_warn "Unknown device type: $device_type"
            return 1
            ;;
    esac

    return 0
}

# Validate partitioning requirements
validate_partitioning_requirements() {
    # Verify filesystem types are set
    if [[ -z "${ROOT_FILESYSTEM_TYPE:-}" ]]; then
        ROOT_FILESYSTEM_TYPE="${ROOT_FILESYSTEM:-ext4}"
        export ROOT_FILESYSTEM_TYPE
    fi
    if [[ -z "${HOME_FILESYSTEM_TYPE:-}" ]]; then
        HOME_FILESYSTEM_TYPE="${HOME_FILESYSTEM:-ext4}"
        export HOME_FILESYSTEM_TYPE
    fi

    # Verify disk exists
    if [[ ! -b "${INSTALL_DISK:-}" ]]; then
        error_exit "Installation disk not found: ${INSTALL_DISK:-not set}"
    fi

    log_info "Partitioning requirements validated"
}

# Get swap size in MiB
get_swap_size_mib() {
    case "${SWAP_SIZE:-2GB}" in
        "1GB") echo "1024" ;;
        "2GB") echo "2048" ;;
        "4GB") echo "4096" ;;
        "8GB") echo "8192" ;;
        "16GB") echo "16384" ;;
        "32GB") echo "32768" ;;
        *G|*GB)
            # Parse numeric value
            local size="${SWAP_SIZE//[!0-9]/}"
            echo "$((size * 1024))"
            ;;
        *M|*MB)
            local size="${SWAP_SIZE//[!0-9]/}"
            echo "$size"
            ;;
        *)
            echo "$DEFAULT_SWAP_SIZE_MIB"
            ;;
    esac
}

# Verify essential mounts for installation (non-interactive)
verify_essential_mounts() {
    local errors=0

    # Verify root is mounted
    if ! mountpoint -q /mnt; then
        log_error "/mnt is not mounted. Root partition must be mounted."
        errors=$((errors + 1))
    fi

    # Verify UEFI mounts if in UEFI mode
    if [[ "${BOOT_MODE:-UEFI}" == "UEFI" ]]; then
        if ! mountpoint -q /mnt/efi && ! mountpoint -q /mnt/boot/efi; then
            log_error "EFI partition is not mounted at /mnt/efi or /mnt/boot/efi"
            errors=$((errors + 1))
        fi

        if ! mountpoint -q /mnt/boot; then
            log_warn "/mnt/boot is not mounted (may be okay if using /mnt/efi for everything)"
        fi
    else
        # BIOS mode - verify boot is mounted
        if ! mountpoint -q /mnt/boot; then
            log_error "/mnt/boot is not mounted. Boot partition must be mounted for BIOS installations."
            errors=$((errors + 1))
        fi
    fi

    if [[ $errors -gt 0 ]]; then
        error_exit "Essential mounts verification failed with $errors error(s)"
    fi

    log_info "Essential mounts verified successfully"
}

# Auto-populate RAID devices for TUI mode
auto_populate_raid_devices() {
    log_info "Auto-populating RAID devices for TUI mode..."

    # Get all available disks
    local -a available_disks=()
    for disk in /dev/sd[a-z] /dev/nvme[0-9]n[0-9]; do
        if [[ -b "$disk" ]] && [[ "$disk" != "${INSTALL_DISK:-}" ]]; then
            available_disks+=("$disk")
        fi
    done

    # Check minimum requirements (RAID needs at least 2 disks total)
    if [[ ${#available_disks[@]} -lt 1 ]]; then
        error_exit "RAID requires at least 2 disks. Only found primary disk: ${INSTALL_DISK:-none}"
    fi

    # Initialize RAID_DEVICES with the primary install disk
    RAID_DEVICES=("${INSTALL_DISK:-}")

    # Add additional disks
    for disk in "${available_disks[@]}"; do
        RAID_DEVICES+=("$disk")
    done

    log_info "RAID devices populated: ${RAID_DEVICES[*]}"
    export RAID_DEVICES
}

# Log completion message
log_partitioning_complete() {
    local strategy="$1"
    log_success "$strategy partitioning complete. Filesystems formatted and mounted."
}

# Setup LUKS encryption with password from variable
# Security: Uses file descriptor to avoid exposing password in process list
setup_luks_encryption() {
    local device="$1"
    local mapper_name="${2:-cryptroot}"

    if [[ -z "${ENCRYPTION_PASSWORD:-}" ]]; then
        error_exit "LUKS encryption requested but ENCRYPTION_PASSWORD is not set"
    fi

    log_info "Setting up LUKS encryption on $device..."

    # Format LUKS container using process substitution to avoid password exposure
    # This prevents the password from appearing in process list or environment
    cryptsetup luksFormat --type luks2 --batch-mode --key-file=<(echo -n "$ENCRYPTION_PASSWORD") "$device" || \
        error_exit "Failed to format LUKS container on $device"

    # Open LUKS container using same secure method
    cryptsetup open --key-file=<(echo -n "$ENCRYPTION_PASSWORD") "$device" "$mapper_name" || \
        error_exit "Failed to open LUKS container on $device"

    # Clear password from memory after use for additional security
    local temp_clear="$ENCRYPTION_PASSWORD"
    ENCRYPTION_PASSWORD=""
    temp_clear=""

    # Capture LUKS UUID for crypttab
    capture_device_info "luks" "$device"

    log_info "LUKS encryption set up successfully on $device"
    echo "/dev/mapper/$mapper_name"
}

# Generate crypttab entry
generate_crypttab() {
    local luks_device="$1"
    local mapper_name="${2:-cryptroot}"

    if [[ -z "${LUKS_UUID:-}" ]]; then
        LUKS_UUID=$(blkid -s UUID -o value "$luks_device" 2>/dev/null) || true
    fi

    if [[ -n "$LUKS_UUID" ]]; then
        log_info "Generating crypttab entry for $mapper_name"
        echo "$mapper_name UUID=$LUKS_UUID none luks" >> /mnt/etc/crypttab
    else
        log_warn "Could not determine LUKS UUID for crypttab"
    fi
}

# Setup Btrfs subvolumes with proper layout
# This creates the recommended @ subvolume layout and remounts properly
setup_btrfs_subvolumes() {
    local device="$1"
    local include_home="${2:-no}"  # Whether to create @home subvolume

    log_info "Setting up Btrfs subvolumes on $device..."

    # Mount the root of the Btrfs filesystem temporarily
    mount "$device" /mnt || error_exit "Failed to mount $device for Btrfs setup"

    # Create subvolumes
    btrfs subvolume create /mnt/@ || error_exit "Failed to create @ subvolume"
    log_info "Created @ subvolume (root)"

    if [[ "$include_home" == "yes" ]]; then
        btrfs subvolume create /mnt/@home || error_exit "Failed to create @home subvolume"
        log_info "Created @home subvolume"
    fi

    btrfs subvolume create /mnt/@var || error_exit "Failed to create @var subvolume"
    log_info "Created @var subvolume"

    btrfs subvolume create /mnt/@tmp || error_exit "Failed to create @tmp subvolume"
    log_info "Created @tmp subvolume"

    btrfs subvolume create /mnt/@snapshots || error_exit "Failed to create @snapshots subvolume"
    log_info "Created @snapshots subvolume"

    # Unmount to remount with subvolume options
    umount /mnt || error_exit "Failed to unmount after creating subvolumes"

    # Mount @ subvolume as root with recommended options
    local btrfs_opts="subvol=@,compress=zstd,noatime"
    mount -o "$btrfs_opts" "$device" /mnt || error_exit "Failed to mount @ subvolume"
    log_info "Mounted @ subvolume to /mnt"

    # Create mount points and mount other subvolumes
    mkdir -p /mnt/{home,var,tmp,.snapshots}

    if [[ "$include_home" == "yes" ]]; then
        mount -o "subvol=@home,compress=zstd,noatime" "$device" /mnt/home || \
            error_exit "Failed to mount @home subvolume"
        log_info "Mounted @home subvolume to /mnt/home"
    fi

    mount -o "subvol=@var,compress=zstd,noatime" "$device" /mnt/var || \
        error_exit "Failed to mount @var subvolume"
    log_info "Mounted @var subvolume to /mnt/var"

    mount -o "subvol=@tmp,compress=zstd,noatime" "$device" /mnt/tmp || \
        error_exit "Failed to mount @tmp subvolume"
    log_info "Mounted @tmp subvolume to /mnt/tmp"

    mount -o "subvol=@snapshots,compress=zstd,noatime" "$device" /mnt/.snapshots || \
        error_exit "Failed to mount @snapshots subvolume"
    log_info "Mounted @snapshots subvolume to /mnt/.snapshots"

    log_success "Btrfs subvolumes configured successfully"
}

# Generate fstab entries for Btrfs subvolumes
generate_btrfs_fstab() {
    local device="$1"
    local uuid
    uuid=$(blkid -s UUID -o value "$device" 2>/dev/null) || true

    if [[ -z "$uuid" ]]; then
        log_warn "Could not get UUID for Btrfs device $device"
        return 1
    fi

    log_info "Generating fstab entries for Btrfs subvolumes..."

    # These will be appended to fstab (genfstab might miss subvolume options)
    cat >> /mnt/etc/fstab << EOF
# Btrfs subvolume mounts
UUID=$uuid  /           btrfs   subvol=@,compress=zstd,noatime           0 0
UUID=$uuid  /var        btrfs   subvol=@var,compress=zstd,noatime        0 0
UUID=$uuid  /tmp        btrfs   subvol=@tmp,compress=zstd,noatime        0 0
UUID=$uuid  /.snapshots btrfs   subvol=@snapshots,compress=zstd,noatime  0 0
EOF

    # Add @home if it exists
    if btrfs subvolume list /mnt | grep -q "@home"; then
        echo "UUID=$uuid  /home       btrfs   subvol=@home,compress=zstd,noatime       0 0" >> /mnt/etc/fstab
    fi

    log_success "Btrfs fstab entries generated"
}
