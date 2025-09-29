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
SWAP_SIZE="${SWAP_SIZE:-2GB}"
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

# Desktop Environment
DESKTOP_ENVIRONMENT="${DESKTOP_ENVIRONMENT:-KDE}"
DISPLAY_MANAGER="${DISPLAY_MANAGER:-sddm}"

# Boot Splash and Final Setup
PLYMOUTH="${PLYMOUTH:-Yes}"
PLYMOUTH_THEME="${PLYMOUTH_THEME:-arch-glow}"
NUMLOCK_ON_BOOT="${NUMLOCK_ON_BOOT:-Yes}"
GIT_REPOSITORY="${GIT_REPOSITORY:-No}"

# Note: Additional configuration variables are defined above to avoid duplication

# --- Partitioning Constants ---
readonly EFI_PARTITION_TYPE="EF00"
readonly LINUX_PARTITION_TYPE="8300"
readonly SWAP_PARTITION_TYPE="8200"
readonly XBOOTLDR_PARTITION_TYPE="EA00"  # Extended Boot Loader Partition
readonly DEFAULT_SWAP_SIZE_MIB=2048
readonly DEFAULT_ROOT_SIZE_MIB=102400
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
        "auto_raid_simple")
            strategy="do_auto_raid_simple_partitioning"
            ;;
        "auto_raid_lvm")
            strategy="do_auto_raid_lvm_partitioning"
            ;;
        "auto_btrfs")
            strategy="do_auto_btrfs_partitioning_efi_xbootldr"  # Use ESP + XBOOTLDR (Arch Wiki recommended)
            ;;
        "manual")
            strategy="do_manual_partitioning_guided"
            ;;
        *)
            echo "ERROR: Unknown partitioning strategy: $PARTITIONING_STRATEGY"
            echo "Available strategies: auto_simple, auto_simple_luks, auto_lvm, auto_luks_lvm, auto_btrfs, auto_raid_simple, auto_raid_lvm, manual"
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
    export PLYMOUTH PLYMOUTH_THEME NUMLOCK_ON_BOOT GIT_REPOSITORY
    export ROOT_FILESYSTEM HOME_FILESYSTEM BTRFS_SNAPSHOTS
    export BOOT_MODE SECURE_BOOT PARTITIONING_STRATEGY ENCRYPTION
    export SEPARATE_HOME SWAP_SIZE SWAP_SIZE BTRFS_FREQUENCY BTRFS_KEEP_COUNT BTRFS_ASSISTANT
    export TIME_SYNC MIRROR_COUNTRY KERNEL MULTILIB ADDITIONAL_PACKAGES
    export BOOTLOADER OS_PROBER GRUB_THEME
    
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
        arch-chroot /mnt grub-install --target=x86_64-efi --efi-directory=/efi --bootloader-id=GRUB
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
    arch-chroot /mnt bash -c "
        cd /home/$MAIN_USERNAME
        sudo -u $MAIN_USERNAME git clone https://github.com/your-repo/archinstall.git
    "
    
    echo "Git repository cloning complete"
}

