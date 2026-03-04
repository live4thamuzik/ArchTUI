#!/bin/bash
# install_bootloader.sh - Install or repair bootloader using ISO tools only
# Usage: ./install_bootloader.sh --type <grub|systemd-boot> --disk <device> [options]

set -euo pipefail

# --- Signal Handling ---
cleanup_and_exit() {
    local sig="$1"
    echo "$(basename "$0"): Received $sig, aborting..." >&2
    [[ "$sig" == "SIGTERM" ]] && exit 143 || exit 130
}
trap 'cleanup_and_exit SIGTERM' SIGTERM
trap 'cleanup_and_exit SIGINT' SIGINT

# Source common utilities via source_or_die
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source_or_die() {
    local script_path="$1"
    local error_msg="${2:-Failed to source required script: $script_path}"
    if [[ ! -f "$script_path" ]]; then
        echo "FATAL: $error_msg (file not found)" >&2
        exit 1
    fi
    # shellcheck source=/dev/null
    if ! source "$script_path"; then
        echo "FATAL: $error_msg (source failed)" >&2
        exit 1
    fi
}
source_or_die "$SCRIPT_DIR/../utils.sh"

require_root

# Default values
BOOTLOADER_TYPE=""
TARGET_DISK=""
EFI_PATH=""
BOOT_MODE=""
ROOT_PATH="/mnt"
REPAIR_MODE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --type)
            BOOTLOADER_TYPE="$2"
            shift 2
            ;;
        --disk)
            TARGET_DISK="$2"
            shift 2
            ;;
        --efi-path)
            EFI_PATH="$2"
            shift 2
            ;;
        --mode)
            BOOT_MODE="$2"
            shift 2
            ;;
        --root)
            ROOT_PATH="$2"
            shift 2
            ;;
        --repair)
            REPAIR_MODE=true
            shift
            ;;
        --help)
            echo "Usage: $0 --type <grub|systemd-boot|refind|limine|efistub> --disk <device> [options]"
            echo ""
            echo "Required:"
            echo "  --type <type>        Bootloader type: grub, systemd-boot, refind, limine, or efistub"
            echo "  --disk <device>      Target disk device (e.g., /dev/sda)"
            echo ""
            echo "Optional:"
            echo "  --efi-path <path>    EFI partition mount point (default: auto-detect)"
            echo "  --mode <mode>        Boot mode: uefi or bios (default: auto-detect)"
            echo "  --root <path>        Root directory (default: /mnt)"
            echo "  --repair             Repair existing bootloader installation"
            echo ""
            echo "Examples:"
            echo "  $0 --type grub --disk /dev/sda"
            echo "  $0 --type systemd-boot --disk /dev/sda --efi-path /efi"
            echo "  $0 --type grub --disk /dev/sda --repair"
            echo ""
            echo "Note: Uses tools available on Arch ISO (no package installation required)"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate required arguments
if [[ -z "$BOOTLOADER_TYPE" ]]; then
    error_exit "Bootloader type is required (--type grub|systemd-boot|refind|limine|efistub)"
fi

if [[ -z "$TARGET_DISK" ]]; then
    error_exit "Target disk is required (--disk /dev/sda)"
fi

# Validate bootloader type
case "$BOOTLOADER_TYPE" in
    grub|systemd-boot|refind|limine|efistub) ;;
    *) error_exit "Invalid bootloader type: $BOOTLOADER_TYPE (must be grub, systemd-boot, refind, limine, or efistub)" ;;
esac

# Validate target disk exists
if [[ ! -b "$TARGET_DISK" ]]; then
    error_exit "Target disk $TARGET_DISK does not exist or is not a block device"
fi

# Auto-detect boot mode if not specified
if [[ -z "$BOOT_MODE" ]]; then
    if [[ -d "/sys/firmware/efi" ]]; then
        BOOT_MODE="uefi"
        log_info "Auto-detected UEFI boot mode"
    else
        BOOT_MODE="bios"
        log_info "Auto-detected BIOS boot mode"
    fi
