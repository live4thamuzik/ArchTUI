#!/usr/bin/env bash
# run_as_user.sh — Execute a command as a non-root user inside arch-chroot
#
# Usage: run_as_user.sh --user <user> --cmd <command> --root <chroot_path> [--workdir <dir>]
#
# CONSTRAINT: makepkg forbids running as root. This script drops
# privileges via `sudo -u <user>` inside arch-chroot.

set -euo pipefail

# --- Signal handling ---
cleanup() {
    log_info "run_as_user: received signal, cleaning up"
    exit 130
}
trap cleanup SIGTERM SIGINT SIGHUP

# --- Logging ---
log_info()  { echo "[INFO]  $*"; }
log_warn()  { echo "[WARN]  $*" >&2; }
log_error() { echo "[ERROR] $*" >&2; }

# --- Argument parsing ---
USER=""
CMD=""
ROOT=""
WORKDIR=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --user)    USER="$2";    shift 2 ;;
        --cmd)     CMD="$2";     shift 2 ;;
        --root)    ROOT="$2";    shift 2 ;;
        --workdir) WORKDIR="$2"; shift 2 ;;
        *)
            log_error "Unknown argument: $1"
            exit 1
            ;;
    esac
done

# --- Validation ---
if [[ -z "$USER" ]]; then
    log_error "--user is required"
    exit 1
fi

if [[ -z "$CMD" ]]; then
    log_error "--cmd is required"
    exit 1
fi

if [[ -z "$ROOT" ]]; then
    log_error "--root is required"
    exit 1
fi

if [[ ! -d "$ROOT" ]]; then
    log_error "Chroot path does not exist: $ROOT"
    exit 1
fi

# Verify the user exists inside the chroot
if ! arch-chroot "$ROOT" id "$USER" &>/dev/null; then
    log_error "User '$USER' does not exist in chroot at $ROOT"
    exit 1
fi

# --- Execute ---
log_info "Running as user '$USER' in chroot '$ROOT': $CMD"

if [[ -n "$WORKDIR" ]]; then
    arch-chroot "$ROOT" sudo -u "$USER" bash -c "cd '$WORKDIR' && $CMD"
else
    arch-chroot "$ROOT" sudo -u "$USER" bash -c "$CMD"
fi

exit_code=$?

if [[ $exit_code -eq 0 ]]; then
    log_info "Command completed successfully"
else
    log_error "Command failed with exit code $exit_code"
fi

exit $exit_code
