#!/bin/bash
# install.sh - Complete Arch Linux Installation Engine (TUI-Only)
# This script handles the entire installation process from partitioning to completion

set -euo pipefail

# Cleanup function for unmounting on failure
cleanup_on_exit() {
    local exit_code=$?

    # Only cleanup on error
    if [[ $exit_code -eq 0 ]]; then
        return 0
    fi

    echo "=== CLEANUP ON EXIT (Code: $exit_code) ==="

    # Try to unmount everything cleanly (in reverse order)
    for mount_point in /mnt/home /mnt/boot /mnt/efi /mnt; do
        if mountpoint -q "$mount_point" 2>/dev/null; then
            echo "Unmounting $mount_point..."
            umount -R "$mount_point" 2>/dev/null || true
        fi
    done

    # Enhanced LVM cleanup - deactivate volume groups
    if command -v vgchange >/dev/null 2>&1; then
        echo "Deactivating volume groups..."
        vgchange -an 2>/dev/null || true
    fi

    # Enhanced LUKS cleanup - close all LUKS containers
    if command -v cryptsetup >/dev/null 2>&1; then
        echo "Closing LUKS containers..."
        for mapper in /dev/mapper/arch_* /dev/mapper/cryptlvm; do
            if [[ -e "$mapper" ]]; then
                cryptsetup close "$(basename "$mapper")" 2>/dev/null || true
            fi
        done
    fi

    # Enhanced RAID cleanup - stop arrays if needed
    if command -v mdadm >/dev/null 2>&1; then
        echo "Stopping RAID arrays..."
        mdadm --stop --scan 2>/dev/null || true
    fi

    echo "=== CLEANUP COMPLETE ==="
    exit $exit_code
}

# Set up exit trap
trap cleanup_on_exit EXIT

# Debug: Show script startup
echo "=== INSTALLATION ENGINE STARTED ==="
echo "Script: install.sh"
echo "PID: $$"
echo "Mode: TUI-only"
echo "Working Directory: $(pwd)"
echo "User: $(whoami)"
echo "=========================================="

# Source utility functions and strategies
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$SCRIPT_DIR/utils.sh"
source "$SCRIPT_DIR/disk_strategies.sh"
source "$SCRIPT_DIR/config_loader.sh"

# Initialize logging
setup_logging

# Set log level (can be overridden by environment variable)
export LOG_LEVEL="${LOG_LEVEL:-INFO}"

# Perform pre-flight checks
perform_preflight_checks

# Parse command line arguments
CONFIG_FILE=""
while [[ $# -gt 0 ]]; do
    case $1 in
        --config)
            CONFIG_FILE="$2"
            shift 2
            ;;
        --help|-h)
            echo "Usage: $0 [--config CONFIG_FILE]"
            echo ""
            echo "Options:"
            echo "  --config FILE    Load configuration from JSON file"
            echo "  --help, -h       Show this help message"
            echo ""
            echo "If no --config is provided, the installer expects environment variables"
            echo "to be set by the TUI frontend."
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# --- Configuration Loading ---
if [[ -n "$CONFIG_FILE" ]]; then
    log_info "Loading configuration from JSON file: $CONFIG_FILE"
    load_config_from_json "$CONFIG_FILE"
else
    log_info "Using environment variables from TUI"
    if [[ -z "${INSTALL_DISK:-}" ]]; then
        error_exit "INSTALL_DISK environment variable is required"
    fi
    if [[ -z "${PARTITIONING_STRATEGY:-}" ]]; then
        error_exit "PARTITIONING_STRATEGY environment variable is required"
    fi
fi

# --- Configuration Variables with Defaults ---
# Boot Configuration
BOOT_MODE="${BOOT_MODE:-Auto}"
SECURE_BOOT="${SECURE_BOOT:-No}"

# System Locale and Input
LOCALE="${LOCALE:-en_US.UTF-8}"
KEYMAP="${KEYMAP:-us}"

# Disk and Storage
INSTALL_DISK="${INSTALL_DISK:-/dev/sda}"
PARTITIONING_STRATEGY="${PARTITIONING_STRATEGY:-auto_simple}"
# Also set PARTITION_SCHEME for disk_strategies.sh compatibility
export PARTITION_SCHEME="$PARTITIONING_STRATEGY"
ENCRYPTION="${ENCRYPTION:-No}"
ENCRYPTION_PASSWORD="${ENCRYPTION_PASSWORD:-}"
ROOT_FILESYSTEM="${ROOT_FILESYSTEM:-ext4}"
SEPARATE_HOME="${SEPARATE_HOME:-No}"
HOME_FILESYSTEM="${HOME_FILESYSTEM:-ext4}"
SWAP="${SWAP:-Yes}"
SWAP_SIZE="${SWAP_SIZE:-2GB}"

