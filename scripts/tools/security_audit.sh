#!/bin/bash
# security_audit.sh - Perform system security audit
# Usage: ./security_audit.sh --action basic|full

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
ACTION="basic"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --action)
            ACTION="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 --action <basic|full>"
            echo "Perform system security audit"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

log_info "Starting security audit (mode: $ACTION)"

# Function to check file permissions
check_file_permissions() {
    log_info "Checking critical file permissions..."
    
    # Check /etc/passwd
    if [[ -f /etc/passwd ]]; then
        local perms=$(stat -c "%a" /etc/passwd)
        if [[ "$perms" != "644" ]]; then
            log_warning "/etc/passwd has incorrect permissions: $perms (should be 644)"
        else
            log_success "/etc/passwd permissions OK: $perms"
        fi
    fi
    
    # Check /etc/shadow
    if [[ -f /etc/shadow ]]; then
        local perms=$(stat -c "%a" /etc/shadow)
        if [[ "$perms" != "640" ]]; then
            log_warning "/etc/shadow has incorrect permissions: $perms (should be 640)"
        else
            log_success "/etc/shadow permissions OK: $perms"
        fi
    fi
    
    # Check /etc/sudoers
    if [[ -f /etc/sudoers ]]; then
        local perms=$(stat -c "%a" /etc/sudoers)
        if [[ "$perms" != "440" ]]; then
            log_warning "/etc/sudoers has incorrect permissions: $perms (should be 440)"
        else
            log_success "/etc/sudoers permissions OK: $perms"
        fi
    fi
}

# Function to check user accounts
check_user_accounts() {
    log_info "Checking user accounts..."
    
    # Check for users with UID 0 (should only be root)
    local uid0_users=$(awk -F: '$3==0 {print $1}' /etc/passwd)
    if [[ $(echo "$uid0_users" | wc -l) -gt 1 ]] || [[ "$uid0_users" != "root" ]]; then
        log_warning "Multiple users with UID 0 found: $uid0_users"
    else
        log_success "Only root has UID 0"
    fi
    
    # Check for users without passwords
    local no_password_users=$(awk -F: '($2 == "" || $2 == "!") {print $1}' /etc/shadow)
    if [[ -n "$no_password_users" ]]; then
        log_warning "Users without passwords: $no_password_users"
    else
        log_success "All users have passwords set"
    fi
    
    # Check for users with shell access
    local shell_users=$(awk -F: '$7 !~ /nologin|false/ {print $1}' /etc/passwd)
    log_info "Users with shell access: $shell_users"
}

# Function to check network services
check_network_services() {
    log_info "Checking network services..."
    
    # Check listening ports
    if command -v netstat >/dev/null 2>&1; then
        log_info "Listening ports:"
        netstat -tlnp | grep LISTEN || log_info "No listening ports found"
    elif command -v ss >/dev/null 2>&1; then
        log_info "Listening ports:"
        ss -tlnp | grep LISTEN || log_info "No listening ports found"
    fi
    
    # Check SSH configuration
    if systemctl is-active sshd >/dev/null 2>&1; then
        log_warning "SSH service is running"
        if [[ -f /etc/ssh/sshd_config ]]; then
            local root_login=$(grep '^PermitRootLogin' /etc/ssh/sshd_config | awk '{print $2}')
            if [[ "$root_login" == "yes" ]]; then
                log_warning "SSH root login is enabled"
            else
                log_success "SSH root login is disabled"
            fi
        fi
    else
        log_success "SSH service is not running"
    fi
}

# Function to check installed packages
check_packages() {
    log_info "Checking installed packages..."
    
    # Check for common security tools
    local security_tools=("firewalld" "ufw" "fail2ban" "rkhunter" "chkrootkit")
    for tool in "${security_tools[@]}"; do
        if pacman -Qi "$tool" >/dev/null 2>&1; then
            log_success "Security tool installed: $tool"
        else
            log_info "Security tool not installed: $tool"
        fi
    done
}

# Function to check system updates
check_updates() {
    log_info "Checking system updates..."
    
    # Check for available updates
    if pacman -Qu >/dev/null 2>&1; then
        local update_count=$(pacman -Qu | wc -l)
        if [[ "$update_count" -gt 0 ]]; then
            log_warning "$update_count packages have updates available"
            if [[ "$ACTION" == "full" ]]; then
                log_info "Available updates:"
                pacman -Qu | head -10
            fi
        else
            log_success "System is up to date"
        fi
    else
        log_info "Could not check for updates (pacman database might be locked)"
    fi
}

# Function to check firewall status
check_firewall() {
    log_info "Checking firewall status..."
    
    # Check for active firewall
    if systemctl is-active firewalld >/dev/null 2>&1; then
        log_success "Firewalld is active"
    elif systemctl is-active ufw >/dev/null 2>&1; then
        log_success "UFW is active"
    elif iptables -L INPUT | grep -q "DROP\|REJECT"; then
        log_success "iptables rules are configured"
    else
        log_warning "No active firewall detected"
    fi
}

# Main audit execution
echo "=========================================="
log_info "System Security Audit - $(date)"
echo "=========================================="

check_file_permissions
echo

check_user_accounts
echo

check_network_services
echo

check_packages
echo

check_updates
echo

check_firewall
echo

# Summary
echo "=========================================="
log_info "Security audit completed"

if [[ "$ACTION" == "full" ]]; then
    log_info "Full audit mode - detailed information provided above"
    log_info "Recommendations:"
    log_info "1. Keep system updated regularly"
    log_info "2. Use strong passwords"
    log_info "3. Configure firewall"
    log_info "4. Disable unnecessary services"
    log_info "5. Monitor system logs"
fi

log_success "Audit finished successfully"
