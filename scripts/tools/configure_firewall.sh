#!/bin/bash
# configure_firewall.sh - Configure firewall (iptables/ufw)
# Usage: ./configure_firewall.sh --action enable|disable|status|rules

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
FIREWALL_TYPE="iptables"
PORT=""
PROTOCOL="tcp"
ALLOW=false
DENY=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --action)
            ACTION="$2"
            shift 2
            ;;
        --type)
            FIREWALL_TYPE="$2"
            shift 2
            ;;
        --port)
            PORT="$2"
            shift 2
            ;;
        --protocol)
            PROTOCOL="$2"
            shift 2
            ;;
        --allow)
            ALLOW=true
            shift
            ;;
        --deny)
            DENY=true
            shift
            ;;
        --help)
            echo "Usage: $0 --action <action> [options]"
            echo "Configure firewall"
            echo "Actions: enable, disable, status, rules, install"
            echo "Options:"
            echo "  --type <iptables|ufw>     Firewall type (default: iptables)"
            echo "  --port <port>            Port number"
            echo "  --protocol <tcp|udp>     Protocol (default: tcp)"
            echo "  --allow                  Allow the specified port"
            echo "  --deny                   Deny the specified port"
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
    error_exit "Action is required (--action enable|disable|status|rules|install)"
fi

# Function to install firewall tools
install_firewall() {
    case "$FIREWALL_TYPE" in
        iptables)
            log_info "Installing iptables..."
            pacman -Sy --noconfirm iptables
            ;;
        ufw)
            log_info "Installing UFW..."
            pacman -Sy --noconfirm ufw
            ;;
        *)
            error_exit "Invalid firewall type: $FIREWALL_TYPE. Use iptables or ufw"
            ;;
    esac
}

# Function to configure iptables
configure_iptables() {
    case "$ACTION" in
        enable)
            log_info "Configuring basic iptables rules..."
            
            # Flush existing rules
            iptables -F
            iptables -X
            iptables -t nat -F
            iptables -t nat -X
            iptables -t mangle -F
            iptables -t mangle -X
            
            # Set default policies
            iptables -P INPUT DROP
            iptables -P FORWARD DROP
            iptables -P OUTPUT ACCEPT
            
            # Allow loopback
            iptables -A INPUT -i lo -j ACCEPT
            iptables -A OUTPUT -o lo -j ACCEPT
            
            # Allow established connections
            iptables -A INPUT -m state --state ESTABLISHED,RELATED -j ACCEPT
            
            # Allow SSH (if running)
            if systemctl is-active sshd >/dev/null 2>&1; then
                iptables -A INPUT -p tcp --dport 22 -j ACCEPT
                log_info "Allowed SSH (port 22)"
            fi
            
            # Allow ping
            iptables -A INPUT -p icmp --icmp-type echo-request -j ACCEPT
            
            log_success "Basic iptables rules configured"
            ;;
        
        disable)
            log_info "Disabling iptables (setting permissive rules)..."
            iptables -F
            iptables -P INPUT ACCEPT
            iptables -P FORWARD ACCEPT
            iptables -P OUTPUT ACCEPT
            log_success "iptables disabled"
            ;;
        
        status)
            log_info "iptables status:"
            iptables -L -n -v
            ;;
        
        rules)
            log_info "Current iptables rules:"
            iptables -L -n --line-numbers
            ;;
    esac
}

# Function to configure UFW
configure_ufw() {
    case "$ACTION" in
        enable)
            log_info "Enabling UFW..."
            
            # Reset UFW
            ufw --force reset
            
            # Set default policies
            ufw default deny incoming
            ufw default allow outgoing
            
            # Allow SSH
            ufw allow 22/tcp
            
            # Allow ping
            ufw allow in on any to any port 22 proto tcp
            
            # Enable UFW
            ufw --force enable
            
            log_success "UFW enabled with basic rules"
            ;;
        
        disable)
            log_info "Disabling UFW..."
            ufw --force disable
            log_success "UFW disabled"
            ;;
        
        status)
            log_info "UFW status:"
            ufw status verbose
            ;;
        
        rules)
            log_info "UFW rules:"
            ufw status numbered
            ;;
    esac
}

# Function to manage port rules
manage_port_rules() {
    if [[ -z "$PORT" ]]; then
        error_exit "Port is required for port management (--port <port>)"
    fi
    
    case "$FIREWALL_TYPE" in
        iptables)
            if [[ "$ALLOW" == true ]]; then
                log_info "Allowing $PROTOCOL port $PORT in iptables..."
                iptables -A INPUT -p "$PROTOCOL" --dport "$PORT" -j ACCEPT
                log_success "Port $PORT allowed"
            elif [[ "$DENY" == true ]]; then
                log_info "Denying $PROTOCOL port $PORT in iptables..."
                iptables -A INPUT -p "$PROTOCOL" --dport "$PORT" -j DROP
                log_success "Port $PORT denied"
            fi
            ;;
        ufw)
            if [[ "$ALLOW" == true ]]; then
                log_info "Allowing $PROTOCOL port $PORT in UFW..."
                ufw allow "$PORT/$PROTOCOL"
                log_success "Port $PORT allowed"
            elif [[ "$DENY" == true ]]; then
                log_info "Denying $PROTOCOL port $PORT in UFW..."
                ufw deny "$PORT/$PROTOCOL"
                log_success "Port $PORT denied"
            fi
            ;;
    esac
}

# Main execution
case "$ACTION" in
    install)
        install_firewall
        ;;
    
    enable|disable|status|rules)
        if [[ "$ALLOW" == true ]] || [[ "$DENY" == true ]]; then
            manage_port_rules
        else
            case "$FIREWALL_TYPE" in
                iptables)
                    configure_iptables
                    ;;
                ufw)
                    configure_ufw
                    ;;
                *)
                    error_exit "Invalid firewall type: $FIREWALL_TYPE"
                    ;;
            esac
        fi
        ;;
    
    *)
        error_exit "Invalid action: $ACTION. Use enable, disable, status, rules, or install"
        ;;
esac

# Show final status
echo
log_info "Firewall status summary:"
case "$FIREWALL_TYPE" in
    iptables)
        if iptables -L INPUT | grep -q "DROP\|REJECT"; then
            log_success "iptables is active with restrictive rules"
        else
            log_warning "iptables is permissive or not configured"
        fi
        ;;
    ufw)
        if ufw status | grep -q "Status: active"; then
            log_success "UFW is active"
        else
            log_warning "UFW is not active"
        fi
        ;;
esac