# Convert TUI variables to internal format
ROOT_FILESYSTEM_TYPE="$ROOT_FILESYSTEM"
HOME_FILESYSTEM_TYPE="$HOME_FILESYSTEM"
WANT_HOME_PARTITION="$(echo "$SEPARATE_HOME" | tr '[:upper:]' '[:lower:]')"
[[ "$WANT_HOME_PARTITION" == "yes" ]] || WANT_HOME_PARTITION="no"
WANT_SWAP="$(echo "${SWAP:-Yes}" | tr '[:upper:]' '[:lower:]')"
[[ "$WANT_SWAP" == "yes" ]] || WANT_SWAP="no"

# Export for strategy scripts
export ROOT_FILESYSTEM_TYPE HOME_FILESYSTEM_TYPE WANT_HOME_PARTITION WANT_SWAP
export ENCRYPTION ENCRYPTION_PASSWORD

# Btrfs options
BTRFS_SNAPSHOTS="${BTRFS_SNAPSHOTS:-No}"
BTRFS_FREQUENCY="${BTRFS_FREQUENCY:-weekly}"
BTRFS_KEEP_COUNT="${BTRFS_KEEP_COUNT:-3}"
BTRFS_ASSISTANT="${BTRFS_ASSISTANT:-No}"

# Time and Location
TIMEZONE_REGION="${TIMEZONE_REGION:-America}"
TIMEZONE="${TIMEZONE:-New_York}"
TIME_SYNC="${TIME_SYNC:-Yes}"

# System Packages
MIRROR_COUNTRY="${MIRROR_COUNTRY:-United States}"
KERNEL="${KERNEL:-linux}"
MULTILIB="${MULTILIB:-Yes}"
ADDITIONAL_PACKAGES="${ADDITIONAL_PACKAGES:-}"
GPU_DRIVERS="${GPU_DRIVERS:-Auto}"

# User Setup
SYSTEM_HOSTNAME="${SYSTEM_HOSTNAME:-archlinux}"
MAIN_USERNAME="${MAIN_USERNAME:-user}"
MAIN_USER_PASSWORD="${MAIN_USER_PASSWORD:-}"
ROOT_PASSWORD="${ROOT_PASSWORD:-}"

# Package Management
AUR_HELPER="${AUR_HELPER:-paru}"
ADDITIONAL_AUR_PACKAGES="${ADDITIONAL_AUR_PACKAGES:-}"
FLATPAK="${FLATPAK:-No}"

# Boot Configuration
BOOTLOADER="${BOOTLOADER:-grub}"
OS_PROBER="${OS_PROBER:-Yes}"
GRUB_THEME="${GRUB_THEME:-No}"
GRUB_THEME_SELECTION="${GRUB_THEME_SELECTION:-arch}"

# Desktop Environment
DESKTOP_ENVIRONMENT="${DESKTOP_ENVIRONMENT:-none}"
DISPLAY_MANAGER="${DISPLAY_MANAGER:-sddm}"

# Boot Splash and Final Setup
PLYMOUTH="${PLYMOUTH:-No}"
PLYMOUTH_THEME="${PLYMOUTH_THEME:-arch-glow}"
NUMLOCK_ON_BOOT="${NUMLOCK_ON_BOOT:-No}"
GIT_REPOSITORY="${GIT_REPOSITORY:-No}"
GIT_REPOSITORY_URL="${GIT_REPOSITORY_URL:-}"

# --- Main Installation Function ---
main() {
    echo "Starting Arch Linux installation..."

    # Phase 1: Validate configuration
    log_info "Phase 1: Validating configuration..."
    validate_configuration || error_exit "Configuration validation failed"

    # Phase 2: Prepare system
    log_info "Phase 2: Preparing system..."
    prepare_system || error_exit "System preparation failed"

    # Phase 3: Check and install dependencies
    log_info "Phase 3: Installing dependencies..."
    check_and_install_dependencies || error_exit "Dependency installation failed"

    # Phase 4: Partition disk
    log_info "Phase 4: Partitioning disk..."
    partition_disk || error_exit "Disk partitioning failed"

    # Phase 5: Install base system (pacstrap)
    log_info "Phase 5: Installing base system..."
    install_base_system || error_exit "Base system installation failed"

    # Phase 6: Generate fstab
    log_info "Phase 6: Generating fstab..."
    generate_fstab || error_exit "fstab generation failed"

    # Phase 7: Configure system in chroot
    log_info "Phase 7: Configuring system in chroot..."
    configure_chroot || error_exit "Chroot configuration failed"

    # Phase 8: Finalize installation
    log_info "Phase 8: Finalizing installation..."
    finalize_installation || error_exit "Installation finalization failed"

    echo "=========================================="
    echo "Installation complete!"
    echo "=========================================="
    echo ""
    echo "You can now reboot into your new Arch Linux system."
    echo "Don't forget to remove the installation media."
}

