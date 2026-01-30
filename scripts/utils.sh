#!/bin/bash
# utils.sh - Utility functions for Arch Linux installer

set -euo pipefail

# =============================================================================
# COLOR DEFINITIONS
# =============================================================================
# ANSI color codes for terminal output
# Usage: echo -e "${COLOR_RED}Error message${COLOR_RESET}"

declare -A COLORS=(
    [RESET]='\033[0m'
    [BOLD]='\033[1m'
    [DIM]='\033[2m'
    # Standard colors
    [BLACK]='\033[30m'
    [RED]='\033[31m'
    [GREEN]='\033[32m'
    [YELLOW]='\033[33m'
    [BLUE]='\033[34m'
    [MAGENTA]='\033[35m'
    [CYAN]='\033[36m'
    [WHITE]='\033[37m'
    # Bright colors
    [BRIGHT_RED]='\033[91m'
    [BRIGHT_GREEN]='\033[92m'
    [BRIGHT_YELLOW]='\033[93m'
    [BRIGHT_BLUE]='\033[94m'
    [BRIGHT_MAGENTA]='\033[95m'
    [BRIGHT_CYAN]='\033[96m'
    [BRIGHT_WHITE]='\033[97m'
)

# Semantic color aliases for log levels
declare -A LOG_COLORS=(
    [DEBUG]="${COLORS[DIM]}${COLORS[WHITE]}"
    [INFO]="${COLORS[WHITE]}"
    [WARN]="${COLORS[YELLOW]}"
    [WARNING]="${COLORS[YELLOW]}"
    [ERROR]="${COLORS[BRIGHT_RED]}"
    [SUCCESS]="${COLORS[BRIGHT_GREEN]}"
    [PHASE]="${COLORS[BRIGHT_CYAN]}"
    [COMMAND]="${COLORS[DIM]}${COLORS[CYAN]}"
)

# =============================================================================
# LOGGING CONFIGURATION
# =============================================================================

LOG_FILE="/tmp/archinstall.log"

# Enhanced logging with automatic log file creation
setup_logging() {
    # Create log file
    {
        echo "=========================================="
        echo "ArchInstall Log - $(date)"
        echo "System: $(uname -a)"
        echo "User: $(whoami)"
        echo "Working Directory: $(pwd)"
        echo "=========================================="
        echo ""
    } > "$LOG_FILE" 2>/dev/null || {
        echo "Warning: Could not create log file, logging to stdout only"
    }
}

# =============================================================================
# LOGGING FUNCTIONS
# =============================================================================
# All log functions output colored text to terminal and plain text to log file

log_debug() {
    if [[ "${LOG_LEVEL:-INFO}" == "DEBUG" ]]; then
        local message="$1"
        local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
        echo -e "${LOG_COLORS[DEBUG]}[$timestamp] DEBUG: $message${COLORS[RESET]}"
        echo "[$timestamp] DEBUG: $message" >> "$LOG_FILE" 2>/dev/null || true
    fi
}

log_info() {
    local message="$1"
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    echo -e "${LOG_COLORS[INFO]}[$timestamp] INFO: $message${COLORS[RESET]}"
    echo "[$timestamp] INFO: $message" >> "$LOG_FILE" 2>/dev/null || true
}

log_warn() {
    local message="$1"
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    echo -e "${LOG_COLORS[WARN]}[$timestamp] WARN: $message${COLORS[RESET]}" >&2
    echo "[$timestamp] WARN: $message" >> "$LOG_FILE" 2>/dev/null || true
}

# Alias for consistency
log_warning() {
    log_warn "$1"
}

log_error() {
    local message="$1"
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    echo -e "${LOG_COLORS[ERROR]}[$timestamp] ERROR: $message${COLORS[RESET]}" >&2
    echo "[$timestamp] ERROR: $message" >> "$LOG_FILE" 2>/dev/null || true
}

log_success() {
    local message="$1"
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    echo -e "${LOG_COLORS[SUCCESS]}[$timestamp] SUCCESS: $message${COLORS[RESET]}"
    echo "[$timestamp] SUCCESS: $message" >> "$LOG_FILE" 2>/dev/null || true
}

