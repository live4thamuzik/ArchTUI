#!/bin/bash
# chroot_config_new.sh - Complete chroot configuration for Arch Linux installer
# This script configures the newly installed Arch Linux system inside chroot

set -euo pipefail

# Source logging functions
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$SCRIPT_DIR/utils.sh"

# Logging functions
_log_message() {
    local level="$1"
    local message="$2"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo "[$timestamp] [$level] $message"
}

_log_info() { _log_message "INFO" "$1"; }
_log_warn() { _log_message "WARN" "$1"; }
_log_error() { _log_message "ERROR" "$1" "$?"; exit 1; }
_log_debug() { _log_message "DEBUG" "$1"; }
_log_success() { echo -e "\n\e[32;1m==================================================\e[0m\n\e[32;1m $* \e[0m\n\e[32;1m==================================================\e[0m\n"; }

# =============================================================================
# MISSING FUNCTION IMPLEMENTATIONS
# =============================================================================

# Configure Pacman (mirrors, multilib, etc.)
configure_pacman_chroot() {
    _log_info "Configuring Pacman..."
    
    # Configure mirrors if mirror country was selected
    if [[ -n "${MIRROR_COUNTRY:-}" && "$MIRROR_COUNTRY" != "None" ]]; then
        _log_info "Configuring mirrors for country: ${MIRROR_COUNTRY}"
        # This would typically run reflector or configure mirrorlist
        # For now, we'll use the default mirrorlist
    fi
    
    # Enable multilib repository if requested
    if [[ "${MULTILIB:-}" == "Yes" ]]; then
        _log_info "Enabling multilib repository..."
        sed -i '/^#\[multilib\]/,/^#Include/s/^#//' /etc/pacman.conf
        pacman -Sy
    fi
    
    # Update package database
    pacman -Sy
    _log_success "Pacman configuration completed"
}

# Install CPU microcode
install_microcode_chroot() {
    _log_info "Installing CPU microcode..."
    
    # Detect CPU and install appropriate microcode
    if lscpu | grep -q "Vendor ID.*GenuineIntel"; then
        _log_info "Intel CPU detected, installing intel-ucode..."
        pacman -S --noconfirm intel-ucode
    elif lscpu | grep -q "Vendor ID.*AuthenticAMD"; then
        _log_info "AMD CPU detected, installing amd-ucode..."
        pacman -S --noconfirm amd-ucode
    else
        _log_warn "Unknown CPU vendor, skipping microcode installation"
    fi
}

# Install essential extra packages
install_essential_extras_chroot() {
    _log_info "Installing essential extra packages..."
    
    local essential_packages=(
        "base-devel"
        "git"
        "vim"
        "neovim"
        "nano"
        "man-db"
        "man-pages"
        "texinfo"
        "sudo"
        "which"
        "file"
        "less"
        "openssh"
        "rsync"
        "wget"
        "curl"
        "unzip"
        "p7zip"
        "htop"
        "tree"
        "bash-completion"
        "usbutils"
        "pciutils"
        "lshw"
        "dmidecode"
        "efibootmgr"
        "os-prober"
    )
    
    pacman -S --noconfirm "${essential_packages[@]}"
    _log_success "Essential extra packages installed"
}

# Set default editor
configure_default_editor_chroot() {
    _log_info "Setting Neovim as default editor..."
    echo "export EDITOR=nvim" >> /etc/environment
    echo "export VISUAL=nvim" >> /etc/environment
}

# Enable systemd services
enable_systemd_service_chroot() {
    local service="$1"
    _log_info "Enabling systemd service: $service"
    systemctl enable "$service"
}

# Install time synchronization packages
install_time_sync_chroot() {
    _log_info "Installing time synchronization package: ${TIME_SYNC}"
    
    case "$TIME_SYNC" in
        "ntpd")
            pacman -S --noconfirm ntp
            ;;
        "chrony")
            pacman -S --noconfirm chrony
            ;;
        "systemd-timesyncd")
            # Already included in systemd
            ;;
        *)
            _log_warn "Unknown time sync choice: ${TIME_SYNC}"
            ;;
    esac
}

