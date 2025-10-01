#!/bin/bash
# check_disk_health.sh - Check disk health using smartctl
# Usage: ./check_disk_health.sh --device /dev/sda

set -euo pipefail

# Source common utilities
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$SCRIPT_DIR/../utils.sh"

# Default values
DEVICE=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --device)
            DEVICE="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 --device <device>"
            echo "Check disk health using smartctl"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate required arguments
if [[ -z "$DEVICE" ]]; then
    error_exit "Device is required (--device /dev/sda)"
fi

# Check if device exists
if [[ ! -b "$DEVICE" ]]; then
    error_exit "Device does not exist: $DEVICE"
fi

# Check if smartctl is available
if ! command -v smartctl >/dev/null 2>&1; then
    log_info "Installing smartmontools..."
    pacman -Sy --noconfirm smartmontools
fi

log_info "Checking disk health for $DEVICE..."

# Check if SMART is enabled
if smartctl -H "$DEVICE" | grep -q "SMART overall-health self-assessment test result: PASSED"; then
    log_success "SMART overall-health: PASSED"
elif smartctl -H "$DEVICE" | grep -q "SMART overall-health self-assessment test result: FAILED"; then
    log_error "SMART overall-health: FAILED"
else
    log_warning "SMART status unclear"
fi

# Display SMART attributes
echo
log_info "SMART Attributes:"
smartctl -A "$DEVICE" | head -20

# Display disk temperature if available
echo
log_info "Disk Temperature:"
smartctl -A "$DEVICE" | grep -i temperature || log_info "Temperature not available"

# Display error log
echo
log_info "Error Log:"
smartctl -l error "$DEVICE" | head -10 || log_info "No error log entries"

log_success "Disk health check completed for $DEVICE"
