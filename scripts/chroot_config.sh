#!/bin/bash
# chroot_config.sh - Complete chroot configuration for Arch Linux installer
# This script configures the newly installed Arch Linux system inside chroot

set -euo pipefail

# Get script directory (we're running from /root inside chroot)
SCRIPT_DIR="/root"

# Source utility functions if available (using source_or_die pattern)
if [[ -f "$SCRIPT_DIR/utils.sh" ]]; then
    # shellcheck source=/dev/null
    if ! source "$SCRIPT_DIR/utils.sh"; then
        echo "FATAL: Failed to source: $SCRIPT_DIR/utils.sh" >&2
        exit 1
    fi
fi

# =============================================================================
# COLOR DEFINITIONS (fallback if utils.sh not sourced)
# =============================================================================
if [[ -z "${COLORS[RESET]:-}" ]]; then
    declare -A COLORS=(
        [RESET]='\033[0m'
        [BOLD]='\033[1m'
        [DIM]='\033[2m'
        [WHITE]='\033[37m'
        [RED]='\033[31m'
        [GREEN]='\033[32m'
        [YELLOW]='\033[33m'
        [CYAN]='\033[36m'
        [BRIGHT_RED]='\033[91m'
        [BRIGHT_GREEN]='\033[92m'
        [BRIGHT_CYAN]='\033[96m'
    )

    declare -A LOG_COLORS=(
        [INFO]="${COLORS[WHITE]}"
        [WARN]="${COLORS[YELLOW]}"
        [ERROR]="${COLORS[BRIGHT_RED]}"
        [SUCCESS]="${COLORS[BRIGHT_GREEN]}"
        [PHASE]="${COLORS[BRIGHT_CYAN]}"
        [COMMAND]="${COLORS[DIM]}${COLORS[CYAN]}"
    )
fi

# Logging functions (fallback if utils.sh wasn't sourced)
if ! declare -f log_info > /dev/null 2>&1; then
    _log() {
        local level="$1"
        local message="$2"
        local timestamp
        local color="${LOG_COLORS[$level]:-${COLORS[WHITE]}}"
        timestamp=$(date '+%Y-%m-%d %H:%M:%S')
        echo -e "${color}[$timestamp] $level: $message${COLORS[RESET]}"
    }

    log_info() { _log "INFO" "$1"; }
    log_warn() { _log "WARN" "$1"; }
    log_error() { _log "ERROR" "$1"; }
    log_success() { _log "SUCCESS" "$1"; }
fi

# Verbose package installation wrapper
# Shows package installation progress for user visibility
install_packages() {
    local description="$1"
    shift
    local packages=("$@")

    if [[ ${#packages[@]} -eq 0 ]]; then
        log_info "No packages to install for: $description"
        return 0
    fi

    log_info "Installing $description (${#packages[@]} packages)..."
    log_info "Packages: ${packages[*]}"

    pacman -S --noconfirm --needed "${packages[@]}" 2>&1 | while IFS= read -r line; do
        case "$line" in
            *"error"*|*"Error"*|*"ERROR"*)
                echo -e "${LOG_COLORS[ERROR]}  [pacman] $line${COLORS[RESET]}"
                ;;
            *"warning"*|*"Warning"*|*"WARNING"*)
                echo -e "${LOG_COLORS[WARN]}  [pacman] $line${COLORS[RESET]}"
                ;;
            *"downloading"*|*"installing"*|*"::"*|*"Packages"*|*"Total"*)
                echo -e "${LOG_COLORS[COMMAND]}  [pacman] $line${COLORS[RESET]}"
                ;;
        esac
    done

    if [[ ${PIPESTATUS[0]} -ne 0 ]]; then
        log_warn "$description installation had issues"
        return 1
    fi

    log_success "$description installed"
    return 0
}

# =============================================================================
# MAIN CONFIGURATION FUNCTION
# =============================================================================

main() {
    log_success "Starting chroot configuration..."

    # Validate required variables
    if [[ -z "${MAIN_USERNAME:-}" ]]; then
        log_error "MAIN_USERNAME is not set"
        exit 1
    fi

    if [[ -z "${ROOT_PASSWORD:-}" ]]; then
        log_error "ROOT_PASSWORD is not set"
        exit 1
    fi

    # --- Phase 1: Basic System Configuration ---
    log_info "=== Phase 1: Basic System Configuration ==="

    configure_localization
    configure_hostname
    create_user_account
    configure_sudoers
    enable_base_services

    # --- Phase 2: Bootloader & Initramfs ---
    log_info "=== Phase 2: Bootloader & Initramfs ==="

    configure_mkinitcpio
    install_bootloader
    configure_grub_settings
    configure_secure_boot

    # --- Phase 3: Desktop Environment ---
    log_info "=== Phase 3: Desktop Environment ==="

    install_desktop_environment
    install_display_manager
    install_gpu_drivers

    # --- Phase 4: Additional Software ---
    log_info "=== Phase 4: Additional Software ==="

    install_aur_helper
    install_flatpak
    install_additional_packages
    configure_plymouth
    configure_snapper

    # --- Phase 5: Final Configuration ---
    log_info "=== Phase 5: Final Configuration ==="

    configure_numlock
    deploy_dotfiles
    final_cleanup

    log_success "Chroot configuration complete!"
}

# =============================================================================
# PHASE 1: BASIC SYSTEM CONFIGURATION
# =============================================================================

configure_localization() {
    log_info "Configuring localization..."

    # Set locale
    if [[ -n "${LOCALE:-}" ]]; then
        log_info "Setting locale to: ${LOCALE}"
        echo "${LOCALE} UTF-8" >> /etc/locale.gen
        locale-gen
        echo "LANG=${LOCALE}" > /etc/locale.conf
    fi

    # Set timezone
    if [[ -n "${TIMEZONE_REGION:-}" && -n "${TIMEZONE:-}" ]]; then
        local tz_path="/usr/share/zoneinfo/${TIMEZONE_REGION}/${TIMEZONE}"
        if [[ -f "$tz_path" ]]; then
            log_info "Setting timezone to: ${TIMEZONE_REGION}/${TIMEZONE}"
            ln -sf "$tz_path" /etc/localtime
        else
            # Try without region
            tz_path="/usr/share/zoneinfo/${TIMEZONE}"
            if [[ -f "$tz_path" ]]; then
                ln -sf "$tz_path" /etc/localtime
            else
                log_warn "Timezone not found: ${TIMEZONE_REGION}/${TIMEZONE}"
            fi
        fi
    fi

    # Set hardware clock
    hwclock --systohc

    # Set keymap
    if [[ -n "${KEYMAP:-}" ]]; then
        log_info "Setting keymap to: ${KEYMAP}"
        echo "KEYMAP=${KEYMAP}" > /etc/vconsole.conf
    fi

    log_success "Localization configured"
}

