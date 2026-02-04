#!/bin/bash
# config_loader.sh - JSON Configuration File Parser for Bash
# This script provides functions to load configuration from JSON files

set -euo pipefail

# Source utility functions via source_or_die pattern
_CONFIG_LOADER_SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
if [[ ! -f "$_CONFIG_LOADER_SCRIPT_DIR/utils.sh" ]]; then
    echo "FATAL: Required script not found: $_CONFIG_LOADER_SCRIPT_DIR/utils.sh" >&2
    exit 1
fi
# shellcheck source=/dev/null
if ! source "$_CONFIG_LOADER_SCRIPT_DIR/utils.sh"; then
    echo "FATAL: Failed to source: $_CONFIG_LOADER_SCRIPT_DIR/utils.sh" >&2
    exit 1
fi

# NOTE: jq is only required for JSON config file mode, not for TUI mode
# TUI mode passes all configuration as environment variables

# Check if jq is available (called only when loading JSON config files)
check_jq_available() {
    if ! command -v jq >/dev/null 2>&1; then
        log_error "jq is required for parsing JSON configuration files"
        log_info "Note: The TUI (archtui) does not require jq - only direct bash script usage needs it"
        log_info "Attempting to install jq automatically..."
        if command -v pacman >/dev/null 2>&1; then
            pacman -Sy --noconfirm jq
            if ! command -v jq >/dev/null 2>&1; then
                error_exit "Failed to install jq. Run: pacman -S jq"
            fi
            log_info "Successfully installed jq"
        else
            error_exit "jq is not installed and pacman is unavailable. Install with: pacman -S jq or apt-get install jq"
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
    # Note: Variable names must match what install.sh and chroot_config.sh expect
    export BOOT_MODE="$(jq -r '.boot_mode // "UEFI"' "$config_file")"
    export INSTALL_DISK="$(jq -r '.install_disk // ""' "$config_file")"
    export PARTITIONING_STRATEGY="$(jq -r '.partitioning_strategy // "auto_simple"' "$config_file")"
    export ROOT_FILESYSTEM="$(jq -r '.root_filesystem // "ext4"' "$config_file")"
    export HOME_FILESYSTEM="$(jq -r '.home_filesystem // "ext4"' "$config_file")"
    export SEPARATE_HOME="$(jq -r '.separate_home // "no"' "$config_file")"
    export ENCRYPTION="$(jq -r '.encryption // "no"' "$config_file")"
    export ENCRYPTION_PASSWORD="$(jq -r '.encryption_password // ""' "$config_file")"
    export SWAP="$(jq -r '.swap // "yes"' "$config_file")"
    export SWAP_SIZE="$(jq -r '.swap_size // "2GB"' "$config_file")"
    export TIMEZONE_REGION="$(jq -r '.timezone_region // "UTC"' "$config_file")"
    export TIMEZONE="$(jq -r '.timezone // "UTC"' "$config_file")"
    export LOCALE="$(jq -r '.locale // "en_US.UTF-8"' "$config_file")"
    export KEYMAP="$(jq -r '.keymap // "us"' "$config_file")"
    export KERNEL="$(jq -r '.kernel // "linux"' "$config_file")"

    # Use SYSTEM_HOSTNAME to avoid conflicts with shell's HOSTNAME
    export SYSTEM_HOSTNAME="$(jq -r '.hostname // "archlinux"' "$config_file")"

    # Use MAIN_USERNAME for the primary user account
    export MAIN_USERNAME="$(jq -r '.username // ""' "$config_file")"
    export USER_PASSWORD="$(jq -r '.user_password // ""' "$config_file")"
    export ROOT_PASSWORD="$(jq -r '.root_password // ""' "$config_file")"

    export MIRROR_COUNTRY="$(jq -r '.mirror_country // ""' "$config_file")"
    export BOOTLOADER="$(jq -r '.bootloader // "systemd-boot"' "$config_file")"
    export OS_PROBER="$(jq -r '.os_prober // "no"' "$config_file")"
    export DESKTOP_ENVIRONMENT="$(jq -r '.desktop_environment // "none"' "$config_file")"
    export DISPLAY_MANAGER="$(jq -r '.display_manager // "none"' "$config_file")"
    export ADDITIONAL_PACKAGES="$(jq -r '.additional_packages // ""' "$config_file")"
    export ADDITIONAL_AUR_PACKAGES="$(jq -r '.additional_aur_packages // ""' "$config_file")"
    export AUR_HELPER="$(jq -r '.aur_helper // "paru"' "$config_file")"
    export PLYMOUTH="$(jq -r '.plymouth // "no"' "$config_file")"
    export PLYMOUTH_THEME="$(jq -r '.plymouth_theme // ""' "$config_file")"
    export GRUB_THEMES="$(jq -r '.grub_themes // "no"' "$config_file")"
    export GRUB_THEME_SELECTION="$(jq -r '.grub_theme_selection // ""' "$config_file")"
    export TIME_SYNC="$(jq -r '.time_sync // "yes"' "$config_file")"
    export GIT_REPOSITORY="$(jq -r '.git_repository // "no"' "$config_file")"
    export GIT_REPOSITORY_URL="$(jq -r '.git_repository_url // ""' "$config_file")"
    export NUMLOCK_ON_BOOT="$(jq -r '.numlock_on_boot // "no"' "$config_file")"
    export SECURE_BOOT="$(jq -r '.secure_boot // "no"' "$config_file")"

    # Convert TUI variables to internal Bash variables (as done in install.sh)
    export ROOT_FILESYSTEM_TYPE="$ROOT_FILESYSTEM"
    export HOME_FILESYSTEM_TYPE="$HOME_FILESYSTEM"
    export WANT_HOME_PARTITION="$SEPARATE_HOME"
    export WANT_SWAP="$SWAP"
    export WANT_SEPARATE_BOOT="$([ "$BOOT_MODE" = "UEFI" ] && echo "yes" || echo "no")"

    # Legacy compatibility aliases
    export HOSTNAME="$SYSTEM_HOSTNAME"
    export USERNAME="$MAIN_USERNAME"
    
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

    if [[ -z "$SYSTEM_HOSTNAME" ]]; then
        errors+=("Hostname must be specified")
    fi

    if [[ -z "$MAIN_USERNAME" ]]; then
        errors+=("Username must be specified")
    fi

    if [[ -z "$USER_PASSWORD" ]]; then
        errors+=("User password must be specified")
    fi

    if [[ -z "$ROOT_PASSWORD" ]]; then
        errors+=("Root password must be specified")
    fi

    # Check encryption password if encryption is enabled
    if [[ "$ENCRYPTION" == "yes" && -z "$ENCRYPTION_PASSWORD" ]]; then
        errors+=("Encryption password must be specified when encryption is enabled")
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
    log_info "  Kernel: $KERNEL"
    log_info "  Root Filesystem: $ROOT_FILESYSTEM"
    log_info "  Home Filesystem: $HOME_FILESYSTEM"
    log_info "  Separate Home: $SEPARATE_HOME"
    log_info "  Encryption: $ENCRYPTION"
    log_info "  Swap: $SWAP"
    log_info "  Hostname: $SYSTEM_HOSTNAME"
    log_info "  Username: $MAIN_USERNAME"
    log_info "  Desktop Environment: $DESKTOP_ENVIRONMENT"
    log_info "  Display Manager: $DISPLAY_MANAGER"
    log_info "  Bootloader: $BOOTLOADER"
    log_info "  AUR Helper: $AUR_HELPER"
}
