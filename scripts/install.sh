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

# Source utility functions and strategies via source_or_die
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"

# Inline source_or_die before utils.sh is loaded
_source_or_die() {
    local script_path="$1"
    if [[ ! -f "$script_path" ]]; then
        echo "FATAL: Required script not found: $script_path" >&2
        exit 1
    fi
    # shellcheck source=/dev/null
    if ! source "$script_path"; then
        echo "FATAL: Failed to source: $script_path" >&2
        exit 1
    fi
}

_source_or_die "$SCRIPT_DIR/utils.sh"
_source_or_die "$SCRIPT_DIR/disk_strategies.sh"
_source_or_die "$SCRIPT_DIR/config_loader.sh"

# --- Credential Validation ---
# ENVIRONMENT CONTRACT: Passwords MUST be passed via environment variables
# The TUI is responsible for setting these before spawning install.sh
#
# Required variables:
#   MAIN_USER_PASSWORD - User account password
#   ROOT_PASSWORD      - Root account password
#   ENCRYPTION_PASSWORD - LUKS encryption password (if ENCRYPTION=Yes)
#
# This script is NON-INTERACTIVE and refuses to prompt for input.

# Validate credentials are present (will be checked again in validate_configuration)
if [[ -z "${MAIN_USER_PASSWORD:-}" ]] || [[ -z "${ROOT_PASSWORD:-}" ]]; then
    echo "ERROR: MAIN_USER_PASSWORD and ROOT_PASSWORD must be set in environment" >&2
    echo "This script is non-interactive and cannot prompt for passwords." >&2
    exit 1
fi

echo "Credentials validated from environment."

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
    log_info "Enabling NTP time synchronization..."
    timedatectl set-ntp true

    # Wait for time sync
    log_info "Waiting for time sync..."
    sleep 2

    # Configure mirrors
    configure_mirrors

    # Update package database
    log_info "Updating package database (pacman -Sy)..."
    pacman -Sy --noconfirm 2>&1 | while IFS= read -r line; do
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
        log_error "pacman -Sy failed"
        return 1
    fi

    log_success "System prepared"
    return 0
}

configure_mirrors() {
    log_info "Configuring package mirrors..."

    # Backup original mirrorlist
    log_info "Backing up original mirrorlist..."
    cp /etc/pacman.d/mirrorlist /etc/pacman.d/mirrorlist.backup

    # Use reflector if available, otherwise use default mirrors
    if command -v reflector >/dev/null 2>&1; then
        log_info "Using reflector to rank mirrors for country: ${MIRROR_COUNTRY:-US}..."
        log_info "This may take a minute while mirrors are tested..."
        reflector --country "${MIRROR_COUNTRY:-US}" --age 12 --protocol https --sort rate --save /etc/pacman.d/mirrorlist 2>&1 | while IFS= read -r line; do
            case "$line" in
                *"error"*|*"Error"*)
                    echo -e "${LOG_COLORS[ERROR]}  [reflector] $line${COLORS[RESET]}"
                    ;;
                *"warning"*|*"Warning"*)
                    echo -e "${LOG_COLORS[WARN]}  [reflector] $line${COLORS[RESET]}"
                    ;;
                *)
                    echo -e "${LOG_COLORS[COMMAND]}  [reflector] $line${COLORS[RESET]}"
                    ;;
            esac
        done
        if [[ ${PIPESTATUS[0]} -ne 0 ]]; then
            log_warn "Reflector failed, using default mirrors"
            cp /etc/pacman.d/mirrorlist.backup /etc/pacman.d/mirrorlist
        else
            log_success "Mirrors ranked and saved"
        fi
    else
        log_info "Reflector not available, using default mirrors"
    fi

    # Enable multilib if requested
    if [[ "$MULTILIB" == "Yes" ]]; then
        log_info "Enabling multilib repository..."
        sed -i '/^#\[multilib\]/,/^#Include/s/^#//' /etc/pacman.conf
        log_success "Multilib repository enabled"
    fi
}

