#!/bin/bash
# configure_network.sh - Configure network interface
# Usage: ./configure_network.sh --interface <iface> --ip <ip> --netmask <mask> --gateway <gw>

set -euo pipefail

# Source common utilities
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$SCRIPT_DIR/../utils.sh"

# Default values
INTERFACE=""
IP_ADDRESS=""
NETMASK=""
GATEWAY=""
DHCP=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --interface)
            INTERFACE="$2"
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
        --dhcp)
            DHCP=true
            shift
            ;;
        --help)
            echo "Usage: $0 --interface <iface> [--ip <ip> --netmask <mask> --gateway <gw> | --dhcp]"
            echo "Configure network interface"
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

# Check if interface exists
if ! ip link show "$INTERFACE" >/dev/null 2>&1; then
    error_exit "Interface $INTERFACE does not exist"
fi

log_info "Configuring network interface: $INTERFACE"

if [[ "$DHCP" == true ]]; then
    # Configure for DHCP
    log_info "Configuring interface for DHCP..."
    
    # Bring interface up
    ip link set "$INTERFACE" up
    
    # Request DHCP lease
    if command -v dhcpcd >/dev/null 2>&1; then
        dhcpcd "$INTERFACE"
    elif command -v dhclient >/dev/null 2>&1; then
        dhclient "$INTERFACE"
    else
        log_warning "No DHCP client found. Install dhcpcd or dhclient."
        exit 1
    fi
    
    log_success "Interface configured for DHCP"
else
    # Validate static IP configuration
    if [[ -z "$IP_ADDRESS" ]]; then
        error_exit "IP address is required for static configuration (--ip <ip>)"
    fi
    if [[ -z "$NETMASK" ]]; then
        error_exit "Netmask is required for static configuration (--netmask <mask>)"
    fi
    if [[ -z "$GATEWAY" ]]; then
        error_exit "Gateway is required for static configuration (--gateway <gw>)"
    fi
    
    # Configure static IP
    log_info "Configuring static IP: $IP_ADDRESS/$NETMASK"
    log_info "Gateway: $GATEWAY"
    
    # Bring interface up
    ip link set "$INTERFACE" up
    
    # Configure IP address
    ip addr add "$IP_ADDRESS/$NETMASK" dev "$INTERFACE"
    
    # Configure gateway
    ip route add default via "$GATEWAY"
    
    log_success "Static IP configuration completed"
fi

# Display current configuration
echo
log_info "Current network configuration:"
ip addr show "$INTERFACE"
echo
ip route show | grep "$INTERFACE" || log_info "No routes found for $INTERFACE"
