#!/bin/bash
# none.sh - No Desktop Environment Installation
# Part of the modular desktop environment system

set -euo pipefail

# Source utility functions
source "$(dirname "${BASH_SOURCE[0]}")/../utils.sh"

install_none_chroot() {
    log_info "No desktop environment requested - installing minimal system only"
    
    # Install basic terminal and essential tools
    install_packages_chroot nano vim htop neofetch || {
        log_warn "Some basic packages failed to install, continuing..."
    }
    
    log_success "Minimal system installation completed (no desktop environment)"
    return 0
}

# Run installation if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    install_none_chroot "$@"
fi
