#!/bin/bash
# update_mirrors.sh - Rank and update pacman mirrorlist using reflector (Sprint 13)
#
# USAGE:
#   update_mirrors.sh --limit <n> --sort <method> [--country <CC>] [--protocol <proto>] [--save]
#
# PREREQUISITES:
#   - reflector must be installed (available on Arch ISO by default)
#   - Network connectivity required
#
# GRACEFUL FAILURE:
#   - Checks network connectivity before running reflector
#   - Returns non-zero exit code if network is down
#   - Does NOT corrupt existing mirrorlist on failure
#
# This script is NON-INTERACTIVE. No prompts, no stdin.

set -euo pipefail

# --- Signal Handling ---
cleanup_and_exit() {
    local sig="$1"
    echo "UPDATE_MIRRORS: Received $sig, aborting..." >&2
    # Restore backup mirrorlist if save was interrupted
    if [[ -f "/etc/pacman.d/mirrorlist.bak" ]]; then
        cp /etc/pacman.d/mirrorlist.bak /etc/pacman.d/mirrorlist 2>/dev/null || true
    fi
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
LIMIT="20"
SORT="rate"
COUNTRY=""
PROTOCOL=""
SAVE=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --limit)    LIMIT="$2"; shift 2 ;;
        --sort)     SORT="$2"; shift 2 ;;
        --country)  COUNTRY="$2"; shift 2 ;;
        --protocol) PROTOCOL="$2"; shift 2 ;;
        --save)     SAVE=true; shift ;;
        *) error_exit "Unknown argument: $1" ;;
    esac
done

# --- Validation ---

# Validate limit is a number
if ! [[ "$LIMIT" =~ ^[0-9]+$ ]]; then
    error_exit "Invalid limit: $LIMIT (must be a positive integer)"
fi

# Validate sort method
case "$SORT" in
    rate|age|country|score)
        # Valid sort methods
        ;;
    *)
        error_exit "Invalid sort method: $SORT (valid: rate, age, country, score)"
        ;;
esac

# Validate protocol if specified
if [[ -n "$PROTOCOL" ]]; then
    case "$PROTOCOL" in
        https|http|ftp)
            # Valid protocols
            ;;
        *)
            error_exit "Invalid protocol: $PROTOCOL (valid: https, http, ftp)"
            ;;
    esac
fi

# Check reflector is installed
if ! command -v reflector &>/dev/null; then
    error_exit "reflector is not installed. Install it: pacman -S reflector"
fi

# --- Network Check ---
log_phase "Mirror Ranking"
log_info "Checking network connectivity..."

if ! ping -c 1 -W 5 archlinux.org &>/dev/null; then
    log_error "Network connectivity check failed"
    log_error "Cannot update mirrors without network access"
    exit 1
fi

log_info "Network connectivity OK"

# --- Build Reflector Command ---
REFLECTOR_ARGS=(
    --latest "$LIMIT"
    --sort "$SORT"
)

if [[ -n "$COUNTRY" ]]; then
    REFLECTOR_ARGS+=(--country "$COUNTRY")
fi

if [[ -n "$PROTOCOL" ]]; then
    REFLECTOR_ARGS+=(--protocol "$PROTOCOL")
fi

if [[ "$SAVE" == "true" ]]; then
    # Backup existing mirrorlist before overwriting
    if [[ -f /etc/pacman.d/mirrorlist ]]; then
        log_info "Backing up current mirrorlist"
        cp /etc/pacman.d/mirrorlist /etc/pacman.d/mirrorlist.bak
    fi

    REFLECTOR_ARGS+=(--save /etc/pacman.d/mirrorlist)
fi

# --- Execute Reflector ---
log_info "Running reflector with: limit=$LIMIT sort=$SORT"
if [[ -n "$COUNTRY" ]]; then
    log_info "Country filter: $COUNTRY"
fi
if [[ -n "$PROTOCOL" ]]; then
    log_info "Protocol filter: $PROTOCOL"
fi

log_cmd "reflector ${REFLECTOR_ARGS[*]}"

if ! reflector "${REFLECTOR_ARGS[@]}"; then
    log_error "Reflector failed to rank mirrors"

    # Restore backup if we were saving
    if [[ "$SAVE" == "true" ]] && [[ -f /etc/pacman.d/mirrorlist.bak ]]; then
        log_warn "Restoring backup mirrorlist"
        cp /etc/pacman.d/mirrorlist.bak /etc/pacman.d/mirrorlist
    fi

    exit 1
fi

# Clean up backup on success
if [[ "$SAVE" == "true" ]] && [[ -f /etc/pacman.d/mirrorlist.bak ]]; then
    rm -f /etc/pacman.d/mirrorlist.bak
fi

log_success "Mirrors updated successfully ($LIMIT mirrors, sorted by $SORT)"
