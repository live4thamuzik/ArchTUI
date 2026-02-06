#!/bin/bash
# configure_network.sh - Configure network interface using ISO tools
# Usage: ./configure_network.sh --interface <iface> [options]

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
INTERFACE=""
ACTION=""
IP_ADDRESS=""
NETMASK=""
GATEWAY=""
DNS_SERVERS=""
DHCP=false
STATIC=false
ENABLE=false
DISABLE=false
STATUS=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --interface)
            INTERFACE="$2"
            shift 2
            ;;
        --action)
            ACTION="$2"
            shift 2
            ;;
        --ip)
            IP_ADDRESS="$2"
            shift 2
            ;;
        --netmask)
            NETMASK="$2"
            shift 2
            ;;
        --gateway)
            GATEWAY="$2"
            shift 2
            ;;
        --dns)
            DNS_SERVERS="$2"
            shift 2
            ;;
        --dhcp)
            DHCP=true
            shift
            ;;
        --static)
            STATIC=true
            shift
            ;;
        --enable)
            ENABLE=true
            shift
            ;;
        --disable)
            DISABLE=true
            shift
            ;;
        --status)
            STATUS=true
            shift
            ;;
        --help)
            echo "Usage: $0 --interface <iface> [--action <action>] [options]"
            echo ""
            echo "Required:"
            echo "  --interface <iface>   Network interface name (e.g., eth0, enp0s3)"
            echo ""
            echo "Actions:"
            echo "  --action configure    Configure network interface"
            echo "  --action enable       Enable network interface"
            echo "  --action disable      Disable network interface"
            echo "  --action status       Show interface status"
            echo "  --action info         Show detailed interface information"
            echo ""
            echo "Configuration Options:"
            echo "  --dhcp                Use DHCP for IP configuration"
            echo "  --static              Use static IP configuration"
            echo "  --ip <address>        Static IP address"
            echo "  --netmask <mask>      Network mask (e.g., 255.255.255.0)"
            echo "  --gateway <gateway>   Default gateway"
            echo "  --dns <servers>       DNS servers (comma-separated)"
            echo ""
            echo "Examples:"
            echo "  $0 --interface eth0 --action status"
            echo "  $0 --interface eth0 --action configure --dhcp"
            echo "  $0 --interface eth0 --action configure --static --ip 192.168.1.100 --netmask 255.255.255.0 --gateway 192.168.1.1"
            echo "  $0 --interface eth0 --action enable"
            echo "  $0 --interface eth0 --action info"
            echo ""
            echo "Note: Uses tools available on Arch ISO (ip, dhcpcd, systemctl)"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate required arguments
if [[ -z "$INTERFACE" ]]; then
    error_exit "Interface is required (--interface <iface>)"
fi

# Validate interface exists
if ! ip link show "$INTERFACE" >/dev/null 2>&1; then
    error_exit "Interface '$INTERFACE' does not exist"
fi

# Auto-detect action if not specified
if [[ -z "$ACTION" ]]; then
    if [[ "$DHCP" == true || "$STATIC" == true ]]; then
        ACTION="configure"
    elif [[ "$ENABLE" == true ]]; then
        ACTION="enable"
    elif [[ "$DISABLE" == true ]]; then
        ACTION="disable"
    elif [[ "$STATUS" == true ]]; then
        ACTION="status"
    else
        ACTION="info"  # Default to info
    fi
fi

log_info "🌐 Network Configuration Tool (ISO Compatible)"
echo "=================================================="
log_info "Interface: $INTERFACE"
log_info "Action: $ACTION"
echo "=================================================="