# Log a phase/section header (bright cyan, bold)
log_phase() {
    local message="$1"
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    echo -e "${COLORS[BOLD]}${LOG_COLORS[PHASE]}[$timestamp] === $message ===${COLORS[RESET]}"
    echo "[$timestamp] === $message ===" >> "$LOG_FILE" 2>/dev/null || true
}

# Log a command being executed (dimmed)
log_cmd() {
    local message="$1"
    echo -e "${LOG_COLORS[COMMAND]}  > $message${COLORS[RESET]}"
    echo "  > $message" >> "$LOG_FILE" 2>/dev/null || true
}

# Error handling
error_exit() {
    local message="$1"
    local command="${2:-}"
    local script_name="$(basename "$0")"
    local line_number="${BASH_LINENO[1]}"
    
    log_error "ERROR in $script_name at line $line_number: $message"
    if [[ -n "$command" ]]; then
        log_error "Failed command: $command"
    fi
    exit 1
}

# Non-critical error handling (log and continue)
log_and_continue() {
    local message="$1"
    local command="$2"
    local script_name="$(basename "$0")"
    local line_number="${BASH_LINENO[1]}"
    
    log_warn "NON-CRITICAL ERROR in $script_name at line $line_number: $message"
    if [[ -n "$command" ]]; then
        log_warn "Failed command: $command"
    fi
    log_warn "Continuing installation..."
}

# Non-critical command execution helper
execute_non_critical() {
    local description="$1"
    shift
    local command=("$@")
    
    log_info "Executing non-critical command: $description"
    log_debug "Command: ${command[*]}"
    
    if ! "${command[@]}"; then
        log_and_continue "Non-critical command failed: $description" "${command[*]}"
        return 1
    fi
    
    log_success "Non-critical command completed: $description"
    return 0
}

# Pre-flight system checks
perform_preflight_checks() {
    log_info "Performing pre-flight system checks..."
    
    # Check if running as root
    if [[ $EUID -ne 0 ]]; then
        error_exit "This script must be run as root"
    fi
    
    # Check internet connectivity
    log_info "Checking internet connectivity..."
    if ! ping -c 1 archlinux.org >/dev/null 2>&1; then
        error_exit "No internet connectivity. Please check your network connection."
    fi
    
    # Wait for network time sync (like official archinstall)
    log_info "Waiting for network time synchronization..."
    local max_wait=60
    local waited=0
    while ! timedatectl show --property=NTPSynchronized --value | grep -q "yes"; do
        if [[ $waited -ge $max_wait ]]; then
            log_warn "Network time sync timeout after ${max_wait}s, continuing anyway..."
            break
        fi
        log_info "Waiting for NTP sync... (${waited}/${max_wait}s)"
        sleep 2
        ((waited += 2))
    done
    
    if timedatectl show --property=NTPSynchronized --value | grep -q "yes"; then
        log_success "Network time synchronized"
    else
        log_warn "Network time not synchronized, but continuing..."
    fi
    
    # Update mirrorlist
    log_info "Updating mirrorlist..."
    if execute_non_critical "Mirrorlist update" pacman -Sy; then
        log_success "Mirrorlist updated successfully"
    else
        log_warn "Mirrorlist update failed, continuing with existing mirrors"
    fi
    
    # Check available disk space
    log_info "Checking available disk space..."
    local available_space=$(df / | awk 'NR==2 {print $4}')
    local min_space=$((2 * 1024 * 1024)) # 2GB in KB
    
    if [[ $available_space -lt $min_space ]]; then
        log_warn "Low disk space: ${available_space}KB available (minimum: ${min_space}KB)"
        log_warn "Installation may fail due to insufficient space"
    else
        log_success "Sufficient disk space available: ${available_space}KB"
    fi
    
    # Check if we're in a live environment
    if [[ -f "/etc/arch-release" ]] && [[ ! -d "/mnt" ]] || [[ -f "/run/archiso/bootmnt/arch/boot/x86_64/vmlinuz-linux" ]]; then
        log_success "Running in Arch Linux live environment"
    else
        log_warn "Not running in Arch Linux live environment - proceed with caution"
    fi
    
    log_success "Pre-flight checks completed"
}

