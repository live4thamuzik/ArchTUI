#!/bin/bash
# hyprland.sh - Hyprland Wayland Compositor Installation
# Part of the modular desktop environment system

set -euo pipefail

# Source utility functions
source "$(dirname "${BASH_SOURCE[0]}")/../utils.sh"

install_hyprland_chroot() {
    log_info "Installing Hyprland Wayland Compositor..."
    
    # Install Hyprland core packages
    install_packages_chroot hyprland || {
        log_error "Failed to install Hyprland core package"
        return 1
    }
    
    # Install Hyprland utilities and tools
    install_packages_chroot waybar swaylock swayidle wlogout || {
        log_error "Failed to install Hyprland utilities"
        return 1
    }
    
    # Install additional useful packages for Hyprland (kitty is the default terminal)
    install_packages_chroot rofi-wayland grim slurp wf-recorder kitty || {
        log_warn "Some optional Hyprland packages failed to install, continuing..."
    }
    
    # Install SDDM for Wayland (required for Hyprland)
    install_packages_chroot sddm || {
        log_error "Failed to install SDDM"
        return 1
    }
    
    # Enable SDDM service
    systemctl enable sddm.service || {
        log_error "Failed to enable SDDM service"
        return 1
    }
    
    log_success "Hyprland Wayland Compositor installed successfully"
    return 0
}

# Run installation if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    install_hyprland_chroot "$@"
fi
