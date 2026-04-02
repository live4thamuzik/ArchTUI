#!/bin/bash
# detect_os.sh — Scan disks for existing operating systems
# Called by: Rust TUI at disk selection time
# Output: JSON to stdout with detected OS entries
# Non-destructive: read-only mounts only, all temp mounts cleaned up
set -euo pipefail

# --- Signal Handling ---
TEMP_MOUNTS=()
# shellcheck disable=SC2317
cleanup() {
    # Unmount all temp mounts in reverse order
    for ((i=${#TEMP_MOUNTS[@]}-1; i>=0; i--)); do
        umount "${TEMP_MOUNTS[i]}" 2>/dev/null || true
        rmdir "${TEMP_MOUNTS[i]}" 2>/dev/null || true
    done
}
# shellcheck disable=SC2317
cleanup_term() {
    cleanup
    exit 143
}
# shellcheck disable=SC2317
cleanup_int() {
    cleanup
    exit 130
}
trap cleanup EXIT
trap cleanup_term SIGTERM
trap cleanup_int SIGINT

# Source common utilities via bootstrap
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../bootstrap.sh
source "$SCRIPT_DIR/../bootstrap.sh" || { echo "FATAL: Cannot source bootstrap.sh" >&2; exit 1; }
source_or_die "$SCRIPT_DIR/../utils.sh"

require_root

# --- Parse Arguments ---
INSTALL_DISK=""

while [[ $# -gt 0 ]]; do
    case "$1" in
        --install-disk)
            INSTALL_DISK="${2:-}"
            shift 2
            ;;
        *)
            log_error "Unknown argument: $1"
            exit 1
            ;;
    esac
done

if [[ -z "$INSTALL_DISK" ]]; then
    log_error "Usage: detect_os.sh --install-disk /dev/sdX"
    exit 1
fi

# Strip to base disk for same-disk comparison (handle comma-separated RAID)
BASE_DISK="${INSTALL_DISK%%,*}"

# --- JSON Output Builder ---
# We build JSON manually to avoid jq dependency
JSON_ENTRIES=()

add_json_entry() {
    local name="$1"
    local device="$2"
    local os_type="$3"
    local same_disk="$4"

    # Escape double quotes in name
    name="${name//\"/\\\"}"
    JSON_ENTRIES+=("{\"name\":\"${name}\",\"device\":\"${device}\",\"type\":\"${os_type}\",\"same_disk\":${same_disk}}")
}

# --- Detect Windows (scan ESPs for bootmgfw.efi) ---
log_info "Scanning for Windows installations..."

esp_list=$(lsblk -rno NAME,PARTTYPE 2>/dev/null \
    | grep -i "c12a7328-f81f-11d2-ba4b-00a0c93ec93b" \
    | awk '{print "/dev/"$1}') || true

while IFS= read -r esp_device; do
    [[ -z "$esp_device" ]] && continue
    [[ -b "$esp_device" ]] || continue

    temp_mount="/tmp/detect_os_esp_$$"
    mkdir -p "$temp_mount"

    if mount -o ro "$esp_device" "$temp_mount" 2>/dev/null; then
        TEMP_MOUNTS+=("$temp_mount")

        if [[ -d "$temp_mount/EFI/Microsoft/Boot" ]] \
           && [[ -f "$temp_mount/EFI/Microsoft/Boot/bootmgfw.efi" ]]; then
            # Determine if this ESP is on the install disk
            is_same="false"
            if [[ "$esp_device" == "${BASE_DISK}"* ]]; then
                is_same="true"
            fi
            log_info "Windows Boot Manager detected on $esp_device"
            add_json_entry "Windows Boot Manager" "$esp_device" "windows" "$is_same"
        fi

        umount "$temp_mount" 2>/dev/null || true
        # Remove from tracking after successful unmount
        unset 'TEMP_MOUNTS[-1]'
    fi
    rmdir "$temp_mount" 2>/dev/null || true
done <<< "$esp_list"

# --- Detect Linux (scan ext4/btrfs/xfs for /etc/os-release) ---
log_info "Scanning for Linux installations..."

linux_candidates=$(lsblk -rno NAME,FSTYPE 2>/dev/null \
    | grep -E "ext4|btrfs|xfs" \
    | awk '{print "/dev/"$1}') || true

while IFS= read -r part; do
    [[ -z "$part" ]] && continue
    [[ -b "$part" ]] || continue

    # Skip partitions that are too small to be root (< 1GB)
    local_size=$(lsblk -brno SIZE "$part" 2>/dev/null || echo "0")
    if [[ "$local_size" -lt 1073741824 ]]; then
        continue
    fi

    temp_mount="/tmp/detect_os_linux_$$"
    mkdir -p "$temp_mount"

    if mount -o ro "$part" "$temp_mount" 2>/dev/null; then
        TEMP_MOUNTS+=("$temp_mount")

        if [[ -f "$temp_mount/etc/os-release" ]]; then
            os_name=$(grep "^NAME=" "$temp_mount/etc/os-release" 2>/dev/null \
                | cut -d= -f2 | tr -d '"') || true

            if [[ -n "$os_name" ]]; then
                is_same="false"
                if [[ "$part" == "${BASE_DISK}"* ]]; then
                    is_same="true"
                fi
                log_info "Found Linux installation: $os_name on $part"
                add_json_entry "$os_name" "$part" "linux" "$is_same"
            fi
        fi

        umount "$temp_mount" 2>/dev/null || true
        unset 'TEMP_MOUNTS[-1]'
    fi
    rmdir "$temp_mount" 2>/dev/null || true
done <<< "$linux_candidates"

# --- Output JSON ---
# Build the final JSON array
if [[ ${#JSON_ENTRIES[@]} -eq 0 ]]; then
    printf '{"os":[]}\n'
else
    printf '{"os":['
    for i in "${!JSON_ENTRIES[@]}"; do
        if [[ $i -gt 0 ]]; then
            printf ','
        fi
        printf '%s' "${JSON_ENTRIES[$i]}"
    done
    printf ']}\n'
fi
