#!/bin/bash
# config_loader.sh - JSON Configuration File Parser for Bash
# This script provides functions to load configuration from JSON files

set -euo pipefail

# Source utility functions
source "$(dirname "${BASH_SOURCE[0]}")/utils.sh"

# Check if jq is available
check_jq_available() {
    if ! command -v jq >/dev/null 2>&1; then
        log_error "jq is required for JSON configuration parsing but is not installed"
        log_info "Installing jq..."
        pacman -Sy --noconfirm jq
        if ! command -v jq >/dev/null 2>&1; then
            error_exit "Failed to install jq"
        fi
    fi
}

# Load configuration from JSON file
load_config_from_json() {
    local config_file="$1"
    
    if [[ ! -f "$config_file" ]]; then
        error_exit "Configuration file not found: $config_file"
    fi
    
    log_info "Loading configuration from: $config_file"
    
    # Check if jq is available
    check_jq_available
    
    # Validate JSON file
    if ! jq empty "$config_file" 2>/dev/null; then
        error_exit "Invalid JSON configuration file: $config_file"
    fi
    
    # Load all configuration variables
    export BOOT_MODE="$(jq -r '.boot_mode // ""' "$config_file")"
    export INSTALL_DISK="$(jq -r '.install_disk // ""' "$config_file")"
    export PARTITIONING_STRATEGY="$(jq -r '.partitioning_strategy // ""' "$config_file")"
    export ROOT_FILESYSTEM="$(jq -r '.root_filesystem // ""' "$config_file")"
    export HOME_FILESYSTEM="$(jq -r '.home_filesystem // ""' "$config_file")"
    export SEPARATE_HOME="$(jq -r '.separate_home // ""' "$config_file")"
    export ENCRYPTION="$(jq -r '.encryption // ""' "$config_file")"
    export SWAP="$(jq -r '.swap // ""' "$config_file")"
    export SWAP_SIZE="$(jq -r '.swap_size // ""' "$config_file")"
    export TIMEZONE_REGION="$(jq -r '.timezone_region // ""' "$config_file")"
    export TIMEZONE="$(jq -r '.timezone // ""' "$config_file")"
    export LOCALE="$(jq -r '.locale // ""' "$config_file")"
    export KEYMAP="$(jq -r '.keymap // ""' "$config_file")"
    export HOSTNAME="$(jq -r '.hostname // ""' "$config_file")"
    export USERNAME="$(jq -r '.username // ""' "$config_file")"
    export USER_PASSWORD="$(jq -r '.user_password // ""' "$config_file")"
    export ROOT_PASSWORD="$(jq -r '.root_password // ""' "$config_file")"
    export MIRROR_COUNTRY="$(jq -r '.mirror_country // ""' "$config_file")"
    export BOOTLOADER="$(jq -r '.bootloader // ""' "$config_file")"
    export OS_PROBER="$(jq -r '.os_prober // ""' "$config_file")"
    export DESKTOP_ENVIRONMENT="$(jq -r '.desktop_environment // ""' "$config_file")"
    export DISPLAY_MANAGER="$(jq -r '.display_manager // ""' "$config_file")"
    export ADDITIONAL_PACKAGES="$(jq -r '.additional_packages // ""' "$config_file")"
    export ADDITIONAL_AUR_PACKAGES="$(jq -r '.additional_aur_packages // ""' "$config_file")"
    export AUR_HELPER="$(jq -r '.aur_helper // ""' "$config_file")"
    export PLYMOUTH="$(jq -r '.plymouth // ""' "$config_file")"
    export PLYMOUTH_THEME="$(jq -r '.plymouth_theme // ""' "$config_file")"
    export GRUB_THEMES="$(jq -r '.grub_themes // ""' "$config_file")"
    export GRUB_THEME_SELECTION="$(jq -r '.grub_theme_selection // ""' "$config_file")"
    export TIME_SYNC="$(jq -r '.time_sync // ""' "$config_file")"
    export GIT_REPOSITORY="$(jq -r '.git_repository // ""' "$config_file")"
    export GIT_REPOSITORY_URL="$(jq -r '.git_repository_url // ""' "$config_file")"
    export NUMLOCK_ON_BOOT="$(jq -r '.numlock_on_boot // ""' "$config_file")"
    export SECURE_BOOT="$(jq -r '.secure_boot // ""' "$config_file")"
    
    # Convert TUI variables to internal Bash variables (as done in install.sh)
    export ROOT_FILESYSTEM_TYPE="$ROOT_FILESYSTEM"
    export HOME_FILESYSTEM_TYPE="$HOME_FILESYSTEM"
    export WANT_HOME_PARTITION="$SEPARATE_HOME"
    export WANT_SWAP="$SWAP"
    export WANT_SEPARATE_BOOT="$([ "$BOOT_MODE" = "UEFI" ] && echo "yes" || echo "no")"
    
    log_success "Configuration loaded successfully from: $config_file"
    
    # Validate critical configuration
    validate_configuration
}