configure_hostname() {
    log_info "Configuring hostname: ${SYSTEM_HOSTNAME:-archlinux}"

    local hostname="${SYSTEM_HOSTNAME:-archlinux}"
    echo "$hostname" > /etc/hostname

    # Configure hosts file
    cat > /etc/hosts << EOF
127.0.0.1   localhost
::1         localhost
127.0.1.1   ${hostname}.localdomain ${hostname}
EOF

    log_success "Hostname configured"
}

create_user_account() {
    log_info "Creating user account: $MAIN_USERNAME"

    # Create user with home directory and add to wheel group
    if ! id "$MAIN_USERNAME" &>/dev/null; then
        useradd -m -G wheel,users,audio,video,storage,optical -s /bin/bash "$MAIN_USERNAME"
        log_info "User $MAIN_USERNAME created"
    else
        log_info "User $MAIN_USERNAME already exists"
    fi

    # Set user password
    if [[ -n "${MAIN_USER_PASSWORD:-}" ]]; then
        echo "$MAIN_USERNAME:$MAIN_USER_PASSWORD" | chpasswd
        log_info "User password set"
    fi

    # Set root password
    if [[ -n "${ROOT_PASSWORD:-}" ]]; then
        echo "root:$ROOT_PASSWORD" | chpasswd
        log_info "Root password set"
    fi

    log_success "User account configured"
}

configure_sudoers() {
    log_info "Configuring sudoers..."

    # Enable wheel group for sudo (without password for installation, can be changed later)
    if [[ -f /etc/sudoers ]]; then
        # Use sed to uncomment the wheel line
        sed -i 's/^# %wheel ALL=(ALL:ALL) ALL/%wheel ALL=(ALL:ALL) ALL/' /etc/sudoers

        # Verify the change was made
        if grep -q "^%wheel ALL=(ALL:ALL) ALL" /etc/sudoers; then
            log_success "Sudoers configured - wheel group enabled"
        else
            # Fallback: add the line directly
            echo "%wheel ALL=(ALL:ALL) ALL" >> /etc/sudoers
            log_info "Sudoers configured via append"
        fi
    fi
}

enable_base_services() {
    log_info "Enabling base services..."

    # NetworkManager
    systemctl enable NetworkManager.service 2>/dev/null || log_warn "NetworkManager service not found"

    # SSH (optional, but useful)
    systemctl enable sshd.service 2>/dev/null || log_warn "sshd service not found"

    # Time synchronization
    case "${TIME_SYNC:-systemd-timesyncd}" in
        "systemd-timesyncd"|"Yes")
            systemctl enable systemd-timesyncd.service 2>/dev/null || true
            ;;
        "ntpd")
            systemctl enable ntpd.service 2>/dev/null || true
            ;;
        "chrony")
            systemctl enable chronyd.service 2>/dev/null || true
            ;;
    esac

    # SSD trim timer (good for SSDs)
    systemctl enable fstrim.timer 2>/dev/null || true

    # Bluetooth support
    systemctl enable bluetooth.service 2>/dev/null || log_warn "bluetooth service not found"

    # Avahi mDNS/DNS-SD (network discovery)
    systemctl enable avahi-daemon.service 2>/dev/null || log_warn "avahi-daemon service not found"

    log_success "Base services enabled"
}

# =============================================================================
# PHASE 2: BOOTLOADER & INITRAMFS
# =============================================================================

configure_mkinitcpio() {
    log_info "Configuring mkinitcpio..."

    # Build hooks list based on configuration
    # Correct hook order per Arch Wiki (2024+):
    # https://wiki.archlinux.org/title/Mkinitcpio
    # https://wiki.archlinux.org/title/Dm-crypt/System_configuration
    #
    # Standard: base udev autodetect microcode modconf kms keyboard keymap consolefont block filesystems fsck
    # Encrypted: base udev autodetect microcode modconf kms keyboard keymap consolefont block [plymouth] encrypt [lvm2] [resume] filesystems [fsck]
    #
    # Key requirements per wiki:
    # - microcode: AFTER autodetect (so autodetect can filter to current CPU only)
    # - keyboard/keymap/consolefont: BEFORE encrypt (so keyboard works for password entry)
    # - plymouth: BEFORE encrypt (for graphical password prompt)
    # - For hardware compatibility (varying keyboards): keyboard BEFORE autodetect
    #
    # We use keyboard before autodetect for maximum hardware compatibility
    local hooks=""

    if [[ "${ENCRYPTION:-no}" == "yes" ]] || [[ "${PARTITIONING_STRATEGY:-}" == *"luks"* ]]; then
        # Encrypted: keyboard before autodetect for hardware compatibility
        # This ensures keyboard works even if booting on different hardware than image was built on
        hooks="base udev keyboard keymap consolefont autodetect microcode modconf kms block"
        log_info "Using encrypted system hook order (keyboard before autodetect for compatibility)"
    else
        # Non-encrypted: standard wiki order
        hooks="base udev autodetect microcode modconf kms keyboard keymap consolefont block"
    fi

    # Add RAID hook if using RAID (must come before encrypt/lvm2)
    if [[ "${PARTITIONING_STRATEGY:-}" == *"raid"* ]]; then
        hooks="$hooks mdadm_udev"
        log_info "Added mdadm_udev hook for RAID"

        # Save mdadm configuration
        if command -v mdadm &>/dev/null; then
            mdadm --detail --scan >> /etc/mdadm.conf 2>/dev/null || true
        fi
    fi

    # Add Plymouth hook BEFORE encrypt (per Arch Wiki: "place plymouth before the encrypt hook")
    # Do NOT use deprecated plymouth-encrypt - use separate hooks
    if [[ "${PLYMOUTH:-No}" == "Yes" ]]; then
        hooks="$hooks plymouth"
        log_info "Added plymouth hook (before encrypt per Arch Wiki)"
    fi

    # Add encryption hook if using LUKS (must come before lvm2 for LUKS-on-LVM)
    if [[ "${ENCRYPTION:-no}" == "yes" ]] || [[ "${PARTITIONING_STRATEGY:-}" == *"luks"* ]]; then
        hooks="$hooks encrypt"
        log_info "Added encrypt hook for LUKS"
    fi

    # Add LVM hook if using LVM (must come after encrypt for LUKS-on-LVM)
    if [[ "${PARTITIONING_STRATEGY:-}" == *"lvm"* ]]; then
        hooks="$hooks lvm2"
        log_info "Added lvm2 hook"
    fi

    # Add resume hook for hibernation (if swap exists)
    if [[ "${WANT_SWAP:-no}" == "yes" ]]; then
        hooks="$hooks resume"
        log_info "Added resume hook for hibernation support"
    fi

    # Final hooks - filesystems is always needed
    hooks="$hooks filesystems"

    # Add fsck hook only for non-Btrfs filesystems (Btrfs uses its own tools)
    if [[ "${ROOT_FILESYSTEM_TYPE:-ext4}" != "btrfs" ]]; then
        hooks="$hooks fsck"
        log_info "Added fsck hook"
    else
        log_info "Skipping fsck hook (Btrfs uses its own check tools)"
    fi

    # Update mkinitcpio.conf
    if [[ -f /etc/mkinitcpio.conf ]]; then
        sed -i "s/^HOOKS=.*/HOOKS=($hooks)/" /etc/mkinitcpio.conf
        log_info "Updated HOOKS in mkinitcpio.conf: $hooks"

        # Add btrfs module if using Btrfs
        if [[ "${ROOT_FILESYSTEM_TYPE:-ext4}" == "btrfs" ]]; then
            if ! grep -q "btrfs" /etc/mkinitcpio.conf; then
                sed -i 's/^MODULES=(\(.*\))/MODULES=(\1 btrfs)/' /etc/mkinitcpio.conf
                # Clean up double spaces
                sed -i 's/MODULES=( /MODULES=(/' /etc/mkinitcpio.conf
                log_info "Added btrfs module to mkinitcpio.conf"
            fi
        fi

        # Add amdgpu module for early KMS if AMD GPU detected
        if lspci 2>/dev/null | grep -qi "amd.*radeon\|radeon.*amd\|amd.*graphics"; then
            if ! grep -q "amdgpu" /etc/mkinitcpio.conf; then
                sed -i 's/^MODULES=(\(.*\))/MODULES=(\1 amdgpu)/' /etc/mkinitcpio.conf
                sed -i 's/MODULES=( /MODULES=(/' /etc/mkinitcpio.conf
                log_info "Added amdgpu module for early KMS"
            fi
        fi

        # Regenerate initramfs
        mkinitcpio -P
        log_success "Initramfs regenerated"
    else
        log_error "mkinitcpio.conf not found"
    fi
}

