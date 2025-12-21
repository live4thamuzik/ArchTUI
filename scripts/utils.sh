#!/bin/bash
# utils.sh - Utility functions for Arch Linux installer

set -euo pipefail

# Logging configuration
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

# Enhanced logging functions with levels
log_debug() {
    if [[ "${LOG_LEVEL:-INFO}" == "DEBUG" ]]; then
        local message="$1"
        local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
        echo "[$timestamp] DEBUG: $message"
        echo "[$timestamp] DEBUG: $message" >> "$LOG_FILE" 2>/dev/null || true
    fi
}

log_info() {
    local message="$1"
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    echo "[$timestamp] INFO: $message"
    echo "[$timestamp] INFO: $message" >> "$LOG_FILE" 2>/dev/null || true
}

log_warn() {
    local message="$1"
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    echo "[$timestamp] WARN: $message" >&2
    echo "[$timestamp] WARN: $message" >> "$LOG_FILE" 2>/dev/null || true
}

log_error() {
    local message="$1"
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    echo "[$timestamp] ERROR: $message" >&2
    echo "[$timestamp] ERROR: $message" >> "$LOG_FILE" 2>/dev/null || true
}

log_success() {
    local message="$1"
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    echo "[$timestamp] SUCCESS: $message"
    echo "[$timestamp] SUCCESS: $message" >> "$LOG_FILE" 2>/dev/null || true
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
validate_disk() {
    local disk="$1"
    if [ -z "$disk" ]; then
        log_error "Disk path is empty"
        return 1
    fi
    
    if [ ! -b "$disk" ]; then
        log_error "Disk $disk does not exist or is not a block device"
        return 1
    fi
    
    # Additional disk validation
    log_debug "Validating disk: $disk"
    
    # Check if disk is mounted
    if mountpoint -q "$disk" 2>/dev/null; then
        error_exit "Disk $disk is currently mounted and cannot be used for installation"
    fi
    
    # Check if any partitions on disk are mounted
    local mounted_parts=$(lsblk -n -o MOUNTPOINT "$disk" 2>/dev/null | grep -v "^$" | wc -l)
    if [[ $mounted_parts -gt 0 ]]; then
        log_warn "Some partitions on $disk are mounted - proceeding anyway"
    fi
    
    # Check disk size (minimum 8GB)
    local disk_size_bytes=$(lsblk -b -d -n -o SIZE "$disk" 2>/dev/null)
    local min_size_bytes=$((8 * 1024 * 1024 * 1024)) # 8GB
    
    if [[ -n "$disk_size_bytes" ]] && [[ $disk_size_bytes -lt $min_size_bytes ]]; then
        log_warn "Disk $disk is small (${disk_size_bytes} bytes), minimum recommended is ${min_size_bytes} bytes"
    fi
    
    log_info "Disk $disk validated successfully"
    return 0
}

validate_username() {
    local username="$1"
    if [ -z "$username" ]; then
        log_error "Username is empty"
        return 1
    fi
    
    if ! echo "$username" | grep -qE '^[a-zA-Z0-9._-]+$'; then
        log_error "Username contains invalid characters"
        return 1
    fi
    
    log_info "Username $username validated successfully"
    return 0
}

validate_hostname() {
    local hostname="$1"
    if [ -z "$hostname" ]; then
        log_error "Hostname is empty"
        return 1
    fi
    
    if ! echo "$hostname" | grep -qE '^[a-zA-Z0-9.-]+$'; then
        log_error "Hostname contains invalid characters"
        return 1
    fi
    
    log_info "Hostname $hostname validated successfully"
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
    
    for package in "${required_packages[@]}"; do
        if ! pacman -Qi "$package" &>/dev/null; then
            missing_packages+=("$package")
        fi
    done
    
    if [ ${#missing_packages[@]} -gt 0 ]; then
        log_info "Installing missing packages: ${missing_packages[*]}"
        pacman -Sy --noconfirm "${missing_packages[@]}" || {
            log_error "Failed to install required packages: ${missing_packages[*]}"
            return 1
        }
        log_success "All required packages installed successfully"
    else
        log_info "All required packages are already installed"
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
format_filesystem() {
    local device="$1"
    local fs_type="$2"
    
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