# Configure localization
configure_localization_chroot() {
    _log_info "Configuring localization..."
    
    # Set locale
    if [[ -n "${LOCALE:-}" ]]; then
        _log_info "Setting locale to: ${LOCALE}"
        echo "${LOCALE} UTF-8" >> /etc/locale.gen
        locale-gen
        echo "LANG=${LOCALE}" > /etc/locale.conf
    fi
    
    # Set timezone
    if [[ -n "${TIMEZONE:-}" ]]; then
        _log_info "Setting timezone to: ${TIMEZONE}"
        ln -sf "/usr/share/zoneinfo/${TIMEZONE}" /etc/localtime
    fi
    
    # Set keymap
    if [[ -n "${KEYMAP:-}" ]]; then
        _log_info "Setting keymap to: ${KEYMAP}"
        echo "KEYMAP=${KEYMAP}" > /etc/vconsole.conf
    fi
    
    # Set hardware clock
    hwclock --systohc
}

# Create user account
create_user() {
    local username="$1"
    _log_info "Creating user account: $username"
    
    useradd -m -G wheel,users "$username"
    return $?
}

# Set passwords
set_passwords() {
    local username="$1"
    local user_password="$2"
    local root_password="$3"
    
    _log_info "Setting passwords..."
    
    # Set root password
    if [[ -n "$root_password" ]]; then
        echo "root:$root_password" | chpasswd
    fi
    
    # Set user password
    if [[ -n "$user_password" ]]; then
        echo "$username:$user_password" | chpasswd
    fi
    
    return 0
}

# Configure hostname
configure_hostname_chroot() {
    _log_info "Configuring hostname: ${SYSTEM_HOSTNAME}"
    echo "${SYSTEM_HOSTNAME}" > /etc/hostname
    
    # Configure hosts file
    cat > /etc/hosts << EOF
127.0.0.1	localhost
::1		localhost
127.0.1.1	${SYSTEM_HOSTNAME}.localdomain	${SYSTEM_HOSTNAME}
EOF
}

# Update sudoers
update_sudoers() {
    _log_info "Configuring sudoers..."
    
    # Enable wheel group for sudo
    sed -i 's/^# %wheel ALL=(ALL:ALL) ALL/%wheel ALL=(ALL:ALL) ALL/' /etc/sudoers
    return 0
}

# Install bootloader
install_bootloader_chroot() {
    _log_info "Installing bootloader: ${BOOTLOADER:-grub}"
    
    case "${BOOTLOADER:-grub}" in
        "grub")
            if [[ "$BOOT_MODE" == "UEFI" ]]; then
                _log_info "Installing GRUB for UEFI..."
                pacman -S --noconfirm grub efibootmgr
                grub-install --target=x86_64-efi --bootloader-id=grub_uefi --recheck --efi-directory=/efi
            else
                _log_info "Installing GRUB for BIOS..."
                pacman -S --noconfirm grub
                grub-install --target=i386-pc --recheck "${INSTALL_DISK}"
            fi
            ;;
        "systemd-boot")
            if [[ "$BOOT_MODE" == "UEFI" ]]; then
                _log_info "Installing systemd-boot for UEFI..."
                bootctl install
            else
                _log_error "systemd-boot requires UEFI firmware"
            fi
            ;;
        *)
            _log_error "Unknown bootloader: ${BOOTLOADER}"
            ;;
    esac
}

# Configure GRUB theme
configure_grub_theme_chroot() {
    _log_info "Configuring GRUB theme: $GRUB_THEME"
    
    if [[ "$GRUB_THEME" != "None" && "$GRUB_THEME" != "No" ]]; then
        _log_info "GRUB theme configuration is handled by install.sh before chroot"
        # Themes are installed by install.sh via external sources
        # We just need to ensure GRUB config is updated
        _log_info "Updating GRUB configuration..."
    fi
}