install_bootloader() {
    log_info "Installing bootloader: ${BOOTLOADER:-grub}"

    case "${BOOTLOADER:-grub}" in
        "grub")
            install_grub
            ;;
        "systemd-boot")
            install_systemd_boot
            ;;
        *)
            log_warn "Unknown bootloader: ${BOOTLOADER}, defaulting to GRUB"
            install_grub
            ;;
    esac
}

install_grub() {
    log_info "Installing GRUB..."

    if [[ "${BOOT_MODE:-UEFI}" == "UEFI" ]]; then
        # UEFI installation
        # Determine EFI directory (check both /efi and /boot/efi)
        local efi_dir="/efi"
        if [[ ! -d "$efi_dir" ]]; then
            efi_dir="/boot/efi"
        fi
        if [[ ! -d "$efi_dir" ]]; then
            # Create it if it doesn't exist but ESP is mounted
            mkdir -p /efi
            efi_dir="/efi"
        fi

        log_info "Installing GRUB for UEFI to $efi_dir"
        grub-install --target=x86_64-efi --efi-directory="$efi_dir" --bootloader-id=GRUB --recheck || {
            log_error "GRUB installation failed"
            return 1
        }
    else
        # BIOS installation
        log_info "Installing GRUB for BIOS to ${INSTALL_DISK:-/dev/sda}"
        grub-install --target=i386-pc "${INSTALL_DISK:-/dev/sda}" --recheck || {
            log_error "GRUB installation failed"
            return 1
        }
    fi

    log_success "GRUB installed"
}

install_systemd_boot() {
    log_info "Installing systemd-boot..."

    if [[ "${BOOT_MODE:-UEFI}" != "UEFI" ]]; then
        log_error "systemd-boot requires UEFI firmware"
        return 1
    fi

    # systemd-boot can only read FAT32 partitions
    # Two valid configurations:
    # 1. ESP mounted at /boot (kernels on ESP) - simpler
    # 2. ESP at /efi + /boot on ESP too (bind mount or same partition)
    #
    # NOT VALID: ESP at /efi + ext4 /boot (systemd-boot can't read ext4)

    local esp_path=""
    local boot_on_esp="no"

    # Check if /boot is on ESP (FAT32)
    local boot_fstype
    boot_fstype=$(findmnt -n -o FSTYPE /boot 2>/dev/null || echo "")

    if [[ "$boot_fstype" == "vfat" ]]; then
        # /boot is FAT32 - ESP is mounted at /boot
        esp_path="/boot"
        boot_on_esp="yes"
        log_info "ESP mounted at /boot - compatible with systemd-boot"
    elif [[ -d "/efi" ]]; then
        # Check if /efi exists and /boot is ext4
        local efi_fstype
        efi_fstype=$(findmnt -n -o FSTYPE /efi 2>/dev/null || echo "")
        if [[ "$efi_fstype" == "vfat" && "$boot_fstype" == "ext4" ]]; then
            log_error "systemd-boot incompatible with current layout!"
            log_error "ESP is at /efi (FAT32) but /boot is ext4"
            log_error "systemd-boot cannot read ext4 - kernels must be on FAT32"
            log_error "Options:"
            log_error "  1. Use GRUB instead (works with separate /boot)"
            log_error "  2. Mount ESP at /boot (put kernels on ESP)"
            log_warn "Falling back to GRUB for this installation"
            export BOOTLOADER="grub"
            install_grub
            return $?
        fi
        esp_path="/efi"
    else
        esp_path="/boot"
    fi

    bootctl install --esp-path="$esp_path" || {
        log_error "systemd-boot installation failed"
        return 1
    }

    # Create boot entry
    mkdir -p "${esp_path}/loader/entries"

    # Get root partition UUID
    local root_uuid="${ROOT_UUID:-}"
    if [[ -z "$root_uuid" ]]; then
        root_uuid=$(findmnt -n -o UUID /)
    fi

    # Build options line
    local options=""

    # Handle encryption
    if [[ "${ENCRYPTION:-no}" == "yes" ]] || [[ "${PARTITIONING_STRATEGY:-}" == *"luks"* ]]; then
        if [[ -n "${LUKS_UUID:-}" ]]; then
            local mapper_name="cryptroot"
            [[ "${PARTITIONING_STRATEGY:-}" == *"lvm"* ]] && mapper_name="cryptlvm"
            options="cryptdevice=UUID=${LUKS_UUID}:${mapper_name} root=/dev/mapper/${mapper_name}"
        else
            options="root=UUID=${root_uuid}"
            log_warn "LUKS_UUID not set for encrypted system"
        fi
    else
        options="root=UUID=${root_uuid}"
    fi

    # Add Btrfs rootflags
    if [[ "${ROOT_FILESYSTEM_TYPE:-ext4}" == "btrfs" ]]; then
        options="$options rootflags=subvol=@"
    fi

    options="$options rw"

    # Resume for hibernation
    if [[ "${WANT_SWAP:-no}" == "yes" && -n "${SWAP_UUID:-}" ]]; then
        options="$options resume=UUID=${SWAP_UUID}"
    fi

    options="$options quiet"

    # Plymouth splash
    if [[ "${PLYMOUTH:-No}" == "Yes" ]]; then
        options="$options splash"
    fi

    # Microcode
    local microcode_initrd=""
    if [[ -f "${esp_path}/intel-ucode.img" ]]; then
        microcode_initrd="intel-ucode.img"
    elif [[ -f "${esp_path}/amd-ucode.img" ]]; then
        microcode_initrd="amd-ucode.img"
    fi

    # Create arch.conf entry
    {
        echo "title   Arch Linux"
        echo "linux   /vmlinuz-${KERNEL:-linux}"
        [[ -n "$microcode_initrd" ]] && echo "initrd  /$microcode_initrd"
        echo "initrd  /initramfs-${KERNEL:-linux}.img"
        echo "options $options"
    } > "${esp_path}/loader/entries/arch.conf"

    # Create fallback entry
    {
        echo "title   Arch Linux (fallback)"
        echo "linux   /vmlinuz-${KERNEL:-linux}"
        [[ -n "$microcode_initrd" ]] && echo "initrd  /$microcode_initrd"
        echo "initrd  /initramfs-${KERNEL:-linux}-fallback.img"
        echo "options $options"
    } > "${esp_path}/loader/entries/arch-fallback.conf"

    # Create loader.conf
    cat > "${esp_path}/loader/loader.conf" << EOF
default arch.conf
timeout 5
console-mode max
editor no
EOF

    # Add Windows entry if detected (systemd-boot auto-detects, but explicit is safer)
    if [[ "${WINDOWS_DETECTED:-}" == "yes" ]]; then
        log_info "Adding Windows Boot Manager entry for systemd-boot"
        # systemd-boot auto-detects Windows, but we can add explicit entry
        # Windows is usually auto-detected from \EFI\Microsoft\Boot\bootmgfw.efi
    fi

    log_success "systemd-boot installed at $esp_path"
}

