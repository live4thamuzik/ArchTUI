#!/bin/bash
# install.sh - Complete Arch Linux Installation Engine (TUI-Only)
# This script handles the entire installation process from partitioning to completion

set -euo pipefail

# Debug: Show script startup
echo "=== INSTALLATION ENGINE STARTED ==="
echo "Script: install.sh"
echo "PID: $$"
echo "Mode: TUI-only"
echo "Working Directory: $(pwd)"
echo "User: $(whoami)"
echo "=========================================="

# Source utility functions and strategies
source "$(dirname "${BASH_SOURCE[0]}")/utils.sh"
source "$(dirname "${BASH_SOURCE[0]}")/disk_strategies.sh"

# Initialize logging
setup_logging

# --- Configuration Variables ---
# These will be set from environment variables passed by the TUI

# Boot Configuration
BOOT_MODE="${BOOT_MODE:-Auto}"
SECURE_BOOT="${SECURE_BOOT:-No}"

# System Locale and Input
LOCALE="${LOCALE:-en_US.UTF-8}"
KEYMAP="${KEYMAP:-us}"

# Disk and Storage
INSTALL_DISK="${INSTALL_DISK:-/dev/sda}"
PARTITIONING_STRATEGY="${PARTITIONING_STRATEGY:-Auto}"
ENCRYPTION="${ENCRYPTION:-No}"
ROOT_FILESYSTEM="${ROOT_FILESYSTEM:-ext4}"
SEPARATE_HOME="${SEPARATE_HOME:-No}"
HOME_FILESYSTEM="${HOME_FILESYSTEM:-ext4}"
SWAP="${SWAP:-Yes}"
SWAP_SIZE="${SWAP_SIZE:-2GB}"

# Convert TUI variables to internal Bash variables (required by modular strategies)
ROOT_FILESYSTEM_TYPE="$ROOT_FILESYSTEM"
HOME_FILESYSTEM_TYPE="$HOME_FILESYSTEM"

# Convert Yes/No to yes/no for internal use (required by modular strategies)
WANT_HOME_PARTITION="$(echo "$SEPARATE_HOME" | tr '[:upper:]' '[:lower:]')"
WANT_SWAP="$(echo "${SWAP:-Yes}" | tr '[:upper:]' '[:lower:]')"

# BIOS always needs separate boot partition, UEFI uses ESP+XBOOTLDR
if [ "$BOOT_MODE" = "BIOS" ]; then
    WANT_SEPARATE_BOOT="yes"
else
    WANT_SEPARATE_BOOT="no"  # UEFI uses ESP+XBOOTLDR approach
fi
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
DESKTOP_ENVIRONMENT="${DESKTOP_ENVIRONMENT:-KDE}"
DISPLAY_MANAGER="${DISPLAY_MANAGER:-sddm}"

# Boot Splash and Final Setup
PLYMOUTH="${PLYMOUTH:-Yes}"
PLYMOUTH_THEME="${PLYMOUTH_THEME:-arch-glow}"
NUMLOCK_ON_BOOT="${NUMLOCK_ON_BOOT:-Yes}"
GIT_REPOSITORY="${GIT_REPOSITORY:-No}"
GIT_REPOSITORY_URL="${GIT_REPOSITORY_URL:-}"

# Note: Additional configuration variables are defined above to avoid duplication

# --- Source Disk Strategies ---
# Load partitioning constants and functions from disk_strategies.sh
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$SCRIPT_DIR/disk_strategies.sh"

# Additional partitioning constants specific to install.sh
readonly XBOOTLDR_PARTITION_TYPE="EA00"  # Extended Boot Loader Partition
readonly EFI_PART_SIZE_MIB=100  # 100MB sufficient for bootloader files only
readonly XBOOTLDR_PART_SIZE_MIB=1024  # 1GB for kernel files