fi

# Auto-detect EFI path if not specified and in UEFI mode
if [[ "$BOOT_MODE" == "uefi" && -z "$EFI_PATH" ]]; then
    # Try common EFI mount points
    for path in "/efi" "/boot/efi" "/boot"; do
        if [[ -d "$ROOT_PATH$path" ]] && mountpoint -q "$ROOT_PATH$path" 2>/dev/null; then
            EFI_PATH="$path"
            log_info "Auto-detected EFI path: $EFI_PATH"
            break
        fi
    done
    
    if [[ -z "$EFI_PATH" ]]; then
        error_exit "Could not auto-detect EFI path. Please specify with --efi-path"
    fi
fi

# Check if target system is mounted
if [[ ! -d "$ROOT_PATH" ]]; then
    error_exit "Root directory $ROOT_PATH does not exist"
fi

if [[ "$REPAIR_MODE" == false && ! -d "$ROOT_PATH/boot" ]]; then
    error_exit "Target system appears not to be properly mounted at $ROOT_PATH"
fi

log_info "🔧 Bootloader Installation/Repair Tool (ISO Compatible)"
echo "=================================================="
log_info "Bootloader type: $BOOTLOADER_TYPE"
log_info "Target disk: $TARGET_DISK"
log_info "Boot mode: $BOOT_MODE"
log_info "Root path: $ROOT_PATH"
if [[ -n "$EFI_PATH" ]]; then
    log_info "EFI path: $EFI_PATH"
fi
if [[ "$REPAIR_MODE" == true ]]; then
    log_info "Mode: Repair existing installation"
fi
echo "=================================================="

# Check if required tools are available
case "$BOOTLOADER_TYPE" in
    grub)
        if ! command -v grub-install >/dev/null 2>&1; then
            error_exit "grub-install not found. GRUB must be installed on the target system."
        fi
        if ! command -v grub-mkconfig >/dev/null 2>&1; then
            error_exit "grub-mkconfig not found. GRUB must be installed on the target system."
        fi
        ;;
    systemd-boot)
        if ! command -v bootctl >/dev/null 2>&1; then
            error_exit "bootctl not found. systemd-boot requires systemd to be installed on the target system."
        fi
        ;;
    refind)
        if [[ "$BOOT_MODE" != "uefi" ]]; then
            error_exit "rEFInd requires UEFI mode (BIOS not supported)"
        fi
        if ! command -v refind-install >/dev/null 2>&1; then
            error_exit "refind-install not found. Install the refind package first."
        fi
        ;;
    limine)
        if ! command -v limine >/dev/null 2>&1; then
            error_exit "limine not found. Install the limine package first."
        fi
        ;;
    efistub)
        if [[ "$BOOT_MODE" != "uefi" ]]; then
            error_exit "EFISTUB requires UEFI mode (BIOS not supported)"
        fi
        if ! command -v efibootmgr >/dev/null 2>&1; then
            error_exit "efibootmgr not found. Install the efibootmgr package first."
        fi
        ;;
esac