# --- EFI + XBOOTLDR Partitioning Functions ---
auto_simple_partitioning_efi_xbootldr() {
    echo "=== PHASE 1: Disk Partitioning (ESP + XBOOTLDR) ==="
    echo "Starting EFI + XBOOTLDR partitioning for $INSTALL_DISK..."
    
    # Wipe disk
    wipefs -a "$INSTALL_DISK"
    
    local current_start_mib=1
    local part_num=1
    
    # Create GPT partition table
    sgdisk -Z "$INSTALL_DISK" || error_exit "Failed to create GPT label on $INSTALL_DISK."
    partprobe "$INSTALL_DISK"
    
    # EFI System Partition (ESP) - mounted to /efi
    echo "Creating EFI System Partition (${EFI_PART_SIZE_MIB}MiB)..."
    local efi_size_mb="${EFI_PART_SIZE_MIB}M"
    sgdisk -n "$part_num:0:+$efi_size_mb" -t "$part_num:$EFI_PARTITION_TYPE" "$INSTALL_DISK"
    partprobe "$INSTALL_DISK"
    local efi_part=$(get_partition_path "$INSTALL_DISK" "$part_num")
    
    echo "EFI System Partition created at: $efi_part"
    format_filesystem "$efi_part" "vfat"
    capture_device_info "efi" "$efi_part"
    mkdir -p /mnt/efi
        safe_mount "$efi_part" "/mnt/efi"
    current_start_mib=$((current_start_mib + EFI_PART_SIZE_MIB))
    part_num=$((part_num + 1))
    
    # Extended Boot Loader Partition (XBOOTLDR) - mounted to /boot
    echo "Creating Extended Boot Loader Partition (${XBOOTLDR_PART_SIZE_MIB}MiB)..."
    local xbootldr_size_mb="${XBOOTLDR_PART_SIZE_MIB}M"
    sgdisk -n "$part_num:$current_start_mib:+$xbootldr_size_mb" -t "$part_num:$XBOOTLDR_PARTITION_TYPE" "$INSTALL_DISK"
    partprobe "$INSTALL_DISK"
    local xbootldr_part=$(get_partition_path "$INSTALL_DISK" "$part_num")
    
    echo "Extended Boot Loader Partition created at: $xbootldr_part"
    format_filesystem "$xbootldr_part" "ext4"  # XBOOTLDR always ext4 for reliability
    capture_device_info "xbootldr" "$xbootldr_part"
    mkdir -p /mnt/boot
    safe_mount "$xbootldr_part" "/mnt/boot"
    current_start_mib=$((current_start_mib + XBOOTLDR_PART_SIZE_MIB))
    part_num=$((part_num + 1))
    
    # Swap partition (if requested)
    if [ "$SWAP_SIZE" != "No" ]; then
        echo "Creating swap partition..."
        local swap_size_mib
        case "$SWAP_SIZE" in
            "2GB") swap_size_mib=2048 ;;
            "4GB") swap_size_mib=4096 ;;
            "8GB") swap_size_mib=8192 ;;
            "16GB") swap_size_mib=16384 ;;
            *) swap_size_mib=$DEFAULT_SWAP_SIZE_MIB ;;
        esac
        
        local swap_size_mb="${swap_size_mib}M"
        sgdisk -n "$part_num:$current_start_mib:+$swap_size_mb" -t "$part_num:$SWAP_PARTITION_TYPE" "$INSTALL_DISK"
        partprobe "$INSTALL_DISK"
        local swap_part=$(get_partition_path "$INSTALL_DISK" "$part_num")
        
        echo "Swap partition created at: $swap_part"
        format_filesystem "$swap_part" "swap"
        capture_device_info "swap" "$swap_part"
        swapon "$swap_part"
        current_start_mib=$((current_start_mib + swap_size_mib))
        part_num=$((part_num + 1))
    fi
    
    # Root partition
    echo "Creating root partition..."
    sgdisk -n "$part_num:$current_start_mib:0" -t "$part_num:$LINUX_PARTITION_TYPE" "$INSTALL_DISK"
    partprobe "$INSTALL_DISK"
    local root_part=$(get_partition_path "$INSTALL_DISK" "$part_num")
    
    echo "Root partition created at: $root_part"
    format_filesystem "$root_part" "$ROOT_FILESYSTEM"
    capture_device_info "root" "$root_part"
    safe_mount "$root_part" "/mnt"
    
    # Separate home partition (if requested)
    if [ "$SEPARATE_HOME" = "Yes" ]; then
        echo "Creating separate home partition..."
        part_num=$((part_num + 1))
        sgdisk -n "$part_num:$current_start_mib:0" -t "$part_num:$LINUX_PARTITION_TYPE" "$INSTALL_DISK"
        partprobe "$INSTALL_DISK"
        local home_part=$(get_partition_path "$INSTALL_DISK" "$part_num")
        
        echo "Home partition created at: $home_part"
        format_filesystem "$home_part" "$HOME_FILESYSTEM"
        capture_device_info "home" "$home_part"
        mkdir -p /mnt/home
        safe_mount "$home_part" "/mnt/home"
    fi
    
    echo "ESP + XBOOTLDR partitioning complete"
    echo "ESP mounted at: /mnt/efi"
    echo "XBOOTLDR mounted at: /mnt/boot"
}