# --- Main Installation Function ---
main() {
    echo "Starting Arch Linux installation..."
    
    # Phase 1: Validate configuration
    if ! validate_configuration; then
        error_exit "Configuration validation failed"
    fi
    
    # Phase 2: Prepare system
    if ! prepare_system; then
        error_exit "System preparation failed"
    fi
    
    # Phase 2.5: Check and install dependencies
    if ! check_and_install_dependencies; then
        error_exit "Dependency installation failed"
    fi
    
    # Phase 3: Partition disk
    if ! partition_disk; then
        error_exit "Disk partitioning failed"
    fi
    
    # Phase 4: Install base system
    if ! install_base_system; then
        error_exit "Base system installation failed"
    fi
    
    # Phase 5: Configure system
    if ! configure_system; then
        error_exit "System configuration failed"
    fi
    
    # Phase 6: Configure chroot
    if ! configure_chroot; then
        error_exit "Chroot configuration failed"
    fi
    
    # Phase 7: Install packages
    if ! install_packages; then
        error_exit "Package installation failed"
    fi
    
    # Phase 8: Configure bootloader
    if ! configure_bootloader; then
        error_exit "Bootloader configuration failed"
    fi
    
    # Phase 9: Configure desktop environment
    if ! configure_desktop_environment; then
        error_exit "Desktop environment configuration failed"
    fi
    
    # Phase 10: Finalize installation
    if ! finalize_installation; then
        error_exit "Installation finalization failed"
    fi
    
    echo "Installation complete!"
}

# --- Validation Functions ---
validate_configuration() {
    echo "Validating configuration..."
    
    local required_vars=(
        "INSTALL_DISK"
        "MAIN_USERNAME"
        "ROOT_PASSWORD"
        "MAIN_USER_PASSWORD"
        "SYSTEM_HOSTNAME"
    )
    
    for var in "${required_vars[@]}"; do
        if [ -z "${!var:-}" ]; then
            echo "ERROR: Required environment variable $var is not set"
            exit 1
        fi
    done
    
    # Validate disk exists
    if [ ! -b "$INSTALL_DISK" ]; then
        echo "ERROR: Installation disk $INSTALL_DISK does not exist or is not a block device"
        exit 1
    fi
    
    # Auto-detect boot mode if needed
    if [ "$BOOT_MODE" = "Auto" ]; then
        if [ -d "/sys/firmware/efi/efivars" ]; then
            BOOT_MODE="UEFI"
            echo "Auto-detected UEFI boot mode"
        else
            BOOT_MODE="BIOS"
            echo "Auto-detected BIOS boot mode"
        fi
    fi
    
    # Validate boot mode
    if [ "$BOOT_MODE" = "UEFI" ]; then
        if [ ! -d "/sys/firmware/efi/efivars" ]; then
            echo "ERROR: System is not booted in UEFI mode but BOOT_MODE is set to UEFI"
            exit 1
        fi
        echo "Confirmed UEFI boot mode"
    elif [ "$BOOT_MODE" = "BIOS" ]; then
        echo "Confirmed BIOS boot mode"
    else
        echo "ERROR: Invalid BOOT_MODE: $BOOT_MODE (must be Auto, UEFI, or BIOS)"
        exit 1
    fi
    
    echo "Configuration validated successfully"
}

