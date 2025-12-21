#!/bin/bash
# xfce.sh - XFCE Desktop Environment Installation
# Part of the modular desktop environment system
# This script must be run inside arch-chroot

set -euo pipefail

install_xfce() {
    echo "Installing XFCE Desktop Environment..."

    # Install XFCE packages
    pacman -S --noconfirm --needed \
        xfce4 \
        xfce4-goodies \
        lightdm \
        lightdm-gtk-greeter || {
        echo "ERROR: Failed to install XFCE packages"
        return 1
    }

    # Enable LightDM service
    systemctl enable lightdm.service || {
        echo "ERROR: Failed to enable LightDM service"
        return 1
    }

    echo "XFCE Desktop Environment installed successfully"
    return 0
}

# Run installation if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    install_xfce "$@"
fi
