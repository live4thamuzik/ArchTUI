#!/bin/bash
# manage_groups.sh - Manage user groups
# Usage: ./manage_groups.sh --action add --user username --group wheel

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
USERNAME=""
GROUP=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --action)
            ACTION="$2"
            shift 2
            ;;
        --user)
            USERNAME="$2"
            shift 2
            ;;
        --group)
            GROUP="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 --action <action> --user <username> [--group <group>]"
            echo "Manage user groups"
            echo "Actions: add, remove, list, create, delete"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate required arguments
if [[ -z "$ACTION" ]]; then
    error_exit "Action is required (--action add|remove|list|create|delete)"
fi

case "$ACTION" in
    list)
        if [[ -z "$USERNAME" ]]; then
            log_info "All system groups:"
            getent group | sort
        else
            # Check if user exists
            if ! id "$USERNAME" >/dev/null 2>&1; then
                error_exit "User $USERNAME does not exist"
            fi
            
            log_info "Groups for user $USERNAME:"
            groups "$USERNAME"
            echo
            log_info "Primary group:"
            id -gn "$USERNAME"
        fi
        exit 0
        ;;
    create)
        if [[ -z "$GROUP" ]]; then
            error_exit "Group name is required for create action (--group <group>)"
        fi
        
        log_info "Creating group: $GROUP"
        if groupadd "$GROUP"; then
            log_success "Group $GROUP created successfully"
        else
            log_error "Failed to create group $GROUP"
            exit 1
        fi
        exit 0
        ;;
    delete)
        if [[ -z "$GROUP" ]]; then
            error_exit "Group name is required for delete action (--group <group>)"
        fi
        
        # Check if group exists
        if ! getent group "$GROUP" >/dev/null 2>&1; then
            error_exit "Group $GROUP does not exist"
        fi
        
        log_warning "Deleting group: $GROUP"

        # ENVIRONMENT CONTRACT: Require explicit confirmation for destructive operation
        if [[ "${CONFIRM_GROUP_DELETE:-}" != "yes" ]]; then
            error_exit "CONFIRM_GROUP_DELETE=yes is required for group deletion"
        fi

        if groupdel "$GROUP"; then
            log_success "Group $GROUP deleted successfully"
        else
            log_error "Failed to delete group $GROUP"
            exit 1
        fi
        exit 0
        ;;
    add|remove)
        if [[ -z "$USERNAME" ]]; then
            error_exit "Username is required (--user <username>)"
        fi
        if [[ -z "$GROUP" ]]; then
            error_exit "Group name is required (--group <group>)"
        fi
        
        # Check if user exists
        if ! id "$USERNAME" >/dev/null 2>&1; then
            error_exit "User $USERNAME does not exist"
        fi
        
        # Check if group exists
        if ! getent group "$GROUP" >/dev/null 2>&1; then
            error_exit "Group $GROUP does not exist"
        fi
        
        if [[ "$ACTION" == "add" ]]; then
            log_info "Adding user $USERNAME to group $GROUP"
            if usermod -aG "$GROUP" "$USERNAME"; then
                log_success "User $USERNAME added to group $GROUP successfully"
            else
                log_error "Failed to add user $USERNAME to group $GROUP"
                exit 1
            fi
        else
            log_info "Removing user $USERNAME from group $GROUP"
            if gpasswd -d "$USERNAME" "$GROUP"; then
                log_success "User $USERNAME removed from group $GROUP successfully"
            else
                log_error "Failed to remove user $USERNAME from group $GROUP"
                exit 1
            fi
        fi
        ;;
    *)
        error_exit "Invalid action: $ACTION. Use add, remove, list, create, or delete"
        ;;
esac

# Show updated group membership
echo
log_info "Updated groups for user $USERNAME:"
groups "$USERNAME"