# --- ESP Detection Functions ---
detect_existing_esp() {
    echo "Checking for existing EFI System Partition..."
    
    # Check if we're in UEFI mode
    if [ ! -d "/sys/firmware/efi/efivars" ]; then
        echo "System not in UEFI mode, skipping ESP detection"
        return 1
    fi
    
    # Look for existing ESP partitions
    local esp_partitions=()
    while IFS= read -r line; do
        local part_name=$(echo "$line" | awk '{print $1}')
        local part_type=$(echo "$line" | awk '{print $6}')
        
        if [[ "$part_type" =~ "EFI" ]] || [[ "$part_type" =~ "FAT" ]]; then
            # Verify it's actually an ESP by checking for EFI directory
            if mount | grep -q "$part_name"; then
                local mount_point=$(mount | grep "$part_name" | awk '{print $3}')
                if [ -d "$mount_point/EFI" ]; then
                    esp_partitions+=("$part_name")
                    echo "Found existing ESP: $part_name mounted at $mount_point"
                fi
            else
                # Try to mount temporarily to check
                local temp_mount="/tmp/esp_check_$$"
                mkdir -p "$temp_mount"
                if mount "$part_name" "$temp_mount" 2>/dev/null; then
                    if [ -d "$temp_mount/EFI" ]; then
                        esp_partitions+=("$part_name")
                        echo "Found existing ESP: $part_name"
                    fi
                    umount "$temp_mount" 2>/dev/null
                fi
                rmdir "$temp_mount" 2>/dev/null
            fi
        fi
    done < <(lsblk -f -n -o NAME,FSTYPE,SIZE,TYPE,LABEL,PTTYPE | grep -E "(EFI|FAT)" | grep -v loop)
    
    if [ ${#esp_partitions[@]} -gt 0 ]; then
        echo "Existing ESP partitions found: ${esp_partitions[*]}"
        return 0
    else
        echo "No existing ESP partitions found"
        return 1
    fi
}

# --- System Preparation ---
prepare_system() {
    echo "Preparing system..."
    
    # Update system clock
    timedatectl set-ntp true
    
    # Configure mirrors
    configure_mirrors
    
    # Update package database
    pacman -Sy
    
    echo "System prepared"
}

configure_mirrors() {
    echo "Configuring package mirrors..."
    
    # Create a basic mirrorlist
    cat > /etc/pacman.d/mirrorlist << EOF
# United States mirrors
Server = https://mirror.rackspace.com/archlinux/\$repo/os/\$arch
Server = https://mirror.lty.me/archlinux/\$repo/os/\$arch
Server = https://mirror.umd.edu/archlinux/\$repo/os/\$arch
Server = https://mirror.clarkson.edu/archlinux/\$repo/os/\$arch
EOF
    
    # Enable multilib if requested
    if [ "$MULTILIB" = "Yes" ]; then
        sed -i '/\[multilib\]/,/^$/s/^#//' /etc/pacman.conf
    fi
}

# --- Disk Partitioning ---
partition_disk() {
    echo "Starting disk partitioning..."
    
    # Check for existing ESP first
    if [ "$BOOT_MODE" = "UEFI" ]; then
        if detect_existing_esp; then
            echo "Using existing ESP partition"
            # TODO: Implement logic to use existing ESP
            # For now, we'll create a new one
        fi
    fi
    
    # Map TUI partitioning options to disk strategy functions
    local strategy=""
    case "$PARTITIONING_STRATEGY" in
        "auto_simple")
            strategy="do_auto_simple_partitioning_efi_xbootldr"  # Use ESP + XBOOTLDR (Arch Wiki recommended)
            ;;
        "auto_simple_luks")
            strategy="do_auto_simple_luks_partitioning"
            ;;
        "auto_lvm")
            strategy="do_auto_lvm_partitioning_efi_xbootldr"  # Use ESP + XBOOTLDR (Arch Wiki recommended)
            ;;
        "auto_luks_lvm")
            strategy="do_auto_luks_lvm_partitioning"
            ;;
        "auto_raid")
            strategy="do_auto_raid_partitioning"
            ;;
        "auto_raid_luks")
            strategy="do_auto_raid_luks_partitioning"
            ;;
        "auto_raid_lvm")
            strategy="do_auto_raid_lvm_partitioning"  # RAID + LVM partitioning
            ;;
        "auto_raid_lvm_luks")
            strategy="do_auto_raid_lvm_luks_partitioning"
            ;;
        "manual")
            strategy="do_manual_partitioning_guided"
            ;;
        *)
            echo "ERROR: Unknown partitioning strategy: $PARTITIONING_STRATEGY"
            echo "Available strategies: auto_simple, auto_simple_luks, auto_lvm, auto_luks_lvm, auto_raid, auto_raid_luks, auto_raid_lvm, auto_raid_lvm_luks, manual"
            exit 1
            ;;
    esac
    
    # Execute the disk strategy
    execute_disk_strategy "$strategy"
    
    echo "Disk partitioning complete"
}

