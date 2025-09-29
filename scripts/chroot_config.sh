#!/bin/bash
# chroot_config.sh - Functions for post-base-install (chroot) configurations
# This script is designed to be copied into the /mnt environment and executed by arch-chroot.

# Preserve variables passed from the installer before sourcing defaults
PASSED_MAIN_USERNAME="${MAIN_USERNAME:-}"
PASSED_MAIN_USER_PASSWORD="${MAIN_USER_PASSWORD:-}"
PASSED_ROOT_PASSWORD="${ROOT_PASSWORD:-}"
PASSED_SYSTEM_HOSTNAME="${SYSTEM_HOSTNAME:-}"
PASSED_TIMEZONE="${TIMEZONE:-}"
PASSED_LOCALE="${LOCALE:-}"
PASSED_KEYMAP="${KEYMAP:-}"
PASSED_DESKTOP_ENVIRONMENT="${DESKTOP_ENVIRONMENT:-}"
PASSED_DISPLAY_MANAGER="${DISPLAY_MANAGER:-}"
PASSED_AUR_HELPER="${AUR_HELPER:-}"
PASSED_ADDITIONAL_AUR_PACKAGES="${ADDITIONAL_AUR_PACKAGES:-}"
PASSED_FLATPAK="${FLATPAK:-}"
PASSED_PLYMOUTH="${PLYMOUTH:-}"
PASSED_PLYMOUTH_THEME="${PLYMOUTH_THEME:-}"
PASSED_NUMLOCK_ON_BOOT="${NUMLOCK_ON_BOOT:-}"
PASSED_GIT_REPOSITORY="${GIT_REPOSITORY:-}"

# Strict mode for this script
set -euo pipefail

# Source utils.sh from its copied location
source ./utils.sh

# Restore passed values so config defaults do not overwrite them
[ -n "$PASSED_MAIN_USERNAME" ] && MAIN_USERNAME="$PASSED_MAIN_USERNAME"
[ -n "$PASSED_MAIN_USER_PASSWORD" ] && MAIN_USER_PASSWORD="$PASSED_MAIN_USER_PASSWORD"
[ -n "$PASSED_ROOT_PASSWORD" ] && ROOT_PASSWORD="$PASSED_ROOT_PASSWORD"
[ -n "$PASSED_SYSTEM_HOSTNAME" ] && SYSTEM_HOSTNAME="$PASSED_SYSTEM_HOSTNAME"
[ -n "$PASSED_TIMEZONE" ] && TIMEZONE="$PASSED_TIMEZONE"
[ -n "$PASSED_LOCALE" ] && LOCALE="$PASSED_LOCALE"
[ -n "$PASSED_KEYMAP" ] && KEYMAP="$PASSED_KEYMAP"
[ -n "$PASSED_DESKTOP_ENVIRONMENT" ] && DESKTOP_ENVIRONMENT="$PASSED_DESKTOP_ENVIRONMENT"
[ -n "$PASSED_DISPLAY_MANAGER" ] && DISPLAY_MANAGER="$PASSED_DISPLAY_MANAGER"
[ -n "$PASSED_AUR_HELPER" ] && AUR_HELPER="$PASSED_AUR_HELPER"
[ -n "$PASSED_ADDITIONAL_AUR_PACKAGES" ] && ADDITIONAL_AUR_PACKAGES="$PASSED_ADDITIONAL_AUR_PACKAGES"
[ -n "$PASSED_FLATPAK" ] && FLATPAK="$PASSED_FLATPAK"
[ -n "$PASSED_PLYMOUTH" ] && PLYMOUTH="$PASSED_PLYMOUTH"
[ -n "$PASSED_PLYMOUTH_THEME" ] && PLYMOUTH_THEME="$PASSED_PLYMOUTH_THEME"
[ -n "$PASSED_NUMLOCK_ON_BOOT" ] && NUMLOCK_ON_BOOT="$PASSED_NUMLOCK_ON_BOOT"
[ -n "$PASSED_GIT_REPOSITORY" ] && GIT_REPOSITORY="$PASSED_GIT_REPOSITORY"

# Note: Variables like INSTALL_DISK, ROOT_PASSWORD, etc. are now populated from the environment passed by install_arch.sh
# Associative arrays like PARTITION_UUIDs are also exported (-A).
# So, they will be directly available in this script's scope.

# Enhanced logging functions for chroot context
_log_message() {
    local level="$1"
    local message="$2"
    local exit_code="${3:-0}"
    local timestamp
    timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    local caller_info="${FUNCNAME[2]:-main}:${BASH_LINENO[1]:-0}"
    
    # Determine log level color
    local color=""
    case "$level" in
        INFO) color="\e[32m" ;;
        WARN) color="\e[33m" ;;
        ERROR) color="\e[31m" ;;
        DEBUG) color="\e[36m" ;;
        *) color="\e[0m" ;;
    esac
    
    # Format log message
    local log_entry="[$timestamp] [$level] [$caller_info] Exit Code: $exit_code - $message"
    
    # Print to terminal (with color)
    echo -e "${color}${log_entry}\e[0m"
    
    # Append to log file if LOG_FILE is set
    if [[ -n "${LOG_FILE:-}" ]]; then
        echo "$log_entry" >> "$LOG_FILE"
    fi
}

