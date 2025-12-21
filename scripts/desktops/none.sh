#!/bin/bash
# none.sh - No Desktop Environment Installation
# Part of the modular desktop environment system
# This script must be run inside arch-chroot

set -euo pipefail

install_none() {
    echo "No desktop environment requested - installing minimal system only"

    # Install basic terminal tools
    pacman -S --noconfirm --needed \
        nano \
        vim \
        htop \
        neofetch || {
        echo "WARN: Some basic packages failed to install, continuing..."
    }

    echo "Minimal system installation completed (no desktop environment)"
    return 0
}

# Run installation if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    install_none "$@"
fi
