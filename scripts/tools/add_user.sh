#!/bin/bash
# add_user.sh - Add user to system using ISO tools
# Usage: ./add_user.sh --username <user> [options]
#
# ENVIRONMENT CONTRACT:
#   USER_PASSWORD - Password for the user (optional, use --no-login if not set)
#
# This script is NON-INTERACTIVE and does not prompt for input.

set -euo pipefail

# Source common utilities via source_or_die
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source_or_die "$SCRIPT_DIR/../utils.sh"

# Default values
USERNAME=""
FULL_NAME=""
GROUPS=""
SHELL="/bin/bash"
HOME_DIR=""
USER_ID=""
GROUP_ID=""
SKEL_DIR=""
CREATE_HOME=true
SYSTEM_USER=false
NO_LOGIN=false

# Get password from environment variable
PASSWORD="${USER_PASSWORD:-}"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --username)
            USERNAME="$2"
            shift 2
            ;;
        --full-name)
            FULL_NAME="$2"
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
        --home-dir)
            HOME_DIR="$2"
            shift 2
            ;;
        --uid)
            USER_ID="$2"
            shift 2
            ;;
        --gid)
            GROUP_ID="$2"
            shift 2
            ;;
        --skel)
            SKEL_DIR="$2"
            shift 2
            ;;
        --no-create-home)
            CREATE_HOME=false
            shift
            ;;
        --system)
            SYSTEM_USER=true
            shift
            ;;
        --no-login)
            NO_LOGIN=true
            shift
            ;;
        --help)
            echo "Usage: $0 --username <user> [options]"
            echo ""
            echo "Required:"
            echo "  --username <user>     Username to create"
            echo ""
            echo "Optional:"
            echo "  --full-name <name>    Full name (GECOS field)"
            echo "  --groups <groups>     Additional groups (comma-separated)"
            echo "  --shell <shell>       Login shell (default: /bin/bash)"
            echo "  --home-dir <path>     Home directory path"
            echo "  --uid <uid>           User ID (default: auto-assign)"
            echo "  --gid <gid>           Primary group ID (default: auto-assign)"
            echo "  --skel <dir>          Skeleton directory for home"
            echo "  --no-create-home      Don't create home directory"
            echo "  --system              Create system user"
            echo "  --no-login            Disable login (no password)"
            echo ""
            echo "Password:"
            echo "  Set USER_PASSWORD environment variable before running"
            echo "  Example: USER_PASSWORD='mypassword' $0 --username john"
            echo ""
            echo "Examples:"
            echo "  USER_PASSWORD='secret' $0 --username john --full-name 'John Doe' --groups wheel,users"
            echo "  $0 --username service --system --no-login --shell /bin/false"
            echo ""
            echo "Note: Uses tools available on Arch ISO (useradd, passwd, usermod)"
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
    error_exit "Username is required (--username <user>)"
fi

# Validate username format
if ! echo "$USERNAME" | grep -qE '^[a-z_][a-z0-9_-]*$'; then
    error_exit "Invalid username format. Use lowercase letters, numbers, underscore, and hyphen only"
fi

# Check if user already exists
if id "$USERNAME" >/dev/null 2>&1; then
    error_exit "User '$USERNAME' already exists"
fi

# Validate shell exists
if [[ -n "$SHELL" && ! -f "$SHELL" ]]; then
    log_warning "Shell '$SHELL' does not exist, using default"
    SHELL="/bin/bash"
fi

# Validate groups exist
if [[ -n "$GROUPS" ]]; then
    IFS=',' read -ra GROUP_ARRAY <<< "$GROUPS"
    for group in "${GROUP_ARRAY[@]}"; do
        group=$(echo "$group" | sed 's/^ *//;s/ *$//')  # trim whitespace
        if ! getent group "$group" >/dev/null 2>&1; then
            log_warning "Group '$group' does not exist, will be created"
        fi
    done
fi

log_info "👤 User Management Tool (ISO Compatible)"
echo "=================================================="
log_info "Username: $USERNAME"
if [[ -n "$FULL_NAME" ]]; then
    log_info "Full name: $FULL_NAME"
fi
if [[ -n "$GROUPS" ]]; then
    log_info "Groups: $GROUPS"
fi
log_info "Shell: $SHELL"
log_info "Create home: $CREATE_HOME"
if [[ "$SYSTEM_USER" == true ]]; then
    log_info "System user: Yes"
fi
if [[ "$NO_LOGIN" == true ]]; then
    log_info "Login disabled: Yes"
fi
echo "=================================================="

# Build useradd command using array (SAFE - no eval needed)
USERADD_ARGS=()

# Add system user flag
if [[ "$SYSTEM_USER" == true ]]; then
    USERADD_ARGS+=(--system)
fi

# Add no login flag
if [[ "$NO_LOGIN" == true ]]; then
    USERADD_ARGS+=(--no-log-init --shell /bin/false)