_log_info() { _log_message "INFO" "$1"; }
_log_warn() { _log_message "WARN" "$1"; }
_log_error() { _log_message "ERROR" "$1" "$?"; exit 1; }
_log_debug() { _log_message "DEBUG" "$1"; }
_log_success() { echo -e "\n\e[32;1m==================================================\e[0m\n\e[32;1m $* \e[0m\n\e[32;1m==================================================\e[0m\n"; }


# Main function for chroot configuration - this is now the entry point for this script
# Performs all post-installation configuration inside the chroot environment
# Global: All configuration variables exported from install_arch.sh
# Begin chroot configuration (no wrapper function)
    echo "=== PHASE 4: System Configuration ==="
    tui_progress_update "SystemConfiguration" "75" "Configuring system settings..."
    _log_info "Starting chroot configuration."
    
    # Verify essential variables are present (concise)
    if [ -z "$ROOT_PASSWORD" ] || [ -z "$MAIN_USERNAME" ] || [ -z "$MAIN_USER_PASSWORD" ]; then
        _log_error "Missing required credentials (MAIN_USERNAME/MAIN_USER_PASSWORD/ROOT_PASSWORD)."
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
    case "$TIME_SYNC_CHOICE" in
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
    
    # Install GRUB (simplified ArchL4TM approach - packages, config, install)
    install_grub_chroot || _log_error "GRUB installation failed."

    # Configure GRUB theme only (final GRUB config after initramfs rebuild)
    if [ "$BOOTLOADER_TYPE" == "grub" ]; then
        configure_grub_theme_chroot || _log_error "GRUB theme configuration failed."
    else
        _log_info "Skipping GRUB-specific configurations (systemd-boot selected)"
    fi

    # Configure Plymouth only if GRUB is selected (systemd-boot has limited Plymouth support)
    if [ "$WANT_PLYMOUTH" == "yes" ]; then
        if [ "$BOOTLOADER_TYPE" == "grub" ]; then
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
    if [ "$BOOTLOADER_TYPE" == "grub" ]; then
        configure_grub_cmdline_chroot || _log_error "GRUB kernel command line configuration failed."
    fi

    _log_info "Configuring Secure Boot..."
    configure_secure_boot_chroot || _log_error "Secure Boot configuration failed."


    # --- Phase 3: Desktop Environment & Drivers ---
    tui_progress_update "DesktopEnvironment" "80" "Installing desktop environment: $DESKTOP_ENVIRONMENT..."
    _log_info "Installing Desktop Environment: $DESKTOP_ENVIRONMENT..."
    case "$DESKTOP_ENVIRONMENT" in
        "gnome") install_packages_chroot "${DESKTOP_ENVIRONMENTS_GNOME_PACKAGES[@]}" || _log_error "Desktop Environment packages installation failed." ;;
        "kde") install_packages_chroot "${DESKTOP_ENVIRONMENTS_KDE_PACKAGES[@]}" || _log_error "Desktop Environment packages installation failed." ;;
        "xfce") install_packages_chroot "${DESKTOP_ENVIRONMENTS_XFCE_PACKAGES[@]}" || _log_error "Desktop Environment packages installation failed." ;;
        "hyprland") install_packages_chroot "${DESKTOP_ENVIRONMENTS_HYPRLAND_PACKAGES[@]}" || _log_error "Desktop Environment packages installation failed." ;;
        "none") _log_info "No desktop environment to install" ;;
    esac
    tui_progress_update "DesktopEnvironment" "85" "Desktop environment installation completed"

    _log_info "Installing Display Manager: $DISPLAY_MANAGER..."
    case "$DISPLAY_MANAGER" in
        "gdm") 
            install_packages_chroot "${DISPLAY_MANAGERS_GDM_PACKAGES[@]}" || _log_error "Display Manager packages installation failed."
            enable_systemd_service_chroot "$DISPLAY_MANAGER" || _log_error "Failed to enable Display Manager service."
            ;;
        "sddm") 
            install_packages_chroot "${DISPLAY_MANAGERS_SDDM_PACKAGES[@]}" || _log_error "Display Manager packages installation failed."
            enable_systemd_service_chroot "$DISPLAY_MANAGER" || _log_error "Failed to enable Display Manager service."
            ;;
        "lightdm") 
            install_packages_chroot "${DISPLAY_MANAGERS_LIGHTDM_PACKAGES[@]}" || _log_error "Display Manager packages installation failed."
            enable_systemd_service_chroot "$DISPLAY_MANAGER" || _log_error "Failed to enable Display Manager service."
            ;;
        "none") _log_info "No display manager to install" ;;
    esac
    
    _log_info "Installing GPU Drivers..."
    install_gpu_drivers_chroot || _log_error "GPU driver installation failed."

    _log_info "Installing CPU Microcode..."
    install_cpu_microcode || _log_error "CPU microcode installation failed."

    # --- Phase 4: Optional Software & User Customization ---
    # Multilib repository is now handled in configure_pacman_chroot()

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
    case "$TIME_SYNC_CHOICE" in
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
    enable_systemd_service_chroot "fstrim.timer" || _log_error "Failed to enable SSD trim timer."

    # --- Phase 6: Btrfs Snapshot Configuration ---
    _log_info "Configuring Btrfs snapshots..."
    configure_btrfs_snapshots_chroot || _log_error "Btrfs snapshots configuration failed."

    # --- Phase 7: Desktop Environment Configuration ---
    _log_info "Configuring desktop environment and display manager..."
    configure_desktop_environment_chroot || _log_error "Desktop environment configuration failed."

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
# End chroot configuration
