#!/bin/bash
# reset_password.sh - Reset user password
# Usage: ./reset_password.sh --username <user> [--root]

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
USERNAME=""
RESET_ROOT=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --username)
            USERNAME="$2"
            shift 2
            ;;
        --root)
            RESET_ROOT=true
            shift
            ;;
        --help)
            echo "Usage: $0 --username <user> [--root]"
            echo "  --username <user>  Username to reset password for"
            echo "  --root            Reset root password instead"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate arguments
if [[ "$RESET_ROOT" == false && -z "$USERNAME" ]]; then
    error_exit "Username is required (--username <user>)"
fi

if [[ "$RESET_ROOT" == true && -n "$USERNAME" ]]; then
    error_exit "Cannot specify both --username and --root"
fi

# Determine target user
if [[ "$RESET_ROOT" == true ]]; then
    TARGET_USER="root"
else
    TARGET_USER="$USERNAME"
fi

# Check if user exists
if ! id "$TARGET_USER" >/dev/null 2>&1; then
    error_exit "User $TARGET_USER does not exist"
fi

log_info "Resetting password for user: $TARGET_USER"
log_warning "You will be prompted to enter a new password"

# Reset the password
if passwd "$TARGET_USER"; then
    log_success "Password reset successfully for user: $TARGET_USER"
else
    error_exit "Failed to reset password for user: $TARGET_USER"
fi