# --- Chroot Configuration ---
configure_chroot() {
    echo "Configuring system in chroot..."
    
    # Copy chroot configuration script to target system
    cp "$(dirname "${BASH_SOURCE[0]}")/chroot_config.sh" /mnt/
    cp "$(dirname "${BASH_SOURCE[0]}")/utils.sh" /mnt/
    cp "$(dirname "${BASH_SOURCE[0]}")/disk_strategies.sh" /mnt/
    
    # Export all configuration variables for chroot
    export MAIN_USERNAME MAIN_USER_PASSWORD ROOT_PASSWORD
    export SYSTEM_HOSTNAME TIMEZONE_REGION TIMEZONE LOCALE KEYMAP
    export DESKTOP_ENVIRONMENT DISPLAY_MANAGER GPU_DRIVERS
    export AUR_HELPER ADDITIONAL_AUR_PACKAGES FLATPAK
    export PLYMOUTH PLYMOUTH_THEME NUMLOCK_ON_BOOT GIT_REPOSITORY GIT_REPOSITORY_URL
    export ROOT_FILESYSTEM HOME_FILESYSTEM BTRFS_SNAPSHOTS
    export BOOT_MODE SECURE_BOOT PARTITIONING_STRATEGY ENCRYPTION INSTALL_DISK
    export SEPARATE_HOME SWAP SWAP_SIZE BTRFS_FREQUENCY BTRFS_KEEP_COUNT BTRFS_ASSISTANT
    export ROOT_FILESYSTEM_TYPE HOME_FILESYSTEM_TYPE WANT_HOME_PARTITION WANT_SWAP WANT_SEPARATE_BOOT
    export TIME_SYNC MIRROR_COUNTRY KERNEL MULTILIB ADDITIONAL_PACKAGES
    export BOOTLOADER OS_PROBER GRUB_THEME GRUB_THEME_SELECTION
    
    # Set PASSED_ variables for chroot script
    export PASSED_MAIN_USERNAME="$MAIN_USERNAME"
    export PASSED_MAIN_USER_PASSWORD="$MAIN_USER_PASSWORD"
    export PASSED_ROOT_PASSWORD="$ROOT_PASSWORD"
    export PASSED_SYSTEM_HOSTNAME="$SYSTEM_HOSTNAME"
    export PASSED_TIMEZONE="$TIMEZONE"
    export PASSED_LOCALE="$LOCALE"
    export PASSED_KEYMAP="$KEYMAP"
    export PASSED_DESKTOP_ENVIRONMENT="$DESKTOP_ENVIRONMENT"
    export PASSED_DISPLAY_MANAGER="$DISPLAY_MANAGER"
    export PASSED_AUR_HELPER="$AUR_HELPER"
    export PASSED_ADDITIONAL_AUR_PACKAGES="$ADDITIONAL_AUR_PACKAGES"
    export PASSED_FLATPAK="$FLATPAK"
    export PASSED_PLYMOUTH="$PLYMOUTH"
    export PASSED_PLYMOUTH_THEME="$PLYMOUTH_THEME"
    export PASSED_NUMLOCK_ON_BOOT="$NUMLOCK_ON_BOOT"
    export PASSED_GIT_REPOSITORY="$GIT_REPOSITORY"
    
    # Execute chroot configuration
    arch-chroot /mnt bash /chroot_config.sh
    
    # Clean up copied scripts
    rm -f /mnt/chroot_config.sh /mnt/utils.sh /mnt/disk_strategies.sh
    
    echo "Chroot configuration complete"
}

# --- Package Installation Functions ---
install_packages() {
    echo "Installing packages..."
    
    # Install base packages
    install_base_packages
    
    # Install desktop environment
    install_desktop_environment
    
    # Install additional packages
    install_additional_packages
    
    # Install AUR helper and packages
    install_aur_packages
    
    # Install GPU drivers
    install_gpu_drivers
    
    echo "Package installation complete"
}

