#!/bin/bash
# chroot_config.sh - Complete chroot configuration for Arch Linux installer
# This script configures the newly installed Arch Linux system inside chroot

set -euo pipefail

# --- Signal Handling ---
cleanup_and_exit() {
    local sig="$1"
    # Remove any stale NOPASSWD sudoers fragments
    rm -f /etc/sudoers.d/archtui_*
    echo "$(basename "$0"): Received $sig, aborting..." >&2
    [[ "$sig" == "SIGTERM" ]] && exit 143 || exit 130
}
trap 'cleanup_and_exit SIGTERM' SIGTERM
trap 'cleanup_and_exit SIGINT' SIGINT

# Get script directory (we're running from / inside chroot)
SCRIPT_DIR="/"

# Source utility functions if available (using source_or_die pattern)
if [[ -f "$SCRIPT_DIR/utils.sh" ]]; then
    # shellcheck source=/dev/null
    if ! source "$SCRIPT_DIR/utils.sh"; then
        echo "FATAL: Failed to source: $SCRIPT_DIR/utils.sh" >&2
        exit 1
    fi
    # Initialize logging inside the chroot (persists to /var/log/archtui/ on installed system)
    # LOG_LEVEL is exported from install_config.sh — if VERBOSE, this enables set -x tracing
    setup_logging
fi

