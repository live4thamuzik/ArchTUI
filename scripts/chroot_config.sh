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

    configure_localization || log_error "Locale configuration failed"
    configure_hostname || log_error "Hostname configuration failed"
    create_user_account || log_error "User account creation failed"
    configure_sudoers || log_error "Sudoers configuration failed"
    enable_base_services || log_error "Base services configuration failed"

    # --- Phase 2: Bootloader & Initramfs ---
    log_info "=== Phase 2: Bootloader & Initramfs ==="
    echo "PROGRESS: Configuring bootloader"

    # Enable multilib repository BEFORE GPU drivers (needed for lib32 packages)
    if [[ "${MULTILIB:-No}" == "Yes" ]]; then
        log_info "Enabling multilib repository in chroot..."
        sed -i '/^#\[multilib\]/,/^#Include/s/^#//' /etc/pacman.conf || log_warn "Failed to enable multilib in pacman.conf"
        if ! grep -q '^\[multilib\]' /etc/pacman.conf; then
            log_warn "Could not enable multilib in pacman.conf — lib32 packages may not install"
        fi
        log_cmd "pacman -Sy"
        pacman -Sy || log_warn "Failed to sync multilib repository"
        log_success "Multilib repository enabled in chroot"
    fi

    # GPU drivers MUST be installed before mkinitcpio so nvidia/amdgpu/i915
    # kernel modules exist when added to MODULES= and initramfs is regenerated
    install_gpu_drivers || log_warn "GPU driver installation failed — continuing"

    # Plymouth theme MUST be set before mkinitcpio -P so initramfs includes the correct theme
    # (plymouth-set-default-theme without -R sets default; mkinitcpio -P picks it up)
    configure_plymouth || log_warn "Plymouth configuration failed — continuing"

    # Boot-critical functions: failures are logged but must not prevent DE/user setup
    # (a partially-configured system is easier to fix from live USB than a bare one)
    local _boot_ok=true
    configure_mkinitcpio || { log_error "mkinitcpio configuration failed — system may not boot"; _boot_ok=false; }
    install_bootloader || { log_error "Bootloader installation failed — system may not boot"; _boot_ok=false; }
    configure_grub_settings || { log_error "GRUB settings failed — system may not boot"; _boot_ok=false; }
    configure_secure_boot || log_warn "Secure boot configuration skipped"
    if [[ "$_boot_ok" == "false" ]]; then
        log_error "WARNING: Boot configuration had errors — system may not boot correctly"
        log_error "The installer will continue to set up DE, users, and software"
    fi

    # --- Phase 3: Desktop Environment ---
    log_info "=== Phase 3: Desktop Environment ==="
    echo "PROGRESS: Installing desktop environment"

    install_aur_helper || log_warn "AUR helper installation failed — continuing"
    install_desktop_environment || log_warn "Desktop environment installation had issues — continuing"
    install_de_aur_packages || log_warn "DE AUR packages had issues — continuing"
    install_display_manager || log_warn "Display manager installation had issues — continuing"

    # --- Phase 4: Additional Software (non-critical) ---
    log_info "=== Phase 4: Additional Software ==="
    echo "PROGRESS: Installing additional software"

    install_flatpak || log_warn "Flatpak installation failed — continuing"
    install_additional_packages || log_warn "Additional packages had issues — continuing"
    configure_snapshots || log_warn "Snapshot configuration failed — continuing"

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
            log_cmd "ln -sf $tz_path /etc/localtime"
            ln -sf "$tz_path" /etc/localtime || { log_error "Failed to set timezone symlink"; return 1; }
        else
            # Try without region
            tz_path="/usr/share/zoneinfo/${TIMEZONE}"
            tz_real="$(realpath -m "$tz_path" 2>/dev/null)" || tz_real=""
            if [[ -n "$tz_real" && "$tz_real" == /usr/share/zoneinfo/* && -f "$tz_path" ]]; then
                log_cmd "ln -sf $tz_path /etc/localtime"
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

    # RFC 1123: hostnames are case-insensitive; store in lowercase per Linux convention
    local hostname="${SYSTEM_HOSTNAME:-archlinux}"
    hostname="${hostname,,}"
    log_cmd "echo $hostname > /etc/hostname"
    echo "$hostname" > /etc/hostname || { log_error "Failed to write /etc/hostname"; return 1; }

    # Configure hosts file
    log_cmd "cat > /etc/hosts (localhost + $hostname)"
    cat > /etc/hosts << EOF || { log_error "Failed to write /etc/hosts"; return 1; }
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
        unset MAIN_USER_PASSWORD  # clear immediately after use
        if [[ "${LOG_LEVEL:-INFO}" == "VERBOSE" || "${LOG_LEVEL:-INFO}" == "DEBUG" ]]; then set -x; fi
        log_info "User password set"
    fi

    # Set root password (tracing disabled to prevent password leak in verbose logs)
    if [[ -n "${ROOT_PASSWORD:-}" ]]; then
        { set +x; } 2>/dev/null
        log_cmd "printf '***:***' | chpasswd (root password)"
        printf '%s:%s\n' "root" "$ROOT_PASSWORD" | chpasswd || log_warn "Failed to set root password"
        unset ROOT_PASSWORD  # clear immediately after use
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
        log_cmd "sed -i sudoers wheel group uncomment"
        sed -i 's/^# %wheel ALL=(ALL:ALL) ALL/%wheel ALL=(ALL:ALL) ALL/' /etc/sudoers || { log_error "sed failed to modify sudoers"; return 1; }

        # Verify the change was made
        if grep -q "^%wheel ALL=(ALL:ALL) ALL" /etc/sudoers; then
            log_success "Sudoers configured - wheel group enabled"
        else
            # Fallback: add the line directly
            echo "%wheel ALL=(ALL:ALL) ALL" >> /etc/sudoers || { log_error "Failed to append wheel to sudoers"; return 1; }
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
    # https://wiki.archlinux.org/title/Mkinitcpio
    # https://wiki.archlinux.org/title/Dm-crypt/System_configuration
    #
    # Traditional (udev-based) hooks — most common, battle-tested:
    #   Standard:  base udev autodetect microcode modconf kms keyboard keymap consolefont block filesystems fsck
    #   Encrypted: base udev keyboard keymap consolefont autodetect microcode modconf kms block [plymouth] encrypt [lvm2] [resume] filesystems [fsck]
    #   encrypt hook uses cryptdevice=UUID:mapper kernel params
    #
    # FIDO2 requires systemd-based hooks (sd-encrypt + crypttab.initramfs):
    #   Encrypted: base systemd keyboard sd-vconsole autodetect microcode modconf kms block [plymouth] sd-encrypt [lvm2] [resume] filesystems [fsck]
    #   sd-encrypt hook uses rd.luks.name=UUID=mapper kernel params
    #
    # Key requirements:
    # - keyboard/keymap: BEFORE encrypt (so keyboard works for password entry)
    # - plymouth: BEFORE encrypt (for graphical password prompt)
    # - For hardware compatibility: keyboard BEFORE autodetect
    local hooks=""
    local use_systemd_hooks=false
    local is_encrypted=false

    if [[ "${ENCRYPTION:-No}" == "Yes" ]] || [[ "${PARTITIONING_STRATEGY:-}" == *"luks"* ]]; then
        is_encrypted=true
    fi

    # FIDO2 encryption requires systemd hooks (sd-encrypt + crypttab.initramfs)
    if [[ "$is_encrypted" == true ]] && [[ "${ENCRYPTION_KEY_TYPE:-Password}" == *"FIDO2"* ]]; then
        use_systemd_hooks=true
    fi

    if [[ "$use_systemd_hooks" == true ]]; then
        # Systemd-based hooks (required for FIDO2)
        if [[ "$is_encrypted" == true ]]; then
            hooks="base systemd keyboard sd-vconsole autodetect microcode modconf kms block"
            log_info "Using systemd hook order (FIDO2 encryption requires sd-encrypt)"
        else
            hooks="base systemd autodetect microcode modconf kms keyboard sd-vconsole block"
        fi
    else
        # Traditional udev-based hooks (standard Arch setup)
        if [[ "$is_encrypted" == true ]]; then
            hooks="base udev keyboard keymap consolefont autodetect microcode modconf kms block"
            log_info "Using encrypted system hook order (keyboard before autodetect for compatibility)"
        else
            hooks="base udev autodetect microcode modconf kms keyboard keymap consolefont block"
        fi
    fi

    # Add RAID hook if using RAID (must come before encrypt/lvm2)
    if [[ "${PARTITIONING_STRATEGY:-}" == *"raid"* ]]; then
        hooks="$hooks mdadm_udev"
        log_info "Added mdadm_udev hook for RAID"

        # Save mdadm configuration
        if command -v mdadm &>/dev/null; then
            log_cmd "mdadm --detail --scan > /etc/mdadm.conf"
            mdadm --detail --scan > /etc/mdadm.conf 2>/dev/null || log_warn "Failed to write mdadm.conf in chroot"
        fi
    fi

    # Add Plymouth hook BEFORE encrypt (per Arch Wiki: "place plymouth before the encrypt hook")
    # Install plymouth BEFORE adding hook so hook files exist when mkinitcpio -P runs
    if [[ "${PLYMOUTH:-No}" == "Yes" ]]; then
        pacman -S plymouth --noconfirm --needed || log_warn "Failed to install plymouth"
        hooks="$hooks plymouth"
        log_info "Added plymouth hook (before encrypt per Arch Wiki)"
    fi

    # Add encrypt hook if using LUKS (must come before lvm2 for LUKS-on-LVM)
    if [[ "$is_encrypted" == true ]]; then
        if [[ "$use_systemd_hooks" == true ]]; then
            hooks="$hooks sd-encrypt"
            log_info "Added sd-encrypt hook for LUKS (FIDO2 mode)"
        else
            hooks="$hooks encrypt"
            log_info "Added encrypt hook for LUKS"
        fi
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

        # Add RAID kernel modules explicitly (autodetect may miss them if live ISO environment differs)
        if [[ "${PARTITIONING_STRATEGY:-}" == *"raid"* ]]; then
            for rmod in md_mod raid0 raid1 raid456 raid10; do
                if ! grep -q "$rmod" /etc/mkinitcpio.conf; then
                    sed -i "s/^MODULES=(\(.*\))/MODULES=(\1 $rmod)/" /etc/mkinitcpio.conf || log_warn "Failed to add $rmod module"
                    sed -i 's/MODULES=( /MODULES=(/' /etc/mkinitcpio.conf || log_warn "Failed to clean MODULES spacing"
                fi
            done
            log_info "Added RAID modules to mkinitcpio.conf: md_mod raid0 raid1 raid456 raid10"
        fi

        # Add dm-mod for LVM/LUKS strategies (explicit inclusion)
        if [[ "${PARTITIONING_STRATEGY:-}" == *"lvm"* ]] || [[ "${PARTITIONING_STRATEGY:-}" == *"luks"* ]]; then
            if ! grep -q "dm_mod" /etc/mkinitcpio.conf; then
                sed -i 's/^MODULES=(\(.*\))/MODULES=(\1 dm_mod)/' /etc/mkinitcpio.conf || log_warn "Failed to add dm_mod module"
                sed -i 's/MODULES=( /MODULES=(/' /etc/mkinitcpio.conf || log_warn "Failed to clean MODULES spacing"
                log_info "Added dm_mod module to mkinitcpio.conf"
            fi
        fi

        # Add GPU modules for early KMS (required for Plymouth + proprietary drivers)
        # Check GPU_DRIVERS env var first, fall back to hardware detection
        local _gpu_drv="${GPU_DRIVERS:-Auto}"
        _gpu_drv="${_gpu_drv,,}"  # lowercase

        if [[ "$_gpu_drv" == "nvidia" || "$_gpu_drv" == "nvidia-open" ]] \
           || { [[ "$_gpu_drv" == "auto" ]] && lspci 2>/dev/null | grep -qi nvidia; }; then
            # NVIDIA early KMS: required for Plymouth, DRM modeset, Wayland compositors
            # https://wiki.archlinux.org/title/NVIDIA#DRM_kernel_mode_setting
            local nvidia_mods="nvidia nvidia_modeset nvidia_uvm nvidia_drm"
            for nmod in $nvidia_mods; do
                if ! grep -q "$nmod" /etc/mkinitcpio.conf; then
                    sed -i "s/^MODULES=(\(.*\))/MODULES=(\1 $nmod)/" /etc/mkinitcpio.conf || log_warn "Failed to add $nmod module"
                    sed -i 's/MODULES=( /MODULES=(/' /etc/mkinitcpio.conf || log_warn "Failed to clean MODULES spacing"
                fi
            done
            log_info "Added NVIDIA modules for early KMS: $nvidia_mods"
        fi

        if [[ "$_gpu_drv" == "amd" ]] \
           || { [[ "$_gpu_drv" == "auto" ]] && lspci 2>/dev/null | grep -qi "amd.*radeon\|radeon.*amd\|amd.*graphics"; }; then
            if ! grep -q "amdgpu" /etc/mkinitcpio.conf; then
                sed -i 's/^MODULES=(\(.*\))/MODULES=(\1 amdgpu)/' /etc/mkinitcpio.conf || log_warn "Failed to add amdgpu module"
                sed -i 's/MODULES=( /MODULES=(/' /etc/mkinitcpio.conf || log_warn "Failed to clean MODULES spacing"
                log_info "Added amdgpu module for early KMS"
            fi
        fi

        if [[ "$_gpu_drv" == "intel" ]] \
           || { [[ "$_gpu_drv" == "auto" ]] && lspci 2>/dev/null | grep -qi "intel.*graphics\|intel.*uhd\|intel.*iris"; }; then
            if ! grep -q "i915" /etc/mkinitcpio.conf; then
                sed -i 's/^MODULES=(\(.*\))/MODULES=(\1 i915)/' /etc/mkinitcpio.conf || log_warn "Failed to add i915 module"
                sed -i 's/MODULES=( /MODULES=(/' /etc/mkinitcpio.conf || log_warn "Failed to clean MODULES spacing"
                log_info "Added i915 module for early KMS"
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

# Build encryption kernel parameters based on hook type
# FIDO2 uses systemd hooks (rd.luks.name=), standard LUKS uses traditional (cryptdevice=)
_build_luks_kernel_params() {
    local luks_uuid="$1"
    local mapper_name="$2"

    if [[ "${ENCRYPTION_KEY_TYPE:-Password}" == *"FIDO2"* ]]; then
        # systemd sd-encrypt: rd.luks.name=UUID=mapper
        echo "rd.luks.name=${luks_uuid}=${mapper_name}"
    else
        # traditional encrypt: cryptdevice=UUID:mapper
        echo "cryptdevice=UUID=${luks_uuid}:${mapper_name}"
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
        log_cmd "grub-install --target=x86_64-efi --efi-directory=$efi_dir --bootloader-id=GRUB --modules=tpm --disable-shim-lock --recheck"
        grub-install --target=x86_64-efi --efi-directory="$efi_dir" --bootloader-id=GRUB --modules="tpm" --disable-shim-lock --recheck || {
            log_error "GRUB installation failed"
            return 1
        }

        # Also install to EFI fallback path (ESP/EFI/BOOT/BOOTX64.EFI)
        # This protects against Windows Update resetting UEFI boot order
        # (per Arch wiki: --removable installs to the fallback path)
        log_info "Installing GRUB to EFI fallback path for boot resilience"
        log_cmd "grub-install --target=x86_64-efi --efi-directory=$efi_dir --removable --modules=tpm --disable-shim-lock --recheck"
        grub-install --target=x86_64-efi --efi-directory="$efi_dir" --removable --modules="tpm" --disable-shim-lock --recheck || {
            log_warn "GRUB fallback installation failed (non-fatal)"
        }
    else
        # BIOS installation
        # For RAID: install GRUB on ALL member disks for redundancy
        # (Arch wiki GRUB#RAID: "run grub-install on both drives")
        if [[ "${PARTITIONING_STRATEGY:-}" == *"raid"* ]]; then
            IFS=',' read -ra _grub_disks <<< "${INSTALL_DISK:?INSTALL_DISK not set}"
            for _disk in "${_grub_disks[@]}"; do
                _disk=$(echo "$_disk" | tr -d '[:space:]')
                if [[ -b "$_disk" ]]; then
                    log_info "Installing GRUB for BIOS to RAID member: $_disk"
                    log_cmd "grub-install --target=i386-pc $_disk --recheck"
                    grub-install --target=i386-pc "$_disk" --recheck || {
                        log_error "GRUB installation failed on $_disk"
                        return 1
                    }
                else
                    log_warn "RAID member $_disk is not a block device — skipping GRUB install"
                fi
            done
        else
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
            log_error "BUG: systemd-boot reached install with incompatible layout (ESP at /efi, ext4 /boot)"
            log_error "This should have been prevented by partitioning — falling back to GRUB"
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

    # Handle encryption
    if [[ "${ENCRYPTION:-No}" == "Yes" ]] || [[ "${PARTITIONING_STRATEGY:-}" == *"luks"* ]]; then
        if [[ -n "${LUKS_UUID:-}" ]]; then
            local mapper_name="cryptroot"
            [[ "${PARTITIONING_STRATEGY:-}" == *"lvm"* ]] && mapper_name="cryptlvm"
            local luks_param
            luks_param=$(_build_luks_kernel_params "$LUKS_UUID" "$mapper_name")
            if [[ "${PARTITIONING_STRATEGY:-}" == *"lvm"* ]]; then
                options="${luks_param} root=/dev/archvg/root"
            else
                options="${luks_param} root=/dev/mapper/${mapper_name}"
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

    # RAID0 layout parameter (kernel 5.3.4+, Arch wiki RAID)
    if [[ "${RAID_LEVEL:-}" == "raid0" ]]; then
        options="$options raid0.default_layout=2"
    fi

    options="$options quiet"

    # Plymouth splash
    if [[ "${PLYMOUTH:-No}" == "Yes" ]]; then
        options="$options splash"
    fi

    # NVIDIA DRM modeset
    local _gpu_sdb="${GPU_DRIVERS:-Auto}"
    _gpu_sdb="${_gpu_sdb,,}"
    if [[ "$_gpu_sdb" == "nvidia" || "$_gpu_sdb" == "nvidia-open" ]] \
       || { [[ "$_gpu_sdb" == "auto" ]] && lspci 2>/dev/null | grep -qi nvidia; }; then
        options="$options nvidia-drm.modeset=1"
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
            local luks_param
            luks_param=$(_build_luks_kernel_params "$LUKS_UUID" "$mapper_name")
            if [[ "${PARTITIONING_STRATEGY:-}" == *"lvm"* ]]; then
                options="${luks_param} root=/dev/archvg/root"
            else
                options="${luks_param} root=/dev/mapper/${mapper_name}"
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

    # RAID0 layout parameter (kernel 5.3.4+, Arch wiki RAID)
    if [[ "${RAID_LEVEL:-}" == "raid0" ]]; then
        options="$options raid0.default_layout=2"
    fi

    options="$options quiet"

    # Plymouth splash
    if [[ "${PLYMOUTH:-No}" == "Yes" ]]; then
        options="$options splash"
    fi

    # NVIDIA DRM modeset
    local _gpu_ko="${GPU_DRIVERS:-Auto}"
    _gpu_ko="${_gpu_ko,,}"
    if [[ "$_gpu_ko" == "nvidia" || "$_gpu_ko" == "nvidia-open" ]] \
       || { [[ "$_gpu_ko" == "auto" ]] && lspci 2>/dev/null | grep -qi nvidia; }; then
        options="$options nvidia-drm.modeset=1"
    fi

    echo "$options"
}

install_refind() {
    log_info "Installing rEFInd..."

    if [[ "${BOOT_MODE:-UEFI}" != "UEFI" ]]; then
        log_error "rEFInd requires UEFI firmware"
        return 1
    fi

    # Determine ESP path for refind-install
    local esp_path="/efi"
    [[ ! -d "$esp_path" ]] && esp_path="/boot/efi"
    [[ ! -d "$esp_path" ]] && esp_path="/boot"

    log_cmd "refind-install --esp-path=$esp_path"
    refind-install --esp-path="$esp_path" || {
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
        microcode_module="module_path: boot:///intel-ucode.img"
    elif [[ -f /boot/amd-ucode.img ]]; then
        microcode_module="module_path: boot:///amd-ucode.img"
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
        if [[ "$PARTITIONING_STRATEGY" == *raid* ]]; then
            # RAID: install to all member disks for boot redundancy
            IFS=',' read -ra _limine_disks <<< "$INSTALL_DISK"
            for _ldisk in "${_limine_disks[@]}"; do
                _ldisk="${_ldisk// /}"
                log_cmd "limine bios-install $_ldisk"
                limine bios-install "$_ldisk" || {
                    log_error "Limine BIOS install failed on $_ldisk"
                    return 1
                }
            done
        else
            local target_disk="${INSTALL_DISK%%,*}"
            log_cmd "limine bios-install $target_disk"
            limine bios-install "$target_disk" || {
                log_error "Limine BIOS install failed on $target_disk"
                return 1
            }
        fi
    fi

    # Generate limine.conf
    {
        echo "timeout: 5"
        echo ""
        echo "/Arch Linux"
        echo "    protocol: linux"
        echo "    kernel_path: boot:///vmlinuz-${KERNEL:-linux}"
        [[ -n "$microcode_module" ]] && echo "    $microcode_module"
        echo "    module_path: boot:///initramfs-${KERNEL:-linux}.img"
        echo "    cmdline: $options"
        echo ""
        echo "/Arch Linux (fallback)"
        echo "    protocol: linux"
        echo "    kernel_path: boot:///vmlinuz-${KERNEL:-linux}"
        [[ -n "$microcode_module" ]] && echo "    $microcode_module"
        echo "    module_path: boot:///initramfs-${KERNEL:-linux}-fallback.img"
        echo "    cmdline: $options"
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

    # Add encryption parameters if needed
    if [[ "${ENCRYPTION:-No}" == "Yes" ]] || [[ "${PARTITIONING_STRATEGY:-}" == *"luks"* ]]; then
        if [[ -n "${LUKS_UUID:-}" ]]; then
            # Determine mapper name based on strategy
            local mapper_name="cryptroot"
            if [[ "${PARTITIONING_STRATEGY:-}" == *"lvm"* ]]; then
                mapper_name="cryptlvm"
            fi
            local luks_param
            luks_param=$(_build_luks_kernel_params "$LUKS_UUID" "$mapper_name")
            cmdline="$cmdline ${luks_param}"
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
    else
        # Non-encrypted: explicit root= parameter
        # For LVM: use /dev/archvg/root device path
        # For plain/RAID: use ROOT_UUID
        if [[ "${PARTITIONING_STRATEGY:-}" == *"lvm"* ]]; then
            cmdline="$cmdline root=/dev/archvg/root"
        else
            local grub_root_uuid="${ROOT_UUID:-}"
            if [[ -z "$grub_root_uuid" ]]; then
                grub_root_uuid=$(findmnt -n -o UUID /) || true
            fi
            if [[ -n "$grub_root_uuid" ]]; then
                cmdline="$cmdline root=UUID=${grub_root_uuid}"
            else
                log_error "Cannot determine root UUID for GRUB — system may not boot"
                return 1
            fi
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

    # NVIDIA DRM modeset (required for Wayland compositors, Plymouth, early KMS)
    local _gpu_grub="${GPU_DRIVERS:-Auto}"
    _gpu_grub="${_gpu_grub,,}"
    if [[ "$_gpu_grub" == "nvidia" || "$_gpu_grub" == "nvidia-open" ]] \
       || { [[ "$_gpu_grub" == "auto" ]] && lspci 2>/dev/null | grep -qi nvidia; }; then
        cmdline="$cmdline nvidia-drm.modeset=1"
        log_info "Added nvidia-drm.modeset=1 for NVIDIA DRM KMS"
    fi

    # RAID0 layout parameter (kernel 5.3.4+, Arch wiki RAID)
    if [[ "${RAID_LEVEL:-}" == "raid0" ]]; then
        cmdline="$cmdline raid0.default_layout=2"
        log_info "Added raid0.default_layout=2 for RAID0 (kernel 5.3.4+)"
    fi

    # Update GRUB_CMDLINE_LINUX_DEFAULT
    sed -i "s|^GRUB_CMDLINE_LINUX_DEFAULT=.*|GRUB_CMDLINE_LINUX_DEFAULT=\"$cmdline\"|" "$grub_default" || log_warn "Failed to update GRUB_CMDLINE_LINUX_DEFAULT"

    # Preload GRUB modules for RAID (ensures GRUB can read RAID boot partitions)
    if [[ "${PARTITIONING_STRATEGY:-}" == *"raid"* ]]; then
        if ! grep -q "^GRUB_PRELOAD_MODULES=" "$grub_default"; then
            echo 'GRUB_PRELOAD_MODULES="mdraid09 mdraid1x part_gpt"' >> "$grub_default"
            log_info "Added GRUB_PRELOAD_MODULES for RAID"
        fi
    fi

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

if [ "\${grub_platform}" == "efi" ]; then
    menuentry "Windows Boot Manager" {
        insmod part_gpt
        insmod fat
        insmod chain
        search --no-floppy --fs-uuid --set=root ${efi_part_uuid}
        chainloader /EFI/Microsoft/Boot/bootmgfw.efi
    }
fi
WINEOF
                log_info "Added Windows chainload entry with EFI UUID: $efi_part_uuid"
            else
                log_warn "Could not detect EFI partition UUID — skipping Windows chainload entry"
                log_warn "os-prober should still detect Windows automatically"
            fi
        fi
    fi

    # Enable GRUB crypto support unconditionally for any LUKS layout.
    # Required for os-prober to detect other OSes on encrypted systems,
    # even when /boot is on a separate unencrypted partition.
    if [[ "${ENCRYPTION:-No}" == "Yes" ]] || [[ "${PARTITIONING_STRATEGY:-}" == *"luks"* ]]; then
        if ! grep -q "^GRUB_ENABLE_CRYPTODISK=y" "$grub_default"; then
            echo "GRUB_ENABLE_CRYPTODISK=y" >> "$grub_default"
            log_info "Enabled GRUB_ENABLE_CRYPTODISK for LUKS layout"
        fi
    fi

    # Disable GRUB's internal shim_lock verifier unconditionally.
    # ArchTUI uses sbctl for Secure Boot (not shim), so the verifier always
    # fails. Some firmware (VirtualBox, certain UEFI) reports Secure Boot
    # active even when unmanaged, triggering the verifier and dropping to
    # grub rescue. Safe to disable — only needed with shim-signed.
    # https://wiki.archlinux.org/title/GRUB#Secure_Boot
    if ! grep -q "^GRUB_DISABLE_SHIM_LOCK=y" "$grub_default"; then
        echo "GRUB_DISABLE_SHIM_LOCK=y" >> "$grub_default" || {
            log_error "Failed to write GRUB_DISABLE_SHIM_LOCK to $grub_default"
            return 1
        }
        log_info "Disabled GRUB shim_lock verifier"
    fi

    # Configure GRUB theme if requested (cosmetic — must not block grub-mkconfig)
    configure_grub_theme || log_warn "GRUB theme installation failed — continuing without theme"

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
    local tmp_dir
    tmp_dir=$(mktemp -d "/tmp/grub-theme.XXXXXX") || { log_error "Failed to create GRUB theme temp directory"; return 1; }

    if ! command -v git &>/dev/null; then
        log_warn "git not installed, cannot clone GRUB theme"
        pacman -S git --noconfirm --needed || {
            log_error "Failed to install git for GRUB theme"
            return 1
        }
    fi

    # Ensure DNS works inside chroot — copy host resolv.conf if missing/empty
    if [[ ! -s /etc/resolv.conf ]]; then
        log_warn "/etc/resolv.conf missing or empty in chroot — DNS may fail"
    fi

    log_cmd "git clone --depth 1 $repo_url"
    local clone_ok=false
    local clone_err
    # Try 3 times — older network cards may need retries for TLS/DNS
    # Git timeout config: abort if transfer drops below 1000 bytes/sec for 10s
    for attempt in 1 2 3; do
        clone_err=$(timeout 60 git \
            -c http.lowSpeedLimit=1000 \
            -c http.lowSpeedTime=10 \
            clone --depth 1 "$repo_url" "$tmp_dir/$clone_dir" 2>&1) && { clone_ok=true; break; }
        log_warn "GRUB theme clone attempt $attempt failed: $clone_err"
        rm -rf "${tmp_dir:?}/$clone_dir"
        sleep 3
    done

    if [[ "$clone_ok" == "true" ]]; then
        mkdir -p "/boot/grub/themes/${theme_name}"
        # Look for theme.txt — prefer "arch" subdirectory if it exists (multi-distro repos)
        local theme_txt=""
        local theme_root=""
        if [[ -f "$tmp_dir/$clone_dir/arch/theme.txt" ]]; then
            theme_txt="$tmp_dir/$clone_dir/arch/theme.txt"
        else
            theme_txt="$(find "$tmp_dir/$clone_dir" -name "theme.txt" -print -quit 2>/dev/null)"
        fi
        if [[ -n "$theme_txt" ]]; then
            theme_root="$(dirname "$theme_txt")"
            cp -r "$theme_root/"* "/boot/grub/themes/${theme_name}/"
            log_success "GRUB theme installed: $theme_name (from $(basename "$theme_root"))"
        else
            log_warn "theme.txt not found in cloned repo: $repo_url"
        fi
    else
        log_error "Failed to clone GRUB theme after 3 attempts: $repo_url"
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
                # HyperFluent is bundled in Source/ — copy arch/ variant to theme_dir
                local _hf_src="/root/grub-themes/HyperFluent-GRUB-Theme"
                if [[ -d "$_hf_src/arch" ]]; then
                    mkdir -p "$theme_dir"
                    cp -r "$_hf_src/arch/"* "$theme_dir/" || log_warn "Failed to copy HyperFluent theme"
                    log_info "Installed HyperFluent theme from bundled source"
                else
                    _grub_theme_git_clone "https://github.com/Coopydood/HyperFluent-GRUB-Theme.git" "HyperFluent-GRUB-Theme" "$theme_name"
                fi
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
            sed -i "s|^GRUB_THEME=.*|GRUB_THEME=\"${theme_dir}/theme.txt\"|" "$grub_default" || log_warn "Failed to update GRUB_THEME in $grub_default"
        else
            echo "GRUB_THEME=\"${theme_dir}/theme.txt\"" >> "$grub_default" || log_warn "Failed to append GRUB_THEME to $grub_default"
        fi

        # Also set gfxmode for better theme rendering
        if ! grep -q "^GRUB_GFXMODE=" "$grub_default"; then
            echo "GRUB_GFXMODE=auto" >> "$grub_default" || log_warn "Failed to set GRUB_GFXMODE"
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
        log_cmd "sbctl create-keys"
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
    local _sign_err
    if [[ -f "/boot/vmlinuz-${kernel}" ]]; then
        log_cmd "sbctl sign -s /boot/vmlinuz-${kernel}"
        if ! _sign_err=$(sbctl sign -s "/boot/vmlinuz-${kernel}" 2>&1); then
            log_warn "Failed to sign vmlinuz-${kernel}: $_sign_err"
        fi
    fi

    # Sign bootloader EFI binaries based on type
    log_info "Signing bootloader EFI binaries for: ${BOOTLOADER:-grub}"
    case "${BOOTLOADER:-grub}" in
        "grub")
            for grub_efi in /efi/EFI/GRUB/grubx64.efi /boot/EFI/GRUB/grubx64.efi; do
                if [[ -f "$grub_efi" ]]; then
                    log_cmd "sbctl sign -s $grub_efi"
                    if ! _sign_err=$(sbctl sign -s "$grub_efi" 2>&1); then
                        log_warn "Failed to sign GRUB: $_sign_err"
                    fi
                fi
            done
            ;;
        "systemd-boot")
            for sd_efi in /efi/EFI/systemd/systemd-bootx64.efi /boot/EFI/systemd/systemd-bootx64.efi; do
                if [[ -f "$sd_efi" ]]; then
                    log_cmd "sbctl sign -s $sd_efi"
                    if ! _sign_err=$(sbctl sign -s "$sd_efi" 2>&1); then
                        log_warn "Failed to sign systemd-boot: $_sign_err"
                    fi
                fi
            done
            ;;
        "refind")
            for refind_efi in /efi/EFI/refind/refind_x64.efi /boot/EFI/refind/refind_x64.efi; do
                if [[ -f "$refind_efi" ]]; then
                    log_cmd "sbctl sign -s $refind_efi"
                    if ! _sign_err=$(sbctl sign -s "$refind_efi" 2>&1); then
                        log_warn "Failed to sign rEFInd: $_sign_err"
                    fi
                fi
            done
            ;;
        "limine")
            for limine_efi in /efi/EFI/BOOT/BOOTX64.EFI /boot/EFI/BOOT/BOOTX64.EFI; do
                if [[ -f "$limine_efi" ]]; then
                    log_cmd "sbctl sign -s $limine_efi"
                    if ! _sign_err=$(sbctl sign -s "$limine_efi" 2>&1); then
                        log_warn "Failed to sign Limine: $_sign_err"
                    fi
                fi
            done
            ;;
        "efistub")
            # EFISTUB uses the kernel directly as EFI binary (already signed above)
            log_info "EFISTUB uses kernel directly — no separate bootloader to sign"
            ;;
    esac

    # Sign EFI fallback bootloader (common to all bootloaders)
    for fallback_efi in /efi/EFI/BOOT/BOOTX64.EFI /boot/EFI/BOOT/BOOTX64.EFI; do
        if [[ -f "$fallback_efi" ]]; then
            log_cmd "sbctl sign -s $fallback_efi"
            if ! _sign_err=$(sbctl sign -s "$fallback_efi" 2>&1); then
                log_warn "Failed to sign EFI fallback: $_sign_err"
            fi
            break
        fi
    done

    # Sign Linux EFI stubs (UKI / systemd-boot auto-entries)
    for linux_efi_dir in /boot/EFI/Linux /efi/EFI/Linux; do
        if [[ -d "$linux_efi_dir" ]]; then
            for efi in "$linux_efi_dir"/*.efi; do
                if [[ -f "$efi" ]]; then
                    if ! _sign_err=$(sbctl sign -s "$efi" 2>&1); then
                        log_warn "Failed to sign $efi: $_sign_err"
                    fi
                fi
            done
        fi
    done

    # Verify all registered files are properly signed
    log_info "Verifying Secure Boot signatures..."
    if ! sbctl verify; then
        log_warn "Some EFI binaries may not be properly signed — check sbctl verify output"
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
    echo "Secure Boot must be in 'Setup Mode' before keys can be enrolled."
    echo "This means clearing the existing Secure Boot keys so the firmware"
    echo "accepts new ones. Your firmware might call this different things:"
    echo ""
    echo "  - 'Setup Mode'                       (most common)"
    echo "  - 'Clear Secure Boot Keys' / 'Reset Keys'"
    echo "  - 'Custom Mode'                      (some ASUS/MSI boards)"
    echo "  - 'Key Management' → 'Clear All Keys'"
    echo ""
    echo "Steps:"
    echo "1. Reboot and enter UEFI/BIOS setup (usually F2, Del, or Esc at POST)"
    echo "2. Navigate to Secure Boot settings (often under Security or Boot)"
    echo "3. Clear/reset existing Secure Boot keys to enter Setup Mode"
    echo "4. Save and exit — boot back into Arch Linux"
    echo "5. Run this script again: /root/enroll-secure-boot-keys.sh"
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
Target = grub
Target = systemd
Target = refind
Target = limine
Target = fwupd

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
            install_packages "KDE Plasma" plasma-meta kde-applications-meta konsole dolphin \
                networkmanager pipewire pipewire-pulse wireplumber firefox ark
            ;;
        "gnome")
            install_packages "GNOME" gnome gnome-tweaks gnome-terminal \
                networkmanager pipewire pipewire-pulse wireplumber firefox file-roller
            ;;
        "xfce")
            install_packages "XFCE" xfce4 xfce4-goodies xorg-server xorg-xinit \
                networkmanager network-manager-applet pipewire pipewire-pulse wireplumber pavucontrol \
                firefox thunar-archive-plugin
            ;;
        "i3"|"i3wm")
            install_packages "i3 Window Manager" i3-wm i3status i3lock dmenu rofi alacritty \
                picom dunst maim xdotool thunar xorg-server xorg-xinit feh \
                networkmanager network-manager-applet pipewire pipewire-pulse wireplumber pavucontrol \
                ttf-dejavu noto-fonts firefox
            ;;
        "hyprland")
            install_packages "Hyprland" hyprland xdg-desktop-portal-hyprland xdg-desktop-portal-gtk \
                polkit hyprpolkitagent hyprlock hypridle hyprpaper waybar \
                rofi grim slurp kitty wl-clipboard cliphist thunar mako brightnessctl blueman \
                networkmanager network-manager-applet pipewire pipewire-pulse wireplumber pavucontrol \
                ttf-jetbrains-mono-nerd noto-fonts noto-fonts-emoji firefox
            ;;
        "sway")
            install_packages "Sway" sway swaylock swayidle waybar \
                rofi grim slurp foot xdg-desktop-portal-wlr \
                wl-clipboard mako thunar \
                networkmanager network-manager-applet pipewire pipewire-pulse wireplumber pavucontrol \
                ttf-dejavu noto-fonts firefox
            ;;
        "cinnamon")
            install_packages "Cinnamon" cinnamon nemo-fileroller gnome-terminal gnome-screenshot \
                xorg-server xorg-xinit \
                networkmanager network-manager-applet pipewire pipewire-pulse wireplumber pavucontrol firefox
            ;;
        "mate")
            install_packages "MATE" mate mate-extra xorg-server xorg-xinit \
                networkmanager network-manager-applet pipewire pipewire-pulse wireplumber pavucontrol firefox
            ;;
        "budgie")
            install_packages "Budgie" budgie-desktop budgie-extras gnome-terminal nautilus gnome-screenshot \
                xorg-server xorg-xinit \
                networkmanager network-manager-applet pipewire pipewire-pulse wireplumber pavucontrol firefox
            ;;
        "cosmic")
            install_packages "COSMIC" cosmic-session \
                cosmic-terminal cosmic-files cosmic-text-editor cosmic-store cosmic-settings \
                cosmic-screenshot cosmic-player cosmic-icon-theme cosmic-wallpapers \
                cosmic-app-library cosmic-initial-setup xdg-desktop-portal-cosmic \
                networkmanager pipewire pipewire-pulse wireplumber pavucontrol firefox
            ;;
        "deepin")
            install_packages "Deepin" deepin deepin-extra xorg-server xorg-xinit \
                networkmanager pipewire pipewire-pulse wireplumber pavucontrol firefox
            ;;
        "lxde")
            install_packages "LXDE" lxde xorg-server xorg-xinit \
                networkmanager pipewire pipewire-pulse wireplumber pavucontrol firefox
            ;;
        "lxqt")
            install_packages "LXQt" lxqt breeze-icons \
                networkmanager pipewire pipewire-pulse wireplumber pavucontrol firefox
            ;;
        "bspwm")
            install_packages "bspwm" bspwm sxhkd xorg-server xorg-xinit alacritty dmenu picom \
                dunst maim xdotool feh thunar \
                networkmanager network-manager-applet pipewire pipewire-pulse wireplumber pavucontrol \
                ttf-dejavu noto-fonts firefox
            ;;
        "awesome")
            install_packages "Awesome WM" awesome xorg-server xorg-xinit alacritty dmenu picom \
                maim xdotool thunar feh \
                networkmanager network-manager-applet pipewire pipewire-pulse wireplumber pavucontrol \
                ttf-dejavu noto-fonts firefox
            ;;
        "qtile")
            install_packages "Qtile" qtile python-psutil xorg-server xorg-xinit alacritty dmenu picom \
                dunst maim xdotool thunar feh \
                networkmanager network-manager-applet pipewire pipewire-pulse wireplumber pavucontrol \
                ttf-dejavu noto-fonts firefox
            ;;
        "river")
            install_packages "River" river xdg-desktop-portal-wlr swaylock swayidle waybar foot rofi mako grim slurp wl-clipboard thunar \
                networkmanager network-manager-applet pipewire pipewire-pulse wireplumber pavucontrol \
                ttf-dejavu noto-fonts firefox
            ;;
        "niri")
            install_packages "Niri" niri xdg-desktop-portal-gnome swaylock swayidle waybar foot fuzzel mako grim slurp wl-clipboard nautilus \
                networkmanager network-manager-applet pipewire pipewire-pulse wireplumber pavucontrol \
                ttf-dejavu noto-fonts firefox
            ;;
        "labwc")
            install_packages "Labwc" labwc xdg-desktop-portal-wlr swaylock swayidle waybar foot rofi mako grim slurp wl-clipboard thunar \
                networkmanager network-manager-applet pipewire pipewire-pulse wireplumber pavucontrol \
                ttf-dejavu noto-fonts firefox
            ;;
        "xmonad")
            install_packages "XMonad" xmonad xmonad-contrib xmobar xorg-server xorg-xinit dmenu alacritty picom \
                dunst maim xdotool thunar feh \
                networkmanager network-manager-applet pipewire pipewire-pulse wireplumber pavucontrol \
                ttf-dejavu noto-fonts firefox
            ;;
        "dwm")
            install_packages "DWM" dwm xorg-server xorg-xinit dmenu alacritty picom \
                dunst maim xdotool thunar feh \
                networkmanager network-manager-applet pipewire pipewire-pulse wireplumber pavucontrol \
                ttf-dejavu noto-fonts firefox
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
            # COSMIC is in official Arch repos since 2025 — no AUR packages needed
            return 0
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
        "cosmic-greeter")
            install_packages "COSMIC Greeter" cosmic-greeter
            log_cmd "systemctl enable cosmic-greeter.service"
            systemctl enable cosmic-greeter.service || log_warn "Failed to enable cosmic-greeter.service"
            ;;
        "greetd")
            install_packages "greetd" greetd greetd-tuigreet
            log_cmd "systemctl enable greetd.service"
            systemctl enable greetd.service || log_warn "Failed to enable greetd.service"
            # Configure tuigreet as default greeter
            log_cmd "mkdir -p /etc/greetd"
            mkdir -p /etc/greetd || log_warn "Failed to create /etc/greetd"
            # Determine session command based on desktop environment
            local greetd_session="/bin/bash"
            local de_lower="${DESKTOP_ENVIRONMENT:-none}"
            de_lower="${de_lower,,}"
            case "$de_lower" in
                "sway")       greetd_session="sway" ;;
                "hyprland")   greetd_session="Hyprland" ;;
                "river")      greetd_session="river" ;;
                "niri")       greetd_session="niri-session" ;;
                "labwc")      greetd_session="labwc" ;;
                "i3"|"i3wm")  greetd_session="i3" ;;
                "awesome")    greetd_session="awesome" ;;
                "qtile")      greetd_session="qtile start" ;;
                "bspwm")      greetd_session="bspwm" ;;
                "xmonad")     greetd_session="xmonad" ;;
                "dwm")        greetd_session="dwm" ;;
                "gnome")      greetd_session="gnome-session" ;;
                "kde"|"plasma") greetd_session="startplasma-wayland" ;;
                "xfce")       greetd_session="startxfce4" ;;
                "cinnamon")   greetd_session="cinnamon-session" ;;
                "mate")       greetd_session="mate-session" ;;
                "budgie")     greetd_session="budgie-desktop" ;;
                "deepin")     greetd_session="startdde" ;;
                "lxde")       greetd_session="startlxde" ;;
                "lxqt")       greetd_session="startlxqt" ;;
                *)            greetd_session="/bin/bash" ;;
            esac
            log_cmd "Writing /etc/greetd/config.toml (session: $greetd_session)"
            cat > /etc/greetd/config.toml << GREETD_EOF
[terminal]
vt = 1

[default_session]
command = "tuigreet --time --cmd $greetd_session"
user = "greeter"
GREETD_EOF
            log_success "greetd configured with tuigreet greeter (session: $greetd_session)"
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
                local nvidia_auto_pkgs=(nvidia-dkms libglvnd nvidia-utils opencl-nvidia nvidia-settings)
                [[ "$use_lib32" == "yes" ]] && nvidia_auto_pkgs+=(lib32-libglvnd lib32-nvidia-utils lib32-opencl-nvidia)
                pacman -S "${nvidia_auto_pkgs[@]}" --noconfirm --needed || log_warn "Failed to install NVIDIA drivers"
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
            local nvidia_pkgs=(nvidia-dkms libglvnd nvidia-utils opencl-nvidia nvidia-settings)
            [[ "$use_lib32" == "yes" ]] && nvidia_pkgs+=(lib32-libglvnd lib32-nvidia-utils lib32-opencl-nvidia)
            pacman -S "${nvidia_pkgs[@]}" --noconfirm --needed || log_warn "Failed to install NVIDIA drivers"
            ;;
        "nvidia-open")
            local nvidia_open_pkgs=(nvidia-open-dkms libglvnd nvidia-utils opencl-nvidia nvidia-settings)
            [[ "$use_lib32" == "yes" ]] && nvidia_open_pkgs+=(lib32-libglvnd lib32-nvidia-utils lib32-opencl-nvidia)
            pacman -S "${nvidia_open_pkgs[@]}" --noconfirm --needed || log_warn "Failed to install NVIDIA-open drivers"
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

    # AUR helpers must be built as non-root user (unique temp dir per invocation)
    local build_dir
    build_dir=$(mktemp -d "/tmp/aur_build.XXXXXX") || { log_error "Failed to create AUR build directory"; return 1; }
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
            AUR_BUILD_DIR="$build_dir" runuser -u "$MAIN_USERNAME" -- bash << 'AUREOF' || log_warn "Failed to build paru from AUR"
set -e
cd "$AUR_BUILD_DIR"
timeout 60 git -c http.lowSpeedLimit=1000 -c http.lowSpeedTime=10 clone https://aur.archlinux.org/paru.git
[[ -d paru ]] || { echo "ERROR: paru clone directory not found"; exit 1; }
cd paru
timeout 300 makepkg -si --noconfirm
AUREOF
            ;;
        "yay")
            # Install yay dependencies
            pacman -S base-devel git go --noconfirm --needed || { log_error "Failed to install yay dependencies"; return 1; }

            log_cmd "runuser -u $MAIN_USERNAME -- bash (clone+build yay)"
            AUR_BUILD_DIR="$build_dir" runuser -u "$MAIN_USERNAME" -- bash << 'AUREOF' || log_warn "Failed to build yay from AUR"
set -e
cd "$AUR_BUILD_DIR"
timeout 60 git -c http.lowSpeedLimit=1000 -c http.lowSpeedTime=10 clone https://aur.archlinux.org/yay.git
[[ -d yay ]] || { echo "ERROR: yay clone directory not found"; exit 1; }
cd yay
timeout 300 makepkg -si --noconfirm
AUREOF
            ;;
        "pikaur")
            pacman -S base-devel git python --noconfirm --needed || { log_error "Failed to install pikaur dependencies"; return 1; }

            log_cmd "runuser -u $MAIN_USERNAME -- bash (clone+build pikaur)"
            AUR_BUILD_DIR="$build_dir" runuser -u "$MAIN_USERNAME" -- bash << 'AUREOF' || log_warn "Failed to build pikaur from AUR"
set -e
cd "$AUR_BUILD_DIR"
timeout 60 git -c http.lowSpeedLimit=1000 -c http.lowSpeedTime=10 clone https://aur.archlinux.org/pikaur.git
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
            # No -R flag: mkinitcpio -P runs after this and will pick up the theme
            plymouth-set-default-theme "${PLYMOUTH_THEME}" || log_warn "Failed to set Plymouth theme"
            log_info "Plymouth theme set to: ${PLYMOUTH_THEME}"
        else
            log_warn "Plymouth theme not found: ${PLYMOUTH_THEME}"
            # List available themes
            log_info "Available themes: $(ls /usr/share/plymouth/themes/ 2>/dev/null || echo 'none')"
        fi
    fi

    log_success "Plymouth configured"
}

configure_snapshots() {
    if [[ "${BTRFS_SNAPSHOTS:-No}" != "Yes" ]]; then
        log_info "Btrfs snapshots not requested"
        return 0
    fi

    if [[ "${ROOT_FILESYSTEM_TYPE:-ext4}" != "btrfs" ]]; then
        log_info "Snapshots require btrfs filesystem, skipping"
        return 0
    fi

    local snapshot_tool="${SNAPSHOT_TOOL:-none}"
    log_info "Configuring btrfs snapshots (tool: $snapshot_tool)..."

    case "$snapshot_tool" in
        "snapper")
            _configure_snapper
            ;;
        "timeshift")
            _configure_timeshift
            ;;
        *)
            log_info "No snapshot tool selected, skipping configuration"
            return 0
            ;;
    esac
}

_configure_snapper() {
    # snapper, snap-pac, and grub-btrfs are already installed via pacstrap
    # (avoids dbus FATAL errors from snap-pac hooks firing in chroot)

    # Temporarily disable snap-pac alpm hooks during remaining chroot package installs
    # snap-pac hooks call snapper which requires dbus — unavailable in chroot
    local hook_dir="/usr/share/libalpm/hooks"
    local hooks_disabled=false

    # shellcheck disable=SC2317
    _re_enable_snap_pac_hooks() {
        if [[ "${hooks_disabled:-false}" == true ]]; then
            mv "$hook_dir/snap-pac-pre.hook.disabled" "$hook_dir/snap-pac-pre.hook" 2>/dev/null || true
            mv "$hook_dir/snap-pac-post.hook.disabled" "$hook_dir/snap-pac-post.hook" 2>/dev/null || true
            log_info "Re-enabled snap-pac hooks"
        fi
    }
    # Guarantee hooks are re-enabled on ANY exit path (normal, error, early return)
    trap _re_enable_snap_pac_hooks RETURN

    if [[ -f "$hook_dir/snap-pac-pre.hook" ]]; then
        mv "$hook_dir/snap-pac-pre.hook" "$hook_dir/snap-pac-pre.hook.disabled" 2>/dev/null || true
        mv "$hook_dir/snap-pac-post.hook" "$hook_dir/snap-pac-post.hook.disabled" 2>/dev/null || true
        hooks_disabled=true
        log_info "Temporarily disabled snap-pac hooks for chroot configuration"
    fi

    # Remove the @snapshots mount if it exists (snapper will recreate it)
    if mountpoint -q /.snapshots 2>/dev/null; then
        umount /.snapshots 2>/dev/null || true
    fi
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
    # snapper creates a new subvolume, but we want to use @snapshots
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

    chmod 750 /.snapshots || log_warn "Failed to set permissions on /.snapshots"

    # Configure snapper settings
    if [[ -f /etc/snapper/configs/root ]]; then
        sed -i 's/^TIMELINE_CREATE=.*/TIMELINE_CREATE="yes"/' /etc/snapper/configs/root || log_warn "Failed to set TIMELINE_CREATE"
        sed -i 's/^TIMELINE_CLEANUP=.*/TIMELINE_CLEANUP="yes"/' /etc/snapper/configs/root || log_warn "Failed to set TIMELINE_CLEANUP"
        sed -i 's/^TIMELINE_MIN_AGE=.*/TIMELINE_MIN_AGE="1800"/' /etc/snapper/configs/root || log_warn "Failed to set TIMELINE_MIN_AGE"

        local keep_count="${BTRFS_KEEP_COUNT:-3}"
        local frequency="${BTRFS_FREQUENCY:-weekly}"

        local hourly_limit="0"
        local daily_limit="0"
        local weekly_limit="0"
        local monthly_limit="0"

        case "${frequency,,}" in
            hourly)  hourly_limit="$keep_count" ;;
            daily)   daily_limit="$keep_count" ;;
            weekly)  weekly_limit="$keep_count" ;;
            monthly) monthly_limit="$keep_count" ;;
            *)       weekly_limit="$keep_count"; log_warn "Unknown frequency '$frequency', defaulting to weekly" ;;
        esac

        sed -i "s/^TIMELINE_LIMIT_HOURLY=.*/TIMELINE_LIMIT_HOURLY=\"$hourly_limit\"/" /etc/snapper/configs/root || log_warn "Failed to set TIMELINE_LIMIT_HOURLY"
        sed -i "s/^TIMELINE_LIMIT_DAILY=.*/TIMELINE_LIMIT_DAILY=\"$daily_limit\"/" /etc/snapper/configs/root || log_warn "Failed to set TIMELINE_LIMIT_DAILY"
        sed -i "s/^TIMELINE_LIMIT_WEEKLY=.*/TIMELINE_LIMIT_WEEKLY=\"$weekly_limit\"/" /etc/snapper/configs/root || log_warn "Failed to set TIMELINE_LIMIT_WEEKLY"
        sed -i "s/^TIMELINE_LIMIT_MONTHLY=.*/TIMELINE_LIMIT_MONTHLY=\"$monthly_limit\"/" /etc/snapper/configs/root || log_warn "Failed to set TIMELINE_LIMIT_MONTHLY"
        sed -i 's/^TIMELINE_LIMIT_YEARLY=.*/TIMELINE_LIMIT_YEARLY="0"/' /etc/snapper/configs/root || log_warn "Failed to set TIMELINE_LIMIT_YEARLY"

        log_info "Configured snapper timeline settings (frequency: $frequency, keep: $keep_count)"
    fi

    # Enable snapper timers
    systemctl enable snapper-timeline.timer 2>/dev/null || log_warn "Failed to enable snapper-timeline.timer"
    systemctl enable snapper-cleanup.timer 2>/dev/null || log_warn "Failed to enable snapper-cleanup.timer"

    # Enable grub-btrfs path monitoring if installed
    if [[ -f /usr/lib/systemd/system/grub-btrfsd.service ]]; then
        systemctl enable grub-btrfsd.service 2>/dev/null || log_warn "Failed to enable grub-btrfsd.service"
        log_info "Enabled grub-btrfs daemon for boot menu updates"
    fi

    # snap-pac hooks are re-enabled automatically by the RETURN trap above

    log_success "Snapper configured for automatic btrfs snapshots"
}

