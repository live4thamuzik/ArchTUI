#!/bin/bash
# configure_locale.sh - Configure system locale, hostname, timezone, and keymap
# Usage: ./configure_locale.sh --root /mnt --hostname archbox --locale en_US.UTF-8 --timezone America/New_York [--keymap us]

set -euo pipefail

# --- Signal Handling ---
cleanup_and_exit() {
    local sig="$1"
    echo "$(basename "$0"): Received $sig, aborting..." >&2
    [[ "$sig" == "SIGTERM" ]] && exit 143 || exit 130
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
ROOT_PATH="/mnt"
HOSTNAME=""
LOCALE=""
TIMEZONE=""
KEYMAP=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --root)
            ROOT_PATH="$2"
            shift 2
            ;;
        --hostname)
            HOSTNAME="$2"
            shift 2
            ;;
        --locale)
            LOCALE="$2"
            shift 2
            ;;
        --timezone)
            TIMEZONE="$2"
            shift 2
            ;;
        --keymap)
            KEYMAP="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 --root <path> --hostname <name> --locale <locale> --timezone <tz> [--keymap <keymap>]"
            echo "  --root <path>       Target root (default: /mnt)"
            echo "  --hostname <name>   System hostname"
            echo "  --locale <locale>   System locale (e.g., en_US.UTF-8)"
            echo "  --timezone <tz>     Timezone (e.g., America/New_York)"
            echo "  --keymap <keymap>   Console keymap (e.g., us)"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate required arguments
if [[ -z "$HOSTNAME" ]]; then
    log_error "Missing required argument: --hostname"
    exit 1
fi
if [[ -z "$LOCALE" ]]; then
    log_error "Missing required argument: --locale"
    exit 1
fi
if [[ -z "$TIMEZONE" ]]; then
    log_error "Missing required argument: --timezone"
    exit 1
fi

# Validate root path
if [[ ! -d "$ROOT_PATH" ]]; then
    log_error "Root path does not exist: $ROOT_PATH"
    exit 1
fi

log_info "Configuring locale settings for $ROOT_PATH"

# --- Hostname ---
log_info "Setting hostname to: $HOSTNAME"
echo "$HOSTNAME" > "$ROOT_PATH/etc/hostname"

# Create /etc/hosts
cat > "$ROOT_PATH/etc/hosts" << EOF
127.0.0.1   localhost
::1         localhost
127.0.1.1   ${HOSTNAME}.localdomain ${HOSTNAME}
EOF
log_success "Hostname configured"

# --- Locale ---
log_info "Configuring locale: $LOCALE"

# Uncomment the locale in locale.gen
if [[ -f "$ROOT_PATH/etc/locale.gen" ]]; then
    sed -i "s|^#${LOCALE}|${LOCALE}|" "$ROOT_PATH/etc/locale.gen"
else
    echo "$LOCALE UTF-8" > "$ROOT_PATH/etc/locale.gen"
fi

# Generate locale inside chroot
if command -v arch-chroot &> /dev/null; then
    arch-chroot "$ROOT_PATH" locale-gen
else
    log_warning "arch-chroot not available, skipping locale-gen"
fi

# Set locale.conf
echo "LANG=$LOCALE" > "$ROOT_PATH/etc/locale.conf"
log_success "Locale configured"

# --- Timezone ---
log_info "Setting timezone to: $TIMEZONE"

# Validate timezone exists and path stays within /usr/share/zoneinfo (prevent path traversal)
tz_real="$(realpath -m "/usr/share/zoneinfo/$TIMEZONE" 2>/dev/null)" || tz_real=""
if [[ -z "$tz_real" || "$tz_real" != /usr/share/zoneinfo/* ]]; then
    log_error "Invalid timezone path (traversal detected): $TIMEZONE"
    exit 1
fi
if [[ ! -f "/usr/share/zoneinfo/$TIMEZONE" ]]; then
    log_error "Invalid timezone: $TIMEZONE"
    exit 1
fi

# Create timezone symlink
ln -sf "/usr/share/zoneinfo/$TIMEZONE" "$ROOT_PATH/etc/localtime"

# Set hardware clock to UTC inside chroot
if command -v arch-chroot &> /dev/null; then
    arch-chroot "$ROOT_PATH" hwclock --systohc
else
    log_warning "arch-chroot not available, skipping hwclock"
fi
log_success "Timezone configured"

# --- Console Keymap (optional) ---
if [[ -n "$KEYMAP" ]]; then
    log_info "Setting console keymap to: $KEYMAP"
    echo "KEYMAP=$KEYMAP" > "$ROOT_PATH/etc/vconsole.conf"
    log_success "Console keymap configured"
fi

log_success "All locale settings configured successfully"
