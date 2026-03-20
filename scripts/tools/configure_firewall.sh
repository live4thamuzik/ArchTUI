#!/bin/bash
# configure_firewall.sh - Configure firewall (iptables/ufw)
# Usage: ./configure_firewall.sh --action enable|disable|status|rules

set -euo pipefail

# --- Signal Handling ---
cleanup_and_exit() {
    local sig="$1"
    echo "$(basename "$0"): Received $sig, aborting..." >&2
    [[ "$sig" == "SIGTERM" ]] && exit 143 || exit 130
}
trap 'cleanup_and_exit SIGTERM' SIGTERM
trap 'cleanup_and_exit SIGINT' SIGINT

# Source common utilities via bootstrap
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../bootstrap.sh
source "$SCRIPT_DIR/../bootstrap.sh" || { echo "FATAL: Cannot source bootstrap.sh" >&2; exit 1; }
source_or_die "$SCRIPT_DIR/../utils.sh"

require_root

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
            log_cmd "pacman -Sy iptables --noconfirm"
            pacman -Sy iptables --noconfirm || error_exit "Failed to install iptables"
            ;;
        ufw)
            log_info "Installing UFW..."
            log_cmd "pacman -Sy ufw --noconfirm"
            pacman -Sy ufw --noconfirm || error_exit "Failed to install ufw"
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
            log_cmd "iptables -F && iptables -X (flush all chains)"
            iptables -F
            iptables -X
            iptables -t nat -F
            iptables -t nat -X
            iptables -t mangle -F
            iptables -t mangle -X

            # Set default policies
            log_cmd "iptables -P INPUT DROP, FORWARD DROP, OUTPUT ACCEPT"
            iptables -P INPUT DROP
            iptables -P FORWARD DROP
            iptables -P OUTPUT ACCEPT

            # Allow loopback
            log_cmd "iptables -A INPUT -i lo -j ACCEPT"
            iptables -A INPUT -i lo -j ACCEPT
            iptables -A OUTPUT -o lo -j ACCEPT

            # Allow established connections (conntrack is the modern replacement for state)
            log_cmd "iptables -A INPUT -m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT"
            iptables -A INPUT -m conntrack --ctstate ESTABLISHED,RELATED -j ACCEPT

            # Allow SSH (if running)
            if systemctl is-active sshd >/dev/null 2>&1; then
                log_cmd "iptables -A INPUT -p tcp --dport 22 -j ACCEPT"
                iptables -A INPUT -p tcp --dport 22 -j ACCEPT
                log_info "Allowed SSH (port 22)"
            fi

            # Allow ping
            log_cmd "iptables -A INPUT -p icmp --icmp-type echo-request -j ACCEPT"
            iptables -A INPUT -p icmp --icmp-type echo-request -j ACCEPT

            log_success "Basic iptables rules configured"
            ;;

        disable)
            log_info "Disabling iptables (setting permissive rules)..."
            log_cmd "iptables -F && iptables -P INPUT/FORWARD/OUTPUT ACCEPT"
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
            log_cmd "ufw --force reset"
            ufw --force reset

            # Set default policies
            log_cmd "ufw default deny incoming && ufw default allow outgoing"
            ufw default deny incoming
            ufw default allow outgoing

            # Allow SSH
            log_cmd "ufw allow 22/tcp"
            ufw allow 22/tcp

            # Enable UFW
            log_cmd "ufw --force enable"
            ufw --force enable

            log_success "UFW enabled with basic rules"
            ;;

        disable)
            log_info "Disabling UFW..."
            log_cmd "ufw --force disable"
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
                log_cmd "iptables -A INPUT -p $PROTOCOL --dport $PORT -j ACCEPT"
                iptables -A INPUT -p "$PROTOCOL" --dport "$PORT" -j ACCEPT
                log_success "Port $PORT allowed"
            elif [[ "$DENY" == true ]]; then
                log_info "Denying $PROTOCOL port $PORT in iptables..."
                log_cmd "iptables -A INPUT -p $PROTOCOL --dport $PORT -j DROP"
                iptables -A INPUT -p "$PROTOCOL" --dport "$PORT" -j DROP
                log_success "Port $PORT denied"
            fi
            ;;
        ufw)
            if [[ "$ALLOW" == true ]]; then
                log_info "Allowing $PROTOCOL port $PORT in UFW..."
                log_cmd "ufw allow $PORT/$PROTOCOL"
                ufw allow "$PORT/$PROTOCOL"
                log_success "Port $PORT allowed"
            elif [[ "$DENY" == true ]]; then
                log_info "Denying $PROTOCOL port $PORT in UFW..."
                log_cmd "ufw deny $PORT/$PROTOCOL"
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