_configure_timeshift() {
    # timeshift is already installed via pacstrap

    # Create timeshift config directory
    mkdir -p /etc/timeshift || {
        log_error "Failed to create /etc/timeshift directory"
        return 1
    }

    # Write default timeshift config for btrfs mode
    cat > /etc/timeshift/timeshift.json << 'TSEOF'
{
  "backup_device_uuid" : "",
  "parent_device_uuid" : "",
  "do_first_run" : "true",
  "btrfs_mode" : "true",
  "include_btrfs_home_for_backup" : "true",
  "include_btrfs_home_for_restore" : "false",
  "stop_cron_emails" : "true",
  "schedule_monthly" : "true",
  "schedule_weekly" : "true",
  "schedule_daily" : "true",
  "schedule_hourly" : "false",
  "schedule_boot" : "false",
  "count_monthly" : "2",
  "count_weekly" : "3",
  "count_daily" : "5",
  "count_hourly" : "0",
  "count_boot" : "0",
  "snapshot_size" : "0",
  "snapshot_count" : "0",
  "exclude" : [],
  "exclude-apps" : []
}
TSEOF

    # Enable timeshift scheduled snapshots via cronie or systemd timer
    if command -v crond &>/dev/null || [[ -f /usr/lib/systemd/system/cronie.service ]]; then
        systemctl enable cronie.service 2>/dev/null || log_warn "Failed to enable cronie for timeshift"
        log_info "Enabled cronie for timeshift scheduled snapshots"
    fi

    log_success "Timeshift configured for btrfs snapshots (use timeshift-gtk to manage)"
}