configure_grub_settings() {
    if [[ "${BOOTLOADER:-grub}" != "grub" ]]; then
        return 0
    fi

    log_info "Configuring GRUB settings..."

    local grub_default="/etc/default/grub"
    if [[ ! -f "$grub_default" ]]; then
        log_warn "GRUB default config not found"
        return 0
    fi

    # Build kernel command line
    local cmdline="quiet"

    # Add encryption parameters if needed
    if [[ "${ENCRYPTION:-no}" == "yes" ]] || [[ "${PARTITIONING_STRATEGY:-}" == *"luks"* ]]; then
        if [[ -n "${LUKS_UUID:-}" ]]; then
            # Determine mapper name based on strategy
            local mapper_name="cryptroot"
            if [[ "${PARTITIONING_STRATEGY:-}" == *"lvm"* ]]; then
                mapper_name="cryptlvm"
            fi
            cmdline="$cmdline cryptdevice=UUID=${LUKS_UUID}:${mapper_name}"
        fi
    fi

    # Add Btrfs subvolume rootflags if using Btrfs
    if [[ "${ROOT_FILESYSTEM_TYPE:-ext4}" == "btrfs" ]]; then
        cmdline="$cmdline rootflags=subvol=@"
        log_info "Added Btrfs subvolume rootflags"
    fi

    # Add resume parameter for hibernation if swap exists
    if [[ "${WANT_SWAP:-no}" == "yes" && -n "${SWAP_UUID:-}" ]]; then
        cmdline="$cmdline resume=UUID=${SWAP_UUID}"
    fi

    # Add Plymouth parameters if enabled
    if [[ "${PLYMOUTH:-No}" == "Yes" ]]; then
        cmdline="$cmdline splash"
    fi

    # Update GRUB_CMDLINE_LINUX_DEFAULT
    sed -i "s/^GRUB_CMDLINE_LINUX_DEFAULT=.*/GRUB_CMDLINE_LINUX_DEFAULT=\"$cmdline\"/" "$grub_default"

    # Enable os-prober if requested OR if other OS was detected during partitioning
    # This ensures dual-boot is properly configured even if user forgot to enable it
    if [[ "${OS_PROBER:-no}" == "yes" ]] || [[ "${OTHER_OS_DETECTED:-}" == "yes" ]] || [[ "${WINDOWS_DETECTED:-}" == "yes" ]]; then
        log_info "Enabling os-prober for dual-boot detection"
        if ! grep -q "^GRUB_DISABLE_OS_PROBER=false" "$grub_default"; then
            echo "GRUB_DISABLE_OS_PROBER=false" >> "$grub_default"
        fi

        # Install os-prober if not present
        if ! command -v os-prober &>/dev/null; then
            pacman -S --noconfirm --needed os-prober || log_warn "Failed to install os-prober"
        fi
    fi

    # Add explicit Windows chainload entry if Windows was detected
    # This provides a fallback if os-prober doesn't detect it
    if [[ "${WINDOWS_DETECTED:-}" == "yes" && -n "${WINDOWS_EFI_PATH:-}" ]]; then
        log_info "Adding Windows Boot Manager chainload entry"
        # The entry will be in /etc/grub.d/40_custom
        if [[ ! -f /etc/grub.d/40_custom ]] || ! grep -q "Windows Boot Manager" /etc/grub.d/40_custom; then
            cat >> /etc/grub.d/40_custom << 'WINEOF'

menuentry "Windows Boot Manager" {
    insmod part_gpt
    insmod fat
    insmod chain
    search --no-floppy --fs-uuid --set=root $hints_string $fs_uuid
    chainloader /EFI/Microsoft/Boot/bootmgfw.efi
}
WINEOF
            log_info "Added Windows chainload entry to 40_custom"
        fi
    fi

    # Enable GRUB_ENABLE_CRYPTODISK for encrypted /boot
    if [[ "${ENCRYPTION:-no}" == "yes" ]] || [[ "${PARTITIONING_STRATEGY:-}" == *"luks"* ]]; then
        if ! grep -q "^GRUB_ENABLE_CRYPTODISK=y" "$grub_default"; then
            echo "GRUB_ENABLE_CRYPTODISK=y" >> "$grub_default"
        fi
    fi

    # Configure GRUB theme if requested
    configure_grub_theme

    # Generate GRUB config
    grub-mkconfig -o /boot/grub/grub.cfg || {
        log_warn "grub-mkconfig failed, trying alternate path"
        grub-mkconfig -o /boot/grub/grub.cfg 2>/dev/null || true
    }

    log_success "GRUB configured"
}