# Configure Plymouth
configure_plymouth_chroot() {
    _log_info "Configuring Plymouth boot splash..."
    
    if [[ "$PLYMOUTH" == "Yes" ]]; then
        pacman -S --noconfirm plymouth
        
        # Install theme (themes should already be copied by install.sh)
        if [[ -n "${PLYMOUTH_THEME:-}" && "$PLYMOUTH_THEME" != "None" ]]; then
            case "$PLYMOUTH_THEME" in
                "arch-glow")
                    if [ -d "/usr/share/plymouth/themes/arch-glow" ]; then
                        plymouth-set-default-theme arch-glow
                    else
                        _log_warn "Plymouth theme arch-glow not found in /usr/share/plymouth/themes/"
                    fi
                    ;;
                "arch-mac-style")
                    if [ -d "/usr/share/plymouth/themes/arch-mac-style" ]; then
                        plymouth-set-default-theme arch-mac-style
                    else
                        _log_warn "Plymouth theme arch-mac-style not found in /usr/share/plymouth/themes/"
                    fi
                    ;;
            esac
        fi
    fi
}

# Configure mkinitcpio hooks
configure_mkinitcpio_hooks_chroot() {
    _log_info "Configuring mkinitcpio hooks..."
    
    # Base hooks for all configurations
    local hooks="base udev autodetect modconf kms keyboard keymap consolefont block"
    
    # Add systemd hooks for systemd-boot
    if [[ "${BOOTLOADER:-grub}" == "systemd-boot" ]]; then
        hooks="$hooks systemd"
    fi
    
    # Add encryption hooks if needed
    if [[ "${ENCRYPTION:-}" == "Yes" ]]; then
        hooks="$hooks encrypt"
    fi
    
    # Add LVM hooks if needed
    if [[ "$PARTITIONING_STRATEGY" == *"lvm"* ]]; then
        hooks="$hooks lvm2"
    fi
    
    # Add RAID hooks if needed
    if [[ "$PARTITIONING_STRATEGY" == *"raid"* ]]; then
        hooks="$hooks mdadm_udev"
    fi
    
    # Add Plymouth hooks if enabled
    if [[ "$PLYMOUTH" == "Yes" ]]; then
        hooks="$hooks plymouth"
    fi
    
    # Add resume hooks for hibernation (if swap is present)
    if [[ "${SWAP:-Yes}" == "Yes" ]]; then
        hooks="$hooks resume"
    fi
    
    # Final hooks
    if [[ "${BOOTLOADER:-grub}" == "systemd-boot" ]]; then
        hooks="$hooks filesystems"
    else
        hooks="$hooks filesystems fsck"
    fi
    
    # Update mkinitcpio.conf
    sed -i "s/^HOOKS=.*/HOOKS=($hooks)/" /etc/mkinitcpio.conf
    
    # Regenerate initramfs
    mkinitcpio -P
}

# Configure GRUB command line
configure_grub_cmdline_chroot() {
    _log_info "Configuring GRUB kernel command line..."
    
    local cmdline="quiet"
    
    # Add encryption parameters if needed
    if [[ "${ENCRYPTION:-}" == "Yes" ]]; then
        cmdline="$cmdline cryptdevice=UUID=$ROOT_UUID:luks"
    fi
    
    # Add LVM parameters if needed
    if [[ "$PARTITIONING_STRATEGY" == *"lvm"* ]]; then
        cmdline="$cmdline root=/dev/mapper/vg0-root"
    fi
    
    # Update GRUB_CMDLINE_LINUX_DEFAULT
    sed -i "s/^GRUB_CMDLINE_LINUX_DEFAULT=.*/GRUB_CMDLINE_LINUX_DEFAULT=\"$cmdline\"/" /etc/default/grub
    
    # Generate GRUB config
    grub-mkconfig -o /boot/grub/grub.cfg
}

# Configure Secure Boot
configure_secure_boot_chroot() {
    _log_info "Configuring Secure Boot..."
    
    if [[ "${SECURE_BOOT:-}" == "Yes" ]]; then
        _log_info "Secure Boot requested - user must configure keys manually"
        # This is a complex process that requires manual key management
        # For now, we'll just log that it was requested
    else
        _log_info "Secure Boot not requested"
    fi
}