# Validation functions
# SECURITY: Enhanced disk validation with path canonicalization and whitelisting
validate_disk() {
    local disk="$1"

    if [ -z "$disk" ]; then
        log_error "Disk path is empty"
        return 1
    fi

    # Canonicalize path to prevent symlink/path traversal attacks
    local canonical_disk
    canonical_disk=$(readlink -f "$disk" 2>/dev/null) || {
        log_error "Failed to canonicalize disk path: $disk"
        return 1
    }

    # Whitelist allowed device patterns for security
    case "$canonical_disk" in
        /dev/sd[a-z]|/dev/sd[a-z][a-z])
            log_debug "Valid SATA/SCSI disk: $canonical_disk"
            ;;
        /dev/nvme[0-9]n[0-9]|/dev/nvme[0-9][0-9]n[0-9])
            log_debug "Valid NVMe disk: $canonical_disk"
            ;;
        /dev/vd[a-z]|/dev/vd[a-z][a-z])
            log_debug "Valid virtio disk: $canonical_disk"
            ;;
        /dev/mmcblk[0-9]|/dev/mmcblk[0-9][0-9])
            log_debug "Valid MMC/SD disk: $canonical_disk"
            ;;
        /dev/loop[0-9]|/dev/loop[0-9][0-9])
            log_warn "Loop device detected: $canonical_disk (allowed for testing)"
            ;;
        *)
            error_exit "Invalid or unsafe disk path: $canonical_disk (must be /dev/sd*, /dev/nvme*, /dev/vd*, or /dev/mmcblk*)"
            ;;
    esac

    # Verify it's a block device
    if [ ! -b "$canonical_disk" ]; then
        log_error "Disk $canonical_disk does not exist or is not a block device"
        return 1
    fi

    # Additional disk validation
    log_debug "Validating disk: $canonical_disk"

    # Check if disk is mounted
    if mountpoint -q "$canonical_disk" 2>/dev/null; then
        error_exit "Disk $canonical_disk is currently mounted and cannot be used for installation"
    fi

    # Check if any partitions on disk are mounted
    local mounted_parts=$(lsblk -n -o MOUNTPOINT "$canonical_disk" 2>/dev/null | grep -v "^$" | wc -l)
    if [[ $mounted_parts -gt 0 ]]; then
        log_warn "Some partitions on $canonical_disk are mounted - this may cause issues"
    fi

    log_success "Disk validation passed: $canonical_disk"
    
    # Check disk size (minimum 8GB)
    local disk_size_bytes=$(lsblk -b -d -n -o SIZE "$disk" 2>/dev/null)
    local min_size_bytes=$((8 * 1024 * 1024 * 1024)) # 8GB
    
    if [[ -n "$disk_size_bytes" ]] && [[ $disk_size_bytes -lt $min_size_bytes ]]; then
        log_warn "Disk $disk is small (${disk_size_bytes} bytes), minimum recommended is ${min_size_bytes} bytes"
    fi
    
    log_info "Disk $disk validated successfully"
    return 0
}

# SECURITY: Enhanced username validation to prevent injection attacks
validate_username() {
    local username="$1"

    if [ -z "$username" ]; then
        log_error "Username is empty"
        return 1
    fi

    # Strict validation: lowercase alphanumeric, underscore, hyphen only
    # Must start with lowercase letter or underscore
    # Max 32 characters (Linux limit)
    if ! echo "$username" | grep -qE '^[a-z_][a-z0-9_-]{0,31}$'; then
        error_exit "Invalid username: '$username' (must be lowercase alphanumeric with _ or -, start with letter or _, max 32 chars)"
    fi

    # Reserved system usernames
    case "$username" in
        root|daemon|bin|sys|sync|games|man|lp|mail|news|uucp|proxy|www-data|backup|list|irc|gnats|nobody)
            error_exit "Username '$username' is reserved by the system"
            ;;
    esac

    log_success "Username $username validated successfully"
    return 0
}