# Install bootloader
case "$BOOTLOADER_TYPE" in
    grub)
        log_info "🔧 Installing GRUB bootloader..."
        
        if [[ "$BOOT_MODE" == "uefi" ]]; then
            log_info "Installing GRUB for UEFI mode..."
            if ! command -v efibootmgr >/dev/null 2>&1; then
                log_warning "efibootmgr not found. EFI boot manager entries may not be created."
            fi
            
            log_cmd "arch-chroot $ROOT_PATH grub-install --target=x86_64-efi --efi-directory=$EFI_PATH --bootloader-id=GRUB --recheck"
            arch-chroot "$ROOT_PATH" grub-install \
                --target=x86_64-efi \
                --efi-directory="$EFI_PATH" \
                --bootloader-id=GRUB \
                --recheck || error_exit "grub-install (UEFI) failed"
        else
            log_info "Installing GRUB for BIOS mode..."
            log_cmd "arch-chroot $ROOT_PATH grub-install --target=i386-pc $TARGET_DISK --recheck"
            arch-chroot "$ROOT_PATH" grub-install \
                --target=i386-pc \
                "$TARGET_DISK" \
                --recheck || error_exit "grub-install (BIOS) failed"
        fi
        
        # Configure GRUB
        log_info "⚙️  Generating GRUB configuration..."
        
        # Check if os-prober is available and install it if needed
        if arch-chroot "$ROOT_PATH" pacman -Qi os-prober >/dev/null 2>&1; then
            log_info "OS prober detected - enabling multi-boot support"
            log_cmd "echo GRUB_DISABLE_OS_PROBER=false >> /etc/default/grub"
            echo "GRUB_DISABLE_OS_PROBER=false" >> "$ROOT_PATH/etc/default/grub" || log_warn "Failed to write GRUB os-prober config"
        else
            echo "GRUB_DISABLE_OS_PROBER=true" >> "$ROOT_PATH/etc/default/grub" || log_warn "Failed to write GRUB os-prober config"
        fi
        
        # Generate GRUB config
        log_cmd "arch-chroot $ROOT_PATH grub-mkconfig -o /boot/grub/grub.cfg"
        arch-chroot "$ROOT_PATH" grub-mkconfig -o /boot/grub/grub.cfg || error_exit "grub-mkconfig failed"
        
        log_success "✅ GRUB bootloader installed successfully!"
        ;;
        
    systemd-boot)
        if [[ "$BOOT_MODE" != "uefi" ]]; then
            error_exit "systemd-boot requires UEFI mode"
        fi
        
        log_info "🔧 Installing systemd-boot..."
        log_cmd "arch-chroot $ROOT_PATH bootctl install"
        arch-chroot "$ROOT_PATH" bootctl install || error_exit "bootctl install failed"
        
        # Create loader configuration
        log_info "⚙️  Configuring systemd-boot loader..."
        cat > "$ROOT_PATH/boot/loader/loader.conf" << EOF
