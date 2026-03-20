#!/bin/bash
# manual_partition.sh - Multi-action disk partitioning tool
# Usage: ./manual_partition.sh --device /dev/sda --action <action> [options]
#
# Actions:
#   create-table  - Create a new GPT or MBR partition table
#   add-partition  - Add a partition with size and type
#   delete-partition - Delete a partition by number
#   cfdisk         - Launch interactive cfdisk editor

set -euo pipefail

# --- Signal Handling ---
cleanup_and_exit() {
    local sig="$1"
    echo "$(basename "$0"): Received $sig, aborting..." >&2
    [[ "$sig" == "SIGTERM" ]] && exit 143 || exit 130
}
trap 'cleanup_and_exit SIGTERM' SIGTERM
trap 'cleanup_and_exit SIGINT' SIGINT

# Source common utilities via bootstrap
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
# shellcheck source=../bootstrap.sh
source "$SCRIPT_DIR/../bootstrap.sh" || { echo "FATAL: Cannot source bootstrap.sh" >&2; exit 1; }
source_or_die "$SCRIPT_DIR/../utils.sh"
source_or_die "$SCRIPT_DIR/../disk_utils.sh"

require_root

# Default values
DEVICE=""
ACTION=""
TABLE_TYPE=""
PART_NUMBER=""
PART_SIZE=""
PART_TYPE=""
PART_LABEL=""

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --device)
            DEVICE="$2"
            shift 2
            ;;
        --action)
            ACTION="$2"
            shift 2
            ;;
        --table-type)
            TABLE_TYPE="$2"
            shift 2
            ;;
        --number)
            PART_NUMBER="$2"
            shift 2
            ;;
        --size)
            PART_SIZE="$2"
            shift 2
            ;;
        --type)
            PART_TYPE="$2"
            shift 2
            ;;
        --label)
            PART_LABEL="$2"
            shift 2
            ;;
        --help)
            echo "Usage: $0 --device <device> --action <action> [options]"
            echo ""
            echo "Actions:"
            echo "  create-table   --table-type gpt|mbr"
            echo "  add-partition  --number N --size SIZE --type CODE [--label LABEL]"
            echo "  delete-partition --number N"
            echo "  cfdisk         (interactive editor)"
            echo ""
            echo "Type codes: EF00 (EFI), EF02 (BIOS Boot), 8300 (Linux),"
            echo "            8200 (Swap), 8E00 (LVM), 8309 (LUKS)"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate required arguments
if [[ -z "$DEVICE" ]]; then
    error_exit "Device is required (--device /dev/sda)"
fi

if [[ -z "$ACTION" ]]; then
    error_exit "Action is required (--action create-table|add-partition|delete-partition|cfdisk)"
fi

# Validate device path (injection prevention)
if ! validate_device_path "$DEVICE"; then
    error_exit "Invalid device path: $DEVICE"
fi

# Check if device exists
if [[ ! -b "$DEVICE" ]]; then
    error_exit "Device does not exist: $DEVICE"
fi

# ENVIRONMENT CONTRACT: Require explicit confirmation for destructive operation
if [[ "${CONFIRM_MANUAL_PARTITION:-}" != "yes" ]]; then
    error_exit "CONFIRM_MANUAL_PARTITION=yes is required. This script refuses to run without explicit environment confirmation."
fi

# --- Helper: unmount all partitions on device ---
unmount_device_partitions() {
    local dev="$1"
    local partition
    for partition in "${dev}"*; do
        if [[ -b "$partition" ]] && mountpoint -q "$partition" 2>/dev/null; then
            log_cmd "umount $partition"
            umount "$partition" || log_warning "Failed to unmount $partition"
        fi
    done
}

# --- Helper: show partition table ---
show_partition_table() {
    local dev="$1"
    echo ""
    log_info "Current partition table for $dev:"
    if sgdisk -p "$dev" 2>/dev/null; then
        : # sgdisk output shown
    else
        # Fallback for MBR tables
        fdisk -l "$dev" 2>/dev/null || log_warning "Could not read partition table"
    fi
    echo ""
    log_info "Block device info:"
    lsblk -o NAME,SIZE,TYPE,FSTYPE,LABEL,MOUNTPOINTS "$dev" 2>/dev/null || true
}