install_base_packages() {
    echo "Installing base packages..."
    
    # Base package list
    local base_packages="base base-devel linux-firmware"
    
    # Add kernel
    base_packages="$base_packages $KERNEL"
    
    # Add essential packages
    base_packages="$base_packages nano vim sudo networkmanager openssh"
    
    # Add multilib if enabled
    if [ "$MULTILIB" = "Yes" ]; then
        base_packages="$base_packages lib32-glibc"
    fi
    
    # Install packages
    pacstrap /mnt $base_packages
    
    echo "Base packages installed"
}

install_desktop_environment() {
    echo "Installing desktop environment: $DESKTOP_ENVIRONMENT"
    
    case "$DESKTOP_ENVIRONMENT" in
        "KDE")
            pacstrap /mnt plasma kde-applications
            ;;
        "GNOME")
            pacstrap /mnt gnome gnome-extra
            ;;
        "XFCE")
            pacstrap /mnt xfce4 xfce4-goodies
            ;;
        "i3")
            pacstrap /mnt i3-wm i3status i3lock dmenu
            ;;
        "No")
            echo "No desktop environment selected"
            ;;
        *)
            echo "Unknown desktop environment: $DESKTOP_ENVIRONMENT"
            ;;
    esac
    
    echo "Desktop environment installation complete"
}

install_additional_packages() {
    echo "Installing additional packages..."
    
    if [ -n "$ADDITIONAL_PACKAGES" ] && [ "$ADDITIONAL_PACKAGES" != "" ]; then
        echo "Installing additional packages: $ADDITIONAL_PACKAGES"
        pacstrap /mnt $ADDITIONAL_PACKAGES
    else
        echo "No additional packages to install"
    fi
    
    echo "Additional package installation complete"
}

install_aur_packages() {
    echo "Installing AUR helper and packages..."
    
    if [ "$AUR_HELPER" != "none" ] && [ -n "$AUR_HELPER" ]; then
        # Install AUR helper in chroot
        arch-chroot /mnt bash -c "
            cd /tmp
            git clone https://aur.archlinux.org/$AUR_HELPER.git
            cd $AUR_HELPER
            makepkg -si --noconfirm
            cd /
            rm -rf /tmp/$AUR_HELPER
        "
        
        # Install AUR packages if specified
        if [ -n "$ADDITIONAL_AUR_PACKAGES" ] && [ "$ADDITIONAL_AUR_PACKAGES" != "" ]; then
            echo "Installing AUR packages: $ADDITIONAL_AUR_PACKAGES"
            arch-chroot /mnt $AUR_HELPER -S $ADDITIONAL_AUR_PACKAGES --noconfirm
        fi
    else
        echo "No AUR helper selected"
    fi
    
    echo "AUR package installation complete"
}

install_gpu_drivers() {
    echo "Installing GPU drivers: $GPU_DRIVERS"
    
    case "$GPU_DRIVERS" in
        "Auto")
            # Auto-detect and install appropriate drivers
            if lspci | grep -i vga | grep -i nvidia; then
                pacstrap /mnt nvidia nvidia-utils
            elif lspci | grep -i vga | grep -i amd; then
                pacstrap /mnt mesa xf86-video-amdgpu
            elif lspci | grep -i vga | grep -i intel; then
                pacstrap /mnt mesa xf86-video-intel
            else
                pacstrap /mnt mesa xf86-video-vesa
            fi
            ;;
        "NVIDIA")
            pacstrap /mnt nvidia nvidia-utils
            ;;
        "AMD")
            pacstrap /mnt mesa xf86-video-amdgpu
            ;;
        "Intel")
            pacstrap /mnt mesa xf86-video-intel
            ;;
        *)
            echo "Unknown GPU driver: $GPU_DRIVERS"
            pacstrap /mnt mesa xf86-video-vesa
            ;;
    esac
    
    echo "GPU driver installation complete"
}

# --- System Configuration Functions ---
configure_system() {
    echo "Configuring system..."
    
    # Configure locale
    configure_locale
    
    # Configure timezone
    configure_timezone
    
    # Configure hostname
    configure_hostname
    
    # Configure users
    configure_users
    
    # Configure network
    configure_network
    
    # Configure services
    configure_services
    
    echo "System configuration complete"
}