# Function to get interface status
get_interface_status() {
    local iface="$1"
    
    echo "📊 Interface Status: $iface"
    echo "--------------------------------------------------"
    
    # Basic interface information
    if ip link show "$iface" >/dev/null 2>&1; then
        echo "Interface: $iface"
        
        # Get interface state
        local state=$(ip link show "$iface" | grep -o "state [A-Z]*" | cut -d' ' -f2)
        echo "State: $state"
        
        # Get MAC address
        local mac=$(ip link show "$iface" | grep -o "link/ether [a-f0-9:]*" | cut -d' ' -f2)
        if [[ -n "$mac" ]]; then
            echo "MAC Address: $mac"
        fi
        
        # Get IP configuration
        if ip addr show "$iface" >/dev/null 2>&1; then
            echo ""
            echo "IP Configuration:"
            ip addr show "$iface" | grep -E "(inet |inet6 )" | sed 's/^/  /'
        fi
        
        # Get routing information
        echo ""
        echo "Routing:"
        ip route show dev "$iface" | sed 's/^/  /'
        
    else
        echo "Interface '$iface' not found"
        return 1
    fi
}

# Function to show detailed interface information
show_interface_info() {
    local iface="$1"
    
    echo "📋 Detailed Interface Information: $iface"
    echo "=================================================="
    
    # Basic link information
    echo "🔗 Link Information:"
    ip link show "$iface" | sed 's/^/  /'
    
    echo ""
    echo "🌐 Address Information:"
    ip addr show "$iface" | sed 's/^/  /'
    
    echo ""
    echo "🛣️  Routing Information:"
    ip route show dev "$iface" | sed 's/^/  /'
    
    # Show statistics if available
    if [[ -r "/sys/class/net/$iface/statistics/rx_bytes" ]]; then
        echo ""
        echo "📊 Interface Statistics:"
        local rx_bytes=$(cat "/sys/class/net/$iface/statistics/rx_bytes")
        local tx_bytes=$(cat "/sys/class/net/$iface/statistics/tx_bytes")
        local rx_packets=$(cat "/sys/class/net/$iface/statistics/rx_packets")
        local tx_packets=$(cat "/sys/class/net/$iface/statistics/tx_packets")
        
        echo "  RX Bytes: $rx_bytes"
        echo "  TX Bytes: $tx_bytes"
        echo "  RX Packets: $rx_packets"
        echo "  TX Packets: $tx_packets"
    fi
    
    # Show available network managers
    echo ""
    echo "🔧 Available Network Managers:"
    if command -v systemctl >/dev/null 2>&1; then
        if systemctl is-active NetworkManager >/dev/null 2>&1; then
            echo "  NetworkManager: Active"
        elif systemctl is-enabled NetworkManager >/dev/null 2>&1; then
            echo "  NetworkManager: Enabled (not active)"
        else
            echo "  NetworkManager: Available"
        fi
        
        if systemctl is-active dhcpcd >/dev/null 2>&1; then
            echo "  dhcpcd: Active"
        elif systemctl is-enabled dhcpcd >/dev/null 2>&1; then
            echo "  dhcpcd: Enabled (not active)"
        else
            echo "  dhcpcd: Available"
        fi
    else
        echo "  systemctl not available"
    fi
}

# Function to enable interface
enable_interface() {
    local iface="$1"
    
    log_info "🔌 Enabling interface '$iface'..."
    
    if ip link set "$iface" up; then
        log_success "✅ Interface '$iface' enabled"
        
        # Wait a moment for interface to come up
        sleep 2
        
        # Show new status
        get_interface_status "$iface"
    else
        error_exit "Failed to enable interface '$iface'"
    fi
}

# Function to disable interface
disable_interface() {
    local iface="$1"
    
    log_info "🔌 Disabling interface '$iface'..."
    
    if ip link set "$iface" down; then
        log_success "✅ Interface '$iface' disabled"
        
        # Show new status
        get_interface_status "$iface"
    else
        error_exit "Failed to disable interface '$iface'"
    fi
}