default arch
timeout 4
editor  no
auto-entries yes
auto-firmware yes
EOF
        
        # Create arch entry
        log_info "📝 Creating Arch Linux boot entry..."
        mkdir -p "$ROOT_PATH/boot/loader/entries" || error_exit "Failed to create loader entries directory"
        
        # Get root partition UUID - find the largest ext4/xfs/btrfs partition
        ROOT_PARTITION=""
        _ROOT_PART_SIZE=0
        
        while IFS= read -r part; do
            if [[ -b "$part" ]]; then
                PART_TYPE=$(blkid -s TYPE -o value "$part" 2>/dev/null || echo "")
                if [[ "$PART_TYPE" == "ext4" || "$PART_TYPE" == "xfs" || "$PART_TYPE" == "btrfs" ]]; then
                    # Get partition size
                    PART_SIZE=$(blockdev --getsize64 "$part" 2>/dev/null || echo "0")
                    if [[ "$PART_SIZE" -gt "$_ROOT_PART_SIZE" ]]; then
                        _ROOT_PART_SIZE="$PART_SIZE"
                        ROOT_PARTITION="$part"
                    fi
                fi
            fi
        done < <(lsblk -ln -o PATH "$TARGET_DISK" | tail -n +2)
        
        if [[ -n "$ROOT_PARTITION" ]]; then
            ROOT_UUID=$(blkid -s UUID -o value "$ROOT_PARTITION" 2>/dev/null) || error_exit "blkid failed for $ROOT_PARTITION"
            [[ -n "$ROOT_UUID" ]] || error_exit "UUID is empty for $ROOT_PARTITION"
            log_info "Using root partition: $ROOT_PARTITION (UUID: $ROOT_UUID)"
        else
            error_exit "Could not find root partition for systemd-boot entry"
        fi

        log_cmd "Writing arch.conf boot entry (root=UUID=$ROOT_UUID)"
        {
            echo "title   Arch Linux"
            echo "linux   /vmlinuz-linux"
            [[ -f "$ROOT_PATH/boot/intel-ucode.img" ]] && echo "initrd  /intel-ucode.img"
            [[ -f "$ROOT_PATH/boot/amd-ucode.img" ]] && echo "initrd  /amd-ucode.img"
            echo "initrd  /initramfs-linux.img"
            echo "options root=UUID=$ROOT_UUID rw"
        } > "$ROOT_PATH/boot/loader/entries/arch.conf" || error_exit "Failed to write arch.conf"
        
        # Update firmware boot manager
        log_info "🔄 Updating firmware boot manager..."
        log_cmd "arch-chroot $ROOT_PATH bootctl update"
        arch-chroot "$ROOT_PATH" bootctl update || log_warn "bootctl update failed (non-fatal)"
        
        log_success "✅ systemd-boot installed successfully!"
        ;;
        
    refind)
        log_info "🔧 Installing rEFInd bootloader..."

        log_cmd "arch-chroot $ROOT_PATH refind-install"
        arch-chroot "$ROOT_PATH" refind-install || error_exit "refind-install failed"

        log_success "✅ rEFInd bootloader installed successfully!"
        ;;

    limine)
        log_info "🔧 Installing Limine bootloader..."

        if [[ "$BOOT_MODE" == "uefi" ]]; then
            log_info "Installing Limine for UEFI mode..."
            limine_efi="$ROOT_PATH/usr/share/limine/BOOTX64.EFI"
            if [[ ! -f "$limine_efi" ]]; then
                error_exit "Limine EFI binary not found at $limine_efi"
            fi
            esp_dir="$ROOT_PATH${EFI_PATH}/EFI/BOOT"
            log_cmd "mkdir -p $esp_dir"
            mkdir -p "$esp_dir" || error_exit "Failed to create EFI boot directory"
            log_cmd "cp $limine_efi $esp_dir/BOOTX64.EFI"
            cp "$limine_efi" "$esp_dir/BOOTX64.EFI" || error_exit "Failed to copy Limine EFI binary"
        else
            log_info "Installing Limine for BIOS mode..."
            log_cmd "limine bios-install $TARGET_DISK"
            limine bios-install "$TARGET_DISK" || error_exit "limine bios-install failed"
        fi

        # Create limine.conf if it doesn't exist
        if [[ ! -f "$ROOT_PATH/boot/limine.conf" ]]; then
            log_info "📝 Creating Limine configuration..."
            cat > "$ROOT_PATH/boot/limine.conf" << EOF
timeout: 4

/Arch Linux
    protocol: linux
    kernel_path: boot():/vmlinuz-linux
    kernel_cmdline: root=UUID=CHANGEME rw
    module_path: boot():/initramfs-linux.img
