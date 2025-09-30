#!/bin/bash
# i3.sh - i3 Window Manager Installation
# Part of the modular desktop environment system

set -euo pipefail

# Source utility functions
source "$(dirname "${BASH_SOURCE[0]}")/../utils.sh"

install_i3_chroot() {
    log_info "Installing i3 Window Manager..."
    
    # Install i3 core packages
    install_packages_chroot i3-wm i3status i3lock || {
        log_error "Failed to install i3 core packages"
        return 1
    }
    
    # Install LightDM and greeter (required for i3)
    install_packages_chroot lightdm lightdm-gtk-greeter || {
        log_error "Failed to install LightDM packages"
        return 1
    }
    
    # Install essential i3 packages
    install_packages_chroot dmenu rofi || {
        log_error "Failed to install essential i3 packages"
        return 1
    }
    
    # Install additional useful packages for i3
    install_packages_chroot feh picom alacritty || {
        log_warn "Some optional i3 packages failed to install, continuing..."
    }
    
    # Enable lightdm service
    systemctl enable lightdm.service || {
        log_error "Failed to enable LightDM service"
        return 1
    }
    
    log_success "i3 Window Manager installed successfully"
    return 0
}

# Run installation if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    install_i3_chroot "$@"
fi