# Validate loaded configuration
validate_configuration() {
    log_info "Validating configuration..."
    
    local errors=()
    
    # Check required fields
    if [[ -z "$INSTALL_DISK" ]]; then
        errors+=("Install disk must be specified")
    fi
    
    if [[ -z "$PARTITIONING_STRATEGY" ]]; then
        errors+=("Partitioning strategy must be specified")
    fi
    
    if [[ -z "$HOSTNAME" ]]; then
        errors+=("Hostname must be specified")
    fi
    
    if [[ -z "$USERNAME" ]]; then
        errors+=("Username must be specified")
    fi
    
    if [[ -z "$USER_PASSWORD" ]]; then
        errors+=("User password must be specified")
    fi
    
    if [[ -z "$ROOT_PASSWORD" ]]; then
        errors+=("Root password must be specified")
    fi
    
    # Check disk path
    if [[ -n "$INSTALL_DISK" && ! "$INSTALL_DISK" =~ ^/dev/ ]]; then
        errors+=("Install disk must be a valid device path (e.g., /dev/sda)")
    fi
    
    # Check partitioning strategy
    local valid_strategies=("auto_simple" "auto_simple_luks" "auto_lvm" "auto_luks_lvm" "auto_raid" "auto_raid_luks" "auto_raid_lvm" "auto_raid_lvm_luks" "manual")
    if [[ -n "$PARTITIONING_STRATEGY" ]]; then
        local is_valid=false
        for strategy in "${valid_strategies[@]}"; do
            if [[ "$PARTITIONING_STRATEGY" == "$strategy" ]]; then
                is_valid=true
                break
            fi
        done
        if [[ "$is_valid" == false ]]; then
            errors+=("Invalid partitioning strategy: $PARTITIONING_STRATEGY")
        fi
    fi
    
    # Report errors
    if [[ ${#errors[@]} -gt 0 ]]; then
        log_error "Configuration validation failed:"
        for error in "${errors[@]}"; do
            log_error "  - $error"
        done
        error_exit "Invalid configuration file"
    fi
    
    log_success "Configuration validation passed"
}

# Display loaded configuration (for debugging)
display_configuration() {
    log_info "Current configuration:"
    log_info "  Boot Mode: $BOOT_MODE"
    log_info "  Install Disk: $INSTALL_DISK"
    log_info "  Partitioning Strategy: $PARTITIONING_STRATEGY"
    log_info "  Root Filesystem: $ROOT_FILESYSTEM"
    log_info "  Home Filesystem: $HOME_FILESYSTEM"
    log_info "  Separate Home: $SEPARATE_HOME"
    log_info "  Encryption: $ENCRYPTION"
    log_info "  Swap: $SWAP"
    log_info "  Hostname: $HOSTNAME"
    log_info "  Username: $USERNAME"
    log_info "  Desktop Environment: $DESKTOP_ENVIRONMENT"
    log_info "  Display Manager: $DISPLAY_MANAGER"
    log_info "  Bootloader: $BOOTLOADER"
}