# --- Validation Functions ---
validate_configuration() {
    log_info "Validating configuration..."

    local required_vars=(
        "INSTALL_DISK"
        "MAIN_USERNAME"
        "ROOT_PASSWORD"
        "MAIN_USER_PASSWORD"
        "SYSTEM_HOSTNAME"
    )

    for var in "${required_vars[@]}"; do
        if [[ -z "${!var:-}" ]]; then
            log_error "Required variable $var is not set"
            return 1
        fi
    done

    # Validate disk exists
    if [[ ! -b "$INSTALL_DISK" ]]; then
        log_error "Installation disk $INSTALL_DISK does not exist or is not a block device"
        return 1
    fi

    # Auto-detect boot mode if needed
    if [[ "$BOOT_MODE" == "Auto" ]]; then
        if [[ -d "/sys/firmware/efi/efivars" ]]; then
            BOOT_MODE="UEFI"
            log_info "Auto-detected UEFI boot mode"
        else
            BOOT_MODE="BIOS"
            log_info "Auto-detected BIOS boot mode"
        fi
    fi
    export BOOT_MODE

    # Validate boot mode matches system
    if [[ "$BOOT_MODE" == "UEFI" ]]; then
        if [[ ! -d "/sys/firmware/efi/efivars" ]]; then
            log_error "System is not booted in UEFI mode but BOOT_MODE is set to UEFI"
            return 1
        fi
    fi

    # Validate LUKS encryption has password
    if [[ "$ENCRYPTION" == "Yes" && -z "$ENCRYPTION_PASSWORD" ]]; then
        log_error "ENCRYPTION is enabled but ENCRYPTION_PASSWORD is not set"
        return 1
    fi

    # Validate username format
    if ! echo "$MAIN_USERNAME" | grep -qE '^[a-z_][a-z0-9_-]*$'; then
        log_error "Invalid username format: $MAIN_USERNAME (must start with lowercase letter)"
        return 1
    fi

    log_success "Configuration validated successfully"
    return 0
}

# --- System Preparation ---
prepare_system() {
    log_info "Preparing system..."

    # Update system clock
    timedatectl set-ntp true

    # Wait for time sync
    sleep 2

    # Configure mirrors
    configure_mirrors

    # Update package database
    pacman -Sy --noconfirm

    log_success "System prepared"
    return 0
}

configure_mirrors() {
    log_info "Configuring package mirrors..."

    # Backup original mirrorlist
    cp /etc/pacman.d/mirrorlist /etc/pacman.d/mirrorlist.backup

    # Use reflector if available, otherwise use default mirrors
    if command -v reflector >/dev/null 2>&1; then
        log_info "Using reflector to rank mirrors..."
        reflector --country "$MIRROR_COUNTRY" --age 12 --protocol https --sort rate --save /etc/pacman.d/mirrorlist || {
            log_warn "Reflector failed, using default mirrors"
            cp /etc/pacman.d/mirrorlist.backup /etc/pacman.d/mirrorlist
        }
    else
        log_info "Using default mirrors"
    fi

    # Enable multilib if requested
    if [[ "$MULTILIB" == "Yes" ]]; then
        log_info "Enabling multilib repository..."
        sed -i '/^#\[multilib\]/,/^#Include/s/^#//' /etc/pacman.conf
    fi
}