configure_grub_theme() {
    if [[ "${GRUB_THEME:-No}" != "Yes" ]]; then
        log_info "GRUB theme not requested"
        return 0
    fi

    local theme_name="${GRUB_THEME_SELECTION:-none}"
    if [[ "$theme_name" == "none" || -z "$theme_name" ]]; then
        log_info "No GRUB theme selected"
        return 0
    fi

    log_info "Configuring GRUB theme: $theme_name"

    local theme_dir="/boot/grub/themes/${theme_name}"
    local grub_default="/etc/default/grub"

    # Check potential theme sources (in order of preference)
    local theme_sources=(
        "/usr/share/archtui/themes/${theme_name}"
        "/root/themes/${theme_name}"
        "/usr/share/grub/themes/${theme_name}"
    )

    local source_found=""
    for source_theme in "${theme_sources[@]}"; do
        if [[ -d "$source_theme" ]]; then
            source_found="$source_theme"
            break
        fi
    done

    if [[ -n "$source_found" ]]; then
        # Copy from local source
        log_info "Installing theme from: $source_found"
        mkdir -p /boot/grub/themes
        cp -r "$source_found" "$theme_dir"
    else
        # Try to install from AUR/community packages
        log_info "Theme not found locally, attempting package installation..."

        # Common GRUB theme packages
        case "${theme_name,,}" in
            "poly-dark"|"polydark")
                pacman -S --noconfirm --needed grub-theme-poly-dark 2>/dev/null || {
                    log_warn "Package grub-theme-poly-dark not available"
                }
                ;;
            "vimix")
                pacman -S --noconfirm --needed grub-theme-vimix 2>/dev/null || {
                    log_warn "Package grub-theme-vimix not available"
                }
                ;;
            "stylish")
                pacman -S --noconfirm --needed grub-theme-stylish 2>/dev/null || {
                    log_warn "Package grub-theme-stylish not available"
                }
                ;;
            *)
                log_warn "Unknown theme package for: $theme_name"
                ;;
        esac

        # Check if theme was installed by package
        local pkg_theme_dirs=(
            "/usr/share/grub/themes/${theme_name}"
            "/boot/grub/themes/${theme_name}"
        )
        for pkg_dir in "${pkg_theme_dirs[@]}"; do
            if [[ -d "$pkg_dir" ]]; then
                if [[ "$pkg_dir" != "$theme_dir" ]]; then
                    mkdir -p /boot/grub/themes
                    cp -r "$pkg_dir" "$theme_dir"
                fi
                source_found="$pkg_dir"
                break
            fi
        done
    fi

    # Verify theme was installed and has theme.txt
    if [[ -f "${theme_dir}/theme.txt" ]]; then
        # Update GRUB config to use the theme
        if grep -q "^GRUB_THEME=" "$grub_default"; then
            sed -i "s|^GRUB_THEME=.*|GRUB_THEME=\"${theme_dir}/theme.txt\"|" "$grub_default"
        else
            echo "GRUB_THEME=\"${theme_dir}/theme.txt\"" >> "$grub_default"
        fi

        # Also set gfxmode for better theme rendering
        if ! grep -q "^GRUB_GFXMODE=" "$grub_default"; then
            echo "GRUB_GFXMODE=auto" >> "$grub_default"
        fi

        log_success "GRUB theme configured: $theme_name"
    else
        log_warn "Theme installation failed or theme.txt not found for: $theme_name"
        return 1
    fi

    return 0
}