# Function to configure interface with DHCP
configure_dhcp() {
    local iface="$1"
    
    log_info "🔧 Configuring '$iface' with DHCP..."
    
    # Enable interface first
    ip link set "$iface" up
    
    # Try dhcpcd if available
    if command -v dhcpcd >/dev/null 2>&1; then
        log_info "Using dhcpcd for DHCP configuration..."
        if dhcpcd "$iface"; then
            log_success "✅ DHCP configuration successful"
        else
            log_warning "⚠️  dhcpcd failed, trying alternative method"
            # Try basic DHCP client
            if command -v dhclient >/dev/null 2>&1; then
                dhclient "$iface"
            fi
        fi
    else
        log_info "dhcpcd not available, using basic configuration..."
        # Just bring interface up - system may auto-configure
        ip link set "$iface" up
        log_info "Interface enabled - may auto-configure via system DHCP"
    fi
    
    # Wait for configuration
    sleep 3
    
    # Show new configuration
    get_interface_status "$iface"
}

# Function to configure interface with static IP
configure_static() {
    local iface="$1"
    local ip="$2"
    local netmask="$3"
    local gateway="$4"
    local dns="$5"
    
    log_info "🔧 Configuring '$iface' with static IP..."
    
    # Enable interface
    ip link set "$iface" up
    
    # Configure IP address
    log_info "Setting IP address: $ip"
    if ip addr add "$ip/$netmask" dev "$iface"; then
        log_success "✅ IP address configured"
    else
        error_exit "Failed to configure IP address"
    fi
    
    # Configure gateway if provided
    if [[ -n "$gateway" ]]; then
        log_info "Setting gateway: $gateway"
        if ip route add default via "$gateway" dev "$iface"; then
            log_success "✅ Gateway configured"
        else
            log_warning "⚠️  Failed to configure gateway"
        fi
    fi
    
    # Configure DNS if provided
    if [[ -n "$dns" ]]; then
        log_info "Setting DNS servers: $dns"
        # Note: This is a temporary configuration
        # For persistent DNS, /etc/resolv.conf would need to be configured
        echo "nameserver $(echo "$dns" | cut -d',' -f1)" > /etc/resolv.conf
        if [[ "$dns" == *,* ]]; then
            echo "nameserver $(echo "$dns" | cut -d',' -f2)" >> /etc/resolv.conf
        fi
        log_success "✅ DNS servers configured (temporary)"
    fi
    
    # Show new configuration
    get_interface_status "$iface"
}

# Main execution based on action
case "$ACTION" in
    "status")
        get_interface_status "$INTERFACE"
        ;;
        
    "info")
        show_interface_info "$INTERFACE"
        ;;
        
    "enable")
        enable_interface "$INTERFACE"
        ;;
        
    "disable")
        disable_interface "$INTERFACE"
        ;;
        
    "configure")
        if [[ "$DHCP" == true ]]; then
            configure_dhcp "$INTERFACE"
        elif [[ "$STATIC" == true ]]; then
            if [[ -z "$IP_ADDRESS" ]]; then
                error_exit "IP address is required for static configuration (--ip)"
            fi
            if [[ -z "$NETMASK" ]]; then
                # Try to determine netmask from IP class
                log_warning "Netmask not specified, using /24"
                NETMASK="24"
            fi
            
            configure_static "$INTERFACE" "$IP_ADDRESS" "$NETMASK" "$GATEWAY" "$DNS_SERVERS"
        else
            error_exit "Configuration type required (--dhcp or --static)"
        fi
        ;;
        
    *)
        error_exit "Invalid action: $ACTION (use: status, info, enable, disable, configure)"
        ;;
esac

log_success "🎉 Network operation completed successfully!"

# Show final status
echo ""
echo "📊 Final Interface Status:"
echo "--------------------------------------------------"
get_interface_status "$INTERFACE"

log_info "Next steps:"
log_info "  • Test connectivity: ping 8.8.8.8"
log_info "  • Check DNS resolution: nslookup google.com"
if [[ "$ACTION" == "configure" && "$STATIC" == true ]]; then
    log_info "  • Configure persistent settings in /etc/netctl/ or systemd-networkd"
fi