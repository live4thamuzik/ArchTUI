#!/bin/bash
# install_bootloader.sh - Install or repair bootloader
# Usage: ./install_bootloader.sh --type grub --disk /dev/sda [--efi-path /boot/efi]

set -euo pipefail

# Source common utilities
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$SCRIPT_DIR/../utils.sh"

# Default values
BOOTLOADER_TYPE=""
TARGET_DISK=""
EFI_PATH=""
BOOT_MODE=""

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
        --help)
            echo "Usage: $0 --type <grub|systemd-boot> --disk <device> [--efi-path <path>] [--mode <uefi|bios>]"
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
    error_exit "Bootloader type is required (--type grub|systemd-boot)"
fi

if [[ -z "$TARGET_DISK" ]]; then
    error_exit "Target disk is required (--disk /dev/sda)"
fi

# Auto-detect boot mode if not specified
if [[ -z "$BOOT_MODE" ]]; then
    if [[ -d "/sys/firmware/efi" ]]; then
        BOOT_MODE="uefi"
    else
        BOOT_MODE="bios"
    fi
fi

# Validate EFI path for UEFI mode
if [[ "$BOOT_MODE" == "uefi" && -z "$EFI_PATH" ]]; then
    error_exit "EFI path is required for UEFI mode (--efi-path /boot/efi)"
fi

# Check if target system is mounted
if ! mountpoint -q /mnt; then
    error_exit "Target system must be mounted at /mnt"
fi

log_info "Installing $BOOTLOADER_TYPE bootloader..."
log_info "Target disk: $TARGET_DISK"
log_info "Boot mode: $BOOT_MODE"

case "$BOOTLOADER_TYPE" in
    grub)
        log_info "Installing GRUB..."
        pacstrap /mnt grub efibootmgr
        
        if [[ "$BOOT_MODE" == "uefi" ]]; then
            log_info "Installing GRUB for UEFI mode..."
            arch-chroot /mnt grub-install --target=x86_64-efi --efi-directory="$EFI_PATH" --bootloader-id=GRUB
        else
            log_info "Installing GRUB for BIOS mode..."
            arch-chroot /mnt grub-install --target=i386-pc "$TARGET_DISK"
        fi
        
        log_info "Generating GRUB configuration..."
        arch-chroot /mnt grub-mkconfig -o /boot/grub/grub.cfg
        ;;
        
    systemd-boot)
        if [[ "$BOOT_MODE" != "uefi" ]]; then
            error_exit "systemd-boot requires UEFI mode"
        fi
        
        log_info "Installing systemd-boot..."
        arch-chroot /mnt bootctl install
        
        # Create loader configuration
        cat > /mnt/boot/loader/loader.conf << EOF
default arch
timeout 4
editor  yes
EOF
        
        # Create arch entry
        mkdir -p /mnt/boot/loader/entries
        cat > /mnt/boot/loader/entries/arch.conf << EOF
title   Arch Linux
linux   /vmlinuz-linux
initrd  /initramfs-linux.img
options root=UUID=$(blkid -s UUID -o value "$TARGET_DISK"2) rw
EOF
        ;;
        
    *)
        error_exit "Unsupported bootloader type: $BOOTLOADER_TYPE"
        ;;
esac

log_success "Bootloader installation completed successfully!"