# --- Helper: detect current table type ---
detect_table_type() {
    local dev="$1"
    local pttype
    pttype=$(blkid -o value -s PTTYPE "$dev" 2>/dev/null || echo "")
    echo "$pttype"
}

# =============================================================================
# ACTION: create-table
# =============================================================================
action_create_table() {
    if [[ -z "$TABLE_TYPE" ]]; then
        error_exit "--table-type is required for create-table (gpt or mbr)"
    fi

    local table_lower="${TABLE_TYPE,,}"

    unmount_device_partitions "$DEVICE"

    case "$table_lower" in
        gpt)
            log_info "Creating GPT partition table on $DEVICE"
            log_warning "WARNING: This will destroy ALL data on $DEVICE"

            log_cmd "sgdisk --zap-all $DEVICE"
            if ! sgdisk --zap-all "$DEVICE"; then
                error_exit "Failed to zap existing partition data on $DEVICE"
            fi

            log_cmd "sgdisk -o $DEVICE"
            if ! sgdisk -o "$DEVICE"; then
                error_exit "Failed to create GPT partition table on $DEVICE"
            fi

            sync_partitions "$DEVICE"
            log_success "GPT partition table created on $DEVICE"
            ;;
        mbr|dos|msdos)
            log_info "Creating MBR partition table on $DEVICE"
            log_warning "WARNING: This will destroy ALL data on $DEVICE"

            log_cmd "echo 'label: dos' | sfdisk $DEVICE"
            if ! echo "label: dos" | sfdisk "$DEVICE"; then
                error_exit "Failed to create MBR partition table on $DEVICE"
            fi

            sync_partitions "$DEVICE"
            log_success "MBR partition table created on $DEVICE"
            ;;
        *)
            error_exit "Invalid table type: $TABLE_TYPE (valid: gpt, mbr)"
            ;;
    esac

    show_partition_table "$DEVICE"
}

# =============================================================================
# ACTION: add-partition
# =============================================================================
action_add_partition() {
    if [[ -z "$PART_NUMBER" ]]; then
        error_exit "--number is required for add-partition"
    fi
    if [[ -z "$PART_SIZE" ]]; then
        error_exit "--size is required for add-partition"
    fi
    if [[ -z "$PART_TYPE" ]]; then
        error_exit "--type is required for add-partition (e.g., EF00, 8300, 8200)"
    fi

    # Validate partition number is numeric
    if ! [[ "$PART_NUMBER" =~ ^[0-9]+$ ]]; then
        error_exit "Partition number must be numeric: $PART_NUMBER"
    fi

    local pttype
    pttype=$(detect_table_type "$DEVICE")

    case "$pttype" in
        gpt)
            _add_partition_gpt
            ;;
        dos)
            _add_partition_mbr
            ;;
        "")
            log_warning "No partition table detected — assuming GPT"
            _add_partition_gpt
            ;;
        *)
            error_exit "Unsupported partition table type: $pttype"
            ;;
    esac

    sync_partitions "$DEVICE"

    local part_path
    part_path=$(get_partition_path "$DEVICE" "$PART_NUMBER")
    if [[ -b "$part_path" ]]; then
        log_success "Created partition $PART_NUMBER: $part_path"
    else
        log_warning "Partition created but $part_path not yet visible"
    fi

    show_partition_table "$DEVICE"
}

_add_partition_gpt() {
    # Build sgdisk args array (no eval)
    local sgdisk_args=()

    # Size: "remaining" maps to 0 (use all remaining space)
    local size_spec
    if [[ "${PART_SIZE,,}" == "remaining" ]]; then
        size_spec="0"
    else
        size_spec="+${PART_SIZE}"
    fi

    sgdisk_args+=("-n" "${PART_NUMBER}:0:${size_spec}")
    sgdisk_args+=("-t" "${PART_NUMBER}:${PART_TYPE}")

    if [[ -n "$PART_LABEL" ]]; then
        sgdisk_args+=("-c" "${PART_NUMBER}:${PART_LABEL}")
    fi

    log_cmd "sgdisk ${sgdisk_args[*]} $DEVICE"
    if ! sgdisk "${sgdisk_args[@]}" "$DEVICE"; then
        error_exit "Failed to create GPT partition $PART_NUMBER on $DEVICE"
    fi
}

