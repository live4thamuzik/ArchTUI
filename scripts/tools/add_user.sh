#!/bin/bash
# add_user.sh - Add a new user to the system
# Usage: ./add_user.sh --username john --password secret123 --root /mnt

set -euo pipefail

# Source common utilities
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$SCRIPT_DIR/../utils.sh"

# Default values
USERNAME=""
PASSWORD=""
ROOT_PATH="/mnt"
GROUPS="wheel"
SHELL="/bin/bash"
HOME_DIR=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --username)
            USERNAME="$2"
            shift 2
            ;;
        --password)
            PASSWORD="$2"
            shift 2
            ;;
        --root)
            ROOT_PATH="$2"
            shift 2
            ;;
        --groups)
            GROUPS="$2"
            shift 2
            ;;
        --shell)
            SHELL="$2"
            shift 2
            ;;
        --home)
            HOME_DIR="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 --username <name> --password <pass> [--root <path>] [--groups <groups>] [--shell <shell>] [--home <path>]"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate required arguments
if [[ -z "$USERNAME" ]]; then
    error_exit "Username is required (--username <name>)"
fi

if [[ -z "$PASSWORD" ]]; then
    error_exit "Password is required (--password <pass>)"
fi

# Validate root path
if [[ ! -d "$ROOT_PATH" ]]; then
    error_exit "Root path does not exist: $ROOT_PATH"
fi

# Validate username (basic check)
if [[ ! "$USERNAME" =~ ^[a-zA-Z][a-zA-Z0-9_-]*$ ]]; then
    error_exit "Invalid username: $USERNAME (must start with letter, contain only letters, numbers, underscore, hyphen)"
fi

# Check if user already exists
if id "$USERNAME" >/dev/null 2>&1; then
    error_exit "User $USERNAME already exists"
fi

log_info "Adding user: $USERNAME"
log_info "Root path: $ROOT_PATH"
log_info "Groups: $GROUPS"
log_info "Shell: $SHELL"

# Set home directory if not specified
if [[ -z "$HOME_DIR" ]]; then
    HOME_DIR="/home/$USERNAME"
fi

# Create user
log_info "Creating user account..."
arch-chroot "$ROOT_PATH" useradd -m -G "$GROUPS" -s "$SHELL" -d "$HOME_DIR" "$USERNAME"

# Set password
log_info "Setting password..."
echo "$USERNAME:$PASSWORD" | arch-chroot "$ROOT_PATH" chpasswd

# Create sudoers entry for wheel group (if wheel is in groups)
if [[ "$GROUPS" == *"wheel"* ]]; then
    log_info "Configuring sudo access..."
    if ! grep -q "^%wheel ALL=(ALL) ALL" "$ROOT_PATH/etc/sudoers" 2>/dev/null; then
        echo "%wheel ALL=(ALL) ALL" >> "$ROOT_PATH/etc/sudoers"
    fi
fi

log_success "User $USERNAME created successfully!"

# Show user information
log_info "User information:"
arch-chroot "$ROOT_PATH" id "$USERNAME"
arch-chroot "$ROOT_PATH" getent passwd "$USERNAME"