# SECURITY: Enhanced hostname validation
validate_hostname() {
    local hostname="$1"

    if [ -z "$hostname" ]; then
        log_error "Hostname is empty"
        return 1
    fi

    # Strict validation: lowercase alphanumeric and hyphens only
    # Cannot start or end with hyphen
    # Max 63 characters per RFC 1035
    if ! echo "$hostname" | grep -qE '^[a-z0-9]([a-z0-9-]{0,61}[a-z0-9])?$'; then
        error_exit "Invalid hostname: '$hostname' (must be lowercase alphanumeric with hyphens, max 63 chars, cannot start/end with hyphen)"
    fi

    # Hostname cannot be 'localhost' or numeric-only
    if [[ "$hostname" == "localhost" ]] || echo "$hostname" | grep -qE '^[0-9]+$'; then
        error_exit "Invalid hostname: '$hostname' (cannot be 'localhost' or numeric-only)"
    fi

    log_success "Hostname $hostname validated successfully"
    return 0
}

# Device and UUID management functions
get_device_uuid() {
    local device="$1"
    if [ -z "$device" ]; then
        log_error "Device path is empty"
        return 1
    fi
    
    if [ ! -b "$device" ]; then
        log_error "Device $device does not exist or is not a block device"
        return 1
    fi
    
    local uuid=$(blkid -s UUID -o value "$device" 2>/dev/null)
    if [ -z "$uuid" ]; then
        log_error "Could not get UUID for device $device"
        return 1
    fi
    
    echo "$uuid"
}

get_device_partuuid() {
    local device="$1"
    if [ -z "$device" ]; then
        log_error "Device path is empty"
        return 1
    fi
    
    if [ ! -b "$device" ]; then
        log_error "Device $device does not exist or is not a block device"
        return 1
    fi
    
    local partuuid=$(blkid -s PARTUUID -o value "$device" 2>/dev/null)
    if [ -z "$partuuid" ]; then
        log_error "Could not get PARTUUID for device $device"
        return 1
    fi
    
    echo "$partuuid"
}

# Global variables for storing device information
declare -g ROOT_DEVICE=""
declare -g ROOT_UUID=""
declare -g EFI_DEVICE=""
declare -g EFI_UUID=""
declare -g XBOOTLDR_DEVICE=""
declare -g XBOOTLDR_UUID=""
declare -g SWAP_DEVICE=""
declare -g SWAP_UUID=""
declare -g HOME_DEVICE=""
declare -g HOME_UUID=""

# Function to capture device information
capture_device_info() {
    local device_type="$1"  # "root", "efi", "xbootldr", "swap", "home"
    local device_path="$2"
    
    if [ -z "$device_type" ] || [ -z "$device_path" ]; then
        log_error "Device type and path are required"
        return 1
    fi
    
    local uuid=$(get_device_uuid "$device_path")
    if [ $? -ne 0 ]; then
        log_error "Failed to get UUID for $device_type device $device_path"
        return 1
    fi
    
    case "$device_type" in
        "root")
            ROOT_DEVICE="$device_path"
            ROOT_UUID="$uuid"
            log_info "Captured root device: $device_path (UUID: $uuid)"
            ;;
        "efi")
            EFI_DEVICE="$device_path"
            EFI_UUID="$uuid"
            log_info "Captured EFI device: $device_path (UUID: $uuid)"
            ;;
        "xbootldr")
            XBOOTLDR_DEVICE="$device_path"
            XBOOTLDR_UUID="$uuid"
            log_info "Captured XBOOTLDR device: $device_path (UUID: $uuid)"
            ;;
        "swap")
            SWAP_DEVICE="$device_path"
            SWAP_UUID="$uuid"
            log_info "Captured swap device: $device_path (UUID: $uuid)"
            ;;
        "home")
            HOME_DEVICE="$device_path"
            HOME_UUID="$uuid"
            log_info "Captured home device: $device_path (UUID: $uuid)"
            ;;
        *)
            log_error "Unknown device type: $device_type"
            return 1
            ;;
    esac
    
    return 0
}

