#!/bin/bash
# hyprland.sh - Hyprland Wayland Compositor Installation
# Part of the modular desktop environment system
# This script must be run inside arch-chroot

set -euo pipefail

install_hyprland() {
    echo "Installing Hyprland Wayland Compositor..."

    # Install Hyprland core and utilities
    pacman -S --noconfirm --needed \
        hyprland \
        xdg-desktop-portal-hyprland \
        waybar \
        swaylock \
        swayidle \
        wlogout \
        rofi-wayland \
        grim \
        slurp \
        wf-recorder \
        kitty \
        sddm || {
        echo "ERROR: Failed to install Hyprland packages"
        return 1
    }

    # Enable SDDM service
    systemctl enable sddm.service || {
        echo "ERROR: Failed to enable SDDM service"
        return 1
    }

    echo "Hyprland Wayland Compositor installed successfully"
    return 0
}

# Run installation if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    install_hyprland "$@"
fi
