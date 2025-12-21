#!/bin/bash
# kde.sh - KDE Plasma Desktop Environment Installation
# Part of the modular desktop environment system
# This script must be run inside arch-chroot

set -euo pipefail

install_kde() {
    echo "Installing KDE Plasma Desktop Environment..."

    # Install KDE Plasma packages
    pacman -S --noconfirm --needed plasma kde-applications sddm || {
        echo "ERROR: Failed to install KDE packages"
        return 1
    }

    # Enable SDDM service
    systemctl enable sddm.service || {
        echo "ERROR: Failed to enable SDDM service"
        return 1
    }

    echo "KDE Plasma Desktop Environment installed successfully"
    return 0
}

# Run installation if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    install_kde "$@"
fi