configure_secure_boot() {
    if [[ "${SECURE_BOOT:-No}" != "Yes" ]]; then
        log_info "Secure Boot not requested"
        return 0
    fi

    # Check if system is UEFI
    if [[ ! -d /sys/firmware/efi ]]; then
        log_warn "Secure Boot requires UEFI. System is booted in BIOS mode, skipping."
        return 0
    fi

    log_info "Configuring Secure Boot with sbctl..."

    # Install sbctl
    pacman -S --noconfirm --needed sbctl || {
        log_warn "Failed to install sbctl"
        return 0
    }

    # Check sbctl status
    log_info "Checking Secure Boot status..."
    sbctl status || true

    # Create Secure Boot keys if they don't exist
    if [[ ! -d /usr/share/secureboot/keys ]]; then
        log_info "Creating Secure Boot keys..."
        sbctl create-keys || {
            log_warn "Failed to create Secure Boot keys"
            return 0
        }
        log_success "Secure Boot keys created"
    else
        log_info "Secure Boot keys already exist"
    fi

    # Sign EFI binaries
    log_info "Signing EFI binaries..."

    # Sign the kernel
    local kernel="${KERNEL:-linux}"
    if [[ -f "/boot/vmlinuz-${kernel}" ]]; then
        sbctl sign -s "/boot/vmlinuz-${kernel}" 2>/dev/null || log_warn "Failed to sign vmlinuz-${kernel}"
    fi

    # Sign bootloader based on type
    if [[ "${BOOTLOADER:-grub}" == "grub" ]]; then
        # Sign GRUB EFI binary
        local grub_efi="/boot/efi/EFI/GRUB/grubx64.efi"
        if [[ -f "$grub_efi" ]]; then
            sbctl sign -s "$grub_efi" 2>/dev/null || log_warn "Failed to sign GRUB"
        fi
        # Also try alternate path
        grub_efi="/boot/EFI/GRUB/grubx64.efi"
        if [[ -f "$grub_efi" ]]; then
            sbctl sign -s "$grub_efi" 2>/dev/null || log_warn "Failed to sign GRUB"
        fi
    else
        # Sign systemd-boot
        local systemd_efi="/boot/efi/EFI/systemd/systemd-bootx64.efi"
        if [[ -f "$systemd_efi" ]]; then
            sbctl sign -s "$systemd_efi" 2>/dev/null || log_warn "Failed to sign systemd-boot"
        fi
        systemd_efi="/boot/EFI/systemd/systemd-bootx64.efi"
        if [[ -f "$systemd_efi" ]]; then
            sbctl sign -s "$systemd_efi" 2>/dev/null || log_warn "Failed to sign systemd-boot"
        fi
        # Also sign the Linux EFI stub
        local linux_efi="/boot/efi/EFI/Linux/"
        if [[ -d "$linux_efi" ]]; then
            for efi in "$linux_efi"/*.efi; do
                [[ -f "$efi" ]] && sbctl sign -s "$efi" 2>/dev/null || true
            done
        fi
    fi

    # Create post-install script for key enrollment
    cat > /root/enroll-secure-boot-keys.sh << 'SBEOF'
#!/bin/bash
# Secure Boot Key Enrollment Script
# Run this after first boot with Secure Boot disabled

echo "Secure Boot Key Enrollment"
echo "=========================="
echo ""
echo "This will enroll your custom Secure Boot keys."
echo "Make sure Secure Boot is in Setup Mode before running this."
echo ""

# Check if we can enroll keys
if sbctl status | grep -q "Setup Mode:.*Enabled"; then
    echo "Setup Mode is enabled. Enrolling keys..."
    sbctl enroll-keys --microsoft || {
        echo "Key enrollment failed!"
        echo "You may need to enter UEFI setup and enable 'Setup Mode'"
        exit 1
    }
    echo ""
    echo "Keys enrolled successfully!"
    echo "You can now reboot and enable Secure Boot in UEFI setup."
else
    echo "Setup Mode is NOT enabled."
    echo ""
    echo "To enroll keys:"
    echo "1. Reboot and enter UEFI setup (usually F2, Del, or Esc)"
    echo "2. Find Secure Boot settings"
    echo "3. Enable 'Setup Mode' or 'Clear Secure Boot Keys'"
    echo "4. Boot into Arch Linux and run this script again"
fi
SBEOF
    chmod +x /root/enroll-secure-boot-keys.sh

    # Set up pacman hook to re-sign kernel on updates
    mkdir -p /etc/pacman.d/hooks
    cat > /etc/pacman.d/hooks/95-secureboot.hook << 'HOOKEOF'
[Trigger]
Operation = Install
Operation = Upgrade
Type = Package
Target = linux
Target = linux-lts
Target = linux-zen
Target = linux-hardened
Target = systemd

[Action]
Description = Signing EFI binaries for Secure Boot...
When = PostTransaction
Exec = /usr/bin/sbctl sign-all
Depends = sbctl
HOOKEOF

    log_info "Created pacman hook for automatic kernel signing"
    log_info "Created /root/enroll-secure-boot-keys.sh for key enrollment"
    log_warn "IMPORTANT: After first boot, run /root/enroll-secure-boot-keys.sh to complete Secure Boot setup"

    log_success "Secure Boot preparation complete"
}

# =============================================================================
# PHASE 3: DESKTOP ENVIRONMENT
# =============================================================================

install_desktop_environment() {
    local de="${DESKTOP_ENVIRONMENT:-none}"
    de="${de,,}"  # Convert to lowercase

    log_info "Installing desktop environment: $de"

    case "$de" in
        "kde"|"plasma")
            install_packages "KDE Plasma" plasma kde-applications
            ;;
        "gnome")
            install_packages "GNOME" gnome gnome-extra
            ;;
        "xfce")
            install_packages "XFCE" xfce4 xfce4-goodies
            ;;
        "i3"|"i3wm")
            install_packages "i3 Window Manager" i3-wm i3status i3lock dmenu rofi alacritty
            ;;
        "hyprland")
            install_packages "Hyprland" hyprland waybar swaylock swayidle wlogout \
                rofi-wayland grim slurp kitty xdg-desktop-portal-hyprland
            ;;
        "sway")
            install_packages "Sway" sway swaylock swayidle waybar \
                rofi-wayland grim slurp foot xdg-desktop-portal-wlr
            ;;
        "cinnamon")
            install_packages "Cinnamon" cinnamon nemo-fileroller
            ;;
        "mate")
            install_packages "MATE" mate mate-extra
            ;;
        "budgie")
            install_packages "Budgie" budgie-desktop budgie-extras
            ;;
        "none"|"minimal"|"")
            log_info "No desktop environment selected - skipping"
            ;;
        *)
            log_warn "Unknown desktop environment: $de - skipping"
            ;;
    esac

    log_success "Desktop environment installation complete"
}

install_display_manager() {
    local dm="${DISPLAY_MANAGER:-none}"
    dm="${dm,,}"  # Convert to lowercase

    log_info "Installing display manager: $dm"

    case "$dm" in
        "sddm")
            install_packages "SDDM" sddm
            log_info "Enabling SDDM service..."
            systemctl enable sddm.service
            ;;
        "gdm")
            install_packages "GDM" gdm
            log_info "Enabling GDM service..."
            systemctl enable gdm.service
            ;;
        "lightdm")
            install_packages "LightDM" lightdm lightdm-gtk-greeter
            log_info "Enabling LightDM service..."
            systemctl enable lightdm.service
            ;;
        "lxdm")
            install_packages "LXDM" lxdm
            log_info "Enabling LXDM service..."
            systemctl enable lxdm.service
            ;;
        "ly")
            install_packages "Ly" ly
            log_info "Enabling Ly service..."
            systemctl enable ly.service
            ;;
        "none"|"")
            log_info "No display manager selected - skipping"
            ;;
        *)
            log_warn "Unknown display manager: $dm"
            ;;
    esac

    log_success "Display manager installation complete"
}

install_gpu_drivers() {
    local gpu="${GPU_DRIVERS:-Auto}"

    log_info "Installing GPU drivers: $gpu"

    case "$gpu" in
        "Auto"|"auto")
            # Auto-detect GPU
            if lspci | grep -qi nvidia; then
                log_info "NVIDIA GPU detected"
                pacman -S --noconfirm --needed nvidia nvidia-utils nvidia-settings || true
            fi
            if lspci | grep -qi "amd.*radeon\|radeon.*amd\|amd.*graphics"; then
                log_info "AMD GPU detected"
                pacman -S --noconfirm --needed mesa lib32-mesa xf86-video-amdgpu vulkan-radeon || true
            fi
            if lspci | grep -qi "intel.*graphics\|intel.*uhd\|intel.*iris"; then
                log_info "Intel GPU detected"
                pacman -S --noconfirm --needed mesa lib32-mesa xf86-video-intel vulkan-intel || true
            fi
            ;;
        "nvidia"|"NVIDIA")
            pacman -S --noconfirm --needed nvidia nvidia-utils nvidia-settings
            ;;
        "nvidia-open")
            pacman -S --noconfirm --needed nvidia-open nvidia-utils nvidia-settings
            ;;
        "amd"|"AMD")
            pacman -S --noconfirm --needed mesa lib32-mesa xf86-video-amdgpu vulkan-radeon
            ;;
        "intel"|"Intel")
            pacman -S --noconfirm --needed mesa lib32-mesa xf86-video-intel vulkan-intel
            ;;
        "nouveau")
            pacman -S --noconfirm --needed mesa lib32-mesa xf86-video-nouveau
            ;;
        "none"|"None")
            log_info "No GPU drivers selected"
            ;;
        *)
            log_warn "Unknown GPU driver option: $gpu"
            ;;
    esac

    log_success "GPU driver installation complete"
}

# =============================================================================
# PHASE 4: ADDITIONAL SOFTWARE
# =============================================================================

install_aur_helper() {
    local helper="${AUR_HELPER:-none}"
    helper="${helper,,}"  # Convert to lowercase

    if [[ "$helper" == "none" || -z "$helper" ]]; then
        log_info "No AUR helper selected"
        return 0
    fi

    log_info "Installing AUR helper: $helper"

    # AUR helpers must be built as non-root user
    local build_dir="/tmp/aur_build"
    mkdir -p "$build_dir"
    chown "$MAIN_USERNAME:$MAIN_USERNAME" "$build_dir"

    case "$helper" in
        "paru")
            # Install paru dependencies
            pacman -S --noconfirm --needed base-devel git

            sudo -u "$MAIN_USERNAME" bash << 'AUREOF'
cd /tmp/aur_build
git clone https://aur.archlinux.org/paru.git
cd paru
makepkg -si --noconfirm
AUREOF
            ;;
        "yay")
            # Install yay dependencies
            pacman -S --noconfirm --needed base-devel git go

            sudo -u "$MAIN_USERNAME" bash << 'AUREOF'
cd /tmp/aur_build
git clone https://aur.archlinux.org/yay.git
cd yay
makepkg -si --noconfirm
AUREOF
            ;;
        "pikaur")
            pacman -S --noconfirm --needed base-devel git python

            sudo -u "$MAIN_USERNAME" bash << 'AUREOF'
cd /tmp/aur_build
git clone https://aur.archlinux.org/pikaur.git
cd pikaur
makepkg -si --noconfirm
AUREOF
            ;;
        *)
            log_warn "Unknown AUR helper: $helper"
            ;;
    esac

    # Cleanup
    rm -rf "$build_dir"

    log_success "AUR helper installation complete"
}

install_flatpak() {
    if [[ "${FLATPAK:-No}" != "Yes" ]]; then
        log_info "Flatpak not requested"
        return 0
    fi

    log_info "Installing Flatpak..."

    pacman -S --noconfirm --needed flatpak

    # Add Flathub repository for the user
    sudo -u "$MAIN_USERNAME" flatpak remote-add --if-not-exists --user flathub https://flathub.org/repo/flathub.flatpakrepo || true

    log_success "Flatpak installed"
}

install_additional_packages() {
    # Install additional pacman packages
    if [[ -n "${ADDITIONAL_PACKAGES:-}" ]]; then
        log_info "Installing additional packages: $ADDITIONAL_PACKAGES"

        # Convert space-separated string to array
        local -a packages
        read -ra packages <<< "$ADDITIONAL_PACKAGES"

        if [[ ${#packages[@]} -gt 0 ]]; then
            pacman -S --noconfirm --needed "${packages[@]}" || log_warn "Some packages may have failed to install"
        fi
    fi

    # Install additional AUR packages (if AUR helper is available)
    if [[ -n "${ADDITIONAL_AUR_PACKAGES:-}" ]]; then
        local helper="${AUR_HELPER:-none}"
        helper="${helper,,}"

        if [[ "$helper" != "none" && -n "$helper" ]] && command -v "$helper" &>/dev/null; then
            log_info "Installing additional AUR packages: $ADDITIONAL_AUR_PACKAGES"

            local -a aur_packages
            read -ra aur_packages <<< "$ADDITIONAL_AUR_PACKAGES"

            if [[ ${#aur_packages[@]} -gt 0 ]]; then
                sudo -u "$MAIN_USERNAME" "$helper" -S --noconfirm "${aur_packages[@]}" || log_warn "Some AUR packages may have failed to install"
            fi
        else
            log_warn "AUR packages requested but no AUR helper available"
        fi
    fi

    log_success "Additional packages installation complete"
}

configure_plymouth() {
    if [[ "${PLYMOUTH:-No}" != "Yes" ]]; then
        log_info "Plymouth not requested"
        return 0
    fi

    log_info "Configuring Plymouth..."

    # Install Plymouth if not already installed
    pacman -S --noconfirm --needed plymouth || true

    # Set Plymouth theme if specified
    if [[ -n "${PLYMOUTH_THEME:-}" && "${PLYMOUTH_THEME}" != "none" ]]; then
        if [[ -d "/usr/share/plymouth/themes/${PLYMOUTH_THEME}" ]]; then
            plymouth-set-default-theme -R "${PLYMOUTH_THEME}" || log_warn "Failed to set Plymouth theme"
            log_info "Plymouth theme set to: ${PLYMOUTH_THEME}"
        else
            log_warn "Plymouth theme not found: ${PLYMOUTH_THEME}"
            # List available themes
            log_info "Available themes: $(ls /usr/share/plymouth/themes/ 2>/dev/null || echo 'none')"
        fi
    fi

    log_success "Plymouth configured"
}

configure_snapper() {
    if [[ "${BTRFS_SNAPSHOTS:-No}" != "Yes" ]]; then
        log_info "Btrfs snapshots not requested"
        return 0
    fi

    # Check if filesystem is btrfs (use ROOT_FILESYSTEM_TYPE, not FILESYSTEM)
    if [[ "${ROOT_FILESYSTEM_TYPE:-ext4}" != "btrfs" ]]; then
        log_info "Snapper requires btrfs filesystem, skipping"
        return 0
    fi

    log_info "Configuring Snapper for Btrfs snapshots..."

    # Install snapper and related packages
    pacman -S --noconfirm --needed snapper snap-pac || {
        log_warn "Failed to install snapper packages"
        return 0
    }

    # Install grub-btrfs for boot integration if using GRUB
    if [[ "${BOOTLOADER:-grub}" == "grub" ]]; then
        pacman -S --noconfirm --needed grub-btrfs || log_warn "grub-btrfs not available"
    fi

    # Remove the @snapshots mount if it exists (snapper will recreate it)
    if mountpoint -q /.snapshots 2>/dev/null; then
        umount /.snapshots 2>/dev/null || true
    fi

    # Remove the .snapshots directory if it exists
    if [[ -d /.snapshots ]]; then
        rmdir /.snapshots 2>/dev/null || true
    fi

    # Create snapper config for root
    if snapper -c root create-config /; then
        log_info "Created snapper config for root"
    else
        log_warn "Failed to create snapper config (may already exist)"
    fi

    # Delete the subvolume snapper created and recreate .snapshots directory
    # This is needed because snapper creates a new subvolume, but we want to use @snapshots
    if btrfs subvolume delete /.snapshots 2>/dev/null; then
        log_info "Removed snapper-created .snapshots subvolume"
    fi
    mkdir -p /.snapshots

    # Remount @snapshots subvolume
    if grep -q "@snapshots" /etc/fstab; then
        mount /.snapshots || log_warn "Failed to remount @snapshots"
        log_info "Remounted @snapshots subvolume"
    fi

    # Set proper permissions
    chmod 750 /.snapshots

    # Configure snapper settings
    if [[ -f /etc/snapper/configs/root ]]; then
        # Set timeline settings for automatic snapshots
        sed -i 's/^TIMELINE_CREATE=.*/TIMELINE_CREATE="yes"/' /etc/snapper/configs/root
        sed -i 's/^TIMELINE_CLEANUP=.*/TIMELINE_CLEANUP="yes"/' /etc/snapper/configs/root
        sed -i 's/^TIMELINE_MIN_AGE=.*/TIMELINE_MIN_AGE="1800"/' /etc/snapper/configs/root

        # Map user's frequency preference to snapper timeline settings
        local keep_count="${BTRFS_KEEP_COUNT:-3}"
        local frequency="${BTRFS_FREQUENCY:-weekly}"

        # Reset all limits to 0 first, then set based on frequency
        local hourly_limit="0"
        local daily_limit="0"
        local weekly_limit="0"
        local monthly_limit="0"

        case "${frequency,,}" in
            hourly)
                hourly_limit="$keep_count"
                log_info "Snapper: keeping $keep_count hourly snapshots"
                ;;
            daily)
                daily_limit="$keep_count"
                log_info "Snapper: keeping $keep_count daily snapshots"
                ;;
            weekly)
                weekly_limit="$keep_count"
                log_info "Snapper: keeping $keep_count weekly snapshots"
                ;;
            monthly)
                monthly_limit="$keep_count"
                log_info "Snapper: keeping $keep_count monthly snapshots"
                ;;
            *)
                # Default to weekly if unknown
                weekly_limit="$keep_count"
                log_warn "Unknown frequency '$frequency', defaulting to weekly"
                ;;
        esac

        sed -i "s/^TIMELINE_LIMIT_HOURLY=.*/TIMELINE_LIMIT_HOURLY=\"$hourly_limit\"/" /etc/snapper/configs/root
        sed -i "s/^TIMELINE_LIMIT_DAILY=.*/TIMELINE_LIMIT_DAILY=\"$daily_limit\"/" /etc/snapper/configs/root
        sed -i "s/^TIMELINE_LIMIT_WEEKLY=.*/TIMELINE_LIMIT_WEEKLY=\"$weekly_limit\"/" /etc/snapper/configs/root
        sed -i "s/^TIMELINE_LIMIT_MONTHLY=.*/TIMELINE_LIMIT_MONTHLY=\"$monthly_limit\"/" /etc/snapper/configs/root
        sed -i 's/^TIMELINE_LIMIT_YEARLY=.*/TIMELINE_LIMIT_YEARLY="0"/' /etc/snapper/configs/root

        log_info "Configured snapper timeline settings (frequency: $frequency, keep: $keep_count)"
    fi

    # Install btrfs-assistant if requested
    if [[ "${BTRFS_ASSISTANT:-No}" == "Yes" ]]; then
        log_info "Installing btrfs-assistant..."
        pacman -S --noconfirm --needed btrfs-assistant || log_warn "Failed to install btrfs-assistant"
    fi

    # Enable snapper timers
    systemctl enable snapper-timeline.timer 2>/dev/null || true
    systemctl enable snapper-cleanup.timer 2>/dev/null || true

    # Enable grub-btrfs path monitoring if installed
    if [[ -f /usr/lib/systemd/system/grub-btrfsd.service ]]; then
        systemctl enable grub-btrfsd.service 2>/dev/null || true
        log_info "Enabled grub-btrfs daemon for boot menu updates"
    fi

    log_success "Snapper configured for automatic Btrfs snapshots"
}

