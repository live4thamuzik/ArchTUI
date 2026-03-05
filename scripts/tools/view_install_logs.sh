#!/bin/bash
# view_install_logs.sh - View ArchTUI installation logs
# Usage: ./view_install_logs.sh [--latest] [--list]

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

LOG_DIR="${ARCHTUI_LOG_DIR:-/var/log/archtui}"
TMP_LOG="/tmp/archtui.log"
ACTION="latest"

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --latest)
            ACTION="latest"
            shift
            ;;
        --list)
            ACTION="list"
            shift
            ;;
        --help)
            echo "Usage: $0 [--latest] [--list]"
            echo ""
            echo "View ArchTUI installation logs."
            echo ""
            echo "Options:"
            echo "  --latest    Show the most recent installation log (default)"
            echo "  --list      List all available log files"
            echo "  --help      Show this help message"
            echo ""
            echo "Log locations:"
            echo "  Master logs:  $LOG_DIR/install-*-master.log"
            echo "  TUI debug:    $TMP_LOG"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

case "$ACTION" in
    list)
        echo "=== ArchTUI Installation Logs ==="
        echo ""

        if [[ -d "$LOG_DIR" ]]; then
            local_logs=()
            while IFS= read -r -d '' f; do
                local_logs+=("$f")
            done < <(find "$LOG_DIR" -name "*.log" -print0 2>/dev/null | sort -z)

            if [[ ${#local_logs[@]} -gt 0 ]]; then
                echo "Master logs ($LOG_DIR):"
                for f in "${local_logs[@]}"; do
                    size=$(stat -c %s "$f" 2>/dev/null || echo "?")
                    date=$(stat -c %y "$f" 2>/dev/null | cut -d. -f1 || echo "?")
                    printf "  %-60s %8s bytes  %s\n" "$(basename "$f")" "$size" "$date"
                done
            else
                echo "  No master logs found in $LOG_DIR"
            fi
        else
            echo "  Log directory $LOG_DIR does not exist"
        fi

        echo ""
        if [[ -f "$TMP_LOG" ]]; then
            size=$(stat -c %s "$TMP_LOG" 2>/dev/null || echo "?")
            echo "TUI debug log: $TMP_LOG ($size bytes)"
        else
            echo "TUI debug log: $TMP_LOG (not found)"
        fi
        ;;

    latest)
        # Find the most recent master log
        latest_log=""
        if [[ -d "$LOG_DIR" ]]; then
            latest_log=$(find "$LOG_DIR" -name "*-master.log" -printf '%T@ %p\n' 2>/dev/null \
                | sort -rn | head -1 | cut -d' ' -f2-)
        fi

        if [[ -n "$latest_log" && -f "$latest_log" ]]; then
            log_info "Showing latest master log: $latest_log"
            echo "=== $(basename "$latest_log") ==="
            echo ""
            cat "$latest_log"
        elif [[ -f "$TMP_LOG" ]]; then
            log_info "No master log found, showing TUI debug log: $TMP_LOG"
            echo "=== TUI Debug Log ==="
            echo ""
            cat "$TMP_LOG"
        else
            log_warn "No installation logs found"
            echo "No installation logs found."
            echo ""
            echo "Log locations checked:"
            echo "  $LOG_DIR/install-*-master.log"
            echo "  $TMP_LOG"
            echo ""
            echo "Logs are created during installation."
        fi
        ;;
esac

log_success "Log viewer complete"