else
    USERADD_ARGS+=(--shell "$SHELL")
fi

# Add home directory options
if [[ "$CREATE_HOME" == true ]]; then
    USERADD_ARGS+=(--create-home)
else
    USERADD_ARGS+=(--no-create-home)
fi

# Add custom home directory
if [[ -n "$HOME_DIR" ]]; then
    USERADD_ARGS+=(--home-dir "$HOME_DIR")
fi

# Add user ID
if [[ -n "$USER_ID" ]]; then
    USERADD_ARGS+=(--uid "$USER_ID")
fi

# Add group ID
if [[ -n "$GROUP_ID" ]]; then
    USERADD_ARGS+=(--gid "$GROUP_ID")
fi

# Add skeleton directory
if [[ -n "$SKEL_DIR" ]]; then
    USERADD_ARGS+=(--skel "$SKEL_DIR")
fi

# Add full name (GECOS field)
if [[ -n "$FULL_NAME" ]]; then
    USERADD_ARGS+=(--comment "$FULL_NAME")
fi

# Add the username as final argument
USERADD_ARGS+=("$USERNAME")

# Execute useradd command (SAFE - array expansion, no injection)
log_info "Creating user '$USERNAME'..."
log_info "Command: useradd ${USERADD_ARGS[*]}"

if useradd "${USERADD_ARGS[@]}"; then
    log_success "User '$USERNAME' created successfully"
else
    error_exit "Failed to create user '$USERNAME'"
fi

# Add user to additional groups
if [[ -n "$GROUPS" ]]; then
    log_info "👥 Adding user to groups: $GROUPS"
    
    IFS=',' read -ra GROUP_ARRAY <<< "$GROUPS"
    for group in "${GROUP_ARRAY[@]}"; do
        group=$(echo "$group" | sed 's/^ *//;s/ *$//')  # trim whitespace
        
        # Create group if it doesn't exist
        if ! getent group "$group" >/dev/null 2>&1; then
            log_info "Creating group '$group'..."
            if groupadd "$group"; then
                log_success "✅ Group '$group' created"
            else
                log_warning "⚠️  Failed to create group '$group'"
                continue
            fi
        fi
        
        # Add user to group
        if usermod -aG "$group" "$USERNAME"; then
            log_success "✅ Added '$USERNAME' to group '$group'"
        else
            log_warning "⚠️  Failed to add '$USERNAME' to group '$group'"
        fi
    done
fi

# Set password (received via USER_PASSWORD environment variable)
PASSWORD_WAS_SET=""
if [[ "$NO_LOGIN" == false ]]; then
    if [[ -n "$PASSWORD" ]]; then
        log_info "Setting password for '$USERNAME'..."
        # Use chpasswd to set password securely
        if printf '%s:%s\n' "$USERNAME" "$PASSWORD" | chpasswd 2>/dev/null; then
            log_success "Password set for '$USERNAME'"
            PASSWORD_WAS_SET="yes"
        else
            log_warning "Failed to set password for '$USERNAME'"
        fi
        # Clear password from memory
        PASSWORD=""
    else
        log_info "No password provided (USER_PASSWORD not set)"
        log_info "Run: passwd $USERNAME"
    fi
fi

# Display user information
log_info "📋 User Information:"
echo "--------------------------------------------------"
echo "Username: $USERNAME"
echo "UID: $(id -u "$USERNAME")"
echo "GID: $(id -g "$USERNAME")"
echo "Groups: $(id -Gn "$USERNAME")"
echo "Home: $(getent passwd "$USERNAME" | cut -d: -f6)"
echo "Shell: $(getent passwd "$USERNAME" | cut -d: -f7)"

if [[ "$CREATE_HOME" == true ]]; then
    HOME_PATH=$(getent passwd "$USERNAME" | cut -d: -f6)
    if [[ -d "$HOME_PATH" ]]; then
        echo "Home directory: $HOME_PATH (created)"
        if [[ -n "$SKEL_DIR" ]]; then
            echo "Skeleton source: $SKEL_DIR"
        fi
    else
        echo "Home directory: Not created"
    fi
fi

if [[ "$NO_LOGIN" == true ]]; then
    echo "Login: Disabled"
elif [[ -n "${PASSWORD_WAS_SET:-}" ]]; then
    echo "Password: Set"
else
    echo "Password: Not set (use 'passwd $USERNAME')"
fi

log_success "🎉 User '$USERNAME' setup completed successfully!"
log_info "Next steps:"
if [[ "$NO_LOGIN" == false && -z "$PASSWORD" ]]; then
    log_info "  • Set password: passwd $USERNAME"
fi
log_info "  • Test login: su - $USERNAME"
if [[ "$GROUPS" == *"wheel"* ]]; then
    log_info "  • Enable sudo: visudo (uncomment %wheel ALL=(ALL) ALL)"
fi