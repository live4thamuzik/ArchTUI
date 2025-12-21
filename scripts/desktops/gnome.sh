#!/bin/bash
# gnome.sh - GNOME Desktop Environment Installation
# Part of the modular desktop environment system
# This script must be run inside arch-chroot

set -euo pipefail

install_gnome() {
    echo "Installing GNOME Desktop Environment..."

    # Install GNOME packages
    pacman -S --noconfirm --needed gnome gnome-extra || {
        echo "ERROR: Failed to install GNOME packages"
        return 1
    }

    # GDM is included in gnome package group
    # Enable GDM service
    systemctl enable gdm.service || {
        echo "ERROR: Failed to enable GDM service"
        return 1
    }

    echo "GNOME Desktop Environment installed successfully"
    return 0
}

# Run installation if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    install_gnome "$@"
fi
