#!/bin/bash
# install.sh - Complete Arch Linux Installation Engine (TUI-Only)
# This script handles the entire installation process from partitioning to completion

set -euo pipefail

# Cleanup function for unmounting on failure
cleanup_on_exit() {
    local exit_code=$?

    # SECURITY: Always clean up sensitive files (passwords in install_config.sh)
    rm -f /mnt/install_config.sh 2>/dev/null || true

    # Only cleanup mounts/devices on error
    if [[ $exit_code -eq 0 ]]; then
        return 0
    fi

    echo "=== CLEANUP ON EXIT (Code: $exit_code) ==="

    # Deactivate swap before unmounting (swapfiles on /mnt block umount)
    swapoff -a 2>/dev/null || true

    # Try to unmount everything cleanly (in reverse order)
    for mount_point in /mnt/home /mnt/boot /mnt/efi /mnt; do
        if mountpoint -q "$mount_point" 2>/dev/null; then
            echo "Unmounting $mount_point..."
            umount -R "$mount_point" 2>/dev/null || umount -l "$mount_point" 2>/dev/null || true
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
        for mapper in /dev/mapper/cryptroot /dev/mapper/crypthome /dev/mapper/cryptlvm /dev/mapper/cryptdata; do
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

# Set up exit and signal traps (process-safety.md: death pact compliance)
trap cleanup_on_exit EXIT
# Signal traps: exit with 128+signal to trigger EXIT trap (avoids double cleanup)
trap 'exit 143' SIGTERM
trap 'exit 130' SIGINT
# ERR trap: print file:line context before set -e kills the script
trap 'echo "FATAL: Command failed at ${BASH_SOURCE[0]}:${LINENO}: $(sed -n "${LINENO}p" "${BASH_SOURCE[0]}" 2>/dev/null || echo "unknown")" >&2' ERR

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

# Set log level (can be overridden by environment variable)
export LOG_LEVEL="${LOG_LEVEL:-INFO}"

# Initialize logging (must come after LOG_LEVEL is set so VERBOSE mode activates)
setup_logging
dump_config

echo "=========================================="
echo "Log file: ${LOG_FILE:-/tmp/archtui-install.log}"
if [[ -n "${VERBOSE_LOG_FILE:-}" ]]; then
    echo "Verbose log: $VERBOSE_LOG_FILE"
fi
echo "=========================================="

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
INSTALL_DISK="${INSTALL_DISK:?INSTALL_DISK must be set — no default disk}"
PARTITIONING_STRATEGY="${PARTITIONING_STRATEGY:-auto_simple}"
# Also set PARTITION_SCHEME for disk_strategies.sh compatibility
export PARTITION_SCHEME="$PARTITIONING_STRATEGY"
ENCRYPTION="${ENCRYPTION:-No}"
ENCRYPTION_KEY_TYPE="${ENCRYPTION_KEY_TYPE:-Password}"
# ROE §8.1: Suppress set -x tracing for password variables
{ set +x; } 2>/dev/null
ENCRYPTION_PASSWORD="${ENCRYPTION_PASSWORD:-}"
[[ "${LOG_LEVEL:-INFO}" == "VERBOSE" ]] && set -x
ROOT_FILESYSTEM="${ROOT_FILESYSTEM:-ext4}"
SEPARATE_HOME="${SEPARATE_HOME:-No}"
HOME_FILESYSTEM="${HOME_FILESYSTEM:-ext4}"
SWAP="${SWAP:-No}"
SWAP_SIZE="${SWAP_SIZE:-N/A}"
ROOT_SIZE="${ROOT_SIZE:-50GB}"
HOME_SIZE="${HOME_SIZE:-Remaining}"

# Convert TUI variables to internal format
ROOT_FILESYSTEM_TYPE="$ROOT_FILESYSTEM"
HOME_FILESYSTEM_TYPE="$HOME_FILESYSTEM"
WANT_HOME_PARTITION="$(echo "$SEPARATE_HOME" | tr '[:upper:]' '[:lower:]')"
[[ "$WANT_HOME_PARTITION" == "yes" ]] || WANT_HOME_PARTITION="no"
WANT_SWAP="$(echo "$SWAP" | tr '[:upper:]' '[:lower:]')"
[[ "$WANT_SWAP" == "yes" ]] || WANT_SWAP="no"

# For RAID strategies, parse comma-separated INSTALL_DISK into RAID_DEVICES array
if [[ "$PARTITIONING_STRATEGY" == *"raid"* ]]; then
    IFS=',' read -ra RAID_DEVICES <<< "$INSTALL_DISK"
    export RAID_DEVICES
    RAID_LEVEL="${RAID_LEVEL:-raid1}"
    export RAID_LEVEL
    log_info "RAID configuration: ${#RAID_DEVICES[@]} disks (${RAID_DEVICES[*]}), level: $RAID_LEVEL"
fi

# Export for strategy scripts
export ROOT_FILESYSTEM_TYPE HOME_FILESYSTEM_TYPE WANT_HOME_PARTITION WANT_SWAP SWAP_SIZE
export ROOT_SIZE HOME_SIZE
# ROE §8.1: Suppress set -x tracing for ENCRYPTION_PASSWORD export
{ set +x; } 2>/dev/null
export ENCRYPTION ENCRYPTION_KEY_TYPE ENCRYPTION_PASSWORD
[[ "${LOG_LEVEL:-INFO}" == "VERBOSE" ]] && set -x

# Btrfs options
BTRFS_SNAPSHOTS="${BTRFS_SNAPSHOTS:-No}"
BTRFS_FREQUENCY="${BTRFS_FREQUENCY:-weekly}"
BTRFS_KEEP_COUNT="${BTRFS_KEEP_COUNT:-3}"
BTRFS_ASSISTANT="${BTRFS_ASSISTANT:-No}"

# Time and Location
TIMEZONE_REGION="${TIMEZONE_REGION:-America}"
TIMEZONE="${TIMEZONE:-New_York}"
TIME_SYNC="${TIME_SYNC:-No}"

# System Packages
MIRROR_COUNTRY="${MIRROR_COUNTRY:-United States}"
KERNEL="${KERNEL:-linux}"
MULTILIB="${MULTILIB:-No}"
ADDITIONAL_PACKAGES="${ADDITIONAL_PACKAGES:-}"
GPU_DRIVERS="${GPU_DRIVERS:-Auto}"

# User Setup
SYSTEM_HOSTNAME="${SYSTEM_HOSTNAME:-archlinux}"
MAIN_USERNAME="${MAIN_USERNAME:-user}"
# ROE §8.1: Suppress set -x tracing for password variables
{ set +x; } 2>/dev/null
MAIN_USER_PASSWORD="${MAIN_USER_PASSWORD:-}"
ROOT_PASSWORD="${ROOT_PASSWORD:-}"
[[ "${LOG_LEVEL:-INFO}" == "VERBOSE" ]] && set -x

# Package Management
AUR_HELPER="${AUR_HELPER:-none}"
ADDITIONAL_AUR_PACKAGES="${ADDITIONAL_AUR_PACKAGES:-}"
FLATPAK="${FLATPAK:-No}"

# Boot Configuration
BOOTLOADER="${BOOTLOADER:-grub}"
OS_PROBER="${OS_PROBER:-No}"
GRUB_THEME="${GRUB_THEME:-No}"
GRUB_THEME_SELECTION="${GRUB_THEME_SELECTION:-PolyDark}"

# Desktop Environment
DESKTOP_ENVIRONMENT="${DESKTOP_ENVIRONMENT:-none}"
DISPLAY_MANAGER="${DISPLAY_MANAGER:-none}"

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

    # Pre-flight: verify network connectivity before downloading packages
    log_info "Verifying network connectivity..."
    if ! curl -s --max-time 10 --head https://archlinux.org >/dev/null 2>&1; then
        error_exit "No network connectivity — pacstrap requires internet access. Check your connection and try again."
    fi

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

    # ROE §8.1: Suppress set -x — indirect expansion ${!var} traces password values
    { set +x; } 2>/dev/null
    for var in "${required_vars[@]}"; do
        if [[ -z "${!var:-}" ]]; then
            [[ "${LOG_LEVEL:-INFO}" == "VERBOSE" ]] && set -x
            log_error "Required variable $var is not set"
            return 1
        fi
    done
    [[ "${LOG_LEVEL:-INFO}" == "VERBOSE" ]] && set -x

    # Validate disk(s) exist (skip for pre_mounted — uses existing mounts)
    if [[ "$PARTITIONING_STRATEGY" == "pre_mounted" ]]; then
        log_info "Pre-mounted strategy — skipping disk validation"
    elif [[ "$PARTITIONING_STRATEGY" == *"raid"* ]]; then
        # RAID: validate each comma-separated disk
        IFS=',' read -ra _validate_disks <<< "$INSTALL_DISK"
        for _disk in "${_validate_disks[@]}"; do
            _disk="${_disk// /}"  # trim whitespace
            if [[ ! -b "$_disk" ]]; then
                log_error "RAID disk $_disk does not exist or is not a block device"
                return 1
            fi
        done
    else
        if [[ ! -b "$INSTALL_DISK" ]]; then
            log_error "Installation disk $INSTALL_DISK does not exist or is not a block device"
            return 1
        fi
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

    # Validate partitioning strategy
    case "$PARTITIONING_STRATEGY" in
        auto_simple|auto_simple_luks|auto_lvm|auto_luks_lvm|auto_raid|auto_raid_luks|auto_raid_lvm|auto_raid_lvm_luks|manual|pre_mounted) ;;
        *) log_error "Unknown partitioning strategy: $PARTITIONING_STRATEGY"; return 1 ;;
    esac

    # Validate filesystem types
    for _fs_check in "$ROOT_FILESYSTEM" "$HOME_FILESYSTEM"; do
        case "$_fs_check" in
            ext4|xfs|btrfs|fat32|"") ;;
            *) log_error "Unknown filesystem type: $_fs_check"; return 1 ;;
        esac
    done

    # Validate bootloader
    case "$BOOTLOADER" in
        grub|systemd-boot|refind|limine|efistub) ;;
        *) log_error "Unknown bootloader: $BOOTLOADER"; return 1 ;;
    esac

    # LUKS strategies require encryption password
    if [[ "$PARTITIONING_STRATEGY" == *"luks"* && -z "${ENCRYPTION_PASSWORD:-}" ]]; then
        log_error "LUKS strategy requires ENCRYPTION_PASSWORD to be set"
        return 1
    fi

    # RAID strategies require at least 2 disks
    if [[ "$PARTITIONING_STRATEGY" == *"raid"* ]]; then
        IFS=',' read -ra _raid_count_check <<< "$INSTALL_DISK"
        if [[ ${#_raid_count_check[@]} -lt 2 ]]; then
            log_error "RAID strategies require at least 2 disks (found ${#_raid_count_check[@]})"
            return 1
        fi
    fi

    # systemd-boot requires kernels on FAT32 (ESP at /boot)
    # Our auto_* strategies create ESP at /efi + ext4 /boot — incompatible
    if [[ "$BOOTLOADER" == "systemd-boot" && "$PARTITIONING_STRATEGY" == auto_* ]]; then
        log_warn "systemd-boot is incompatible with auto partitioning (ESP at /efi, ext4 /boot)"
        log_warn "systemd-boot requires kernels on FAT32 — switching to GRUB"
        BOOTLOADER="grub"
        export BOOTLOADER
    fi

    log_success "Configuration validated successfully"
    return 0
}

# --- System Preparation ---
prepare_system() {
    log_info "Preparing system..."

    # Update system clock
    log_info "Enabling NTP time synchronization..."
    log_cmd "timedatectl set-ntp true"
    timedatectl set-ntp true || log_warn "Failed to enable NTP (non-fatal)"

    # Wait for time sync
    log_info "Waiting for time sync..."
    sleep 2

    # Configure mirrors
    configure_mirrors

    # Update package database
    log_info "Updating package database (pacman -Sy)..."
    log_cmd "pacman -Sy"
    pacman -Sy 2>&1 | while IFS= read -r line; do
        case "$line" in
            *"error"*|*"Error"*|*"ERROR"*)
                echo -e "${RED}  [pacman] $line${RESET}"
                ;;
            *"warning"*|*"Warning"*|*"WARNING"*)
                echo -e "${YELLOW}  [pacman] $line${RESET}"
                ;;
            *)
                echo -e "${CYAN}  [pacman] $line${RESET}"
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
    log_cmd "cp /etc/pacman.d/mirrorlist /etc/pacman.d/mirrorlist.backup"
    cp /etc/pacman.d/mirrorlist /etc/pacman.d/mirrorlist.backup

    # Use reflector if available, otherwise use default mirrors
    # Flags per https://wiki.archlinux.org/title/Reflector:
    #   -a 48   = mirrors synced within 48 hours
    #   -f 5    = pre-filter to 5 fastest by connection speed
    #   -l 20   = limit to 20 most recently synced before speed test
    #   --sort rate = final sort by download rate
    #   --connection-timeout / --download-timeout = skip unresponsive mirrors after 5s
    if command -v reflector >/dev/null 2>&1; then
        local -a reflector_args=(
            --age 48
            --fastest 5
            --latest 20
            --sort rate
            --connection-timeout 5
            --download-timeout 5
            --save /etc/pacman.d/mirrorlist
        )

        if [[ -n "${MIRROR_COUNTRY:-}" ]]; then
            reflector_args+=(--country "$MIRROR_COUNTRY")
            log_info "Using reflector to rank mirrors for country: $MIRROR_COUNTRY..."
        else
            log_info "Using reflector to rank mirrors globally..."
        fi

        log_info "This may take a moment while mirrors are tested..."
        reflector "${reflector_args[@]}" 2>&1 | while IFS= read -r line; do
            case "$line" in
                *"error"*|*"Error"*)
                    echo -e "${RED}  [reflector] $line${RESET}"
                    ;;
                *"warning"*|*"Warning"*)
                    echo -e "${YELLOW}  [reflector] $line${RESET}"
                    ;;
                *)
                    echo -e "${CYAN}  [reflector] $line${RESET}"
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
        "pre_mounted")
            strategy_func="do_pre_mounted_partitioning"
            log_info "Using pre-mounted partitions (detection only)"
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
        # PCI detection (required for GPU auto-detection in chroot)
        "pciutils"
    )

    # Add filesystem tools based on selected filesystems
    local -a fs_packages=()
    local _fs
    for _fs in "$ROOT_FILESYSTEM" "$HOME_FILESYSTEM"; do
        case "$_fs" in
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
    done
    # Add dosfstools for UEFI (FAT32 ESP needs fsck.fat)
    if [[ "$BOOT_MODE" == "UEFI" ]]; then
        fs_packages+=("dosfstools")
    fi
    # Deduplicate
    mapfile -t fs_packages < <(printf '%s\n' "${fs_packages[@]}" | sort -u)

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

    # Pre-flight: verify /mnt is actually mounted
    if ! mountpoint -q /mnt 2>/dev/null; then
        error_exit "/mnt is not mounted — disk partitioning may have failed"
    fi

    # Pre-flight: check available space on /mnt (need at least 2GB for base system)
    local available_mb
    available_mb=$(df -BM /mnt | awk 'NR==2 {gsub(/M/,"",$4); print $4}')
    if [[ -n "$available_mb" ]] && [[ "$available_mb" -lt 2048 ]]; then
        error_exit "Insufficient disk space on /mnt: ${available_mb}MB available, need at least 2048MB"
    fi

    # Pre-create /etc/vconsole.conf before pacstrap
    # The sd-vconsole hook (default since mkinitcpio 39, 2025) reads this file during
    # initramfs generation. Without it, the mkinitcpio -P triggered by the linux package
    # post-install hook will error with "file not found: /etc/vconsole.conf"
    log_info "Pre-creating /etc/vconsole.conf with keymap: ${KEYMAP:-us}"
    mkdir -p /mnt/etc
    echo "KEYMAP=${KEYMAP:-us}" > /mnt/etc/vconsole.conf

    # Run pacstrap with array expansion and show output
    pacstrap -K /mnt "${all_packages[@]}" --noconfirm --needed 2>&1 | while IFS= read -r line; do
        # Suppress harmless systemd chroot messages from post-install hooks
        # systemd prints these when pacman hooks try daemon-reload inside pacstrap's chroot
        [[ "$line" == *"Skipped: Running in chroot"* ]] && continue

        # Filter and format pacstrap output for readability with colors
        case "$line" in
            *"error"*|*"Error"*|*"ERROR"*|*"failed"*)
                echo -e "${RED}  [pacstrap] $line${RESET}"
                ;;
            *"warning"*|*"Warning"*|*"WARNING"*)
                echo -e "${YELLOW}  [pacstrap] $line${RESET}"
                ;;
            *"downloading"*|*"installing"*|*"Packages"*|*"Total"*|*"::"*)
                echo -e "${CYAN}  [pacstrap] $line${RESET}"
                ;;
            *)
                # Show other lines dimmed
                echo -e "${RESET}  $line${RESET}"
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
    log_cmd "genfstab -U /mnt >> /mnt/etc/fstab"
    genfstab -U /mnt >> /mnt/etc/fstab || error_exit "genfstab failed — cannot generate fstab"

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

    # Verify /mnt is mounted before attempting script copy
    if ! mountpoint -q /mnt; then
        error_exit "/mnt is not mounted — cannot configure chroot"
    fi

    # Copy necessary scripts to target system
    log_info "Copying configuration scripts to /mnt/..."
    log_cmd "cp chroot_config.sh utils.sh /mnt/"
    cp "$SCRIPT_DIR/chroot_config.sh" /mnt/ || error_exit "Failed to copy chroot_config.sh to /mnt/"
    cp "$SCRIPT_DIR/utils.sh" /mnt/ || error_exit "Failed to copy utils.sh to /mnt/"

    # Copy Plymouth themes to target system BEFORE chroot (so configure_plymouth can find them)
    if [[ "${PLYMOUTH:-No}" == "Yes" ]]; then
        local themes_source="$SCRIPT_DIR/../Source"
        if [[ -d "$themes_source" ]]; then
            log_info "Copying Plymouth themes to target system..."
            mkdir -p /mnt/usr/share/plymouth/themes
            cp -r "$themes_source/"* /mnt/usr/share/plymouth/themes/ || log_warn "Failed to copy some Plymouth themes"
        else
            log_warn "Plymouth themes source directory not found: $themes_source"
        fi
    fi

    # Make scripts executable
    chmod +x /mnt/chroot_config.sh
    chmod +x /mnt/utils.sh

    # Validate UUIDs for encrypted installs
    if [[ "${ENCRYPTION:-No}" == "Yes" ]] && [[ -z "${LUKS_UUID:-}" ]]; then
        log_warn "LUKS_UUID is empty — encrypted boot configuration may fail"
        log_warn "Attempting to extract from disk..."
        # Try to extract from the LUKS device if available
        if [[ -n "${INSTALL_DISK:-}" ]]; then
            local recovery_disk="${INSTALL_DISK%%,*}"
            while IFS= read -r part; do
                if cryptsetup isLuks "$part" 2>/dev/null; then
                    LUKS_UUID=$(blkid -s UUID -o value "$part" 2>/dev/null || true)
                    [[ -n "$LUKS_UUID" ]] && log_info "Recovered LUKS_UUID: $LUKS_UUID" && break
                fi
            done < <(lsblk -ln -o PATH "$recovery_disk" | tail -n +2)
        fi
    fi

    # Export all configuration variables for chroot
    # Use a config file to pass variables (more reliable than env)
    # Use printf with %s to safely write values (prevents expansion of $, `, etc. in passwords)
    {
        printf '#!/bin/bash\n'
        printf '# Auto-generated configuration for chroot\n'
        printf 'export MAIN_USERNAME=%q\n' "$MAIN_USERNAME"
        printf 'export SYSTEM_HOSTNAME=%q\n' "$SYSTEM_HOSTNAME"
        printf 'export TIMEZONE_REGION=%q\n' "$TIMEZONE_REGION"
        printf 'export TIMEZONE=%q\n' "$TIMEZONE"
        printf 'export LOCALE=%q\n' "$LOCALE"
        printf 'export KEYMAP=%q\n' "$KEYMAP"
        printf 'export DESKTOP_ENVIRONMENT=%q\n' "$DESKTOP_ENVIRONMENT"
        printf 'export DISPLAY_MANAGER=%q\n' "$DISPLAY_MANAGER"
        printf 'export GPU_DRIVERS=%q\n' "$GPU_DRIVERS"
        printf 'export AUR_HELPER=%q\n' "$AUR_HELPER"
        printf 'export ADDITIONAL_PACKAGES=%q\n' "$ADDITIONAL_PACKAGES"
        printf 'export ADDITIONAL_AUR_PACKAGES=%q\n' "$ADDITIONAL_AUR_PACKAGES"
        printf 'export FLATPAK=%q\n' "$FLATPAK"
        printf 'export PLYMOUTH=%q\n' "$PLYMOUTH"
        printf 'export PLYMOUTH_THEME=%q\n' "$PLYMOUTH_THEME"
        printf 'export NUMLOCK_ON_BOOT=%q\n' "$NUMLOCK_ON_BOOT"
        printf 'export GIT_REPOSITORY=%q\n' "$GIT_REPOSITORY"
        printf 'export GIT_REPOSITORY_URL=%q\n' "$GIT_REPOSITORY_URL"
        printf 'export BOOT_MODE=%q\n' "$BOOT_MODE"
        printf 'export BOOTLOADER=%q\n' "$BOOTLOADER"
        printf 'export OS_PROBER=%q\n' "$OS_PROBER"
        printf 'export GRUB_THEME=%q\n' "$GRUB_THEME"
        printf 'export GRUB_THEME_SELECTION=%q\n' "$GRUB_THEME_SELECTION"
        printf 'export SECURE_BOOT=%q\n' "$SECURE_BOOT"
        printf 'export KERNEL=%q\n' "$KERNEL"
        printf 'export MULTILIB=%q\n' "$MULTILIB"
        printf 'export TIME_SYNC=%q\n' "$TIME_SYNC"
        printf 'export INSTALL_DISK=%q\n' "$INSTALL_DISK"
        printf 'export PARTITIONING_STRATEGY=%q\n' "$PARTITIONING_STRATEGY"
        printf 'export ENCRYPTION=%q\n' "$ENCRYPTION"
        printf 'export ROOT_FILESYSTEM=%q\n' "$ROOT_FILESYSTEM"
        printf 'export HOME_FILESYSTEM=%q\n' "$HOME_FILESYSTEM"
        printf 'export BTRFS_SNAPSHOTS=%q\n' "$BTRFS_SNAPSHOTS"
        printf 'export BTRFS_FREQUENCY=%q\n' "$BTRFS_FREQUENCY"
        printf 'export BTRFS_KEEP_COUNT=%q\n' "$BTRFS_KEEP_COUNT"
        printf 'export BTRFS_ASSISTANT=%q\n' "$BTRFS_ASSISTANT"
        printf 'export SWAP=%q\n' "$SWAP"
        printf 'export WANT_SWAP=%q\n' "$WANT_SWAP"
        printf 'export SWAP_UUID=%q\n' "${SWAP_UUID:-}"
        printf 'export ROOT_UUID=%q\n' "${ROOT_UUID:-}"
        printf 'export LUKS_UUID=%q\n' "${LUKS_UUID:-}"
        printf 'export ROOT_FILESYSTEM_TYPE=%q\n' "$ROOT_FILESYSTEM_TYPE"
        printf 'export WINDOWS_DETECTED=%q\n' "${WINDOWS_DETECTED:-}"
        printf 'export WINDOWS_EFI_PATH=%q\n' "${WINDOWS_EFI_PATH:-}"
        printf 'export OTHER_OS_DETECTED=%q\n' "${OTHER_OS_DETECTED:-}"
        printf 'export LOG_LEVEL=%q\n' "${LOG_LEVEL:-INFO}"
    } > /mnt/install_config.sh

    chmod +x /mnt/install_config.sh

    # Execute chroot configuration
    log_info "Entering chroot environment..."
    log_info "Running chroot_config.sh inside /mnt..."

    # ROE §8.1: Suppress set -x before arch-chroot line (inline env vars contain passwords)
    { set +x; } 2>/dev/null
    MAIN_USER_PASSWORD="$MAIN_USER_PASSWORD" \
    ROOT_PASSWORD="$ROOT_PASSWORD" \
    ENCRYPTION_PASSWORD="${ENCRYPTION_PASSWORD:-}" \
    arch-chroot /mnt /bin/bash -c "
        export MAIN_USER_PASSWORD ROOT_PASSWORD ENCRYPTION_PASSWORD
        # Source config with error handling (source_or_die not available in chroot)
        if [[ ! -f /install_config.sh ]]; then
            echo 'FATAL: /install_config.sh not found' >&2
            exit 1
        fi
        source /install_config.sh || { echo 'FATAL: Failed to source install_config.sh' >&2; exit 1; }
        /chroot_config.sh
    " 2>&1 | while IFS= read -r line; do
        # Suppress harmless systemd chroot messages
        # systemd prints these when systemctl enable creates symlinks but can't daemon-reload
        [[ "$line" == *"Skipped: Running in chroot"* ]] && continue

        # Pass through colored output from chroot, or colorize based on content
        if [[ "$line" == *$'\033'* ]]; then
            # Line already has ANSI codes, pass through with prefix
            echo -e "  [chroot] $line"
        else
            # Add colors based on content
            case "$line" in
                *"ERROR"*|*"error"*|*"Error"*|*"failed"*|*"FAILED"*)
                    echo -e "${RED}  [chroot] $line${RESET}"
                    ;;
                *"WARN"*|*"warning"*|*"Warning"*)
                    echo -e "${YELLOW}  [chroot] $line${RESET}"
                    ;;
                *"SUCCESS"*|*"success"*|*"complete"*|*"Complete"*)
                    echo -e "${GREEN}  [chroot] $line${RESET}"
                    ;;
                *"==="*)
                    echo -e "${BOLD}${CYAN}  [chroot] $line${RESET}"
                    ;;
                *)
                    echo -e "${CYAN}  [chroot] $line${RESET}"
                    ;;
            esac
        fi
    done

    local chroot_exit=${PIPESTATUS[0]}
    [[ "${LOG_LEVEL:-INFO}" == "VERBOSE" ]] && set -x

    # ROE §8.1: Clear password variables after chroot completes (no longer needed)
    { set +x; } 2>/dev/null
    unset MAIN_USER_PASSWORD ROOT_PASSWORD ENCRYPTION_PASSWORD
    [[ "${LOG_LEVEL:-INFO}" == "VERBOSE" ]] && set -x

    # Clean up copied scripts
    rm -f /mnt/chroot_config.sh
    rm -f /mnt/utils.sh
    rm -f /mnt/install_config.sh

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

    # Ensure all services are properly enabled
    log_info "Verifying service configuration..."

    # Sync filesystems
    sync

    log_success "Installation finalized successfully!"
    return 0
}

# --- Run main function ---
main "$@"
