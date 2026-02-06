#!/bin/bash
# configure_ssh.sh - Configure SSH server
# Usage: ./configure_ssh.sh --action install|configure|enable|disable

set -euo pipefail

# --- Signal Handling ---
cleanup_and_exit() {
    local sig="$1"
    echo "$(basename "$0"): Received $sig, aborting..." >&2
    exit 130
}
trap 'cleanup_and_exit SIGTERM' SIGTERM
trap 'cleanup_and_exit SIGINT' SIGINT

# Source common utilities via source_or_die
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source_or_die() {
    local script_path="$1"
    local error_msg="${2:-Failed to source required script: $script_path}"
    if [[ ! -f "$script_path" ]]; then
        echo "FATAL: $error_msg (file not found)" >&2
        exit 1
    fi
    # shellcheck source=/dev/null
    if ! source "$script_path"; then
        echo "FATAL: $error_msg (source failed)" >&2
        exit 1
    fi
}
source_or_die "$SCRIPT_DIR/../utils.sh"

# Default values
ACTION=""
ENABLE_ROOT_LOGIN=false
DISABLE_ROOT_LOGIN=false
ENABLE_PASSWORD_AUTH=true
DISABLE_PASSWORD_AUTH=false
PORT=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --action)
            ACTION="$2"
            shift 2
            ;;
        --port)
            PORT="$2"
            shift 2
            ;;
        --enable-root-login)
            ENABLE_ROOT_LOGIN=true
            shift
            ;;
        --disable-root-login)
            DISABLE_ROOT_LOGIN=true
            shift
            ;;
        --enable-password-auth)
            ENABLE_PASSWORD_AUTH=true
            shift
            ;;
        --disable-password-auth)
            DISABLE_PASSWORD_AUTH=true
            shift
            ;;
        --help)
            echo "Usage: $0 --action <action> [options]"
            echo "Configure SSH server"
            echo "Actions: install, configure, enable, disable, status"
            echo "Options:"
            echo "  --port <port>              Set SSH port"
            echo "  --enable-root-login        Enable root login"
            echo "  --disable-root-login       Disable root login"
            echo "  --enable-password-auth     Enable password authentication"
            echo "  --disable-password-auth    Disable password authentication"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate required arguments
if [[ -z "$ACTION" ]]; then
    error_exit "Action is required (--action install|configure|enable|disable|status)"
fi

case "$ACTION" in
    install)
        log_info "Installing OpenSSH server..."
        if pacman -Sy --noconfirm openssh; then
            log_success "OpenSSH server installed successfully"
        else
            log_error "Failed to install OpenSSH server"
            exit 1
        fi
        
        # Generate host keys if they don't exist
        if [[ ! -f /etc/ssh/ssh_host_rsa_key ]]; then
            log_info "Generating SSH host keys..."
            ssh-keygen -A
        fi
        ;;
    
    enable)
        log_info "Enabling SSH service..."
        if systemctl enable sshd; then
            log_success "SSH service enabled successfully"
        else
            log_error "Failed to enable SSH service"
            exit 1
        fi
        
        if systemctl start sshd; then
            log_success "SSH service started successfully"
        else
            log_error "Failed to start SSH service"
            exit 1
        fi
        ;;
    
    disable)
        log_info "Disabling SSH service..."
        if systemctl stop sshd; then
            log_success "SSH service stopped successfully"
        else
            log_warning "SSH service was not running"
        fi
        
        if systemctl disable sshd; then
            log_success "SSH service disabled successfully"
        else
            log_error "Failed to disable SSH service"
            exit 1
        fi
        ;;
    
    configure)
        log_info "Configuring SSH server..."
        
        # Backup original config
        if [[ ! -f /etc/ssh/sshd_config.backup ]]; then
            cp /etc/ssh/sshd_config /etc/ssh/sshd_config.backup
            log_info "Backed up original SSH config to /etc/ssh/sshd_config.backup"
        fi
        
        # Configure port if specified
        if [[ -n "$PORT" ]]; then
            log_info "Setting SSH port to $PORT"
            sed -i "s/^#Port 22/Port $PORT/" /etc/ssh/sshd_config
            sed -i "s/^Port [0-9]*/Port $PORT/" /etc/ssh/sshd_config
        fi
        
        # Configure root login
        if [[ "$DISABLE_ROOT_LOGIN" == true ]]; then
            log_info "Disabling root login"
            sed -i 's/^#PermitRootLogin.*/PermitRootLogin no/' /etc/ssh/sshd_config
            sed -i 's/^PermitRootLogin.*/PermitRootLogin no/' /etc/ssh/sshd_config
        elif [[ "$ENABLE_ROOT_LOGIN" == true ]]; then
            log_info "Enabling root login"
            sed -i 's/^#PermitRootLogin.*/PermitRootLogin yes/' /etc/ssh/sshd_config
            sed -i 's/^PermitRootLogin.*/PermitRootLogin yes/' /etc/ssh/sshd_config
        fi
        
        # Configure password authentication
        if [[ "$DISABLE_PASSWORD_AUTH" == true ]]; then
            log_info "Disabling password authentication"
            sed -i 's/^#PasswordAuthentication.*/PasswordAuthentication no/' /etc/ssh/sshd_config
            sed -i 's/^PasswordAuthentication.*/PasswordAuthentication no/' /etc/ssh/sshd_config
        elif [[ "$ENABLE_PASSWORD_AUTH" == true ]]; then
            log_info "Enabling password authentication"
            sed -i 's/^#PasswordAuthentication.*/PasswordAuthentication yes/' /etc/ssh/sshd_config
            sed -i 's/^PasswordAuthentication.*/PasswordAuthentication yes/' /etc/ssh/sshd_config
        fi
        
        # Basic security settings
        log_info "Applying basic security settings..."
        sed -i 's/^#Protocol 2/Protocol 2/' /etc/ssh/sshd_config
        sed -i 's/^Protocol.*/Protocol 2/' /etc/ssh/sshd_config
        
        # Restart SSH service to apply changes
        if systemctl is-active sshd >/dev/null 2>&1; then
            log_info "Restarting SSH service to apply changes..."
            systemctl restart sshd
            log_success "SSH configuration updated successfully"
        else
            log_info "SSH service is not running. Configuration saved."
        fi
        ;;
    
    status)
        log_info "SSH Service Status:"
        if systemctl is-active sshd >/dev/null 2>&1; then
            log_success "SSH service is running"
        else
            log_warning "SSH service is not running"
        fi
        
        if systemctl is-enabled sshd >/dev/null 2>&1; then
            log_success "SSH service is enabled"
        else
            log_warning "SSH service is not enabled"
        fi
        
        echo
        log_info "SSH Configuration:"
        if [[ -f /etc/ssh/sshd_config ]]; then
            echo "Port: $(grep '^Port' /etc/ssh/sshd_config | awk '{print $2}' || echo '22 (default)')"
            echo "Root Login: $(grep '^PermitRootLogin' /etc/ssh/sshd_config | awk '{print $2}' || echo 'yes (default)')"
            echo "Password Auth: $(grep '^PasswordAuthentication' /etc/ssh/sshd_config | awk '{print $2}' || echo 'yes (default)')"
        else
            log_error "SSH configuration file not found"
        fi
        ;;
    
    *)
        error_exit "Invalid action: $ACTION. Use install, configure, enable, disable, or status"
        ;;
esac