# Package dependency checking and installation
check_and_install_dependencies() {
    log_info "Checking required packages for installation..."

    local required_packages=(
        "dosfstools"      # For mkfs.fat (FAT32 formatting)
        "exfatprogs"      # For exFAT support and FAT32 utilities
        "e2fsprogs"       # For mkfs.ext4
        "xfsprogs"        # For mkfs.xfs
        "btrfs-progs"     # For mkfs.btrfs
        "parted"          # For disk partitioning
        "gptfdisk"        # For sgdisk (GPT partitioning)
        "lvm2"            # For LVM operations
        "mdadm"           # For RAID operations
        "cryptsetup"      # For LUKS encryption
        "grub"            # For GRUB bootloader
        "efibootmgr"      # For UEFI boot management
    )

    local missing_packages=()

    log_info "Checking ${#required_packages[@]} required packages..."
    for package in "${required_packages[@]}"; do
        if ! pacman -Qi "$package" &>/dev/null; then
            echo -e "${LOG_COLORS[WARN]}  Package missing: $package${COLORS[RESET]}"
            missing_packages+=("$package")
        else
            echo -e "${LOG_COLORS[SUCCESS]}  Package OK: $package${COLORS[RESET]}"
        fi
    done

    if [ ${#missing_packages[@]} -gt 0 ]; then
        log_info "Installing ${#missing_packages[@]} missing packages: ${missing_packages[*]}"
        pacman -Sy --noconfirm "${missing_packages[@]}" 2>&1 | while IFS= read -r line; do
            case "$line" in
                *"error"*|*"Error"*|*"ERROR"*)
                    echo -e "${LOG_COLORS[ERROR]}  [pacman] $line${COLORS[RESET]}"
                    ;;
                *"warning"*|*"Warning"*|*"WARNING"*)
                    echo -e "${LOG_COLORS[WARN]}  [pacman] $line${COLORS[RESET]}"
                    ;;
                *)
                    echo -e "${LOG_COLORS[COMMAND]}  [pacman] $line${COLORS[RESET]}"
                    ;;
            esac
        done
        if [[ ${PIPESTATUS[0]} -ne 0 ]]; then
            log_error "Failed to install required packages: ${missing_packages[*]}"
            return 1
        fi
        log_success "All required packages installed successfully"
    else
        log_success "All ${#required_packages[@]} required packages are already installed"
    fi

    return 0
}

# Check for specific package before running commands
check_package_available() {
    local package="$1"
    local command="$2"
    
    if ! pacman -Qi "$package" &>/dev/null; then
        log_error "Package '$package' is required for '$command' but not installed"
        log_info "Installing $package..."
        pacman -Sy --noconfirm "$package" || {
            log_error "Failed to install $package"
            return 1
        }
    fi
    
    return 0
}

# Format filesystem based on type
# Supports dry-run mode for preview
format_filesystem() {
    local device="$1"
    local fs_type="$2"

    if [[ "${DRY_RUN:-false}" == "true" ]]; then
        log_info "[DRY-RUN] Would format $device as $fs_type"
        return 0
    fi

    case "$fs_type" in
        "ext4")
            check_package_available "e2fsprogs" "mkfs.ext4" || return 1
            mkfs.ext4 -F "$device"
            ;;
        "xfs")
            check_package_available "xfsprogs" "mkfs.xfs" || return 1
            mkfs.xfs -f "$device"
            ;;
        "btrfs")
            check_package_available "btrfs-progs" "mkfs.btrfs" || return 1
            mkfs.btrfs -f "$device"
            ;;
        "vfat")
            check_package_available "dosfstools" "mkfs.fat" || return 1
            check_package_available "exfatprogs" "exFAT/FAT32 utilities" || return 1
            mkfs.fat -F32 "$device"
            ;;
        "swap")
            mkswap "$device"
            ;;
        *)
            log_error "Unknown filesystem type: $fs_type"
            return 1
            ;;
    esac
}

