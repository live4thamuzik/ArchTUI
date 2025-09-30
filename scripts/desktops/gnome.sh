#!/bin/bash
# gnome.sh - GNOME Desktop Environment Installation
# Part of the modular desktop environment system

set -euo pipefail

# Source utility functions
source "$(dirname "${BASH_SOURCE[0]}")/../utils.sh"

install_gnome_chroot() {
    log_info "Installing GNOME Desktop Environment..."
    
    # Install GNOME packages
    install_packages_chroot gnome gnome-extra || {
        log_error "Failed to install GNOME packages"
        return 1
    }
    
    # GDM is included in gnome package group, but let's ensure it's available
    # Install GDM explicitly if not included
    install_packages_chroot gdm || {
        log_warn "GDM may already be installed with GNOME package group"
    }
    
    # Enable GDM service
    systemctl enable gdm.service || {
        log_error "Failed to enable GDM service"
        return 1
    }
    
    log_success "GNOME Desktop Environment installed successfully"
    return 0
}

# Run installation if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    install_gnome_chroot "$@"
fi