auto_btrfs_partitioning_efi_xbootldr() {
    echo "=== PHASE 1: Btrfs Partitioning (ESP + XBOOTLDR) ==="
    echo "Starting Btrfs EFI + XBOOTLDR partitioning for $INSTALL_DISK..."
    
    # Wipe disk
    wipefs -a "$INSTALL_DISK"
    
    local current_start_mib=1
    local part_num=1
    
    # Create GPT partition table
    sgdisk -Z "$INSTALL_DISK"
    partprobe "$INSTALL_DISK"
    
    # EFI System Partition (ESP) - mounted to /efi
    echo "Creating EFI System Partition (${EFI_PART_SIZE_MIB}MiB)..."
    local efi_size_mb="${EFI_PART_SIZE_MIB}M"
    sgdisk -n "$part_num:0:+$efi_size_mb" -t "$part_num:$EFI_PARTITION_TYPE" "$INSTALL_DISK"
    partprobe "$INSTALL_DISK"
    local efi_part=$(get_partition_path "$INSTALL_DISK" "$part_num")
    
    echo "EFI System Partition created at: $efi_part"
    format_filesystem "$efi_part" "vfat"
    capture_device_info "efi" "$efi_part"
    mkdir -p /mnt/efi
        safe_mount "$efi_part" "/mnt/efi"
    current_start_mib=$((current_start_mib + EFI_PART_SIZE_MIB))
    part_num=$((part_num + 1))
    
    # Extended Boot Loader Partition (XBOOTLDR) - mounted to /boot
    echo "Creating Extended Boot Loader Partition (${XBOOTLDR_PART_SIZE_MIB}MiB)..."
    local xbootldr_size_mb="${XBOOTLDR_PART_SIZE_MIB}M"
    sgdisk -n "$part_num:$current_start_mib:+$xbootldr_size_mb" -t "$part_num:$XBOOTLDR_PARTITION_TYPE" "$INSTALL_DISK"
    partprobe "$INSTALL_DISK"
    local xbootldr_part=$(get_partition_path "$INSTALL_DISK" "$part_num")
    
    echo "Extended Boot Loader Partition created at: $xbootldr_part"
    format_filesystem "$xbootldr_part" "ext4"  # XBOOTLDR always ext4 for reliability
    capture_device_info "xbootldr" "$xbootldr_part"
    mkdir -p /mnt/boot
    safe_mount "$xbootldr_part" "/mnt/boot"
    current_start_mib=$((current_start_mib + XBOOTLDR_PART_SIZE_MIB))
    part_num=$((part_num + 1))
    
    # Btrfs root partition
    echo "Creating Btrfs root partition..."
    sgdisk -n "$part_num:$current_start_mib:0" -t "$part_num:$LINUX_PARTITION_TYPE" "$INSTALL_DISK"
    partprobe "$INSTALL_DISK"
    local root_part=$(get_partition_path "$INSTALL_DISK" "$part_num")
    
    echo "Btrfs root partition created at: $root_part"
    format_filesystem "$root_part" "btrfs"
    capture_device_info "root" "$root_part"
    safe_mount "$root_part" "/mnt"
    
    # Create Btrfs subvolumes
    cd /mnt
    btrfs subvolume create @
    btrfs subvolume create @home
    btrfs subvolume create @var
    btrfs subvolume create @snapshots
    cd /
    
    # Unmount and remount with subvolumes
    umount /mnt
    safe_mount -o subvol=@ "$root_part" "/mnt"
    mkdir -p /mnt/{home,var,.snapshots}
    safe_mount -o subvol=@home "$root_part" "/mnt/home"
    safe_mount -o subvol=@var "$root_part" "/mnt/var"
    safe_mount -o subvol=@snapshots "$root_part" "/mnt/.snapshots"
    
    echo "Btrfs ESP + XBOOTLDR partitioning complete"
}