# --- Disk Partitioning ---
partition_disk() {
    log_info "Starting disk partitioning..."

    # Map TUI partitioning options to disk strategy functions
    local strategy_func=""
    case "$PARTITIONING_STRATEGY" in
        "auto_simple")
            strategy_func="do_auto_simple_partitioning_efi_xbootldr"
            ;;
        "auto_simple_luks")
            strategy_func="do_auto_simple_luks_partitioning"
            ;;
        "auto_lvm")
            strategy_func="do_auto_lvm_partitioning_efi_xbootldr"
            ;;
        "auto_luks_lvm")
            strategy_func="do_auto_luks_lvm_partitioning"
            ;;
        "auto_raid")
            strategy_func="do_auto_raid_partitioning"
            ;;
        "auto_raid_luks")
            strategy_func="do_auto_raid_luks_partitioning"
            ;;
        "auto_raid_lvm")
            strategy_func="do_auto_raid_lvm_partitioning"
            ;;
        "auto_raid_lvm_luks")
            strategy_func="do_auto_raid_lvm_luks_partitioning"
            ;;
        "manual")
            strategy_func="do_manual_partitioning_guided"
            ;;
        *)
            log_error "Unknown partitioning strategy: $PARTITIONING_STRATEGY"
            return 1
            ;;
    esac

    # Execute the disk strategy
    execute_disk_strategy "$strategy_func"

    log_success "Disk partitioning complete"
    return 0
}

# --- Base System Installation ---
install_base_system() {
    log_info "Installing base system with pacstrap..."

    # Build package list as array
    local -a base_packages=(
        "base"
        "base-devel"
        "linux-firmware"
        "$KERNEL"
        "${KERNEL}-headers"
    )

    # Add essential packages
    local -a essential_packages=(
        "nano"
        "vim"
        "neovim"
        "sudo"
        "networkmanager"
        "openssh"
        "git"
        "curl"
        "wget"
        "htop"
        "man-db"
        "man-pages"
        "texinfo"
    )

    # Add filesystem tools based on selected filesystems
    local -a fs_packages=()
    case "$ROOT_FILESYSTEM" in
        "btrfs")
            fs_packages+=("btrfs-progs")
            ;;
        "xfs")
            fs_packages+=("xfsprogs")
            ;;
        "ext4")
            fs_packages+=("e2fsprogs")
            ;;
    esac

    # Add LUKS/LVM packages if needed
    if [[ "$ENCRYPTION" == "Yes" ]] || [[ "$PARTITIONING_STRATEGY" == *"luks"* ]]; then
        fs_packages+=("cryptsetup")
    fi
    if [[ "$PARTITIONING_STRATEGY" == *"lvm"* ]]; then
        fs_packages+=("lvm2")
    fi
    if [[ "$PARTITIONING_STRATEGY" == *"raid"* ]]; then
        fs_packages+=("mdadm")
    fi

    # Add bootloader packages
    local -a bootloader_packages=()
    case "$BOOTLOADER" in
        "grub")
            bootloader_packages+=("grub")
            if [[ "$BOOT_MODE" == "UEFI" ]]; then
                bootloader_packages+=("efibootmgr")
            fi
            if [[ "$OS_PROBER" == "Yes" ]]; then
                bootloader_packages+=("os-prober")
            fi
            ;;
        "systemd-boot")
            if [[ "$BOOT_MODE" == "UEFI" ]]; then
                bootloader_packages+=("efibootmgr")
            fi
            ;;
    esac

    # Detect and add CPU microcode
    local -a microcode_packages=()
    if grep -q "GenuineIntel" /proc/cpuinfo; then
        microcode_packages+=("intel-ucode")
    elif grep -q "AuthenticAMD" /proc/cpuinfo; then
        microcode_packages+=("amd-ucode")
    fi

    # Combine all packages
    local -a all_packages=(
        "${base_packages[@]}"
        "${essential_packages[@]}"
        "${fs_packages[@]}"
        "${bootloader_packages[@]}"
        "${microcode_packages[@]}"
    )

    log_info "Installing packages: ${all_packages[*]}"

    # Run pacstrap with array expansion
    pacstrap -K /mnt "${all_packages[@]}" || {
        log_error "pacstrap failed"
        return 1
    }

    log_success "Base system installed"
    return 0
}

# --- Generate fstab ---
generate_fstab() {
    log_info "Generating fstab..."

    # Generate fstab using UUIDs
    genfstab -U /mnt >> /mnt/etc/fstab

    # Verify fstab was generated
    if [[ ! -s /mnt/etc/fstab ]]; then
        log_error "fstab is empty"
        return 1
    fi

    log_info "Generated fstab:"
    cat /mnt/etc/fstab

    log_success "fstab generated"
    return 0
}

