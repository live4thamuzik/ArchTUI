#!/bin/bash
# enable_services.sh - Enable systemd services in chroot (Sprint 12)
#
# USAGE:
#   enable_services.sh --root <mountpoint> --services <svc1,svc2,...>
#
# Enables systemd services inside the installed system via arch-chroot.
#
# This script is NON-INTERACTIVE. No prompts, no stdin.

set -euo pipefail

# --- Signal Handling ---
cleanup_and_exit() {
    local sig="$1"
    echo "ENABLE_SERVICES: Received $sig, aborting..." >&2
    exit 130
}
trap 'cleanup_and_exit SIGTERM' SIGTERM
trap 'cleanup_and_exit SIGINT' SIGINT

# Source common utilities
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

# --- Argument Parsing ---
ROOT=""
SERVICES_CSV=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --root)     ROOT="$2"; shift 2 ;;
        --services) SERVICES_CSV="$2"; shift 2 ;;
        *) error_exit "Unknown argument: $1" ;;
    esac
done

# --- Validation ---
if [[ -z "$ROOT" ]]; then
    error_exit "Missing required argument: --root"
fi

if [[ -z "$SERVICES_CSV" ]]; then
    error_exit "Missing required argument: --services"
fi

# Verify root exists and looks like a mount point
if [[ ! -d "$ROOT" ]]; then
    error_exit "Root directory does not exist: $ROOT"
fi

# Check arch-chroot is available
if ! command -v arch-chroot &>/dev/null; then
    error_exit "arch-chroot is not available. Are you running from the Arch ISO?"
fi

# --- Enable Services ---
log_phase "Enabling systemd services"
log_info "Root: $ROOT"
log_info "Services: $SERVICES_CSV"

# Split comma-separated list into array
IFS=',' read -ra SERVICES <<< "$SERVICES_CSV"

failed=0
for service in "${SERVICES[@]}"; do
    # Trim whitespace
    service=$(echo "$service" | tr -d '[:space:]')

    if [[ -z "$service" ]]; then
        continue
    fi

    # Validate service name (alphanumeric, dash, underscore, dot, @)
    if [[ ! "$service" =~ ^[a-zA-Z0-9._@-]+$ ]]; then
        log_error "Invalid service name: $service"
        failed=1
        continue
    fi

    log_cmd "arch-chroot $ROOT systemctl enable $service"
    if arch-chroot "$ROOT" systemctl enable "$service" 2>/dev/null; then
        log_success "Enabled: $service"
    else
        log_error "Failed to enable: $service"
        failed=1
    fi
done

if [[ "$failed" -ne 0 ]]; then
    error_exit "One or more services failed to enable"
fi

log_success "All services enabled successfully"