configure_locale() {
    echo "Configuring locale: $LOCALE"
    
    # Generate locale
    echo "$LOCALE UTF-8" > /mnt/etc/locale.gen
    arch-chroot /mnt locale-gen
    
    # Set system locale
    echo "LANG=$LOCALE" > /mnt/etc/locale.conf
    
    # Configure console keymap
    echo "KEYMAP=$KEYMAP" > /mnt/etc/vconsole.conf
    
    echo "Locale configuration complete"
}

configure_timezone() {
    echo "Configuring timezone: $TIMEZONE_REGION/$TIMEZONE"
    
    # Set timezone
    arch-chroot /mnt ln -sf /usr/share/zoneinfo/$TIMEZONE_REGION/$TIMEZONE /etc/localtime
    arch-chroot /mnt hwclock --systohc
    
    # Configure NTP if enabled
    if [ "$TIME_SYNC" = "Yes" ]; then
        arch-chroot /mnt systemctl enable systemd-timesyncd
    fi
    
    echo "Timezone configuration complete"
}

configure_hostname() {
    echo "Configuring hostname: $SYSTEM_HOSTNAME"
    
    # Set hostname
    echo "$SYSTEM_HOSTNAME" > /mnt/etc/hostname
    
    # Configure hosts file
    cat > /mnt/etc/hosts << EOF
127.0.0.1	localhost
::1		localhost
127.0.1.1	$SYSTEM_HOSTNAME.localdomain	$SYSTEM_HOSTNAME
EOF
    
    echo "Hostname configuration complete"
}

configure_users() {
    echo "Configuring users..."
    
    # Set root password
    echo "root:$ROOT_PASSWORD" | arch-chroot /mnt chpasswd
    
    # Create main user
    arch-chroot /mnt useradd -m -G wheel -s /bin/bash "$MAIN_USERNAME"
    echo "$MAIN_USERNAME:$MAIN_USER_PASSWORD" | arch-chroot /mnt chpasswd
    
    # Configure sudo
    echo "%wheel ALL=(ALL) ALL" > /mnt/etc/sudoers.d/wheel
    
    echo "User configuration complete"
}

configure_network() {
    echo "Configuring network..."
    
    # Enable NetworkManager
    arch-chroot /mnt systemctl enable NetworkManager
    
    echo "Network configuration complete"
}

configure_services() {
    echo "Configuring services..."
    
    # Enable essential services
    arch-chroot /mnt systemctl enable sshd
    
    echo "Service configuration complete"
}

# --- Enhanced Bootloader Configuration ---
configure_bootloader() {
    echo "Configuring bootloader: $BOOTLOADER"
    
    case "$BOOTLOADER" in
        "grub")
            configure_grub
            ;;
        "systemd-boot")
            configure_systemd_boot
            ;;
        *)
            echo "Unknown bootloader: $BOOTLOADER"
            configure_grub  # Default to GRUB
            ;;
    esac
    
    echo "Bootloader configuration complete"
}

configure_grub() {
    echo "Installing GRUB..."
    
    if [ "$BOOT_MODE" = "UEFI" ]; then
        # ESP is mounted at /efi (Arch Wiki recommended for dual-boot)
        arch-chroot /mnt grub-install --target=x86_64-efi --efi-directory=/efi --bootloader-id=grub_uefi --recheck
    else
        # BIOS mode
        arch-chroot /mnt grub-install --target=i386-pc "$INSTALL_DISK"
    fi
    
    # Configure GRUB
    arch-chroot /mnt grub-mkconfig -o /boot/grub/grub.cfg
    
    # Install GRUB theme if specified
    if [ "$GRUB_THEME" != "none" ] && [ -n "$GRUB_THEME" ]; then
        install_grub_theme
    fi
    
    echo "GRUB installation complete"
}