# =============================================================================
# PHASE 5: FINAL CONFIGURATION
# =============================================================================

configure_numlock() {
    if [[ "${NUMLOCK_ON_BOOT:-No}" != "Yes" ]]; then
        return 0
    fi

    log_info "Configuring numlock on boot..."

    # For console (TTY)
    if [[ -f /etc/vconsole.conf ]]; then
        if ! grep -q "^KEYMAP_TOGGLE=" /etc/vconsole.conf; then
            # Create a systemd service for numlock
            cat > /etc/systemd/system/numlock.service << 'EOF'
[Unit]
Description=Activate numlock on boot

[Service]
Type=oneshot
ExecStart=/bin/bash -c 'for tty in /dev/tty{1..6}; do /usr/bin/setleds -D +num < "$tty"; done'

[Install]
WantedBy=multi-user.target
EOF
            systemctl enable numlock.service
        fi
    fi

    # For SDDM
    if [[ "${DISPLAY_MANAGER:-}" == "sddm" ]]; then
        mkdir -p /etc/sddm.conf.d
        cat > /etc/sddm.conf.d/numlock.conf << 'EOF'
[General]
Numlock=on
EOF
    fi

    log_success "Numlock configured"
}

deploy_dotfiles() {
    if [[ "${GIT_REPOSITORY:-No}" != "Yes" || -z "${GIT_REPOSITORY_URL:-}" ]]; then
        return 0
    fi

    log_info "Deploying dotfiles from: $GIT_REPOSITORY_URL"

    local user_home="/home/$MAIN_USERNAME"

    sudo -u "$MAIN_USERNAME" git clone "$GIT_REPOSITORY_URL" "$user_home/dotfiles" || {
        log_warn "Failed to clone dotfiles repository"
        return 0
    }

    # Run install script if it exists
    if [[ -x "$user_home/dotfiles/install.sh" ]]; then
        sudo -u "$MAIN_USERNAME" bash -c "cd \"$user_home/dotfiles\" && ./install.sh" || log_warn "Dotfiles install script failed"
    fi

    log_success "Dotfiles deployed"
}

final_cleanup() {
    log_info "Performing final cleanup..."

    # Clear package cache (keep last 2 versions)
    if command -v paccache &>/dev/null; then
        paccache -rk2 || true
    fi

    # Update man database
    mandb &>/dev/null || true

    log_success "Final cleanup complete"
}

# =============================================================================
# RUN MAIN FUNCTION
# =============================================================================

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
