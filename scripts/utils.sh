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

# Require root privileges — exits with clear error if not root
require_root() {
    if [[ "$(id -u)" -ne 0 ]]; then
        error_exit "This operation requires root privileges. Run with sudo or as root."
    fi
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

# format_filesystem is defined in disk_utils.sh with full error handling

# --- Initialization Functions ---

# Initialize logging to a file on disk
# Sets LOG_FILE and creates the log directory
# Falls back to /dev/null if directory creation fails
setup_logging() {
    local log_dir="${ARCHTUI_LOG_DIR:-/var/log/archtui}"
    if ! mkdir -p "$log_dir" 2>/dev/null; then
        log_dir="/tmp"
    fi
    local timestamp
    timestamp="$(date +%Y%m%d-%H%M%S)"
    export LOG_FILE="${log_dir}/install-${timestamp}.log"
    log_info "Logging initialized: $LOG_FILE"

    # VERBOSE mode: enable set -x trace to a separate log file via BASH_XTRACEFD
    if [[ "${LOG_LEVEL:-INFO}" == "VERBOSE" || "${LOG_LEVEL:-INFO}" == "DEBUG" ]]; then
        export VERBOSE_LOG_FILE="${log_dir}/install-${timestamp}-verbose.log"
        # Open fd 3 for the verbose trace log
        exec 3>>"$VERBOSE_LOG_FILE"
        export BASH_XTRACEFD=3
        # Readable trace prefix: [script:line] before each traced command
        export PS4='+[${BASH_SOURCE[0]##*/}:${LINENO}] '
        set -x
        log_info "Verbose logging enabled: $VERBOSE_LOG_FILE"
    fi
}

# Dump all configuration env vars to the log file (passwords redacted)
# Called after setup_logging to capture the full config snapshot
dump_config() {
    local timestamp
    timestamp="$(date +"%Y-%m-%d %H:%M:%S")"
    {
        echo "[$timestamp] === CONFIGURATION DUMP ==="
        local var
        for var in \
            INSTALL_DISK PARTITIONING_STRATEGY BOOT_MODE ENCRYPTION \
            ROOT_FILESYSTEM HOME_FILESYSTEM SEPARATE_HOME SWAP SWAP_SIZE \
            ROOT_SIZE HOME_SIZE RAID_LEVEL \
            LOCALE KEYMAP TIMEZONE_REGION TIMEZONE TIME_SYNC \
            MIRROR_COUNTRY KERNEL MULTILIB ADDITIONAL_PACKAGES GPU_DRIVERS \
            SYSTEM_HOSTNAME MAIN_USERNAME \
            AUR_HELPER ADDITIONAL_AUR_PACKAGES FLATPAK \
            BOOTLOADER OS_PROBER GRUB_THEME GRUB_THEME_SELECTION SECURE_BOOT \
            DESKTOP_ENVIRONMENT DISPLAY_MANAGER \
            PLYMOUTH PLYMOUTH_THEME NUMLOCK_ON_BOOT \
            GIT_REPOSITORY GIT_REPOSITORY_URL \
            BTRFS_SNAPSHOTS BTRFS_FREQUENCY BTRFS_KEEP_COUNT BTRFS_ASSISTANT \
            LOG_LEVEL \
            MAIN_USER_PASSWORD ROOT_PASSWORD ENCRYPTION_PASSWORD
        do
            local val="${!var:-}"
            # Redact passwords
            case "$var" in
                *PASSWORD*) val="********" ;;
            esac
            echo "[$timestamp]   $var=$val"
        done
        echo "[$timestamp] === END CONFIGURATION DUMP ==="
    } >> "${LOG_FILE:-/dev/null}" 2>/dev/null || true
}

# Execute a command with set -x temporarily disabled (for password-sensitive operations)
# Usage: run_secret <command> [args...]
# When VERBOSE mode is active, this suppresses tracing to prevent passwords
# from appearing in the verbose log. Re-enables tracing after the command.
run_secret() {
    if [[ "${LOG_LEVEL:-INFO}" == "VERBOSE" || "${LOG_LEVEL:-INFO}" == "DEBUG" ]]; then
        { set +x; } 2>/dev/null
        "$@"
        local rc=$?
        set -x
        return $rc
    else
        "$@"
    fi
}

# Pre-flight checks before installation begins
# Validates: root privileges, live ISO environment, EFI state
# Fails fast if any check fails (per architecture.md)
perform_preflight_checks() {
    log_info "Running pre-flight checks..."

    # Must be root
    if [[ "$(id -u)" -ne 0 ]]; then
        log_error "This script must be run as root"
        return 1
    fi

    # Must be on Arch live ISO (check for archiso marker)
    if [[ ! -d "/run/archiso" ]] && [[ ! -f "/etc/arch-release" ]]; then
        log_warn "Not running on Arch Linux ISO — proceeding with caution"
    fi

    # Check that we have basic network connectivity
    if ! ping -c 1 -W 3 archlinux.org >/dev/null 2>&1; then
        log_error "No network connectivity. Cannot reach archlinux.org"
        log_error "Please configure networking before running the installer"
        return 1
    fi

    log_success "Pre-flight checks passed"
    return 0
}

# Check for and install required dependencies on the live ISO
# These are tools needed by partitioning strategies and install phases
check_and_install_dependencies() {
    log_info "Checking required dependencies..."

    local -a missing_deps=()
    local -a required_cmds=(
        "sgdisk"
        "mkfs.ext4"
        "mkfs.fat"
        "arch-chroot"
        "genfstab"
        "pacstrap"
        "reflector"
    )

    for cmd in "${required_cmds[@]}"; do
        if ! command -v "$cmd" >/dev/null 2>&1; then
            missing_deps+=("$cmd")
        fi
    done

    if [[ ${#missing_deps[@]} -eq 0 ]]; then
        log_success "All required dependencies present"
        return 0
    fi

    log_warn "Missing commands: ${missing_deps[*]}"
    log_info "Attempting to install missing dependencies..."

    # Map commands to packages
    local -a packages_to_install=()
    for cmd in "${missing_deps[@]}"; do
        case "$cmd" in
            sgdisk)       packages_to_install+=("gptfdisk") ;;
            mkfs.ext4)    packages_to_install+=("e2fsprogs") ;;
            mkfs.fat)     packages_to_install+=("dosfstools") ;;
            arch-chroot)  packages_to_install+=("arch-install-scripts") ;;
            genfstab)     packages_to_install+=("arch-install-scripts") ;;
            pacstrap)     packages_to_install+=("arch-install-scripts") ;;
            reflector)    packages_to_install+=("reflector") ;;
            *)            log_warn "Unknown command: $cmd — cannot resolve package" ;;
        esac
    done

    # Deduplicate
    local -a unique_packages=()
    local seen=""
    for pkg in "${packages_to_install[@]}"; do
        if [[ "$seen" != *"$pkg"* ]]; then
            unique_packages+=("$pkg")
            seen="$seen $pkg"
        fi
    done

    if [[ ${#unique_packages[@]} -gt 0 ]]; then
        log_info "Installing: ${unique_packages[*]}"
        if ! pacman -Sy "${unique_packages[@]}" --noconfirm 2>&1; then
            log_error "Failed to install dependencies: ${unique_packages[*]}"
            return 1
        fi
        log_success "Dependencies installed"
    fi

    return 0
}