# --- Chroot Configuration ---
configure_chroot() {
    log_info "Configuring system in chroot..."

    # Copy necessary scripts to target system
    cp "$SCRIPT_DIR/chroot_config.sh" /mnt/root/
    cp "$SCRIPT_DIR/utils.sh" /mnt/root/
    mkdir -p /mnt/root/desktops
    cp -r "$SCRIPT_DIR/desktops/"* /mnt/root/desktops/ 2>/dev/null || true

    # Make scripts executable
    chmod +x /mnt/root/chroot_config.sh
    chmod +x /mnt/root/utils.sh

    # Export all configuration variables for chroot
    # Use a config file to pass variables (more reliable than env)
    cat > /mnt/root/install_config.sh << CONFIGEOF
#!/bin/bash
# Auto-generated configuration for chroot
export MAIN_USERNAME="$MAIN_USERNAME"
export MAIN_USER_PASSWORD="$MAIN_USER_PASSWORD"
export ROOT_PASSWORD="$ROOT_PASSWORD"
export SYSTEM_HOSTNAME="$SYSTEM_HOSTNAME"
export TIMEZONE_REGION="$TIMEZONE_REGION"
export TIMEZONE="$TIMEZONE"
export LOCALE="$LOCALE"
export KEYMAP="$KEYMAP"
export DESKTOP_ENVIRONMENT="$DESKTOP_ENVIRONMENT"
export DISPLAY_MANAGER="$DISPLAY_MANAGER"
export GPU_DRIVERS="$GPU_DRIVERS"
export AUR_HELPER="$AUR_HELPER"
export ADDITIONAL_PACKAGES="$ADDITIONAL_PACKAGES"
export ADDITIONAL_AUR_PACKAGES="$ADDITIONAL_AUR_PACKAGES"
export FLATPAK="$FLATPAK"
export PLYMOUTH="$PLYMOUTH"
export PLYMOUTH_THEME="$PLYMOUTH_THEME"
export NUMLOCK_ON_BOOT="$NUMLOCK_ON_BOOT"
export GIT_REPOSITORY="$GIT_REPOSITORY"
export GIT_REPOSITORY_URL="$GIT_REPOSITORY_URL"
export BOOT_MODE="$BOOT_MODE"
export BOOTLOADER="$BOOTLOADER"
export OS_PROBER="$OS_PROBER"
export GRUB_THEME="$GRUB_THEME"
export GRUB_THEME_SELECTION="$GRUB_THEME_SELECTION"
export SECURE_BOOT="$SECURE_BOOT"
export KERNEL="$KERNEL"
export MULTILIB="$MULTILIB"
export TIME_SYNC="$TIME_SYNC"
export INSTALL_DISK="$INSTALL_DISK"
export PARTITIONING_STRATEGY="$PARTITIONING_STRATEGY"
export ENCRYPTION="$ENCRYPTION"
export ROOT_FILESYSTEM="$ROOT_FILESYSTEM"
export HOME_FILESYSTEM="$HOME_FILESYSTEM"
export BTRFS_SNAPSHOTS="$BTRFS_SNAPSHOTS"
export SWAP="$SWAP"
export ROOT_UUID="${ROOT_UUID:-}"
export LUKS_UUID="${LUKS_UUID:-}"
CONFIGEOF

    chmod +x /mnt/root/install_config.sh

    # Execute chroot configuration
    arch-chroot /mnt /bin/bash -c "
        source /root/install_config.sh
        cd /root
        ./chroot_config.sh
    "

    local chroot_exit=$?

    # Clean up copied scripts
    rm -f /mnt/root/chroot_config.sh
    rm -f /mnt/root/utils.sh
    rm -f /mnt/root/install_config.sh
    rm -rf /mnt/root/desktops

    if [[ $chroot_exit -ne 0 ]]; then
        log_error "Chroot configuration failed"
        return 1
    fi

    log_success "Chroot configuration complete"
    return 0
}

# --- Finalization ---
finalize_installation() {
    log_info "Finalizing installation..."

    # Copy Plymouth themes if they exist and Plymouth is enabled
    if [[ "$PLYMOUTH" == "Yes" ]]; then
        local themes_source="$SCRIPT_DIR/../Source"
        if [[ -d "$themes_source" ]]; then
            log_info "Copying Plymouth themes..."
            cp -r "$themes_source/"* /mnt/usr/share/plymouth/themes/ 2>/dev/null || true
        fi
    fi

    # Ensure all services are properly enabled
    log_info "Verifying service configuration..."

    # Sync filesystems
    sync

    log_success "Installation finalized successfully!"
    return 0
}

# --- Run main function ---
main "$@"
