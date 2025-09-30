#!/bin/bash
# kde.sh - KDE Plasma Desktop Environment Installation
# Part of the modular desktop environment system

set -euo pipefail

# Source utility functions
source "$(dirname "${BASH_SOURCE[0]}")/../utils.sh"

install_kde_chroot() {
    log_info "Installing KDE Plasma Desktop Environment..."
    
    # Install KDE packages
    install_packages_chroot plasma kde-applications || {
        log_error "Failed to install KDE packages"
        return 1
    }
    
    # Install SDDM (display manager for KDE)
    install_packages_chroot sddm || {
        log_error "Failed to install SDDM"
        return 1
    }
    
    # Enable SDDM service
    systemctl enable sddm.service || {
        log_error "Failed to enable SDDM service"
        return 1
    }
    
    log_success "KDE Plasma Desktop Environment installed successfully"
    return 0
}

# Run installation if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    install_kde_chroot "$@"
fi