# Install packages
install_packages_chroot() {
    local packages=("$@")
    _log_info "Installing packages: ${packages[*]}"
    pacman -S --noconfirm "${packages[@]}"
}

# Install display manager
install_display_manager_chroot() {
    _log_info "Installing display manager: $DISPLAY_MANAGER"
    
    case "$DISPLAY_MANAGER" in
        "gdm")
            pacman -S --noconfirm gdm
            ;;
        "sddm")
            pacman -S --noconfirm sddm
            ;;
        "lightdm")
            pacman -S --noconfirm lightdm lightdm-gtk-greeter
            ;;
        "lxdm")
            pacman -S --noconfirm lxdm
            ;;
        "none")
            _log_info "No display manager requested"
            return 0
            ;;
    esac
}

# Install GPU drivers
install_gpu_drivers_chroot() {
    _log_info "Installing GPU drivers: $GPU_DRIVERS"
    
    case "$GPU_DRIVERS" in
        "nvidia")
            pacman -S --noconfirm nvidia nvidia-utils nvidia-settings
            ;;
        "nvidia-lts")
            pacman -S --noconfirm nvidia-lts nvidia-utils nvidia-settings
            ;;
        "amd")
            pacman -S --noconfirm mesa lib32-mesa xf86-video-amdgpu
            ;;
        "intel")
            pacman -S --noconfirm mesa lib32-mesa xf86-video-intel
            ;;
        "nouveau")
            pacman -S --noconfirm mesa lib32-mesa xf86-video-nouveau
            ;;
    esac
}

# Install AUR helper
install_aur_helper_chroot() {
    _log_info "Installing AUR helper: $AUR_HELPER"
    
    if [[ "$AUR_HELPER" != "None" ]]; then
        case "$AUR_HELPER" in
            "paru")
                # Install paru
                cd /tmp
                git clone https://aur.archlinux.org/paru.git
                cd paru
                chown -R nobody:nobody .
                sudo -u nobody makepkg -si --noconfirm
                ;;
            "yay")
                # Install yay
                cd /tmp
                git clone https://aur.archlinux.org/yay.git
                cd yay
                chown -R nobody:nobody .
                sudo -u nobody makepkg -si --noconfirm
                ;;
        esac
    fi
}

# Install Flatpak
install_flatpak_chroot() {
    if [[ "$FLATPAK" == "Yes" ]]; then
        _log_info "Installing Flatpak..."
        pacman -S --noconfirm flatpak
    fi
}

# Install custom packages
install_custom_packages_chroot() {
    if [[ -n "${ADDITIONAL_PACKAGES:-}" ]]; then
        _log_info "Installing custom packages: $ADDITIONAL_PACKAGES"
        # Convert space-separated string to array for safe execution
        IFS=' ' read -ra PACKAGES <<< "$ADDITIONAL_PACKAGES"
        pacman -S --noconfirm "${PACKAGES[@]}"
    fi
}

# Install custom AUR packages
install_custom_aur_packages_chroot() {
    if [[ -n "${ADDITIONAL_AUR_PACKAGES:-}" && "$AUR_HELPER" != "None" ]]; then
        _log_info "Installing custom AUR packages: $ADDITIONAL_AUR_PACKAGES"
        # Convert space-separated string to array for safe execution
        IFS=' ' read -ra AUR_PACKAGES <<< "$ADDITIONAL_AUR_PACKAGES"
        case "$AUR_HELPER" in
            "paru")
                sudo -u "$MAIN_USERNAME" paru -S --noconfirm "${AUR_PACKAGES[@]}"
                ;;
            "yay")
                sudo -u "$MAIN_USERNAME" yay -S --noconfirm "${AUR_PACKAGES[@]}"
                ;;
        esac
    fi
}

# Configure numlock on boot
configure_numlock_chroot() {
    if [[ "$NUMLOCK_ON_BOOT" == "Yes" ]]; then
        _log_info "Configuring numlock on boot..."
        pacman -S --noconfirm numlockx
        echo "numlockx on" >> /etc/xdg/openbox/autostart
    fi
}

