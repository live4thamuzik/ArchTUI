#!/bin/bash
# i3.sh - i3 Window Manager Installation
# Part of the modular desktop environment system
# This script must be run inside arch-chroot

set -euo pipefail

install_i3() {
    echo "Installing i3 Window Manager..."

    # Install i3 core and utilities
    pacman -S --noconfirm --needed \
        i3-wm \
        i3status \
        i3lock \
        dmenu \
        rofi \
        feh \
        picom \
        alacritty \
        lightdm \
        lightdm-gtk-greeter || {
        echo "ERROR: Failed to install i3 packages"
        return 1
    }

    # Enable LightDM service
    systemctl enable lightdm.service || {
        echo "ERROR: Failed to enable LightDM service"
        return 1
    }

    echo "i3 Window Manager installed successfully"
    return 0
}

# Run installation if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    install_i3 "$@"
fi