install_grub_theme() {
    echo "Installing GRUB theme: $GRUB_THEME"
    
    # Create GRUB themes directory
    arch-chroot /mnt mkdir -p /boot/grub/themes
    
    # Clone and install theme
    arch-chroot /mnt bash -c "
        cd /tmp
        git clone https://github.com/vinceliuice/grub2-themes.git
        cd grub2-themes
        ./install.sh -t $GRUB_THEME -s 1080p
        cd /
        rm -rf /tmp/grub2-themes
    "
    
    # Update GRUB configuration
    arch-chroot /mnt grub-mkconfig -o /boot/grub/grub.cfg
    
    echo "GRUB theme installation complete"
}

configure_systemd_boot() {
    echo "Installing systemd-boot..."
    
    if [ "$BOOT_MODE" = "UEFI" ]; then
        # ESP is mounted at /efi (Arch Wiki recommended for dual-boot)
        arch-chroot /mnt bootctl --path=/efi install
        
        # Create boot entry
        cat > /mnt/efi/loader/entries/arch.conf << EOF
title Arch Linux
linux /vmlinuz-linux
initrd /initramfs-linux.img
options root=UUID=$ROOT_UUID rw
EOF
        
        # Configure loader
        cat > /mnt/efi/loader/loader.conf << EOF
default arch
timeout 5
EOF
    else
        echo "systemd-boot only supports UEFI mode"
        exit 1
    fi
    
    echo "systemd-boot installation complete"
}

# --- Desktop Environment Configuration ---
configure_desktop_environment() {
    echo "Configuring desktop environment..."
    
    if [ "$DESKTOP_ENVIRONMENT" != "No" ]; then
        # Configure display manager
        configure_display_manager
        
        # Configure desktop-specific settings
        configure_desktop_settings
        
        # Configure Plymouth if enabled
        if [ "$PLYMOUTH" = "Yes" ]; then
            configure_plymouth
        fi
    fi
    
    echo "Desktop environment configuration complete"
}

configure_display_manager() {
    echo "Configuring display manager: $DISPLAY_MANAGER"
    
    case "$DISPLAY_MANAGER" in
        "sddm")
            arch-chroot /mnt systemctl enable sddm
            ;;
        "gdm")
            arch-chroot /mnt systemctl enable gdm
            ;;
        "lightdm")
            arch-chroot /mnt systemctl enable lightdm
            ;;
        "No")
            echo "No display manager selected"
            ;;
        *)
            echo "Unknown display manager: $DISPLAY_MANAGER"
            ;;
    esac
    
    echo "Display manager configuration complete"
}

configure_desktop_settings() {
    echo "Configuring desktop settings..."
    
    case "$DESKTOP_ENVIRONMENT" in
        "KDE")
            configure_kde
            ;;
        "GNOME")
            configure_gnome
            ;;
        "XFCE")
            configure_xfce
            ;;
        "i3")
            configure_i3
            ;;
        *)
            echo "No specific desktop configuration for: $DESKTOP_ENVIRONMENT"
            ;;
    esac
    
    echo "Desktop settings configuration complete"
}

configure_kde() {
    echo "Configuring KDE..."
    
    # Enable KDE services
    arch-chroot /mnt systemctl enable sddm
    
    # Configure KDE settings
    arch-chroot /mnt bash -c "
        sudo -u $MAIN_USERNAME mkdir -p /home/$MAIN_USERNAME/.config
        echo '[General]' > /home/$MAIN_USERNAME/.config/kdeglobals
        echo 'ColorScheme=Breeze' >> /home/$MAIN_USERNAME/.config/kdeglobals
    "
    
    echo "KDE configuration complete"
}

configure_gnome() {
    echo "Configuring GNOME..."
    
    # Enable GNOME services
    arch-chroot /mnt systemctl enable gdm
    
    echo "GNOME configuration complete"
}

configure_xfce() {
    echo "Configuring XFCE..."
    
    # Enable XFCE services
    arch-chroot /mnt systemctl enable lightdm
    
    echo "XFCE configuration complete"
}

