#!/bin/bash
# utils.sh - Utility functions for Arch Linux installer
#
# This is the foundational utility library. All other scripts should source
# this file first. Source-once guard prevents readonly variable errors.

set -euo pipefail

# --- Source-Once Guard ---
# Prevents errors from re-sourcing (readonly variable redefinition)
if [[ -n "${_UTILS_SH_SOURCED:-}" ]]; then
    # shellcheck disable=SC2317
    return 0 2>/dev/null || true
fi
readonly _UTILS_SH_SOURCED=1

# --- Dependency Management ---

# Source a script file or exit with error if it fails
# Usage: source_or_die "path/to/script.sh" ["error message"]
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

# --- Input Sanitization ---

# Escape a string for safe use in shell commands
# Handles: single quotes, double quotes, backticks, $, newlines, etc.
# Usage: escaped=$(shell_escape "$user_input")
shell_escape() {
    local input="$1"
    # Use printf %q for proper shell escaping
    printf '%q' "$input"
}

# Validate that a string contains only safe characters for filenames
# Returns 0 if safe, 1 if contains dangerous characters
validate_safe_string() {
    local input="$1"
    # Allow alphanumeric, dash, underscore, dot
    if [[ "$input" =~ ^[a-zA-Z0-9._-]+$ ]]; then
        return 0
    fi
    return 1
}

# Validate a device path looks like a valid block device
validate_device_path() {
    local device="$1"
    # Must start with /dev/ and contain only safe characters
    if [[ "$device" =~ ^/dev/[a-zA-Z0-9/_-]+$ ]]; then
        return 0
    fi
    return 1
}

# --- Color Definitions ---
# We use standard variables instead of arrays for the internal logger.
# This prevents associative array index crashes in strict mode (set -u).

# Export COLORS marker for source-once checks in other scripts
export COLORS=1

RESET='\033[0m'
BOLD='\033[1m'
RED='\033[31m'
GREEN='\033[32m'
YELLOW='\033[33m'
BLUE='\033[34m'
CYAN='\033[36m'

# --- Logging Functions ---

log_debug() {
    if [[ "${LOG_LEVEL:-INFO}" == "DEBUG" ]]; then
        local message="$1"
        local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
        # Use direct color variables - fail-safe and strict-mode compliant
        echo -e "${BLUE}[$timestamp] DEBUG: $message${RESET}"
        echo "[$timestamp] DEBUG: $message" >> "${LOG_FILE:-/dev/null}" 2>/dev/null || true
    fi
}

log_info() {
    local message="$1"
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    echo -e "${RESET}[$timestamp] INFO: $message${RESET}"
    echo "[$timestamp] INFO: $message" >> "${LOG_FILE:-/dev/null}" 2>/dev/null || true
}

log_warn() {
    local message="$1"
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    echo -e "${YELLOW}[$timestamp] WARN: $message${RESET}" >&2
    echo "[$timestamp] WARN: $message" >> "${LOG_FILE:-/dev/null}" 2>/dev/null || true
}

log_warning() { log_warn "$1"; }

log_error() {
    local message="$1"
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    echo -e "${RED}[$timestamp] ERROR: $message${RESET}" >&2
    echo "[$timestamp] ERROR: $message" >> "${LOG_FILE:-/dev/null}" 2>/dev/null || true
}

log_success() {
    local message="$1"
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    echo -e "${GREEN}[$timestamp] SUCCESS: $message${RESET}"
    echo "[$timestamp] SUCCESS: $message" >> "${LOG_FILE:-/dev/null}" 2>/dev/null || true
}

log_phase() {
    local message="$1"
    local timestamp=$(date +"%Y-%m-%d %H:%M:%S")
    echo -e "${BOLD}${CYAN}[$timestamp] === $message ===${RESET}"
    echo "[$timestamp] === $message ===" >> "${LOG_FILE:-/dev/null}" 2>/dev/null || true
}

log_cmd() {
    local message="$1"
    echo -e "${BOLD}  > $message${RESET}"
    echo "  > $message" >> "${LOG_FILE:-/dev/null}" 2>/dev/null || true
}

# --- Helper Functions ---

error_exit() {
    log_error "$1"
    exit 1
}

execute_non_critical() {
    local desc="$1"
    shift
    log_info "$desc"
    if ! "$@"; then
        log_warn "NON-CRITICAL: $desc failed"
        return 1
    fi
    return 0
}

log_and_continue() {
    log_warn "$1"
    # Do not exit
}

validate_username() {
    local user="$1"
    if [[ -z "$user" ]]; then return 1; fi
    if [[ ! "$user" =~ ^[a-z_][a-z0-9_-]*$ ]]; then return 1; fi
    return 0
}

validate_hostname() {
    local host="$1"
    if [[ -z "$host" ]]; then return 1; fi
    if [[ ! "$host" =~ ^[a-zA-Z0-9][a-zA-Z0-9.-]*$ ]]; then return 1; fi
    return 0
}

check_package_available() {
    pacman -Si "$1" >/dev/null 2>&1
}

format_filesystem() {
    local dev="$1"
    local fs="$2"
    case "$fs" in
        ext4) mkfs.ext4 -F "$dev" ;;
        btrfs) mkfs.btrfs -f "$dev" ;;
        xfs) mkfs.xfs -f "$dev" ;;
        vfat|fat32) mkfs.fat -F32 "$dev" ;;
        swap) mkswap "$dev" ;;
        *) return 1 ;;
    esac
}