# --- Disk Partitioning ---
partition_disk() {
    log_info "Starting disk partitioning..."
    log_info "Target disk: $INSTALL_DISK"
    log_info "Partitioning strategy: $PARTITIONING_STRATEGY"
    log_info "Boot mode: $BOOT_MODE"

    # Map TUI partitioning options to disk strategy functions
    local strategy_func=""
    case "$PARTITIONING_STRATEGY" in
        "auto_simple")
            strategy_func="do_auto_simple_partitioning_efi_xbootldr"
            log_info "Using auto simple partitioning (EFI + XBOOTLDR)"
            ;;
        "auto_simple_luks")
            strategy_func="do_auto_simple_luks_partitioning"
            log_info "Using auto simple LUKS encrypted partitioning"
            ;;
        "auto_lvm")
            strategy_func="do_auto_lvm_partitioning_efi_xbootldr"
            log_info "Using auto LVM partitioning (EFI + XBOOTLDR)"
            ;;
        "auto_luks_lvm")
            strategy_func="do_auto_luks_lvm_partitioning"
            log_info "Using auto LUKS + LVM partitioning"
            ;;
        "auto_raid")
            strategy_func="do_auto_raid_partitioning"
            log_info "Using auto RAID partitioning"
            ;;
        "auto_raid_luks")
            strategy_func="do_auto_raid_luks_partitioning"
            log_info "Using auto RAID + LUKS partitioning"
            ;;
        "auto_raid_lvm")
            strategy_func="do_auto_raid_lvm_partitioning"
            log_info "Using auto RAID + LVM partitioning"
            ;;
        "auto_raid_lvm_luks")
            strategy_func="do_auto_raid_lvm_luks_partitioning"
            log_info "Using auto RAID + LVM + LUKS partitioning"
            ;;
        "manual")
            strategy_func="do_manual_partitioning_guided"
            log_info "Using manual partitioning (guided)"
            ;;
        *)
            log_error "Unknown partitioning strategy: $PARTITIONING_STRATEGY"
            return 1
            ;;
    esac

    # Execute the disk strategy
    log_info "Executing disk strategy: $strategy_func"
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
        "sof-firmware"  # For modern onboard audio (Sound Open Firmware)
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
        # Bluetooth support
        "bluez"
        "bluez-utils"
        # Network discovery (mDNS/Bonjour)
        "avahi"
        "nss-mdns"
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

    log_info "Total packages to install: ${#all_packages[@]}"
    log_info "Package list: ${all_packages[*]}"
    log_info "Starting pacstrap - this will take several minutes..."
    log_info "Downloading and installing packages to /mnt..."

    # Run pacstrap with array expansion and show output
    pacstrap -K /mnt "${all_packages[@]}" 2>&1 | while IFS= read -r line; do
        # Filter and format pacstrap output for readability with colors
        case "$line" in
            *"error"*|*"Error"*|*"ERROR"*|*"failed"*)
                echo -e "${LOG_COLORS[ERROR]}  [pacstrap] $line${COLORS[RESET]}"
                ;;
            *"warning"*|*"Warning"*|*"WARNING"*)
                echo -e "${LOG_COLORS[WARN]}  [pacstrap] $line${COLORS[RESET]}"
                ;;
            *"downloading"*|*"installing"*|*"Packages"*|*"Total"*|*"::"*)
                echo -e "${LOG_COLORS[COMMAND]}  [pacstrap] $line${COLORS[RESET]}"
                ;;
            *)
                # Show other lines dimmed
                echo -e "${COLORS[DIM]}  $line${COLORS[RESET]}"
                ;;
        esac
    done

    if [[ ${PIPESTATUS[0]} -ne 0 ]]; then
        log_error "pacstrap failed"
        return 1
    fi

    log_success "Base system installed successfully"
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
    log_info "This phase includes: timezone, locale, users, bootloader, desktop environment..."

    # Copy necessary scripts to target system
    log_info "Copying configuration scripts to /mnt/root/..."
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
export BTRFS_FREQUENCY="$BTRFS_FREQUENCY"
export BTRFS_KEEP_COUNT="$BTRFS_KEEP_COUNT"
export BTRFS_ASSISTANT="$BTRFS_ASSISTANT"
export SWAP="$SWAP"
export WANT_SWAP="$WANT_SWAP"
export SWAP_UUID="${SWAP_UUID:-}"
export ROOT_UUID="${ROOT_UUID:-}"
export LUKS_UUID="${LUKS_UUID:-}"
export ROOT_FILESYSTEM_TYPE="$ROOT_FILESYSTEM_TYPE"
CONFIGEOF

    chmod +x /mnt/root/install_config.sh

    # Execute chroot configuration
    log_info "Entering chroot environment..."
    log_info "Running chroot_config.sh inside /mnt..."

    arch-chroot /mnt /bin/bash -c "
        # Source config with error handling (source_or_die not available in chroot)
        if [[ ! -f /root/install_config.sh ]]; then
            echo 'FATAL: /root/install_config.sh not found' >&2
            exit 1
        fi
        source /root/install_config.sh || { echo 'FATAL: Failed to source install_config.sh' >&2; exit 1; }
        cd /root
        ./chroot_config.sh
    " 2>&1 | while IFS= read -r line; do
        # Pass through colored output from chroot, or colorize based on content
        if [[ "$line" == *$'\033'* ]]; then
            # Line already has ANSI codes, pass through with prefix
            echo -e "  [chroot] $line"
        else
            # Add colors based on content
            case "$line" in
                *"ERROR"*|*"error"*|*"Error"*|*"failed"*|*"FAILED"*)
                    echo -e "${LOG_COLORS[ERROR]}  [chroot] $line${COLORS[RESET]}"
                    ;;
                *"WARN"*|*"warning"*|*"Warning"*)
                    echo -e "${LOG_COLORS[WARN]}  [chroot] $line${COLORS[RESET]}"
                    ;;
                *"SUCCESS"*|*"success"*|*"complete"*|*"Complete"*)
                    echo -e "${LOG_COLORS[SUCCESS]}  [chroot] $line${COLORS[RESET]}"
                    ;;
                *"==="*)
                    echo -e "${LOG_COLORS[PHASE]}  [chroot] $line${COLORS[RESET]}"
                    ;;
                *)
                    echo -e "${LOG_COLORS[COMMAND]}  [chroot] $line${COLORS[RESET]}"
                    ;;
            esac
        fi
    done

    local chroot_exit=${PIPESTATUS[0]}

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
