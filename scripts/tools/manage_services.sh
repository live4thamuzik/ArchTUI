#!/bin/bash
# manage_services.sh - Enable/disable systemd services
# Usage: ./manage_services.sh --action enable --service gdm

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
SERVICE=""
ENABLE=false
DISABLE=false
START=false
STOP=false
STATUS=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --action)
            ACTION="$2"
            shift 2
            ;;
        --service)
            SERVICE="$2"
            shift 2
            ;;
        --enable)
            ENABLE=true
            shift
            ;;
        --disable)
            DISABLE=true
            shift
            ;;
        --start)
            START=true
            shift
            ;;
        --stop)
            STOP=true
            shift
            ;;
        --status)
            STATUS=true
            shift
            ;;
        --help)
            echo "Usage: $0 [--action <action>] [--service <service>] [--enable] [--disable] [--start] [--stop] [--status]"
            echo "Manage systemd services"
            echo "Actions: enable, disable, start, stop, status, list"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Check if systemctl is available
if ! command -v systemctl >/dev/null 2>&1; then
    error_exit "systemctl not found. This script requires systemd."
fi

# Handle list action (show available services)
if [[ "$ACTION" == "list" ]]; then
    log_info "Available systemd services:"
    systemctl list-unit-files --type=service --state=enabled,disabled | head -20
    log_info "Use --service <name> to manage a specific service"
    exit 0
fi

# Handle status action
if [[ "$STATUS" == true ]] || [[ "$ACTION" == "status" ]]; then
    if [[ -z "$SERVICE" ]]; then
        log_info "System service status overview:"
        systemctl --failed
        echo
        log_info "Active services:"
        systemctl list-units --type=service --state=active | head -10
    else
        log_info "Status of service: $SERVICE"
        systemctl status "$SERVICE" || log_info "Service $SERVICE not found or not running"
    fi
    exit 0
fi

# Validate service name
if [[ -z "$SERVICE" ]]; then
    error_exit "Service name is required (--service <service_name>)"
fi

# Check if service exists
if ! systemctl list-unit-files --type=service | grep -q "^$SERVICE.service"; then
    log_warning "Service $SERVICE not found in systemd"
    log_info "Available services containing '$SERVICE':"
    systemctl list-unit-files --type=service | grep "$SERVICE" || log_info "No matching services found"
    exit 1
fi

# Perform actions based on flags or action parameter
if [[ "$ENABLE" == true ]] || [[ "$ACTION" == "enable" ]]; then
    log_info "Enabling service: $SERVICE"
    if systemctl enable "$SERVICE"; then
        log_success "Service $SERVICE enabled successfully"
    else
        log_error "Failed to enable service $SERVICE"
        exit 1
    fi
fi

if [[ "$DISABLE" == true ]] || [[ "$ACTION" == "disable" ]]; then
    log_info "Disabling service: $SERVICE"
    if systemctl disable "$SERVICE"; then
        log_success "Service $SERVICE disabled successfully"
    else
        log_error "Failed to disable service $SERVICE"
        exit 1
    fi
fi

if [[ "$START" == true ]] || [[ "$ACTION" == "start" ]]; then
    log_info "Starting service: $SERVICE"
    if systemctl start "$SERVICE"; then
        log_success "Service $SERVICE started successfully"
    else
        log_error "Failed to start service $SERVICE"
        exit 1
    fi
fi

if [[ "$STOP" == true ]] || [[ "$ACTION" == "stop" ]]; then
    log_info "Stopping service: $SERVICE"
    if systemctl stop "$SERVICE"; then
        log_success "Service $SERVICE stopped successfully"
    else
        log_error "Failed to stop service $SERVICE"
        exit 1
    fi
fi

# Show final status
echo
log_info "Current status of $SERVICE:"
systemctl is-enabled "$SERVICE" && log_info "Enabled: Yes" || log_info "Enabled: No"
systemctl is-active "$SERVICE" && log_info "Active: Yes" || log_info "Active: No"
