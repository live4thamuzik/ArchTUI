#!/usr/bin/env bash
# install_aur_helper.sh — Install an AUR helper (paru/yay) as a non-root user
#
# Usage: install_aur_helper.sh --helper <paru|yay> --user <user> --root <chroot_path>
#
# CONSTRAINT: makepkg forbids running as root.
# This script drops privileges via `sudo -u <user>` inside arch-chroot.
#
# FAILURE POLICY: Non-fatal. Caller should log warning and continue
# if this script exits non-zero.

set -euo pipefail

# --- Signal handling ---
# shellcheck disable=SC2317  # Trap handler is invoked indirectly via signal
cleanup() {
    log_info "install_aur_helper: received signal, cleaning up"
    # Remove partial build directory if it exists
    if [[ -n "${BUILD_DIR:-}" && -d "$ROOT/$BUILD_DIR" ]]; then
        rm -rf "${ROOT:?}/${BUILD_DIR}"
        log_info "Cleaned up partial build directory"
    fi
    exit 130
}
trap cleanup SIGTERM SIGINT SIGHUP

# --- Logging ---
log_info()  { echo "[INFO]  $*"; }
log_warn()  { echo "[WARN]  $*" >&2; }
log_error() { echo "[ERROR] $*" >&2; }

# --- Argument parsing ---
HELPER=""
USER=""
ROOT=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --helper) HELPER="$2"; shift 2 ;;
        --user)   USER="$2";   shift 2 ;;
        --root)   ROOT="$2";   shift 2 ;;
        *)
            log_error "Unknown argument: $1"
            exit 1
            ;;
    esac
done

# --- Validation ---
if [[ -z "$HELPER" ]]; then
    log_error "--helper is required (paru or yay)"
    exit 1
fi

if [[ -z "$USER" ]]; then
    log_error "--user is required"
    exit 1
fi

if [[ -z "$ROOT" ]]; then
    log_error "--root is required"
    exit 1
fi

if [[ "$HELPER" != "paru" && "$HELPER" != "yay" ]]; then
    log_error "Invalid AUR helper: $HELPER (must be 'paru' or 'yay')"
    exit 1
fi

if [[ ! -d "$ROOT" ]]; then
    log_error "Chroot path does not exist: $ROOT"
    exit 1
fi

# Verify user exists inside chroot
if ! arch-chroot "$ROOT" id "$USER" &>/dev/null; then
    log_error "User '$USER' does not exist in chroot at $ROOT"
    exit 1
fi

# Verify git is available inside chroot
if ! arch-chroot "$ROOT" which git &>/dev/null; then
    log_error "git is not installed in the chroot — cannot clone AUR helper"
    exit 1
fi

# --- AUR helper URLs ---
case "$HELPER" in
    paru) AUR_URL="https://aur.archlinux.org/paru.git" ;;
    yay)  AUR_URL="https://aur.archlinux.org/yay.git"  ;;
esac

BUILD_DIR="/home/$USER/$HELPER"

log_info "Installing AUR helper: $HELPER"
log_info "  User: $USER"
log_info "  Clone URL: $AUR_URL"
log_info "  Build dir: $BUILD_DIR (inside chroot)"

# --- Step 1: Clone the AUR helper repo ---
log_info "Step 1/3: Cloning $HELPER repository..."

# Remove any previous partial clone
if [[ -d "$ROOT/$BUILD_DIR" ]]; then
    log_warn "Removing previous build directory: $BUILD_DIR"
    rm -rf "${ROOT:?}/${BUILD_DIR}"
fi

arch-chroot "$ROOT" sudo -u "$USER" git clone "$AUR_URL" "$BUILD_DIR"

if [[ ! -d "$ROOT/$BUILD_DIR/PKGBUILD" ]] && [[ ! -f "$ROOT/$BUILD_DIR/PKGBUILD" ]]; then
    # Check if PKGBUILD exists (file, not dir)
    if [[ ! -f "$ROOT/$BUILD_DIR/PKGBUILD" ]]; then
        log_error "PKGBUILD not found in $BUILD_DIR — clone may have failed"
        exit 1
    fi
fi

# --- Step 2: Build and install ---
log_info "Step 2/3: Building and installing $HELPER (makepkg -si --noconfirm)..."

arch-chroot "$ROOT" sudo -u "$USER" bash -c "cd '$BUILD_DIR' && makepkg -si --noconfirm"

# --- Step 3: Verify installation ---
log_info "Step 3/3: Verifying $HELPER installation..."

if arch-chroot "$ROOT" which "$HELPER" &>/dev/null; then
    log_info "$HELPER installed successfully"
else
    log_error "$HELPER binary not found after installation"
    exit 1
fi

# --- Cleanup build directory ---
log_info "Cleaning up build directory: $BUILD_DIR"
rm -rf "${ROOT:?}/${BUILD_DIR}"

log_info "AUR helper $HELPER installation complete"
exit 0
