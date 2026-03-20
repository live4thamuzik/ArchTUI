#!/bin/bash
# bootstrap.sh - Minimal bootstrap for sourcing other scripts
#
# This file defines ONLY source_or_die(), solving the chicken-and-egg problem
# where scripts need source_or_die() to safely source utils.sh, but
# source_or_die() was previously defined IN utils.sh.
#
# Usage (2-line inline source before utils.sh is available):
#   SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
#   source "$SCRIPT_DIR/bootstrap.sh" || { echo "FATAL: Cannot source bootstrap.sh" >&2; exit 1; }
#   source_or_die "$SCRIPT_DIR/utils.sh"

# Source-once guard
if [[ -n "${_BOOTSTRAP_SH_SOURCED:-}" ]]; then
    # shellcheck disable=SC2317
    return 0 2>/dev/null || true
fi
readonly _BOOTSTRAP_SH_SOURCED=1

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