configure_i3() {
    echo "Configuring i3..."
    
    # Create basic i3 config
    arch-chroot /mnt bash -c "
        sudo -u $MAIN_USERNAME mkdir -p /home/$MAIN_USERNAME/.config/i3
        sudo -u $MAIN_USERNAME mkdir -p /home/$MAIN_USERNAME/.config/i3status
    "
    
    echo "i3 configuration complete"
}

configure_plymouth() {
    echo "Configuring Plymouth theme: $PLYMOUTH_THEME"
    
    if [ "$PLYMOUTH_THEME" != "none" ]; then
        # Install Plymouth
        pacstrap /mnt plymouth
        
        # Copy Plymouth theme
        if [ -d "Source/$PLYMOUTH_THEME" ]; then
            cp -r "Source/$PLYMOUTH_THEME" /mnt/usr/share/plymouth/themes/
            
            # Configure Plymouth
            arch-chroot /mnt plymouth-set-default-theme -R "$PLYMOUTH_THEME"
        fi
        
        # Enable Plymouth
        arch-chroot /mnt systemctl enable plymouth-quit-wait.service
    fi
    
    echo "Plymouth configuration complete"
}

# --- Final System Configuration ---
finalize_installation() {
    echo "Finalizing installation..."
    
    # Configure numlock
    if [ "$NUMLOCK_ON_BOOT" = "Yes" ]; then
        configure_numlock
    fi
    
    # Configure Flatpak if enabled
    if [ "$FLATPAK" = "Yes" ]; then
        configure_flatpak
    fi
    
    # Clone git repository if specified
    if [ "$GIT_REPOSITORY" = "Yes" ]; then
        clone_git_repository
    fi
    
    # Final system updates
    arch-chroot /mnt pacman -Syu --noconfirm
    
    echo "Installation finalization complete"
}

configure_numlock() {
    echo "Configuring numlock on boot..."
    
    # Enable numlock service
    arch-chroot /mnt systemctl enable numlock-on-console.service
    
    echo "Numlock configuration complete"
}

configure_flatpak() {
    echo "Configuring Flatpak..."
    
    # Install Flatpak
    pacstrap /mnt flatpak
    
    # Configure Flatpak for user
    arch-chroot /mnt bash -c "
        sudo -u $MAIN_USERNAME flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
    "
    
    echo "Flatpak configuration complete"
}

clone_git_repository() {
    echo "Cloning git repository..."
    
    # Clone the installation repository
    if [ -n "$GIT_REPOSITORY_URL" ]; then
        arch-chroot /mnt bash -c "
            cd /home/$MAIN_USERNAME
            sudo -u $MAIN_USERNAME git clone $GIT_REPOSITORY_URL
        "
    else
        echo "No git repository URL specified"
    fi
    
    echo "Git repository cloning complete"
}

# --- Partitioning functions are now in modular scripts/strategies/ ---
# See scripts/disk_strategies.sh for the main dispatcher
# Individual strategies are in scripts/strategies/*.sh

# --- Service Configuration ---
configure_services() {
    echo "Configuring services..."
    
    # Enable NetworkManager
    arch-chroot /mnt systemctl enable NetworkManager
    
    # Enable display manager
    case "$DISPLAY_MANAGER" in
        "sddm")
            arch-chroot /mnt systemctl enable sddm
            ;;
        "gdm")
            arch-chroot /mnt systemctl enable gdm
            ;;
        "lightdm")
            arch-chroot /mnt systemctl enable lightdm
            ;;
    esac
    
    # Enable numlock
    if [ "$NUMLOCK_ON_BOOT" = "Yes" ]; then
        echo "setleds +num" >> /mnt/etc/profile
    fi
    
    echo "Services configured"
}

# --- Finalization ---
finalize_installation() {
    echo "Finalizing installation..."
    
    # Copy Plymouth themes to target system
    if [ "$PLYMOUTH" = "Yes" ] && [ -d "Source" ]; then
        echo "Copying Plymouth themes..."
        cp -r Source/* /mnt/usr/share/plymouth/themes/ 2>/dev/null || true
    fi
    
    echo "Installation finalized successfully!"
}

# Plymouth theme installation is now handled in chroot_config.sh

# --- Run main function ---
main "$@"
