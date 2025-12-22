#!/bin/bash
# reset_password.sh - Reset user password
# Usage: ./reset_password.sh --username <user> [--root]

set -euo pipefail

# Source common utilities
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$SCRIPT_DIR/../utils.sh"

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
