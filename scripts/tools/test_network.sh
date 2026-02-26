#!/bin/bash
# test_network.sh - Test network connectivity
# Usage: ./test_network.sh --action ping|dns|http|full

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
ACTION="full"
HOST=""
TIMEOUT=5

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --action)
            ACTION="$2"
            shift 2
            ;;
        --host)
            HOST="$2"
            shift 2
            ;;
        --timeout)
            TIMEOUT="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 --action <ping|dns|http|full> [--host <host>] [--timeout <seconds>]"
            echo "Test network connectivity"
            echo "Actions: ping, dns, http, full"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Default hosts for testing
DEFAULT_HOSTS=("8.8.8.8" "1.1.1.1" "google.com" "archlinux.org")

# Function to test ping connectivity (verbose — shows command + output + return code)
test_ping() {
    local target="$1"
    log_info "$ ping -c 3 -W $TIMEOUT $target"
    if ping -c 3 -W "$TIMEOUT" "$target"; then
        log_success "  → Return: 0 (OK)"
        return 0
    else
        local rc=$?
        log_error "  → Return: $rc (FAILED)"
        return 1
    fi
}

# Function to test DNS resolution (verbose — shows command + output + return code)
test_dns() {
    local target="$1"
    log_info "$ getent hosts $target"

    local ip=""
    if ip=$(getent hosts "$target" 2>&1); then
        echo "$ip"
        local resolved
        resolved=$(echo "$ip" | awk '{print $1; exit}')
        if [[ -n "$resolved" ]]; then
            log_success "  → Return: 0 (OK) — resolved to $resolved"
            return 0
        fi
    else
        local rc=$?
        echo "$ip"
        log_error "  → Return: $rc (FAILED)"
    fi

    # Fallback to dig if available
    if command -v dig >/dev/null 2>&1; then
        log_info "$ dig +short $target"
        if ip=$(dig +short "$target" 2>&1); then
            echo "$ip"
            local first
            first=$(echo "$ip" | head -1)
            if [[ -n "$first" ]]; then
                log_success "  → Return: 0 (OK) — resolved to $first"
                return 0
            fi
        else
            local rc=$?
            echo "$ip"
            log_error "  → Return: $rc (FAILED)"
        fi
    fi

    log_error "DNS resolution for $target failed"
    return 1
}

# Function to test HTTP connectivity (verbose — shows command + output + return code)
test_http() {
    local target="$1"

    # Try curl first
    if command -v curl >/dev/null 2>&1; then
        log_info "$ curl -sv --connect-timeout $TIMEOUT http://$target"
        if curl -sv --connect-timeout "$TIMEOUT" "http://$target" 2>&1 | head -30; then
            log_success "  → Return: 0 (OK)"
            return 0
        else
            local rc=$?
            log_error "  → Return: $rc (FAILED)"
            return 1
        fi
    fi

    # Try wget if curl not available
    if command -v wget >/dev/null 2>&1; then
        log_info "$ wget --timeout=$TIMEOUT --spider http://$target"
        if wget --timeout="$TIMEOUT" --spider "http://$target" 2>&1; then
            log_success "  → Return: 0 (OK)"
            return 0
        else
            local rc=$?
            log_error "  → Return: $rc (FAILED)"
            return 1
        fi
    fi

    log_error "No HTTP client available (curl/wget)"
    return 1
}

# Function to get network interface information
get_network_info() {
    log_info "Network interface information:"
    
    # Show active interfaces
    if command -v ip >/dev/null 2>&1; then
        ip addr show | grep -E "inet |UP|DOWN" | head -20
    elif command -v ifconfig >/dev/null 2>&1; then
        ifconfig | grep -E "inet |UP|DOWN" | head -20
    else
        log_warning "No network configuration tools found"
    fi
    
    echo
    log_info "Routing table:"
    if command -v ip >/dev/null 2>&1; then
        ip route show | head -10
    elif command -v route >/dev/null 2>&1; then
        route -n | head -10
    else
        log_warning "No routing tools found"
    fi
}

# Function to test specific connectivity
test_specific() {
    local test_type="$1"
    shift
    local targets=("$@")

    local success_count=0
    local total_count=${#targets[@]}

    for target in "${targets[@]}"; do
        case "$test_type" in
            ping)
                if test_ping "$target"; then
                    success_count=$((success_count + 1))
                fi
                ;;
            dns)
                if test_dns "$target"; then
                    success_count=$((success_count + 1))
                fi
                ;;
            http)
                if test_http "$target"; then
                    success_count=$((success_count + 1))
                fi
                ;;
        esac
        echo
    done

    log_info "Results: $success_count/$total_count tests passed"
    if [[ "$success_count" -eq "$total_count" ]]; then
        return 0
    else
        return 1
    fi
}

# Main execution
echo "=========================================="
log_info "Network Connectivity Test - $(date)"
echo "=========================================="

# Determine targets
if [[ -n "$HOST" ]]; then
    targets=("$HOST")
else
    targets=("${DEFAULT_HOSTS[@]}")
fi

case "$ACTION" in
    ping)
        test_specific "ping" "${targets[@]}"
        ;;
    dns)
        test_specific "dns" "${targets[@]}"
        ;;
    http)
        test_specific "http" "${targets[@]}"
        ;;
    full)
        log_info "Running full network connectivity test..."
        echo
        
        ping_success=0
        dns_success=0
        http_success=0
        
        # Test ping
        log_info "=== PING TESTS ==="
        if test_specific "ping" "${targets[@]}"; then
            ping_success=1
        fi
        echo
        
        # Test DNS
        log_info "=== DNS TESTS ==="
        if test_specific "dns" "${targets[@]}"; then
            dns_success=1
        fi
        echo
        
        # Test HTTP
        log_info "=== HTTP TESTS ==="
        if test_specific "http" "${targets[@]}"; then
            http_success=1
        fi
        echo
        
        # Summary
        log_info "=== SUMMARY ==="
        [[ $ping_success -eq 1 ]] && log_success "Ping tests: PASSED" || log_error "Ping tests: FAILED"
        [[ $dns_success -eq 1 ]] && log_success "DNS tests: PASSED" || log_error "DNS tests: FAILED"
        [[ $http_success -eq 1 ]] && log_success "HTTP tests: PASSED" || log_error "HTTP tests: FAILED"
        
        if [[ $ping_success -eq 1 && $dns_success -eq 1 && $http_success -eq 1 ]]; then
            log_success "All network tests passed - connectivity is good"
            exit 0
        else
            log_error "Some network tests failed - check connectivity"
            exit 1
        fi
        ;;
    *)
        error_exit "Invalid action: $ACTION. Use ping, dns, http, or full"
        ;;
esac