# =============================================================================
# PHASE 5: FINAL CONFIGURATION
# =============================================================================

configure_numlock() {
    if [[ "${NUMLOCK_ON_BOOT:-No}" != "Yes" ]]; then
        return 0
    fi

    log_info "Configuring numlock on boot via mkinitcpio-numlock (AUR)..."

    # mkinitcpio-numlock activates numlock in early userspace (initramfs).
    # Works universally — TTY, X11, Wayland, all display managers.
    local _aur_helper="${AUR_HELPER:-none}"

    if [[ "$_aur_helper" == "none" ]]; then
        log_warn "Numlock on boot requires an AUR helper (mkinitcpio-numlock is an AUR package)"
        log_warn "Skipping numlock — install mkinitcpio-numlock manually after boot"
        return 0
    fi

    # Install mkinitcpio-numlock via the configured AUR helper
    local _user="${MAIN_USERNAME:?MAIN_USERNAME not set}"
    if command -v "$_aur_helper" &>/dev/null; then
        log_cmd "runuser -u $_user -- $_aur_helper -S --noconfirm mkinitcpio-numlock"
        runuser -u "$_user" -- "$_aur_helper" -S --noconfirm mkinitcpio-numlock || {
            log_warn "Failed to install mkinitcpio-numlock — numlock will not activate on boot"
            return 0
        }
    else
        log_warn "AUR helper '$_aur_helper' not found — skipping mkinitcpio-numlock"
        return 0
    fi

    # Insert numlock hook between modconf and block in mkinitcpio.conf
    if [[ -f /etc/mkinitcpio.conf ]]; then
        if ! grep -q "numlock" /etc/mkinitcpio.conf; then
            sed -i 's/\(modconf\)/\1 numlock/' /etc/mkinitcpio.conf || {
                log_warn "Failed to add numlock hook to mkinitcpio.conf"
                return 0
            }
            log_info "Added numlock hook to mkinitcpio.conf (after modconf)"
        fi

        # Rebuild initramfs with the new hook
        log_cmd "mkinitcpio -P"
        mkinitcpio -P || log_warn "mkinitcpio rebuild failed after adding numlock hook"
    fi

    log_success "Numlock on boot configured via mkinitcpio-numlock"
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

    timeout 60 runuser -u "$MAIN_USERNAME" -- git -c http.lowSpeedLimit=1000 -c http.lowSpeedTime=10 clone "$GIT_REPOSITORY_URL" "$user_home/dotfiles" || {
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