# Save mdadm configuration
save_mdadm_conf_chroot() {
    if [[ "$PARTITIONING_STRATEGY" == *"raid"* ]]; then
        _log_info "Saving mdadm configuration..."
        mdadm --detail --scan >> /etc/mdadm.conf
    fi
}

# Deploy dotfiles
deploy_dotfiles_chroot() {
    if [[ "$GIT_REPOSITORY" == "Yes" && -n "${GIT_REPOSITORY_URL:-}" ]]; then
        _log_info "Deploying dotfiles from: $GIT_REPOSITORY_URL"
        sudo -u "$MAIN_USERNAME" git clone "$GIT_REPOSITORY_URL" "/home/$MAIN_USERNAME/dotfiles"
        sudo -u "$MAIN_USERNAME" bash -c "cd /home/$MAIN_USERNAME/dotfiles && ./install.sh" || _log_warn "Dotfiles deployment failed"
    fi
}

# =============================================================================
# MAIN CHROOT CONFIGURATION
# =============================================================================

main() {
    _log_success "Starting chroot configuration..."
    
    # Get variables from PASSED_ prefixed exports (as set by install.sh)
    [ -n "${PASSED_MAIN_USERNAME:-}" ] && MAIN_USERNAME="$PASSED_MAIN_USERNAME"
    [ -n "${PASSED_MAIN_USER_PASSWORD:-}" ] && MAIN_USER_PASSWORD="$PASSED_MAIN_USER_PASSWORD"
    [ -n "${PASSED_ROOT_PASSWORD:-}" ] && ROOT_PASSWORD="$PASSED_ROOT_PASSWORD"
    [ -n "${PASSED_SYSTEM_HOSTNAME:-}" ] && SYSTEM_HOSTNAME="$PASSED_SYSTEM_HOSTNAME"
    [ -n "${PASSED_TIMEZONE:-}" ] && TIMEZONE="$PASSED_TIMEZONE"
    [ -n "${PASSED_LOCALE:-}" ] && LOCALE="$PASSED_LOCALE"
    [ -n "${PASSED_KEYMAP:-}" ] && KEYMAP="$PASSED_KEYMAP"
    [ -n "${PASSED_DESKTOP_ENVIRONMENT:-}" ] && DESKTOP_ENVIRONMENT="$PASSED_DESKTOP_ENVIRONMENT"
    [ -n "${PASSED_DISPLAY_MANAGER:-}" ] && DISPLAY_MANAGER="$PASSED_DISPLAY_MANAGER"
    [ -n "${PASSED_AUR_HELPER:-}" ] && AUR_HELPER="$PASSED_AUR_HELPER"
    [ -n "${PASSED_ADDITIONAL_AUR_PACKAGES:-}" ] && ADDITIONAL_AUR_PACKAGES="$PASSED_ADDITIONAL_AUR_PACKAGES"
    [ -n "${PASSED_FLATPAK:-}" ] && FLATPAK="$PASSED_FLATPAK"
    [ -n "${PASSED_PLYMOUTH:-}" ] && PLYMOUTH="$PASSED_PLYMOUTH"
    [ -n "${PASSED_PLYMOUTH_THEME:-}" ] && PLYMOUTH_THEME="$PASSED_PLYMOUTH_THEME"
    [ -n "${PASSED_NUMLOCK_ON_BOOT:-}" ] && NUMLOCK_ON_BOOT="$PASSED_NUMLOCK_ON_BOOT"
    [ -n "${PASSED_GIT_REPOSITORY:-}" ] && GIT_REPOSITORY="$PASSED_GIT_REPOSITORY"
    
    # Validate required variables
    if [[ -z "${MAIN_USERNAME:-}" ]]; then
        _log_error "MAIN_USERNAME is not set"
    fi
    
    if [[ -z "${ROOT_PASSWORD:-}" ]]; then
        _log_error "ROOT_PASSWORD is not set"
    fi
    
    # --- Phase 1: Basic System Configuration ---
    configure_pacman_chroot || _log_error "Pacman configuration failed."
    
    # Microcode before other additions
    install_microcode_chroot || _log_error "CPU Microcode installation failed."
    
    # Essential extras beyond pacstrap base: editors, docs, fs utils, storage stacks
    install_essential_extras_chroot || _log_error "Essential extra packages installation failed."
    
    # Set Neovim as default editor
    _log_info "Setting Neovim as default editor..."
    configure_default_editor_chroot || _log_error "Failed to set Neovim as default editor."
    
    # Enable core services early
    enable_systemd_service_chroot "NetworkManager" || _log_error "Failed to enable NetworkManager service."
    
    # Ensure the chosen time sync package is present before enabling its service
    install_time_sync_chroot || _log_warn "Time sync package installation step returned non-zero; continuing."
    case "${TIME_SYNC:-systemd-timesyncd}" in
        "ntpd") enable_systemd_service_chroot "ntpd" || _log_error "Failed to enable ntpd service." ;;
        "chrony") enable_systemd_service_chroot "chronyd" || _log_error "Failed to enable chronyd service." ;;
        "systemd-timesyncd") enable_systemd_service_chroot "systemd-timesyncd" || _log_error "Failed to enable systemd-timesyncd service." ;;
    esac
    
    # Locale > Timezone as requested (initramfs moved later to avoid multiple rebuilds)
    configure_localization_chroot || _log_error "Localization configuration failed."
    
    # User and password, then hostname, then sudoers
    USERNAME="$MAIN_USERNAME"
    USER_PASSWORD="$MAIN_USER_PASSWORD"
    
    if create_user "$USERNAME"; then
        _log_success "User account creation completed successfully"
    else
        _log_error "User account creation failed"
        exit 1
    fi
    
    if ! set_passwords "$USERNAME" "$USER_PASSWORD" "$ROOT_PASSWORD"; then
        _log_error "Setting passwords failed"
        exit 1
    fi
    
    configure_hostname_chroot || _log_error "Hostname configuration failed."
    
    if update_sudoers; then
        _log_success "sudoers configuration completed successfully"
    else
        _log_error "sudoers configuration failed"
        exit 1
    fi
    
    # --- Phase 2: Bootloader & Initramfs ---
    _log_info "Configuring bootloader, GRUB defaults, theme, and mkinitpio hooks."
    
    # Install bootloader (GRUB or systemd-boot)
    install_bootloader_chroot || _log_error "Bootloader installation failed."
    
    # Configure GRUB theme only (final GRUB config after initramfs rebuild)
    if [[ "${BOOTLOADER:-grub}" == "grub" ]]; then
        configure_grub_theme_chroot || _log_error "GRUB theme configuration failed."
    else
        _log_info "Skipping GRUB-specific configurations (systemd-boot selected)"
    fi
    
    # Configure Plymouth only if GRUB is selected (systemd-boot has limited Plymouth support)
    if [[ "${PLYMOUTH:-No}" == "Yes" ]]; then
        if [[ "${BOOTLOADER:-grub}" == "grub" ]]; then
            _log_info "Configuring Plymouth boot splash..."
            configure_plymouth_chroot || _log_error "Plymouth configuration failed."
        else
            _log_warn "Plymouth requested but systemd-boot selected - limited Plymouth support"
            _log_info "Configuring Plymouth boot splash..."
            configure_plymouth_chroot || _log_error "Plymouth configuration failed."
        fi
    else
        _log_info "Skipping Plymouth configuration (not requested)"
    fi
    
    # With Plymouth/GPU in place, configure mkinitcpio hooks and rebuild initramfs once
    configure_mkinitcpio_hooks_chroot || _log_error "Mkinitpio hooks configuration or initramfs rebuild failed."
    
    # Finalize GRUB cmdline and regenerate GRUB config once
    if [[ "${BOOTLOADER:-grub}" == "grub" ]]; then
        configure_grub_cmdline_chroot || _log_error "GRUB kernel command line configuration failed."
    fi
    
    _log_info "Configuring Secure Boot..."
    configure_secure_boot_chroot || _log_error "Secure Boot configuration failed."
    
    # --- Phase 3: Desktop Environment & Drivers ---
    _log_info "Installing Desktop Environment: ${DESKTOP_ENVIRONMENT:-none}..."
    # Install desktop environment using modular system
    local de_name="${DESKTOP_ENVIRONMENT:-none}"
    local de_script_path="./desktops/${de_name,,}.sh"  # ,, makes it lowercase
    
    if [ -f "$de_script_path" ]; then
        _log_info "Running modular configuration for $de_name desktop environment..."
        source "$de_script_path" || {
            _log_error "Failed to install $de_name desktop environment"
            return 1
        }
    else
        _log_error "Desktop environment script not found: $de_script_path"
        _log_info "Available desktop environments: gnome, kde, xfce, i3, hyprland, none"
        return 1
    fi
    
    # Install display manager
    install_display_manager_chroot || _log_error "Display manager installation failed."
    
    # Install GPU drivers
    install_gpu_drivers_chroot || _log_error "GPU drivers installation failed."
    
    # --- Phase 4: Optional Software & User Customization ---
    _log_info "Installing AUR Helper..."
    install_aur_helper_chroot || _log_error "AUR Helper installation failed."
    
    _log_info "Installing Flatpak..."
    install_flatpak_chroot || _log_error "Flatpak installation failed."
    
    _log_info "Installing Custom Packages..."
    install_custom_packages_chroot || _log_error "Custom packages installation failed."
    
    _log_info "Installing Custom AUR Packages..."
    install_custom_aur_packages_chroot || _log_error "Custom AUR packages installation failed."
    
    _log_info "Installing AUR Numlock on Boot..."
    configure_numlock_chroot || _log_error "Numlock on boot configuration failed."
    
    # --- Phase 5: Final System Services ---
    _log_info "Enabling essential system services..."
    enable_systemd_service_chroot "NetworkManager" || _log_error "Failed to enable NetworkManager service."
    # Enable time synchronization service based on user choice
    case "${TIME_SYNC:-systemd-timesyncd}" in
        "ntpd")
            enable_systemd_service_chroot "ntpd" || _log_error "Failed to enable ntpd service."
            ;;
        "chrony")
            enable_systemd_service_chroot "chronyd" || _log_error "Failed to enable chronyd service."
            ;;
        "systemd-timesyncd")
            enable_systemd_service_chroot "systemd-timesyncd" || _log_error "Failed to enable systemd-timesyncd service."
            ;;
    esac
    
    # Enable SSD trim timer
    enable_systemd_service_chroot "fstrim.timer" || _log_error "Failed to enable SSD trim timer."
    
    # --- Phase 6: Final System Services ---
    _log_info "Enabling display manager service..."
    if [[ "${DISPLAY_MANAGER:-}" != "none" && -n "${DISPLAY_MANAGER:-}" ]]; then
        enable_systemd_service_chroot "$DISPLAY_MANAGER" || _log_error "Failed to enable Display Manager service."
    fi
    
    # --- Phase 7: Final System Services ---
    _log_info "Installing CPU Microcode..."
    install_microcode_chroot || _log_error "CPU microcode installation failed."
    
    # --- Phase 8: Finalizations ---
    _log_info "Saving mdadm.conf for RAID arrays..."
    save_mdadm_conf_chroot || _log_error "Mdadm.conf saving failed."
    
    _log_info "Deploying Dotfiles..."
    deploy_dotfiles_chroot || _log_error "Dotfile deployment failed."
    
    _log_success "Chroot configuration complete."
    
    # Preserve logs in the chroot environment
    if [[ -n "${LOG_FILE:-}" ]]; then
        _log_info "Preserving chroot logs..."
        mkdir -p "/var/log"
        if [[ -f "$LOG_FILE" ]]; then
            cp "$LOG_FILE" "/var/log/archinstall-chroot.log"
            _log_info "Chroot logs preserved at: /var/log/archinstall-chroot.log"
        fi
    fi
}

# Execute main function only if script is run directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
