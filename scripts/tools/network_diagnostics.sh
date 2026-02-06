#!/bin/bash
# network_diagnostics.sh - Comprehensive network diagnostics
# Usage: ./network_diagnostics.sh --action basic|detailed|troubleshoot

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
            echo "Usage: $0 --action <basic|detailed|troubleshoot>"
            echo "Comprehensive network diagnostics"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Function to show network interfaces
show_interfaces() {
    log_info "Network Interfaces:"
    
    if command -v ip >/dev/null 2>&1; then
        ip addr show
    elif command -v ifconfig >/dev/null 2>&1; then
        ifconfig
    else
        log_error "No network configuration tools found"
    fi
    
    echo
    log_info "Interface Statistics:"
    if [[ -f /proc/net/dev ]]; then
        cat /proc/net/dev | head -20
    fi
}

# Function to show routing information
show_routing() {
    log_info "Routing Table:"
    
    if command -v ip >/dev/null 2>&1; then
        ip route show
    elif command -v route >/dev/null 2>&1; then
        route -n
    else
        log_error "No routing tools found"
    fi
    
    echo
    log_info "ARP Table:"
    if command -v ip >/dev/null 2>&1; then
        ip neigh show
    elif command -v arp >/dev/null 2>&1; then
        arp -a
    fi
}

# Function to show DNS information
show_dns() {
    log_info "DNS Configuration:"
    
    if [[ -f /etc/resolv.conf ]]; then
        cat /etc/resolv.conf
    else
        log_warning "/etc/resolv.conf not found"
    fi
    
    echo
    log_info "DNS Resolution Test:"
    local test_hosts=("google.com" "archlinux.org" "8.8.8.8")
    for host in "${test_hosts[@]}"; do
        if nslookup "$host" >/dev/null 2>&1; then
            local ip=$(nslookup "$host" | grep -A1 "Name:" | tail -1 | awk '{print $2}')
            log_success "DNS: $host -> $ip"
        else
            log_error "DNS: $host -> FAILED"
        fi
    done
}

# Function to show network connections
show_connections() {
    log_info "Active Network Connections:"
    
    if command -v ss >/dev/null 2>&1; then
        ss -tuln | head -20
    elif command -v netstat >/dev/null 2>&1; then
        netstat -tuln | head -20
    else
        log_error "No network connection tools found"
    fi
    
    echo
    log_info "Listening Services:"
    if command -v ss >/dev/null 2>&1; then
        ss -tlnp | grep LISTEN
    elif command -v netstat >/dev/null 2>&1; then
        netstat -tlnp | grep LISTEN
    fi
}

# Function to test connectivity
test_connectivity() {
    log_info "Connectivity Tests:"
    
    local test_hosts=("8.8.8.8" "1.1.1.1" "google.com")
    for host in "${test_hosts[@]}"; do
        log_info "Testing connectivity to $host..."
        if ping -c 2 -W 3 "$host" >/dev/null 2>&1; then
            log_success "Ping to $host: OK"
        else
            log_error "Ping to $host: FAILED"
        fi
    done
    
    echo
    log_info "HTTP Connectivity Test:"
    if command -v curl >/dev/null 2>&1; then
        if curl -s --connect-timeout 5 http://httpbin.org/ip >/dev/null 2>&1; then
            local external_ip=$(curl -s --connect-timeout 5 http://httpbin.org/ip | grep -o '[0-9.]*' | head -1)
            log_success "HTTP connectivity: OK (External IP: $external_ip)"
        else
            log_error "HTTP connectivity: FAILED"
        fi
    else
        log_warning "curl not available for HTTP test"
    fi
}

# Function to show network statistics
show_network_stats() {
    log_info "Network Statistics:"
    
    if [[ -f /proc/net/snmp ]]; then
        echo "IP Statistics:"
        grep -A1 "Ip:" /proc/net/snmp
        echo
    fi
    
    if [[ -f /proc/net/netstat ]]; then
        echo "TCP Statistics:"
        grep -A1 "Tcp:" /proc/net/netstat
        echo
    fi
    
    if [[ -f /proc/net/udp ]]; then
        echo "UDP Statistics:"
        wc -l /proc/net/udp | awk '{print "UDP sockets: " $1}'
    fi
}

# Function to troubleshoot network issues
troubleshoot_network() {
    log_info "Network Troubleshooting:"
    echo
    
    # Check if network interfaces are up
    log_info "Checking interface status..."
    local interfaces=$(ip link show | grep -E "^[0-9]+:" | awk -F: '{print $2}' | tr -d ' ')
    for iface in $interfaces; do
        if [[ "$iface" != "lo" ]]; then
            local status=$(ip link show "$iface" | grep -o "state [A-Z]*" | awk '{print $2}')
            if [[ "$status" == "UP" ]]; then
                log_success "Interface $iface: UP"
            else
                log_error "Interface $iface: DOWN"
            fi
        fi
    done
    
    echo
    
    # Check for IP addresses
    log_info "Checking IP addresses..."
    local has_ip=false
    if command -v ip >/dev/null 2>&1; then
        local ips=$(ip addr show | grep "inet " | grep -v "127.0.0.1")
        if [[ -n "$ips" ]]; then
            log_success "IP addresses assigned:"
            echo "$ips"
            has_ip=true
        else
            log_error "No IP addresses assigned"
        fi
    fi
    
    echo
    
    # Check for default gateway
    log_info "Checking default gateway..."
    local gateway=$(ip route show default | awk '{print $3}' | head -1)
    if [[ -n "$gateway" ]]; then
        log_success "Default gateway: $gateway"
        if ping -c 1 -W 2 "$gateway" >/dev/null 2>&1; then
            log_success "Gateway reachable: YES"
        else
            log_error "Gateway reachable: NO"
        fi
    else
        log_error "No default gateway configured"
    fi
    
    echo
    
    # Check DNS resolution
    log_info "Checking DNS resolution..."
    if nslookup google.com >/dev/null 2>&1; then
        log_success "DNS resolution: WORKING"
    else
        log_error "DNS resolution: FAILED"
        log_info "Trying alternative DNS servers..."
        echo "nameserver 8.8.8.8" > /tmp/resolv.conf.test
        if nslookup google.com 8.8.8.8 >/dev/null 2>&1; then
            log_success "Alternative DNS (8.8.8.8): WORKING"
        else
            log_error "Alternative DNS (8.8.8.8): FAILED"
        fi
        rm -f /tmp/resolv.conf.test
    fi
    
    echo
    
    # Check firewall
    log_info "Checking firewall status..."
    if iptables -L INPUT | grep -q "DROP\|REJECT"; then
        log_warning "Firewall is active with restrictive rules"
    elif ufw status | grep -q "Status: active"; then
        log_warning "UFW firewall is active"
    else
        log_info "No active firewall detected"
    fi
}

# Main execution
echo "=========================================="
log_info "Network Diagnostics - $(date)"
echo "=========================================="

case "$ACTION" in
    basic)
        show_interfaces
        echo
        show_routing
        echo
        test_connectivity
        ;;
    
    detailed)
        show_interfaces
        echo
        show_routing
        echo
        show_dns
        echo
        show_connections
        echo
        show_network_stats
        echo
        test_connectivity
        ;;
    
    troubleshoot)
        troubleshoot_network
        echo
        log_info "Additional diagnostics:"
        show_interfaces
        echo
        show_routing
        echo
        show_dns
        ;;
    
    *)
        error_exit "Invalid action: $ACTION. Use basic, detailed, or troubleshoot"
        ;;
esac

echo "=========================================="
log_success "Network diagnostics completed"