_add_partition_mbr() {
    # Map GPT type codes to MBR type IDs
    local mbr_type
    case "${PART_TYPE^^}" in
        EF00) mbr_type="ef" ;;
        8300|8309) mbr_type="83" ;;
        8200) mbr_type="82" ;;
        8E00) mbr_type="8e" ;;
        EF02) mbr_type="ef" ;; # BIOS boot — best MBR approximation
        *) mbr_type="83" ;; # Default to Linux
    esac

    # Size: "remaining" → use all remaining space (empty size field in sfdisk)
    local size_field
    if [[ "${PART_SIZE,,}" == "remaining" ]]; then
        size_field=""
    else
        size_field="size=${PART_SIZE},"
    fi

    local sfdisk_input="${size_field} type=${mbr_type}"

    log_cmd "echo '$sfdisk_input' | sfdisk --append $DEVICE"
    if ! echo "$sfdisk_input" | sfdisk --append "$DEVICE"; then
        error_exit "Failed to create MBR partition $PART_NUMBER on $DEVICE"
    fi
}

# =============================================================================
# ACTION: delete-partition
# =============================================================================
action_delete_partition() {
    if [[ -z "$PART_NUMBER" ]]; then
        error_exit "--number is required for delete-partition"
    fi

    # Validate partition number is numeric
    if ! [[ "$PART_NUMBER" =~ ^[0-9]+$ ]]; then
        error_exit "Partition number must be numeric: $PART_NUMBER"
    fi

    # Unmount the partition if mounted
    local part_path
    part_path=$(get_partition_path "$DEVICE" "$PART_NUMBER")
    if [[ -b "$part_path" ]] && mountpoint -q "$part_path" 2>/dev/null; then
        log_cmd "umount $part_path"
        umount "$part_path" || log_warning "Failed to unmount $part_path"
    fi

    local pttype
    pttype=$(detect_table_type "$DEVICE")

    case "$pttype" in
        gpt)
            log_info "Deleting GPT partition $PART_NUMBER from $DEVICE"
            log_cmd "sgdisk -d $PART_NUMBER $DEVICE"
            if ! sgdisk -d "$PART_NUMBER" "$DEVICE"; then
                error_exit "Failed to delete partition $PART_NUMBER from $DEVICE"
            fi
            ;;
        dos)
            log_info "Deleting MBR partition $PART_NUMBER from $DEVICE"
            log_cmd "sfdisk --delete $DEVICE $PART_NUMBER"
            if ! sfdisk --delete "$DEVICE" "$PART_NUMBER"; then
                error_exit "Failed to delete partition $PART_NUMBER from $DEVICE"
            fi
            ;;
        *)
            error_exit "Cannot detect partition table type on $DEVICE (got: '$pttype')"
            ;;
    esac

    sync_partitions "$DEVICE"
    log_success "Partition $PART_NUMBER deleted from $DEVICE"
    show_partition_table "$DEVICE"
}

# =============================================================================
# ACTION: cfdisk (interactive)
# =============================================================================
action_cfdisk() {
    unmount_device_partitions "$DEVICE"

    log_info "Launching cfdisk for manual partitioning of $DEVICE"
    log_warning "WARNING: This will modify the partition table of $DEVICE"

    # Check if cfdisk is available
    if ! command -v cfdisk >/dev/null 2>&1; then
        log_info "Installing util-linux (contains cfdisk)..."
        log_cmd "pacman -Sy util-linux --noconfirm"
        pacman -Sy util-linux --noconfirm || error_exit "Failed to install util-linux"
    fi

    log_cmd "cfdisk $DEVICE"
    if cfdisk "$DEVICE"; then
        log_success "Partitioning completed successfully"
        sync_partitions "$DEVICE"
        show_partition_table "$DEVICE"
    else
        log_error "Partitioning failed or was cancelled"
        exit 1
    fi
}

# =============================================================================
# ACTION ROUTER
# =============================================================================
case "$ACTION" in
    create-table)
        action_create_table
        ;;
    add-partition)
        action_add_partition
        ;;
    delete-partition)
        action_delete_partition
        ;;
    cfdisk)
        action_cfdisk
        ;;
    *)
        error_exit "Unknown action: $ACTION (valid: create-table, add-partition, delete-partition, cfdisk)"
        ;;
esac