auto_lvm_partitioning_efi_xbootldr() {
    echo "=== PHASE 1: LVM Partitioning (ESP + XBOOTLDR) ==="
    echo "Starting LVM EFI + XBOOTLDR partitioning for $INSTALL_DISK..."
    
    # Wipe disk
    wipefs -a "$INSTALL_DISK"
    
    local current_start_mib=1
    local part_num=1
    
    # Create GPT partition table
    sgdisk -Z "$INSTALL_DISK"
    partprobe "$INSTALL_DISK"
    
    # EFI System Partition (ESP) - mounted to /efi
    echo "Creating EFI System Partition (${EFI_PART_SIZE_MIB}MiB)..."
    local efi_size_mb="${EFI_PART_SIZE_MIB}M"
    sgdisk -n "$part_num:0:+$efi_size_mb" -t "$part_num:$EFI_PARTITION_TYPE" "$INSTALL_DISK"
    partprobe "$INSTALL_DISK"
    local efi_part=$(get_partition_path "$INSTALL_DISK" "$part_num")
    
    echo "EFI System Partition created at: $efi_part"
    format_filesystem "$efi_part" "vfat"
    capture_device_info "efi" "$efi_part"
    mkdir -p /mnt/efi
        safe_mount "$efi_part" "/mnt/efi"
    current_start_mib=$((current_start_mib + EFI_PART_SIZE_MIB))
    part_num=$((part_num + 1))
    
    # Extended Boot Loader Partition (XBOOTLDR) - mounted to /boot
    echo "Creating Extended Boot Loader Partition (${XBOOTLDR_PART_SIZE_MIB}MiB)..."
    local xbootldr_size_mb="${XBOOTLDR_PART_SIZE_MIB}M"
    sgdisk -n "$part_num:$current_start_mib:+$xbootldr_size_mb" -t "$part_num:$XBOOTLDR_PARTITION_TYPE" "$INSTALL_DISK"
    partprobe "$INSTALL_DISK"
    local xbootldr_part=$(get_partition_path "$INSTALL_DISK" "$part_num")
    
    echo "Extended Boot Loader Partition created at: $xbootldr_part"
    format_filesystem "$xbootldr_part" "ext4"  # XBOOTLDR always ext4 for reliability
    capture_device_info "xbootldr" "$xbootldr_part"
    mkdir -p /mnt/boot
    safe_mount "$xbootldr_part" "/mnt/boot"
    current_start_mib=$((current_start_mib + XBOOTLDR_PART_SIZE_MIB))
    part_num=$((part_num + 1))
    
    # LVM partition
    echo "Creating LVM partition..."
    sgdisk -n "$part_num:$current_start_mib:0" -t "$part_num:8E00" "$INSTALL_DISK"
    partprobe "$INSTALL_DISK"
    local lvm_part=$(get_partition_path "$INSTALL_DISK" "$part_num")
    
    # Create LVM setup
    pvcreate "$lvm_part"
    vgcreate arch "$lvm_part"
    
    # Create logical volumes
    lvcreate -L 20G -n root arch
    lvcreate -L 8G -n swap arch
    lvcreate -l 100%FREE -n home arch
    
    # Format logical volumes
    format_filesystem "/dev/arch/root" "$ROOT_FILESYSTEM"
    format_filesystem "/dev/arch/swap" "swap"
    format_filesystem "/dev/arch/home" "$HOME_FILESYSTEM"
    
    # Mount logical volumes
    safe_mount "/dev/arch/root" "/mnt"
    mkdir -p /mnt/home
    safe_mount "/dev/arch/home" "/mnt/home"
    swapon "/dev/arch/swap"
    
    echo "LVM ESP + XBOOTLDR partitioning complete"
}