# =============================================================================
# FALLBACK LOGGING (if utils.sh not sourced)
# =============================================================================
if ! declare -f log_info > /dev/null 2>&1; then
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

    log_cmd "pacman -S ${packages[*]} --noconfirm --needed"
    pacman -S "${packages[@]}" --noconfirm --needed 2>&1 | while IFS= read -r line; do
        case "$line" in
            *"error"*|*"Error"*|*"ERROR"*)
                echo -e "\033[91m  [pacman] $line\033[0m"
                ;;
            *"warning"*|*"Warning"*|*"WARNING"*)
                echo -e "\033[33m  [pacman] $line\033[0m"
                ;;
            *"downloading"*|*"installing"*|*"::"*|*"Packages"*|*"Total"*)
                echo -e "\033[2m\033[36m  [pacman] $line\033[0m"
                ;;
            *)
                echo "  [pacman] $line"
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
    echo "PROGRESS: Configuring base system"

    configure_localization
    configure_hostname
    create_user_account
    configure_sudoers
    enable_base_services

    # --- Phase 2: Bootloader & Initramfs ---
    log_info "=== Phase 2: Bootloader & Initramfs ==="
    echo "PROGRESS: Configuring bootloader"

    configure_mkinitcpio
    install_bootloader
    configure_grub_settings
    configure_secure_boot || log_warn "Secure boot configuration skipped"

    # --- Phase 3: Desktop Environment ---
    log_info "=== Phase 3: Desktop Environment ==="
    echo "PROGRESS: Installing desktop environment"

    # Enable multilib repository in chroot if requested
    if [[ "${MULTILIB:-No}" == "Yes" ]]; then
        log_info "Enabling multilib repository in chroot..."
        sed -i '/^#\[multilib\]/,/^#Include/s/^#//' /etc/pacman.conf
        if ! grep -q '^\[multilib\]' /etc/pacman.conf; then
            log_warn "Could not enable multilib in pacman.conf — lib32 packages may not install"
        fi
        pacman -Sy || log_warn "Failed to sync multilib repository"
        log_success "Multilib repository enabled in chroot"
    fi

    install_aur_helper || log_warn "AUR helper installation failed — continuing"
    install_desktop_environment
    install_de_aur_packages || log_warn "DE AUR packages had issues — continuing"
    install_display_manager
    install_gpu_drivers || log_warn "GPU driver installation failed — continuing"

    # --- Phase 4: Additional Software (non-critical) ---
    log_info "=== Phase 4: Additional Software ==="
    echo "PROGRESS: Installing additional software"

    install_flatpak || log_warn "Flatpak installation failed — continuing"
    install_additional_packages || log_warn "Additional packages had issues — continuing"
    configure_plymouth || log_warn "Plymouth configuration failed — continuing"
    configure_snapper || log_warn "Snapper configuration failed — continuing"

    # --- Phase 5: Final Configuration (non-critical) ---
    log_info "=== Phase 5: Final Configuration ==="
    echo "PROGRESS: Running final configuration"

    configure_numlock || log_warn "Numlock configuration failed — continuing"
    deploy_dotfiles || log_warn "Dotfiles deployment failed — continuing"
    final_cleanup || true

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
        # Avoid duplicate entries
        if ! grep -q "^${LOCALE} UTF-8" /etc/locale.gen 2>/dev/null; then
            echo "${LOCALE} UTF-8" >> /etc/locale.gen || { log_error "Failed to update locale.gen"; return 1; }
        fi
        locale-gen || { log_error "locale-gen failed"; return 1; }
        echo "LANG=${LOCALE}" > /etc/locale.conf
    fi

    # Set timezone
    if [[ -n "${TIMEZONE_REGION:-}" && -n "${TIMEZONE:-}" ]]; then
        local tz_path="/usr/share/zoneinfo/${TIMEZONE_REGION}/${TIMEZONE}"
        # Validate timezone path stays within /usr/share/zoneinfo (prevent path traversal)
        local tz_real
        tz_real="$(realpath -m "$tz_path" 2>/dev/null)" || tz_real=""
        if [[ -n "$tz_real" && "$tz_real" == /usr/share/zoneinfo/* && -f "$tz_path" ]]; then
            log_info "Setting timezone to: ${TIMEZONE_REGION}/${TIMEZONE}"
            ln -sf "$tz_path" /etc/localtime || { log_error "Failed to set timezone symlink"; return 1; }
        else
            # Try without region
            tz_path="/usr/share/zoneinfo/${TIMEZONE}"
            tz_real="$(realpath -m "$tz_path" 2>/dev/null)" || tz_real=""
            if [[ -n "$tz_real" && "$tz_real" == /usr/share/zoneinfo/* && -f "$tz_path" ]]; then
                ln -sf "$tz_path" /etc/localtime || { log_error "Failed to set timezone symlink"; return 1; }
            else
                log_warn "Timezone not found or invalid path: ${TIMEZONE_REGION}/${TIMEZONE}"
            fi
        fi
    fi

    # Set hardware clock
    hwclock --systohc || log_warn "hwclock --systohc failed — system time may be incorrect after reboot"

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
        log_cmd "useradd -m -G wheel,users,audio,video,storage,optical -s /bin/bash $MAIN_USERNAME"
        useradd -m -G wheel,users,audio,video,storage,optical -s /bin/bash "$MAIN_USERNAME" || { log_error "Failed to create user $MAIN_USERNAME"; return 1; }
        log_info "User $MAIN_USERNAME created"
    else
        log_info "User $MAIN_USERNAME already exists"
    fi

    # Set user password (tracing disabled to prevent password leak in verbose logs)
    if [[ -n "${MAIN_USER_PASSWORD:-}" ]]; then
        { set +x; } 2>/dev/null
        log_cmd "printf '***:***' | chpasswd (user password)"
        printf '%s:%s\n' "$MAIN_USERNAME" "$MAIN_USER_PASSWORD" | chpasswd || log_warn "Failed to set user password"
        unset MAIN_USER_PASSWORD  # ROE §8.1: clear immediately after use
        if [[ "${LOG_LEVEL:-INFO}" == "VERBOSE" || "${LOG_LEVEL:-INFO}" == "DEBUG" ]]; then set -x; fi
        log_info "User password set"
    fi

    # Set root password (tracing disabled to prevent password leak in verbose logs)
    if [[ -n "${ROOT_PASSWORD:-}" ]]; then
        { set +x; } 2>/dev/null
        log_cmd "printf '***:***' | chpasswd (root password)"
        printf '%s:%s\n' "root" "$ROOT_PASSWORD" | chpasswd || log_warn "Failed to set root password"
        unset ROOT_PASSWORD  # ROE §8.1: clear immediately after use
        if [[ "${LOG_LEVEL:-INFO}" == "VERBOSE" || "${LOG_LEVEL:-INFO}" == "DEBUG" ]]; then set -x; fi
        log_info "Root password set"
    fi

    log_success "User account configured"
}

configure_sudoers() {
    log_info "Configuring sudoers..."

    # Enable wheel group for sudo (without password for installation, can be changed later)
    if [[ -f /etc/sudoers ]]; then
        # Use sed to uncomment the wheel line
        sed -i 's/^# %wheel ALL=(ALL:ALL) ALL/%wheel ALL=(ALL:ALL) ALL/' /etc/sudoers || log_warn "sed failed to modify sudoers"

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
    log_cmd "systemctl enable NetworkManager.service"
    systemctl enable NetworkManager.service 2>/dev/null || log_warn "NetworkManager service not found"

    # SSH (optional, but useful)
    log_cmd "systemctl enable sshd.service"
    systemctl enable sshd.service 2>/dev/null || log_warn "sshd service not found"

    # Time synchronization
    case "${TIME_SYNC:-No}" in
        "systemd-timesyncd"|"Yes")
            systemctl enable systemd-timesyncd.service 2>/dev/null || log_warn "Failed to enable systemd-timesyncd"
            ;;
        "ntpd")
            systemctl enable ntpd.service 2>/dev/null || log_warn "Failed to enable ntpd"
            ;;
        "chrony")
            systemctl enable chronyd.service 2>/dev/null || log_warn "Failed to enable chronyd"
            ;;
        "No"|"")
            log_info "Time synchronization disabled by user"
            ;;
    esac

    # SSD trim timer (good for SSDs)
    systemctl enable fstrim.timer 2>/dev/null || log_warn "Failed to enable fstrim.timer"

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
    # Systemd-based hooks (default since mkinitcpio 39, 2025):
    # https://wiki.archlinux.org/title/Mkinitcpio
    # https://wiki.archlinux.org/title/Dm-crypt/System_configuration
    #
    # Standard: base systemd autodetect microcode modconf kms keyboard sd-vconsole block filesystems fsck
    # Encrypted: base systemd keyboard sd-vconsole autodetect microcode modconf kms block [plymouth] sd-encrypt [lvm2] [resume] filesystems [fsck]
    #
    # Key requirements per wiki:
    # - systemd replaces udev (provides device management + more)
    # - sd-vconsole replaces keymap + consolefont (reads /etc/vconsole.conf)
    # - sd-encrypt replaces encrypt (uses rd.luks.name= kernel params)
    # - keyboard/sd-vconsole: BEFORE sd-encrypt (so keyboard works for password entry)
    # - plymouth: BEFORE sd-encrypt (for graphical password prompt)
    # - For hardware compatibility (varying keyboards): keyboard BEFORE autodetect
    local hooks=""

    if [[ "${ENCRYPTION:-No}" == "Yes" ]] || [[ "${PARTITIONING_STRATEGY:-}" == *"luks"* ]]; then
        # Encrypted: keyboard before autodetect for hardware compatibility
        # This ensures keyboard works even if booting on different hardware than image was built on
        hooks="base systemd keyboard sd-vconsole autodetect microcode modconf kms block"
        log_info "Using encrypted system hook order (keyboard before autodetect for compatibility)"
    else
        # Non-encrypted: standard systemd-based hook order
        hooks="base systemd autodetect microcode modconf kms keyboard sd-vconsole block"
    fi

    # Add RAID hook if using RAID (must come before sd-encrypt/lvm2)
    if [[ "${PARTITIONING_STRATEGY:-}" == *"raid"* ]]; then
        hooks="$hooks mdadm_udev"
        log_info "Added mdadm_udev hook for RAID"

        # Save mdadm configuration
        if command -v mdadm &>/dev/null; then
            mdadm --detail --scan >> /etc/mdadm.conf 2>/dev/null || true
        fi
    fi

    # Add Plymouth hook BEFORE sd-encrypt (per Arch Wiki: "place plymouth before the encrypt hook")
    # Install plymouth BEFORE adding hook so hook files exist when mkinitcpio -P runs
    if [[ "${PLYMOUTH:-No}" == "Yes" ]]; then
        pacman -S plymouth --noconfirm --needed || log_warn "Failed to install plymouth"
        hooks="$hooks plymouth"
        log_info "Added plymouth hook (before sd-encrypt per Arch Wiki)"
    fi

    # Add sd-encrypt hook if using LUKS (must come before lvm2 for LUKS-on-LVM)
    # sd-encrypt uses rd.luks.name= kernel params instead of cryptdevice=
    if [[ "${ENCRYPTION:-No}" == "Yes" ]] || [[ "${PARTITIONING_STRATEGY:-}" == *"luks"* ]]; then
        hooks="$hooks sd-encrypt"
        log_info "Added sd-encrypt hook for LUKS"
    fi

    # Add LVM hook if using LVM (must come after sd-encrypt for LUKS-on-LVM)
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
        sed -i "s|^HOOKS=.*|HOOKS=($hooks)|" /etc/mkinitcpio.conf || { log_error "sed failed on HOOKS"; return 1; }
        # Verify HOOKS was actually updated (sed returns 0 even if pattern didn't match)
        if ! grep -q "^HOOKS=($hooks)" /etc/mkinitcpio.conf; then
            log_error "HOOKS not updated — expected: HOOKS=($hooks)"
            log_error "Actual: $(grep '^HOOKS=' /etc/mkinitcpio.conf || echo 'NO HOOKS LINE FOUND')"
            return 1
        fi
        log_info "Updated HOOKS in mkinitcpio.conf: $hooks"

        # Add btrfs module if using Btrfs
        if [[ "${ROOT_FILESYSTEM_TYPE:-ext4}" == "btrfs" ]]; then
            if ! grep -q "btrfs" /etc/mkinitcpio.conf; then
                sed -i 's/^MODULES=(\(.*\))/MODULES=(\1 btrfs)/' /etc/mkinitcpio.conf || log_warn "Failed to add btrfs module"
                # Clean up double spaces
                sed -i 's/MODULES=( /MODULES=(/' /etc/mkinitcpio.conf || log_warn "Failed to clean MODULES spacing"
                log_info "Added btrfs module to mkinitcpio.conf"
            fi
        fi

        # Add amdgpu module for early KMS if AMD GPU detected
        if lspci 2>/dev/null | grep -qi "amd.*radeon\|radeon.*amd\|amd.*graphics"; then
            if ! grep -q "amdgpu" /etc/mkinitcpio.conf; then
                sed -i 's/^MODULES=(\(.*\))/MODULES=(\1 amdgpu)/' /etc/mkinitcpio.conf || log_warn "Failed to add amdgpu module"
                sed -i 's/MODULES=( /MODULES=(/' /etc/mkinitcpio.conf || log_warn "Failed to clean MODULES spacing"
                log_info "Added amdgpu module for early KMS"
            fi
        fi

        # FIDO2 crypttab.initramfs entry (for sd-encrypt to use fido2-device=auto)
        if [[ "${ENCRYPTION_KEY_TYPE:-Password}" == *"FIDO2"* ]]; then
            if [[ -n "${LUKS_UUID:-}" ]]; then
                local mapper_name="cryptroot"
                [[ "${PARTITIONING_STRATEGY:-}" == *"lvm"* ]] && mapper_name="cryptlvm"
                echo "${mapper_name} UUID=${LUKS_UUID} - fido2-device=auto" >> /etc/crypttab.initramfs
                log_info "Added FIDO2 entry to crypttab.initramfs (mapper: $mapper_name)"
            else
                log_warn "FIDO2 configured but LUKS_UUID not set — skipping crypttab.initramfs"
            fi
        fi

        # UKI (Unified Kernel Image) support
        if [[ "${UNIFIED_KERNEL_IMAGE:-No}" == "Yes" ]]; then
            log_info "Configuring Unified Kernel Image (UKI)..."

            local uki_options
            uki_options=$(_build_kernel_options 2>/dev/null || true)
            if [[ -n "$uki_options" ]]; then
                mkdir -p /etc/kernel
                echo "$uki_options" > /etc/kernel/cmdline
                log_info "Wrote kernel cmdline to /etc/kernel/cmdline"
            fi

            # Configure mkinitcpio preset for UKI output
            local preset="/etc/mkinitcpio.d/${KERNEL:-linux}.preset"
            if [[ -f "$preset" ]]; then
                local efi_dir="/efi/EFI/Linux"
                [[ -d "/boot/EFI" ]] && efi_dir="/boot/EFI/Linux"
                mkdir -p "$efi_dir"
                # Add UKI output path to preset
                if ! grep -q "default_uki" "$preset"; then
                    echo "default_uki=\"${efi_dir}/arch-linux-${KERNEL:-linux}.efi\"" >> "$preset"
                    echo "fallback_uki=\"${efi_dir}/arch-linux-${KERNEL:-linux}-fallback.efi\"" >> "$preset"
                    log_info "Added UKI paths to mkinitcpio preset"
                fi
            fi
        fi

        # Regenerate initramfs
        log_cmd "mkinitcpio -P"
        if ! mkinitcpio -P; then
            log_error "mkinitcpio failed — system may not boot"
            return 1
        fi
        log_success "Initramfs regenerated"
    else
        log_error "mkinitcpio.conf not found"
        return 1
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
        "refind")
            install_refind
            ;;
        "limine")
            install_limine
            ;;
        "efistub")
            install_efistub
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
        log_cmd "grub-install --target=x86_64-efi --efi-directory=$efi_dir --bootloader-id=GRUB --recheck"
        grub-install --target=x86_64-efi --efi-directory="$efi_dir" --bootloader-id=GRUB --recheck || {
            log_error "GRUB installation failed"
            return 1
        }
    else
        # BIOS installation — extract first disk for RAID (comma-separated INSTALL_DISK)
        local bios_disk="${INSTALL_DISK%%,*}"
        if [[ -z "$bios_disk" ]]; then
            log_error "INSTALL_DISK not set for BIOS GRUB install"
            return 1
        fi
        log_info "Installing GRUB for BIOS to $bios_disk"
        log_cmd "grub-install --target=i386-pc $bios_disk --recheck"
        grub-install --target=i386-pc "$bios_disk" --recheck || {
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

    log_cmd "bootctl install --esp-path=$esp_path"
    bootctl install --esp-path="$esp_path" || {
        log_error "systemd-boot installation failed"
        return 1
    }

    # Create boot entry
    mkdir -p "${esp_path}/loader/entries"

    # Get root partition UUID
    local root_uuid="${ROOT_UUID:-}"
    if [[ -z "$root_uuid" ]]; then
        root_uuid=$(findmnt -n -o UUID /) || true
    fi
    if [[ -z "$root_uuid" ]]; then
        log_error "Cannot determine root partition UUID for systemd-boot entry"
        return 1
    fi

    # Build options line
    local options=""

    # Handle encryption (sd-encrypt uses rd.luks.name= instead of cryptdevice=)
    if [[ "${ENCRYPTION:-No}" == "Yes" ]] || [[ "${PARTITIONING_STRATEGY:-}" == *"luks"* ]]; then
        if [[ -n "${LUKS_UUID:-}" ]]; then
            local mapper_name="cryptroot"
            [[ "${PARTITIONING_STRATEGY:-}" == *"lvm"* ]] && mapper_name="cryptlvm"
            if [[ "${PARTITIONING_STRATEGY:-}" == *"lvm"* ]]; then
                options="rd.luks.name=${LUKS_UUID}=${mapper_name} root=/dev/archvg/root"
            else
                options="rd.luks.name=${LUKS_UUID}=${mapper_name} root=/dev/mapper/${mapper_name}"
            fi
        else
            log_error "LUKS_UUID not set for encrypted system — systemd-boot entry will be invalid"
            return 1
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

# Build kernel command line options (shared by rEFInd, Limine, EFISTUB)
_build_kernel_options() {
    local options=""

    # Handle encryption
    if [[ "${ENCRYPTION:-No}" == "Yes" ]] || [[ "${PARTITIONING_STRATEGY:-}" == *"luks"* ]]; then
        if [[ -n "${LUKS_UUID:-}" ]]; then
            local mapper_name="cryptroot"
            [[ "${PARTITIONING_STRATEGY:-}" == *"lvm"* ]] && mapper_name="cryptlvm"
            if [[ "${PARTITIONING_STRATEGY:-}" == *"lvm"* ]]; then
                options="rd.luks.name=${LUKS_UUID}=${mapper_name} root=/dev/archvg/root"
            else
                options="rd.luks.name=${LUKS_UUID}=${mapper_name} root=/dev/mapper/${mapper_name}"
            fi
        else
            log_error "LUKS_UUID not set for encrypted system"
            echo ""
            return 1
        fi
    else
        local root_uuid="${ROOT_UUID:-}"
        if [[ -z "$root_uuid" ]]; then
            root_uuid=$(findmnt -n -o UUID /) || true
        fi
        if [[ -z "$root_uuid" ]]; then
            log_error "Cannot determine root partition UUID"
            echo ""
            return 1
        fi
        options="root=UUID=${root_uuid}"
    fi

    # Btrfs rootflags
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

    echo "$options"
}

install_refind() {
    log_info "Installing rEFInd..."

    if [[ "${BOOT_MODE:-UEFI}" != "UEFI" ]]; then
        log_error "rEFInd requires UEFI firmware"
        return 1
    fi

    log_cmd "refind-install"
    refind-install || {
        log_error "refind-install failed"
        return 1
    }

    local options
    options=$(_build_kernel_options) || return 1

    # Microcode
    local microcode=""
    if [[ -f /boot/intel-ucode.img ]]; then
        microcode="initrd=intel-ucode.img"
    elif [[ -f /boot/amd-ucode.img ]]; then
        microcode="initrd=amd-ucode.img"
    fi

    # Generate refind_linux.conf
    {
        echo "\"Boot with defaults\" \"$options $microcode initrd=initramfs-${KERNEL:-linux}.img\""
        echo "\"Boot fallback\"      \"$options $microcode initrd=initramfs-${KERNEL:-linux}-fallback.img\""
    } > /boot/refind_linux.conf

    log_success "rEFInd installed"
}

install_limine() {
    log_info "Installing Limine..."

    local options
    options=$(_build_kernel_options) || return 1

    # Microcode
    local microcode_module=""
    if [[ -f /boot/intel-ucode.img ]]; then
        microcode_module="MODULE_PATH=boot:///intel-ucode.img"
    elif [[ -f /boot/amd-ucode.img ]]; then
        microcode_module="MODULE_PATH=boot:///amd-ucode.img"
    fi

    if [[ "${BOOT_MODE:-UEFI}" == "UEFI" ]]; then
        # UEFI: copy EFI binary to ESP
        local esp_path="/efi"
        [[ -d "/boot/EFI" ]] && esp_path="/boot"
        mkdir -p "${esp_path}/EFI/BOOT" || { log_error "Failed to create EFI directory"; return 1; }
        log_cmd "cp /usr/share/limine/BOOTX64.EFI ${esp_path}/EFI/BOOT/"
        cp /usr/share/limine/BOOTX64.EFI "${esp_path}/EFI/BOOT/" || {
            log_error "Failed to copy Limine EFI binary"
            return 1
        }
    else
        # BIOS: install to disk MBR
        local target_disk="${INSTALL_DISK%%,*}"  # First disk for RAID
        log_cmd "limine bios-install $target_disk"
        limine bios-install "$target_disk" || {
            log_error "Limine BIOS install failed on $target_disk"
            return 1
        }
    fi

    # Generate limine.conf
    {
        echo "timeout: 5"
        echo ""
        echo "/Arch Linux"
        echo "    protocol: linux"
        echo "    kernel_path: boot:///vmlinuz-${KERNEL:-linux}"
        [[ -n "$microcode_module" ]] && echo "    $microcode_module"
        echo "    MODULE_PATH=boot:///initramfs-${KERNEL:-linux}.img"
        echo "    KERNEL_CMDLINE=$options"
        echo ""
        echo "/Arch Linux (fallback)"
        echo "    protocol: linux"
        echo "    kernel_path: boot:///vmlinuz-${KERNEL:-linux}"
        [[ -n "$microcode_module" ]] && echo "    $microcode_module"
        echo "    MODULE_PATH=boot:///initramfs-${KERNEL:-linux}-fallback.img"
        echo "    KERNEL_CMDLINE=$options"
    } > /boot/limine.conf

    log_success "Limine installed"
}

install_efistub() {
    log_info "Installing EFISTUB..."

    if [[ "${BOOT_MODE:-UEFI}" != "UEFI" ]]; then
        log_error "EFISTUB requires UEFI firmware"
        return 1
    fi

    local options
    options=$(_build_kernel_options) || return 1

    # Get ESP disk and partition number from findmnt
    local esp_source
    esp_source=$(findmnt -n -o SOURCE /efi 2>/dev/null || findmnt -n -o SOURCE /boot 2>/dev/null || echo "")
    if [[ -z "$esp_source" ]]; then
        log_error "Cannot determine ESP device for efibootmgr"
        return 1
    fi

    # Extract disk and partition number (e.g., /dev/sda1 → /dev/sda + 1, /dev/nvme0n1p1 → /dev/nvme0n1 + 1)
    local esp_disk esp_partnum
    if [[ "$esp_source" =~ ^(/dev/nvme[0-9]+n[0-9]+)p([0-9]+)$ ]]; then
        esp_disk="${BASH_REMATCH[1]}"
        esp_partnum="${BASH_REMATCH[2]}"
    elif [[ "$esp_source" =~ ^(/dev/[a-z]+)([0-9]+)$ ]]; then
        esp_disk="${BASH_REMATCH[1]}"
        esp_partnum="${BASH_REMATCH[2]}"
    else
        log_error "Cannot parse ESP device: $esp_source"
        return 1
    fi

    # Build initrd args (microcode first, then initramfs)
    local initrd_args=""
    if [[ -f /boot/intel-ucode.img ]]; then
        initrd_args="initrd=\\intel-ucode.img "
    elif [[ -f /boot/amd-ucode.img ]]; then
        initrd_args="initrd=\\amd-ucode.img "
    fi
    initrd_args="${initrd_args}initrd=\\initramfs-${KERNEL:-linux}.img"

    log_cmd "efibootmgr --create --disk $esp_disk --part $esp_partnum --loader /vmlinuz-${KERNEL:-linux} --label 'Arch Linux'"
    efibootmgr --create --disk "$esp_disk" --part "$esp_partnum" \
        --loader "/vmlinuz-${KERNEL:-linux}" \
        --label "Arch Linux" \
        --unicode "$options $initrd_args" || {
        log_error "efibootmgr failed to create boot entry"
        return 1
    }

    # Install pacman hook for automatic EFI entry update on kernel upgrade
    mkdir -p /etc/pacman.d/hooks
    cat > /etc/pacman.d/hooks/efistub.hook << 'HOOKEOF'
[Trigger]
Type = Package
Operation = Upgrade
Target = linux
Target = linux-lts
Target = linux-zen
Target = linux-hardened

[Action]
Description = Updating EFISTUB boot entry...
When = PostTransaction
Exec = /usr/bin/bash -c 'efibootmgr -v 2>/dev/null | grep -q "Arch Linux" && echo "EFISTUB entry exists" || echo "WARNING: EFISTUB entry missing — run efibootmgr manually"'
HOOKEOF

    log_success "EFISTUB installed (EFI entry created)"
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

    # Add encryption parameters if needed (sd-encrypt uses rd.luks.name= instead of cryptdevice=)
    if [[ "${ENCRYPTION:-No}" == "Yes" ]] || [[ "${PARTITIONING_STRATEGY:-}" == *"luks"* ]]; then
        if [[ -n "${LUKS_UUID:-}" ]]; then
            # Determine mapper name based on strategy
            local mapper_name="cryptroot"
            if [[ "${PARTITIONING_STRATEGY:-}" == *"lvm"* ]]; then
                mapper_name="cryptlvm"
            fi
            cmdline="$cmdline rd.luks.name=${LUKS_UUID}=${mapper_name}"
            # Add root= for the decrypted mapper device
            if [[ "${PARTITIONING_STRATEGY:-}" == *"lvm"* ]]; then
                cmdline="$cmdline root=/dev/archvg/root"
            else
                cmdline="$cmdline root=/dev/mapper/${mapper_name}"
            fi
        else
            log_error "LUKS_UUID not set for encrypted system — GRUB cmdline will be invalid"
            return 1
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
    sed -i "s|^GRUB_CMDLINE_LINUX_DEFAULT=.*|GRUB_CMDLINE_LINUX_DEFAULT=\"$cmdline\"|" "$grub_default"

    # Enable os-prober if requested OR if other OS was detected during partitioning
    # This ensures dual-boot is properly configured even if user forgot to enable it
    if [[ "${OS_PROBER:-No}" == "Yes" ]] || [[ "${OTHER_OS_DETECTED:-}" == "yes" ]] || [[ "${WINDOWS_DETECTED:-}" == "yes" ]]; then
        log_info "Enabling os-prober for dual-boot detection"
        if ! grep -q "^GRUB_DISABLE_OS_PROBER=false" "$grub_default"; then
            echo "GRUB_DISABLE_OS_PROBER=false" >> "$grub_default"
        fi

        # Install os-prober if not present
        if ! command -v os-prober &>/dev/null; then
            pacman -S os-prober --noconfirm --needed || log_warn "Failed to install os-prober"
        fi
    fi

    # Add explicit Windows chainload entry if Windows was detected
    # This provides a fallback if os-prober doesn't detect it
    if [[ "${WINDOWS_DETECTED:-}" == "yes" && -n "${WINDOWS_EFI_PATH:-}" ]]; then
        log_info "Adding Windows Boot Manager chainload entry"
        if [[ ! -f /etc/grub.d/40_custom ]] || ! grep -q "Windows Boot Manager" /etc/grub.d/40_custom; then
            # Detect the EFI partition UUID for the search command
            local efi_part_uuid=""
            local esp_mount=""
            # Find the ESP mount point
            for mp in /boot/efi /boot /efi; do
                if mountpoint -q "$mp" 2>/dev/null; then
                    esp_mount="$mp"
                    break
                fi
            done
            if [[ -n "$esp_mount" ]]; then
                local esp_dev
                esp_dev=$(findmnt -n -o SOURCE "$esp_mount" 2>/dev/null || true)
                if [[ -n "$esp_dev" ]]; then
                    efi_part_uuid=$(blkid -s UUID -o value "$esp_dev" 2>/dev/null || true)
                fi
            fi

            if [[ -n "$efi_part_uuid" ]]; then
                cat >> /etc/grub.d/40_custom << WINEOF

menuentry "Windows Boot Manager" {
    insmod part_gpt
    insmod fat
    insmod chain
    search --no-floppy --fs-uuid --set=root ${efi_part_uuid}
    chainloader /EFI/Microsoft/Boot/bootmgfw.efi
}
WINEOF
                log_info "Added Windows chainload entry with EFI UUID: $efi_part_uuid"
            else
                log_warn "Could not detect EFI partition UUID — skipping Windows chainload entry"
                log_warn "os-prober should still detect Windows automatically"
            fi
        fi
    fi

    # Enable GRUB_ENABLE_CRYPTODISK for encrypted /boot
    if [[ "${ENCRYPTION:-No}" == "Yes" ]] || [[ "${PARTITIONING_STRATEGY:-}" == *"luks"* ]]; then
        if ! grep -q "^GRUB_ENABLE_CRYPTODISK=y" "$grub_default"; then
            echo "GRUB_ENABLE_CRYPTODISK=y" >> "$grub_default"
        fi
    fi

    # Configure GRUB theme if requested
    configure_grub_theme

    # Generate GRUB config
    mkdir -p /boot/grub
    log_cmd "grub-mkconfig -o /boot/grub/grub.cfg"
    grub-mkconfig -o /boot/grub/grub.cfg || {
        log_error "grub-mkconfig failed — system may not boot correctly"
        return 1
    }

    log_success "GRUB configured"
}

# Clone a GRUB theme from git and install to /boot/grub/themes/
_grub_theme_git_clone() {
    local repo_url="$1"
    local clone_dir="$2"
    local theme_name="$3"
    local tmp_dir="/tmp/grub-theme-$$"

    if ! command -v git &>/dev/null; then
        log_warn "git not installed, cannot clone GRUB theme"
        pacman -S git --noconfirm --needed || {
            log_error "Failed to install git for GRUB theme"
            return 1
        }
    fi

    mkdir -p "$tmp_dir"
    if timeout 30 git clone --depth 1 "$repo_url" "$tmp_dir/$clone_dir" 2>/dev/null; then
        mkdir -p "/boot/grub/themes/${theme_name}"
        # Look for theme.txt to find the theme root (may be in a subdirectory)
        local theme_txt
        theme_txt="$(find "$tmp_dir/$clone_dir" -name "theme.txt" -print -quit 2>/dev/null)"
        if [[ -n "$theme_txt" ]]; then
            local theme_root
            theme_root="$(dirname "$theme_txt")"
            cp -r "$theme_root/"* "/boot/grub/themes/${theme_name}/"
            log_info "GRUB theme cloned and installed: $theme_name"
        else
            log_warn "theme.txt not found in cloned repo: $repo_url"
        fi
    else
        log_warn "Failed to clone GRUB theme: $repo_url"
    fi
    rm -rf "${tmp_dir:?}"
}

configure_grub_theme() {
    if [[ "${GRUB_THEME:-No}" != "Yes" ]]; then
        log_info "GRUB theme not requested"
        return 0
    fi

    local theme_name="${GRUB_THEME_SELECTION:-PolyDark}"
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

        # Clone GRUB theme from git
        case "${theme_name}" in
            "PolyDark"|"poly-dark"|"polydark")
                _grub_theme_git_clone "https://github.com/shvchk/poly-dark.git" "poly-dark" "$theme_name"
                ;;
            "CyberEXS"|"cyberexs")
                _grub_theme_git_clone "https://github.com/HenriqueLopes42/themeGrub.CyberEXS.git" "themeGrub.CyberEXS" "$theme_name"
                ;;
            "CyberPunk"|"cyberpunk")
                _grub_theme_git_clone "https://github.com/NayamAmarshe/Cyberpunk-GRUB-Theme.git" "Cyberpunk-GRUB-Theme" "$theme_name"
                ;;
            "HyperFluent"|"hyperfluent")
                _grub_theme_git_clone "https://github.com/Coopydood/HyperFluent-GRUB-Theme.git" "HyperFluent-GRUB-Theme" "$theme_name"
                ;;
            *)
                log_warn "Unknown GRUB theme: $theme_name"
                ;;
        esac
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
    pacman -S sbctl --noconfirm --needed || {
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
            install_packages "Hyprland" hyprland waybar swaylock swayidle \
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
        "cosmic")
            install_packages "COSMIC (base)" networkmanager pipewire pipewire-pulse firefox
            ;;
        "deepin")
            install_packages "Deepin" deepin deepin-extra
            ;;
        "lxde")
            install_packages "LXDE" lxde
            ;;
        "lxqt")
            install_packages "LXQt" lxqt breeze-icons
            ;;
        "bspwm")
            install_packages "bspwm" bspwm sxhkd xorg-server xorg-xinit alacritty dmenu picom feh thunar
            ;;
        "awesome")
            install_packages "Awesome WM" awesome xorg-server xorg-xinit alacritty picom thunar feh
            ;;
        "qtile")
            install_packages "Qtile" qtile python-psutil xorg-server xorg-xinit alacritty picom thunar
            ;;
        "river")
            install_packages "River" river xdg-desktop-portal-wlr waybar foot wofi mako grim slurp wl-clipboard thunar
            ;;
        "niri")
            install_packages "Niri" niri xdg-desktop-portal-gnome waybar foot fuzzel mako grim slurp wl-clipboard nautilus
            ;;
        "labwc")
            install_packages "Labwc" labwc xdg-desktop-portal-wlr waybar foot wofi mako grim slurp wl-clipboard thunar
            ;;
        "xmonad")
            install_packages "XMonad" xmonad xmonad-contrib xmobar xorg-server xorg-xinit dmenu alacritty picom thunar
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

install_de_aur_packages() {
    local de="${DESKTOP_ENVIRONMENT:-none}"
    de="${de,,}"

    local -a aur_packages=()

    case "$de" in
        "hyprland")
            aur_packages=(wlogout)
            ;;
        "cosmic")
            aur_packages=(cosmic-session cosmic-comp cosmic-panel cosmic-settings
                          cosmic-applets cosmic-bg cosmic-greeter cosmic-launcher
                          cosmic-notifications cosmic-osd cosmic-screenshot
                          cosmic-workspaces)
            ;;
        *)
            return 0
            ;;
    esac

    if [[ ${#aur_packages[@]} -eq 0 ]]; then
        return 0
    fi

    local helper="${AUR_HELPER:-none}"
    helper="${helper,,}"

    if [[ "$helper" == "none" || -z "$helper" ]] || ! command -v "$helper" &>/dev/null; then
        log_warn "DE '$de' has AUR packages (${aur_packages[*]}) but no AUR helper available — skipping"
        return 0
    fi

    log_info "Installing AUR packages for $de: ${aur_packages[*]}"

    local sudoers_drop="/etc/sudoers.d/temp-de-aur"
    echo "$MAIN_USERNAME ALL=(ALL) NOPASSWD: ALL" > "$sudoers_drop"
    chmod 440 "$sudoers_drop" || { log_error "Failed to set sudoers permissions"; rm -f "$sudoers_drop"; return 1; }
    trap 'rm -f /etc/sudoers.d/temp-de-aur' RETURN

    timeout 600 runuser -u "$MAIN_USERNAME" -- "$helper" -S "${aur_packages[@]}" --noconfirm || log_warn "Some DE AUR packages may have failed to install"
}

install_display_manager() {
    local dm="${DISPLAY_MANAGER:-none}"
    dm="${dm,,}"  # Convert to lowercase

    log_info "Installing display manager: $dm"

    case "$dm" in
        "sddm")
            install_packages "SDDM" sddm
            log_info "Enabling SDDM service..."
            systemctl enable sddm.service || log_warn "Failed to enable sddm.service"
            ;;
        "gdm")
            install_packages "GDM" gdm
            log_info "Enabling GDM service..."
            systemctl enable gdm.service || log_warn "Failed to enable gdm.service"
            ;;
        "lightdm")
            install_packages "LightDM" lightdm lightdm-gtk-greeter
            log_info "Enabling LightDM service..."
            systemctl enable lightdm.service || log_warn "Failed to enable lightdm.service"
            ;;
        "lxdm")
            install_packages "LXDM" lxdm
            log_info "Enabling LXDM service..."
            systemctl enable lxdm.service || log_warn "Failed to enable lxdm.service"
            ;;
        "ly")
            install_packages "Ly" ly
            log_info "Enabling Ly service..."
            systemctl enable ly.service || log_warn "Failed to enable ly.service"
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
    local multilib="${MULTILIB:-No}"

    log_info "Installing GPU drivers: $gpu"

    # Only include lib32 packages if multilib repo is enabled
    local use_lib32="no"
    if [[ "$multilib" == "Yes" ]]; then
        use_lib32="yes"
    fi

    case "$gpu" in
        "Auto"|"auto")
            # Auto-detect GPU
            if lspci | grep -qi nvidia; then
                log_info "NVIDIA GPU detected"
                pacman -S nvidia nvidia-utils nvidia-settings --noconfirm --needed || log_warn "Failed to install NVIDIA drivers"
            fi
            if lspci | grep -qi "amd.*radeon\|radeon.*amd\|amd.*graphics"; then
                log_info "AMD GPU detected"
                local amd_pkgs=(mesa xf86-video-amdgpu vulkan-radeon)
                [[ "$use_lib32" == "yes" ]] && amd_pkgs+=(lib32-mesa)
                pacman -S "${amd_pkgs[@]}" --noconfirm --needed || log_warn "Failed to install AMD drivers"
            fi
            if lspci | grep -qi "intel.*graphics\|intel.*uhd\|intel.*iris"; then
                log_info "Intel GPU detected"
                local intel_pkgs=(mesa xf86-video-intel vulkan-intel)
                [[ "$use_lib32" == "yes" ]] && intel_pkgs+=(lib32-mesa)
                pacman -S "${intel_pkgs[@]}" --noconfirm --needed || log_warn "Failed to install Intel drivers"
            fi
            ;;
        "nvidia"|"NVIDIA")
            pacman -S nvidia nvidia-utils nvidia-settings --noconfirm --needed || log_warn "Failed to install NVIDIA drivers"
            ;;
        "nvidia-open")
            pacman -S nvidia-open nvidia-utils nvidia-settings --noconfirm --needed || log_warn "Failed to install NVIDIA-open drivers"
            ;;
        "amd"|"AMD")
            local amd_pkgs=(mesa xf86-video-amdgpu vulkan-radeon)
            [[ "$use_lib32" == "yes" ]] && amd_pkgs+=(lib32-mesa)
            pacman -S "${amd_pkgs[@]}" --noconfirm --needed || log_warn "Failed to install AMD drivers"
            ;;
        "intel"|"Intel")
            local intel_pkgs=(mesa xf86-video-intel vulkan-intel)
            [[ "$use_lib32" == "yes" ]] && intel_pkgs+=(lib32-mesa)
            pacman -S "${intel_pkgs[@]}" --noconfirm --needed || log_warn "Failed to install Intel drivers"
            ;;
        "nouveau")
            local nouveau_pkgs=(mesa xf86-video-nouveau)
            [[ "$use_lib32" == "yes" ]] && nouveau_pkgs+=(lib32-mesa)
            pacman -S "${nouveau_pkgs[@]}" --noconfirm --needed || log_warn "Failed to install Nouveau drivers"
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

    # Grant temporary passwordless sudo — makepkg -si calls sudo pacman internally,
    # which would hang waiting for a password since stdin is not a terminal
    local sudoers_drop="/etc/sudoers.d/temp-aur-build"
    echo "$MAIN_USERNAME ALL=(ALL) NOPASSWD: ALL" > "$sudoers_drop"
    chmod 440 "$sudoers_drop" || { log_error "Failed to set sudoers permissions"; rm -f "$sudoers_drop"; return 1; }
    trap 'rm -f /etc/sudoers.d/temp-aur-build' RETURN

    case "$helper" in
        "paru")
            # Install paru dependencies
            pacman -S base-devel git --noconfirm --needed || { log_error "Failed to install paru dependencies"; return 1; }

            log_cmd "runuser -u $MAIN_USERNAME -- bash (clone+build paru)"
            runuser -u "$MAIN_USERNAME" -- bash << 'AUREOF' || log_warn "Failed to build paru from AUR"
set -e
cd /tmp/aur_build
timeout 60 git clone https://aur.archlinux.org/paru.git
[[ -d paru ]] || { echo "ERROR: paru clone directory not found"; exit 1; }
cd paru
timeout 300 makepkg -si --noconfirm
AUREOF
            ;;
        "yay")
            # Install yay dependencies
            pacman -S base-devel git go --noconfirm --needed || { log_error "Failed to install yay dependencies"; return 1; }

            log_cmd "runuser -u $MAIN_USERNAME -- bash (clone+build yay)"
            runuser -u "$MAIN_USERNAME" -- bash << 'AUREOF' || log_warn "Failed to build yay from AUR"
set -e
cd /tmp/aur_build
timeout 60 git clone https://aur.archlinux.org/yay.git
[[ -d yay ]] || { echo "ERROR: yay clone directory not found"; exit 1; }
cd yay
timeout 300 makepkg -si --noconfirm
AUREOF
            ;;
        "pikaur")
            pacman -S base-devel git python --noconfirm --needed || { log_error "Failed to install pikaur dependencies"; return 1; }

            log_cmd "runuser -u $MAIN_USERNAME -- bash (clone+build pikaur)"
            runuser -u "$MAIN_USERNAME" -- bash << 'AUREOF' || log_warn "Failed to build pikaur from AUR"
set -e
cd /tmp/aur_build
timeout 60 git clone https://aur.archlinux.org/pikaur.git
[[ -d pikaur ]] || { echo "ERROR: pikaur clone directory not found"; exit 1; }
cd pikaur
timeout 300 makepkg -si --noconfirm
AUREOF
            ;;
        *)
            log_warn "Unknown AUR helper: $helper"
            ;;
    esac

    # Cleanup build artifacts
    rm -rf "${build_dir:?}"

    log_success "AUR helper installation complete"
}

install_flatpak() {
    if [[ "${FLATPAK:-No}" != "Yes" ]]; then
        log_info "Flatpak not requested"
        return 0
    fi

    log_info "Installing Flatpak..."

    pacman -S flatpak --noconfirm --needed || {
        log_error "Failed to install flatpak"
        return 1
    }

    # Add Flathub repository for the user (--user flag, no sudo needed)
    runuser -u "$MAIN_USERNAME" -- flatpak remote-add --if-not-exists --user flathub https://flathub.org/repo/flathub.flatpakrepo || log_warn "Failed to add Flathub repository"

    log_success "Flatpak installed"
}

install_additional_packages() {
    # Install additional pacman packages
    if [[ -n "${ADDITIONAL_PACKAGES:-}" ]]; then
        log_info "Installing additional packages: $ADDITIONAL_PACKAGES"

        # Convert space-separated string to array
        local -a packages
        read -ra packages <<< "$ADDITIONAL_PACKAGES"

        # Validate package names (alphanumeric, hyphens, dots, underscores, plus signs, @)
        local -a validated_packages=()
        for pkg in "${packages[@]}"; do
            if [[ "$pkg" =~ ^[a-zA-Z0-9@._+-]+$ ]]; then
                validated_packages+=("$pkg")
            else
                log_warn "Skipping invalid package name: $pkg"
            fi
        done
        packages=("${validated_packages[@]}")

        if [[ ${#packages[@]} -gt 0 ]]; then
            pacman -S "${packages[@]}" --noconfirm --needed || log_warn "Some packages may have failed to install"
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
                # NOPASSWD needed — AUR helpers call sudo pacman internally
                local sudoers_drop="/etc/sudoers.d/temp-aur-packages"
                echo "$MAIN_USERNAME ALL=(ALL) NOPASSWD: ALL" > "$sudoers_drop"
                chmod 440 "$sudoers_drop" || { log_error "Failed to set sudoers permissions"; rm -f "$sudoers_drop"; return 1; }
                trap 'rm -f /etc/sudoers.d/temp-aur-packages' RETURN

                timeout 600 runuser -u "$MAIN_USERNAME" -- "$helper" -S "${aur_packages[@]}" --noconfirm || log_warn "Some AUR packages may have failed to install"
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
    pacman -S plymouth --noconfirm --needed || {
        log_warn "Failed to install plymouth"
        return 0
    }

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

    # Install snapper first (without snap-pac to avoid noisy pacman hooks
    # in chroot where dbus is unavailable)
    pacman -S snapper --noconfirm --needed || {
        log_error "Failed to install snapper"
        return 1
    }

    # Remove the @snapshots mount if it exists (snapper will recreate it)
    if mountpoint -q /.snapshots 2>/dev/null; then
        umount /.snapshots 2>/dev/null || true
    fi

    # Remove the .snapshots directory if it exists
    if [[ -d /.snapshots ]]; then
        rmdir /.snapshots 2>/dev/null || true
    fi

    # Create snapper config for root (--no-dbus: dbus is not running in chroot)
    if snapper --no-dbus -c root create-config /; then
        log_info "Created snapper config for root"
    else
        log_warn "Failed to create snapper config (may already exist)"
    fi

    # Delete the subvolume snapper created and recreate .snapshots directory
    # This is needed because snapper creates a new subvolume, but we want to use @snapshots
    if btrfs subvolume delete /.snapshots 2>/dev/null; then
        log_info "Removed snapper-created .snapshots subvolume"
    fi
    mkdir -p /.snapshots || {
        log_error "Failed to create /.snapshots directory"
        return 1
    }

    # Remount @snapshots subvolume
    if grep -q "@snapshots" /etc/fstab; then
        mount /.snapshots || {
            log_error "Failed to remount @snapshots subvolume"
            return 1
        }
        log_info "Remounted @snapshots subvolume"
    else
        log_warn "No @snapshots entry in fstab — snapshots will not use dedicated subvolume"
    fi

    # Set proper permissions
    chmod 750 /.snapshots || log_warn "Failed to set permissions on /.snapshots"

    # Configure snapper settings
    if [[ -f /etc/snapper/configs/root ]]; then
        # Set timeline settings for automatic snapshots
        sed -i 's/^TIMELINE_CREATE=.*/TIMELINE_CREATE="yes"/' /etc/snapper/configs/root || log_warn "Failed to set TIMELINE_CREATE"
        sed -i 's/^TIMELINE_CLEANUP=.*/TIMELINE_CLEANUP="yes"/' /etc/snapper/configs/root || log_warn "Failed to set TIMELINE_CLEANUP"
        sed -i 's/^TIMELINE_MIN_AGE=.*/TIMELINE_MIN_AGE="1800"/' /etc/snapper/configs/root || log_warn "Failed to set TIMELINE_MIN_AGE"

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

        sed -i "s/^TIMELINE_LIMIT_HOURLY=.*/TIMELINE_LIMIT_HOURLY=\"$hourly_limit\"/" /etc/snapper/configs/root || log_warn "Failed to set TIMELINE_LIMIT_HOURLY"
        sed -i "s/^TIMELINE_LIMIT_DAILY=.*/TIMELINE_LIMIT_DAILY=\"$daily_limit\"/" /etc/snapper/configs/root || log_warn "Failed to set TIMELINE_LIMIT_DAILY"
        sed -i "s/^TIMELINE_LIMIT_WEEKLY=.*/TIMELINE_LIMIT_WEEKLY=\"$weekly_limit\"/" /etc/snapper/configs/root || log_warn "Failed to set TIMELINE_LIMIT_WEEKLY"
        sed -i "s/^TIMELINE_LIMIT_MONTHLY=.*/TIMELINE_LIMIT_MONTHLY=\"$monthly_limit\"/" /etc/snapper/configs/root || log_warn "Failed to set TIMELINE_LIMIT_MONTHLY"
        sed -i 's/^TIMELINE_LIMIT_YEARLY=.*/TIMELINE_LIMIT_YEARLY="0"/' /etc/snapper/configs/root || log_warn "Failed to set TIMELINE_LIMIT_YEARLY"

        log_info "Configured snapper timeline settings (frequency: $frequency, keep: $keep_count)"
    fi

    # Enable snapper timers (before installing snap-pac to avoid hook noise)
    systemctl enable snapper-timeline.timer 2>/dev/null || log_warn "Failed to enable snapper-timeline.timer"
    systemctl enable snapper-cleanup.timer 2>/dev/null || log_warn "Failed to enable snapper-cleanup.timer"

    # Install snap-pac LAST so its pacman hooks don't spam dbus errors during
    # the above pacman calls (hooks fire on every transaction, dbus unavailable in chroot)
    pacman -S snap-pac --noconfirm --needed || log_warn "Failed to install snap-pac"

    # Install grub-btrfs for boot integration if using GRUB
    if [[ "${BOOTLOADER:-grub}" == "grub" ]]; then
        pacman -S grub-btrfs --noconfirm --needed || log_warn "grub-btrfs not available"
        # Enable grub-btrfs path monitoring if installed
        if [[ -f /usr/lib/systemd/system/grub-btrfsd.service ]]; then
            systemctl enable grub-btrfsd.service 2>/dev/null || log_warn "Failed to enable grub-btrfsd.service"
            log_info "Enabled grub-btrfs daemon for boot menu updates"
        fi
    fi

    # Install btrfs-assistant if requested
    if [[ "${BTRFS_ASSISTANT:-No}" == "Yes" ]]; then
        log_info "Installing btrfs-assistant..."
        pacman -S btrfs-assistant --noconfirm --needed || log_warn "Failed to install btrfs-assistant"
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

    # For console (TTY) — always create the systemd service
    mkdir -p /etc/systemd/system
    cat > /etc/systemd/system/numlock.service << 'EOF'
[Unit]
Description=Activate numlock on boot

[Service]
Type=oneshot
ExecStart=/bin/bash -c 'for tty in /dev/tty{1..6}; do /usr/bin/setleds -D +num < "$tty"; done'

[Install]
WantedBy=multi-user.target
EOF
    systemctl enable numlock.service || log_warn "Failed to enable numlock.service"

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

    # Validate URL scheme — only allow https:// for security (no ssh://, git://, file://)
    if [[ "$GIT_REPOSITORY_URL" != https://* ]]; then
        log_warn "Dotfiles URL rejected: only https:// URLs are allowed (got: $GIT_REPOSITORY_URL)"
        return 0
    fi

    log_info "Deploying dotfiles from: $GIT_REPOSITORY_URL"

    local user_home="/home/$MAIN_USERNAME"

    timeout 60 runuser -u "$MAIN_USERNAME" -- git clone "$GIT_REPOSITORY_URL" "$user_home/dotfiles" || {
        log_warn "Failed to clone dotfiles repository"
        return 0
    }

    # Run install script if it exists
    if [[ -x "$user_home/dotfiles/install.sh" ]]; then
        timeout 120 runuser -u "$MAIN_USERNAME" -- bash -c "cd $(printf '%q' "$user_home/dotfiles") && ./install.sh" || log_warn "Dotfiles install script failed or timed out"
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
