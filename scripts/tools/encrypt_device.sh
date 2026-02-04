#!/bin/bash
# encrypt_device.sh - LUKS2 encryption operations (Sprint 11)
#
# ACTIONS:
#   format  - Format a device with LUKS2 encryption
#   open    - Open (unlock) an encrypted LUKS device
#   close   - Close (lock) an opened LUKS device
#
# ENVIRONMENT CONTRACT:
#   CONFIRM_LUKS_FORMAT=yes   Required for format action only.
#
# SECURITY:
#   - Passwords are NEVER passed via CLI arguments
#   - Passwords are read from keyfiles only
#   - Keyfile path is passed via --key-file flag
#   - The Rust caller manages keyfile lifecycle (SecretFile RAII)
#   - This script does NOT print keyfile contents to logs
#
# This script is NON-INTERACTIVE. All confirmation from environment.

set -euo pipefail

# --- Signal Handling ---
cleanup_and_exit() {
    local sig="$1"
    echo "ENCRYPT_DEVICE: Received $sig, aborting..." >&2
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
ACTION=""
DEVICE=""
CIPHER="aes-xts-plain64"
KEY_SIZE="512"
KEY_FILE=""
MAPPER_NAME=""
LABEL=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --action)   ACTION="$2"; shift 2 ;;
        --device)   DEVICE="$2"; shift 2 ;;
        --cipher)   CIPHER="$2"; shift 2 ;;
        --key-size) KEY_SIZE="$2"; shift 2 ;;
        --key-file) KEY_FILE="$2"; shift 2 ;;
        --mapper)   MAPPER_NAME="$2"; shift 2 ;;
        --label)    LABEL="$2"; shift 2 ;;
        *) error_exit "Unknown argument: $1" ;;
    esac
done

# --- Validation ---
if [[ -z "$ACTION" ]]; then
    error_exit "Missing required argument: --action (format|open|close)"
fi

# --- Action Dispatch ---
case "$ACTION" in
    format)
        # Validate required arguments
        if [[ -z "$DEVICE" ]]; then
            error_exit "Missing required argument: --device"
        fi
        if [[ -z "$KEY_FILE" ]]; then
            error_exit "Missing required argument: --key-file"
        fi

        # Validate device path
        if ! validate_device_path "$DEVICE"; then
            error_exit "Invalid device path: $DEVICE"
        fi

        # Environment contract enforcement
        if [[ "${CONFIRM_LUKS_FORMAT:-}" != "yes" ]]; then
            error_exit "CONFIRM_LUKS_FORMAT=yes is required. Refusing to format without confirmation."
        fi

        # Validate keyfile exists and has correct permissions
        if [[ ! -f "$KEY_FILE" ]]; then
            error_exit "Key file not found: $KEY_FILE"
        fi

        # Check keyfile permissions (must be 0600)
        local_perms=$(stat -c '%a' "$KEY_FILE" 2>/dev/null || echo "unknown")
        if [[ "$local_perms" != "600" ]]; then
            log_warn "Key file permissions are $local_perms (expected 600)"
        fi

        # DESTRUCTIVE: Format with LUKS2
        log_phase "LUKS Format: $DEVICE"
        log_info "Cipher: $CIPHER"
        log_info "Key size: $KEY_SIZE bits"
        log_warn "THIS WILL DESTROY ALL DATA ON $DEVICE"

        # Build cryptsetup command
        LUKS_ARGS=(
            luksFormat
            --type luks2
            --cipher "$CIPHER"
            --key-size "$KEY_SIZE"
            --hash sha256
            --iter-time 5000
            --batch-mode
            --key-file "$KEY_FILE"
        )

        if [[ -n "$LABEL" ]]; then
            LUKS_ARGS+=(--label "$LABEL")
        fi

        LUKS_ARGS+=("$DEVICE")

        log_cmd "cryptsetup ${LUKS_ARGS[*]}"
        if ! cryptsetup "${LUKS_ARGS[@]}"; then
            error_exit "LUKS format failed on $DEVICE"
        fi

        log_success "LUKS2 format completed on $DEVICE"
        ;;

    open)
        # Validate required arguments
        if [[ -z "$DEVICE" ]]; then
            error_exit "Missing required argument: --device"
        fi
        if [[ -z "$MAPPER_NAME" ]]; then
            error_exit "Missing required argument: --mapper"
        fi
        if [[ -z "$KEY_FILE" ]]; then
            error_exit "Missing required argument: --key-file"
        fi

        # Validate device path
        if ! validate_device_path "$DEVICE"; then
            error_exit "Invalid device path: $DEVICE"
        fi

        # Validate keyfile exists
        if [[ ! -f "$KEY_FILE" ]]; then
            error_exit "Key file not found: $KEY_FILE"
        fi

        # Validate mapper name (alphanumeric, dash, underscore only)
        if ! validate_safe_string "$MAPPER_NAME"; then
            error_exit "Invalid mapper name: $MAPPER_NAME (alphanumeric, dash, underscore only)"
        fi

        log_phase "LUKS Open: $DEVICE -> /dev/mapper/$MAPPER_NAME"
        log_cmd "cryptsetup luksOpen --key-file [REDACTED] $DEVICE $MAPPER_NAME"

        if ! cryptsetup luksOpen --key-file "$KEY_FILE" "$DEVICE" "$MAPPER_NAME"; then
            error_exit "LUKS open failed for $DEVICE"
        fi

        log_success "LUKS device opened at /dev/mapper/$MAPPER_NAME"
        ;;

    close)
        # Validate required arguments
        if [[ -z "$MAPPER_NAME" ]]; then
            error_exit "Missing required argument: --mapper"
        fi

        # Validate mapper name
        if ! validate_safe_string "$MAPPER_NAME"; then
            error_exit "Invalid mapper name: $MAPPER_NAME"
        fi

        log_phase "LUKS Close: /dev/mapper/$MAPPER_NAME"
        log_cmd "cryptsetup luksClose $MAPPER_NAME"

        if ! cryptsetup luksClose "$MAPPER_NAME"; then
            error_exit "LUKS close failed for $MAPPER_NAME"
        fi

        log_success "LUKS device closed: $MAPPER_NAME"
        ;;

    *)
        error_exit "Unknown action: $ACTION (valid: format, open, close)"
        ;;
esac