# --- Helper Functions ---
get_partition_path() {
    local disk="$1"
    local part_num="$2"
    
    if [[ "$disk" =~ nvme ]]; then
        echo "${disk}p${part_num}"
    else
        echo "${disk}${part_num}"
    fi
}

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

safe_mount() {
    local device="$1"
    local mountpoint="$2"
    local options="${3:-}"
    
    mkdir -p "$mountpoint"
    if [ -n "$options" ]; then
        mount -o "$options" "$device" "$mountpoint"
    else
        mount "$device" "$mountpoint"
    fi
}

# --- Base System Installation ---
install_base_system() {
    echo "Installing base system..."
    
    # Install base packages
    local packages="base base-devel linux linux-firmware"
    
    # Add kernel variant
    case "$KERNEL" in
        "linux-lts")
            packages="$packages linux-lts"
            ;;
        "linux-zen")
            packages="$packages linux-zen"
            ;;
        *)
            packages="$packages linux"
            ;;
    esac
    
    # Add essential packages
    packages="$packages grub efibootmgr networkmanager"
    
    # Add GPU drivers
    case "$GPU_DRIVERS" in
        "NVIDIA")
            packages="$packages nvidia nvidia-utils"
            ;;
        "AMD")
            packages="$packages mesa xf86-video-amdgpu"
            ;;
        "Intel")
            packages="$packages mesa xf86-video-intel"
            ;;
    esac
    
    # Install packages
    pacstrap /mnt $packages
    
    echo "Base system installed"
}

# --- System Configuration ---
configure_system() {
    echo "Configuring system..."
    
    # Generate fstab
    genfstab -U /mnt >> /mnt/etc/fstab
    
    # Set hostname
    echo "$SYSTEM_HOSTNAME" > /mnt/etc/hostname
    
    # Set timezone
    ln -sf /usr/share/zoneinfo/$TIMEZONE_REGION/$TIMEZONE /mnt/etc/localtime
    
    # Configure locale
    echo "en_US.UTF-8 UTF-8" > /mnt/etc/locale.gen
    echo "LANG=en_US.UTF-8" > /mnt/etc/locale.conf
    
    # Configure keymap
    echo "KEYMAP=$KEYMAP" > /mnt/etc/vconsole.conf
    
    # Configure hosts
    cat > /mnt/etc/hosts << EOF
127.0.0.1	localhost
::1		localhost
127.0.1.1	$SYSTEM_HOSTNAME.localdomain	$SYSTEM_HOSTNAME
EOF
    
    # Enable time sync
    if [ "$TIME_SYNC" = "Yes" ]; then
        arch-chroot /mnt systemctl enable systemd-timesyncd
    fi
    
    echo "System configured"
}

# --- Package Installation ---
install_packages() {
    echo "Installing additional packages..."
    
    # Install additional pacman packages
    if [ -n "$ADDITIONAL_PACKAGES" ]; then
        echo "Installing additional pacman packages: $ADDITIONAL_PACKAGES"
        arch-chroot /mnt pacman -S --noconfirm $ADDITIONAL_PACKAGES
    fi
    
    echo "Additional packages installed"
}

# AUR helper installation is now handled in chroot_config.sh

# Duplicate functions removed - using the enhanced versions defined earlier

# --- User Setup ---
setup_users() {
    echo "Setting up users..."
    
    # Set root password
    echo "root:$ROOT_PASSWORD" | arch-chroot /mnt chpasswd
    
    # Create user
    arch-chroot /mnt useradd -m -G wheel,audio,video,optical,storage "$MAIN_USERNAME"
    echo "$MAIN_USERNAME:$MAIN_USER_PASSWORD" | arch-chroot /mnt chpasswd
    
    # Configure sudo
    echo "%wheel ALL=(ALL) ALL" >> /mnt/etc/sudoers
    
    echo "Users configured"
}

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