EOF
            log_warn "limine.conf created with placeholder root UUID — edit /boot/limine.conf before reboot"
        fi

        log_success "✅ Limine bootloader installed successfully!"
        ;;

    efistub)
        log_info "🔧 Creating EFISTUB boot entry..."

        # Find the ESP partition number
        esp_part=""
        while IFS= read -r part; do
            if [[ -b "$part" ]]; then
                part_type=$(blkid -s TYPE -o value "$part" 2>/dev/null || echo "")
                if [[ "$part_type" == "vfat" ]]; then
                    esp_part="$part"
                    break
                fi
            fi
        done < <(lsblk -ln -o PATH "$TARGET_DISK" | tail -n +2)

        if [[ -z "$esp_part" ]]; then
            error_exit "Could not find ESP (vfat) partition on $TARGET_DISK"
        fi

        # Get disk and partition number for efibootmgr
        disk_for_efi="$TARGET_DISK"
        part_num=$(echo "$esp_part" | grep -oE '[0-9]+$')

        # Get root UUID
        root_part=""
        max_size=0
        while IFS= read -r part; do
            if [[ -b "$part" ]]; then
                pt=$(blkid -s TYPE -o value "$part" 2>/dev/null || echo "")
                if [[ "$pt" == "ext4" || "$pt" == "xfs" || "$pt" == "btrfs" || "$pt" == "f2fs" ]]; then
                    ps=$(blockdev --getsize64 "$part" 2>/dev/null || echo "0")
                    if [[ "$ps" -gt "$max_size" ]]; then
                        max_size="$ps"
                        root_part="$part"
                    fi
                fi
            fi
        done < <(lsblk -ln -o PATH "$TARGET_DISK" | tail -n +2)

        if [[ -z "$root_part" ]]; then
            error_exit "Could not find root partition for EFISTUB entry"
        fi

        root_uuid=$(blkid -s UUID -o value "$root_part" 2>/dev/null) || error_exit "Failed to get UUID for $root_part"
        [[ -n "$root_uuid" ]] || error_exit "UUID is empty for $root_part"

        cmdline="root=UUID=$root_uuid rw"

        # Build initrd args
        initrd_args=""
        [[ -f "$ROOT_PATH/boot/intel-ucode.img" ]] && initrd_args="--unicode initrd=\\intel-ucode.img"
        [[ -f "$ROOT_PATH/boot/amd-ucode.img" ]] && initrd_args="$initrd_args --unicode initrd=\\amd-ucode.img"
        initrd_args="$initrd_args --unicode initrd=\\initramfs-linux.img"

        log_cmd "efibootmgr --create --disk $disk_for_efi --part $part_num --label 'Arch Linux' --loader /vmlinuz-linux"
        # shellcheck disable=SC2086
        efibootmgr --create \
            --disk "$disk_for_efi" \
            --part "$part_num" \
            --label "Arch Linux" \
            --loader "/vmlinuz-linux" \
            $initrd_args \
            --unicode "$cmdline" || error_exit "efibootmgr --create failed"

        log_success "✅ EFISTUB boot entry created successfully!"
        ;;

    *)
        error_exit "Unsupported bootloader type: $BOOTLOADER_TYPE"
        ;;
esac

# Verify installation
log_info "🔍 Verifying bootloader installation..."
case "$BOOTLOADER_TYPE" in
    grub)
        if [[ -f "$ROOT_PATH/boot/grub/grub.cfg" ]]; then
            log_success "✅ GRUB configuration file created"
            log_info "GRUB config size: $(du -h "$ROOT_PATH/boot/grub/grub.cfg" | cut -f1)"
        else
            log_warning "⚠️  GRUB configuration file not found"
        fi
        ;;
    systemd-boot)
        if [[ -f "$ROOT_PATH/boot/loader/loader.conf" ]]; then
            log_success "✅ systemd-boot loader configuration created"
        else
            log_warning "⚠️  systemd-boot loader configuration not found"
        fi
        if [[ -f "$ROOT_PATH/boot/loader/entries/arch.conf" ]]; then
            log_success "✅ Arch Linux boot entry created"
        else
            log_warning "⚠️  Arch Linux boot entry not found"
        fi
        ;;
    refind)
        if [[ -f "$ROOT_PATH/boot/refind_linux.conf" ]] || [[ -d "$ROOT_PATH/boot/EFI/refind" ]]; then
            log_success "✅ rEFInd configuration found"
        else
            log_warning "⚠️  rEFInd configuration not found — may need manual setup"
        fi
        ;;
    limine)
        if [[ -f "$ROOT_PATH/boot/limine.conf" ]]; then
            log_success "✅ Limine configuration file found"
        else
            log_warning "⚠️  Limine configuration file not found at /boot/limine.conf"
        fi
        ;;
    efistub)
        log_info "Checking EFI boot entries..."
        efibootmgr 2>/dev/null | grep -q "Arch Linux" && log_success "✅ EFISTUB entry found" || log_warning "⚠️  EFISTUB entry not found"
        ;;
esac

log_success "🎉 Bootloader installation completed successfully!"
log_info "Next steps:"
log_info "  • Reboot your system"
log_info "  • Select your bootloader from the firmware boot menu"
if [[ "$BOOTLOADER_TYPE" == "grub" ]]; then
    log_info "  • GRUB will scan for available operating systems"
fi