#!/bin/bash
# xfce.sh - XFCE Desktop Environment Installation
# Part of the modular desktop environment system

set -euo pipefail

# Source utility functions
source "$(dirname "${BASH_SOURCE[0]}")/../utils.sh"

install_xfce_chroot() {
    log_info "Installing XFCE Desktop Environment..."
    
    # Install XFCE packages
    install_packages_chroot xfce4 xfce4-goodies || {
        log_error "Failed to install XFCE packages"
        return 1
    }
    
    # Install LightDM and greeter (required for XFCE)
    install_packages_chroot lightdm lightdm-gtk-greeter || {
        log_error "Failed to install LightDM packages"
        return 1
    }
    
    # Enable lightdm service
    systemctl enable lightdm.service || {
        log_error "Failed to enable LightDM service"
        return 1
    }
    
    log_success "XFCE Desktop Environment installed successfully"
    return 0
}

# Run installation if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    install_xfce_chroot "$@"
fi
