#!/bin/bash
# install_dotfiles.sh - Clone and install dotfiles from a Git repository (Sprint 12)
#
# USAGE:
#   install_dotfiles.sh --repo <url> --user <username> [--target <dir>] [--branch <branch>] [--backup]
#
# PREREQUISITES:
#   - git must be installed
#   - Target user must exist
#
# This script is NON-INTERACTIVE. No prompts, no stdin.

set -euo pipefail

# --- Signal Handling ---
cleanup_and_exit() {
    local sig="$1"
    echo "INSTALL_DOTFILES: Received $sig, aborting..." >&2
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
REPO_URL=""
TARGET_USER=""
TARGET_DIR=""
BRANCH=""
BACKUP=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --repo)     REPO_URL="$2"; shift 2 ;;
        --user)     TARGET_USER="$2"; shift 2 ;;
        --target)   TARGET_DIR="$2"; shift 2 ;;
        --branch)   BRANCH="$2"; shift 2 ;;
        --backup)   BACKUP=true; shift ;;
        *) error_exit "Unknown argument: $1" ;;
    esac
done

# --- Validation ---
if [[ -z "$REPO_URL" ]]; then
    error_exit "Missing required argument: --repo"
fi

if [[ -z "$TARGET_USER" ]]; then
    error_exit "Missing required argument: --user"
fi

# Validate username
if ! validate_username "$TARGET_USER"; then
    error_exit "Invalid username: $TARGET_USER"
fi

# Validate repo URL scheme (must be https:// or git://)
case "$REPO_URL" in
    https://*|git://*)
        # Valid schemes
        ;;
    *)
        error_exit "Invalid repository URL scheme: $REPO_URL (must be https:// or git://)"
        ;;
esac

# Check git is installed
if ! command -v git &>/dev/null; then
    error_exit "git is not installed. Install git first: pacman -S git"
fi

# Determine target directory
if [[ -z "$TARGET_DIR" ]]; then
    TARGET_DIR="/home/$TARGET_USER"
fi

# Verify target directory exists
if [[ ! -d "$TARGET_DIR" ]]; then
    error_exit "Target directory does not exist: $TARGET_DIR"
fi

# --- Dotfiles Installation ---
log_phase "Installing dotfiles"
log_info "Repository: $REPO_URL"
log_info "User: $TARGET_USER"
log_info "Target: $TARGET_DIR"

# Create temporary clone directory
CLONE_DIR=$(mktemp -d "/tmp/dotfiles-XXXXXX")

# Cleanup on exit
cleanup_clone() {
    if [[ -d "$CLONE_DIR" ]]; then
        rm -rf "$CLONE_DIR"
    fi
}
trap 'cleanup_clone; cleanup_and_exit EXIT' EXIT

# Clone the repository
GIT_ARGS=(clone --depth 1)
if [[ -n "$BRANCH" ]]; then
    GIT_ARGS+=(--branch "$BRANCH")
fi
GIT_ARGS+=("$REPO_URL" "$CLONE_DIR")

log_cmd "git ${GIT_ARGS[*]}"
if ! git "${GIT_ARGS[@]}"; then
    error_exit "Failed to clone dotfiles repository: $REPO_URL"
fi

log_info "Repository cloned successfully"

# Copy dotfiles to target directory
# Skip .git directory
cd "$CLONE_DIR"

# Find all files (excluding .git)
while IFS= read -r -d '' file; do
    relative_path="${file#./}"
    target_path="$TARGET_DIR/$relative_path"
    target_parent=$(dirname "$target_path")

    # Create parent directory if needed
    if [[ ! -d "$target_parent" ]]; then
        mkdir -p "$target_parent"
    fi

    # Backup existing file if requested
    if [[ "$BACKUP" == "true" ]] && [[ -f "$target_path" ]]; then
        backup_path="${target_path}.bak.$(date +%s)"
        cp "$target_path" "$backup_path"
        log_info "Backed up: $relative_path -> $(basename "$backup_path")"
    fi

    # Copy file
    cp "$file" "$target_path"
    log_debug "Copied: $relative_path"
done < <(find . -not -path './.git/*' -not -path './.git' -not -name '.git' -type f -print0)

# Fix ownership
log_info "Setting ownership to $TARGET_USER"
chown -R "$TARGET_USER:$TARGET_USER" "$TARGET_DIR"

log_success "Dotfiles installed successfully for $TARGET_USER"