# SECURITY: RAID disk compatibility validation
# Validates that disks in a RAID array are compatible
validate_raid_disks() {
    local -a disks=("$@")

    if [[ ${#disks[@]} -lt 2 ]]; then
        error_exit "RAID requires at least 2 disks, got ${#disks[@]}"
    fi

    log_info "Validating RAID disk compatibility for ${#disks[@]} disks..."

    # Get size of first disk as reference
    local first_disk="${disks[0]}"
    local first_size=$(blockdev --getsize64 "$first_disk" 2>/dev/null)
    local first_sector=$(blockdev --getss "$first_disk" 2>/dev/null)

    if [[ -z "$first_size" ]] || [[ -z "$first_sector" ]]; then
        error_exit "Failed to get size/sector info for $first_disk"
    fi

    log_debug "Reference disk: $first_disk (size: $first_size bytes, sector: $first_sector bytes)"

    # Check all disks for compatibility
    for disk in "${disks[@]:1}"; do
        local disk_size=$(blockdev --getsize64 "$disk" 2>/dev/null)
        local disk_sector=$(blockdev --getss "$disk" 2>/dev/null)

        # Check sector size compatibility
        if [[ "$disk_sector" != "$first_sector" ]]; then
            log_error "RAID disk sector size mismatch:"
            log_error "  $first_disk: $first_sector bytes"
            log_error "  $disk: $disk_sector bytes"
            error_exit "All RAID disks must have matching sector sizes"
        fi

        # Warn about size differences (allow up to 5% difference)
        local size_diff=$((disk_size - first_size))
        local size_diff_abs=${size_diff#-}  # Absolute value
        local size_diff_pct=$((size_diff_abs * 100 / first_size))

        if [[ $size_diff_pct -gt 5 ]]; then
            log_warn "⚠️  RAID disk size mismatch (${size_diff_pct}% difference):"
            log_warn "  $first_disk: $(numfmt --to=iec $first_size 2>/dev/null || echo $first_size)"
            log_warn "  $disk: $(numfmt --to=iec $disk_size 2>/dev/null || echo $disk_size)"
            log_warn "Smallest disk will limit RAID capacity"
        fi

        # Check for existing RAID metadata
        if mdadm --examine "$disk" &>/dev/null; then
            log_warn "⚠️  Disk $disk has existing RAID metadata"
            log_warn "This will be overwritten during RAID creation"
        fi
    done

    log_success "RAID disk compatibility check passed"
    return 0
}

# Wait for device to be ready (replaces hardcoded sleep)
wait_for_device() {
    local device="$1"
    local max_wait="${2:-10}"
    local waited=0

    log_debug "Waiting for device $device to be ready..."

    while [[ $waited -lt $max_wait ]]; do
        if [[ -b "$device" ]]; then
            # Device exists, wait for udev to settle
            udevadm settle --timeout=2 2>/dev/null || sleep 1
            log_debug "Device $device is ready"
            return 0
        fi
        sleep 0.5
        waited=$((waited + 1))
    done

    log_warn "Timeout waiting for device $device after ${max_wait}s"
    return 1
}

# Check disk SMART health status
check_disk_health() {
    local disk="$1"

    # Check if smartctl is available
    if ! command -v smartctl &>/dev/null; then
        log_debug "smartctl not available, skipping SMART health check"
        return 0
    fi

    log_info "Checking SMART health status for $disk..."

    # Check if SMART is supported
    if ! smartctl -i "$disk" &>/dev/null; then
        log_debug "SMART not supported on $disk, skipping health check"
        return 0
    fi

    # Get SMART health status
    local health_status
    health_status=$(smartctl -H "$disk" 2>/dev/null | grep -i "SMART overall-health" | awk '{print $NF}')

    if [[ -z "$health_status" ]]; then
        log_warn "Could not determine SMART health status for $disk"
        return 0
    fi

    if [[ "$health_status" =~ PASSED|OK ]]; then
        log_success "SMART health check passed for $disk: $health_status"
        return 0
    else
        log_error "⚠️  SMART health check FAILED for $disk: $health_status"
        log_error "This disk may be failing and could cause data loss!"

        if [[ "${FORCE_UNSAFE_DISK:-false}" != "true" ]]; then
            error_exit "Refusing to use potentially failing disk $disk (use FORCE_UNSAFE_DISK=true to override)"
        else
            log_warn "Proceeding with potentially failing disk due to FORCE_UNSAFE_DISK=true"
        fi
    fi
}

# Enhanced RAID validation with SMART checks
validate_raid_disks_with_health() {
    local -a disks=("$@")

    # First do standard compatibility checks
    validate_raid_disks "$@"

    # Then check SMART health on all disks
    log_info "Performing SMART health checks on RAID disks..."
    for disk in "${disks[@]}"; do
        check_disk_health "$disk"
    done

    log_success "All RAID disks passed health checks"
}
