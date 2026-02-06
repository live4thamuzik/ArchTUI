#!/bin/bash
# mount_partitions.sh - Comprehensive mount/unmount partition management
# Usage: ./mount_partitions.sh --action mount --device /dev/sda1 --mountpoint /mnt

set -euo pipefail

# --- Signal Handling ---
cleanup_and_exit() {
    local sig="$1"
    echo "$(basename "$0"): Received $sig, aborting..." >&2
    exit 130
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
ACTION=""
DEVICE=""
MOUNTPOINT=""
FILESYSTEM=""
OPTIONS=""
READONLY=false
LAZY=false
FORCE=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --action)
            ACTION="$2"
            shift 2
            ;;
        --device)
            DEVICE="$2"
            shift 2
            ;;
        --mountpoint)
            MOUNTPOINT="$2"
            shift 2
            ;;
        --filesystem)
            FILESYSTEM="$2"
            shift 2
            ;;
        --options)
            OPTIONS="$2"
            shift 2
            ;;
        --readonly)
            READONLY=true
            shift
            ;;
        --lazy)
            LAZY=true
            shift
            ;;
        --force)
            FORCE=true
            shift
            ;;
        --help)
            echo "Usage: $0 --action <mount|umount|list|info> [options]"
            echo ""
            echo "Actions:"
            echo "  mount     Mount a device to a directory"
            echo "  umount    Unmount a device or directory (uses umount -R for safety)"
            echo "  list      List all mounted filesystems"
            echo "  info      Show detailed partition information"
            echo ""
            echo "Options:"
            echo "  --device <device>        Device to mount/umount (e.g., /dev/sda1)"
            echo "  --mountpoint <path>      Directory to mount to (user-specified)"
            echo "  --filesystem <type>      Filesystem type (auto-detected if not specified)"
            echo "  --options <opts>         Additional mount options"
            echo "  --readonly               Mount as read-only"
            echo "  --lazy                   Lazy umount (for busy filesystems)"
            echo "  --force                  Force operation (use with caution)"
            echo ""
            echo "Examples:"
            echo "  $0 --action mount --device /dev/sda1 --mountpoint /home/user/data"
            echo "  $0 --action mount --device /dev/sda1 --mountpoint /mnt --readonly"
            echo "  $0 --action umount --device /dev/sda1"
            echo "  $0 --action umount --mountpoint /home/user/data"
            echo "  $0 --action list"
            echo "  $0 --action info --device /dev/sda1"
            echo ""
            echo "Note: Uses 'umount -R' (recursive) for safe unmounting, especially for system repair"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Validate required arguments
if [[ -z "$ACTION" ]]; then
    error_exit "Action is required (--action mount|unmount|list|info)"
fi

# Helper function to detect filesystem type
detect_filesystem() {
    local device="$1"
    local fs_type=""
    
    # Try blkid first (most reliable)
    if command -v blkid >/dev/null 2>&1; then
        fs_type=$(blkid -o value -s TYPE "$device" 2>/dev/null || echo "")
    fi
    
    # Fallback to file command
    if [[ -z "$fs_type" ]] && command -v file >/dev/null 2>&1; then
        fs_type=$(file -s "$device" | grep -o '[a-zA-Z0-9]* filesystem' | head -1 | awk '{print $1}')
    fi
    
    # Fallback to lsblk
    if [[ -z "$fs_type" ]] && command -v lsblk >/dev/null 2>&1; then
        fs_type=$(lsblk -no FSTYPE "$device" 2>/dev/null || echo "")
    fi
    
    echo "$fs_type"
}

# Helper function to get mount options for filesystem
get_mount_options() {
    local fs_type="$1"
    local options=""
    
    case "$fs_type" in
        ext4)
            options="defaults,relatime"
            ;;
        ext3|ext2)
            options="defaults,relatime"
            ;;
        xfs)
            options="defaults,relatime"
            ;;
        btrfs)
            options="defaults,relatime,space_cache"
            ;;
        ntfs)
            options="defaults,uid=1000,gid=1000,umask=0022"
            ;;
        vfat|fat32)
            options="defaults,uid=1000,gid=1000,umask=0022,shortname=mixed"
            ;;
        *)
            options="defaults"
            ;;
    esac
    
    echo "$options"
}

case "$ACTION" in
    info)
        if [[ -z "$DEVICE" ]]; then
            error_exit "Device is required for info (--device /dev/sda1)"
        fi
        
        if [[ ! -b "$DEVICE" ]]; then
            error_exit "Device does not exist: $DEVICE"
        fi
        
        log_info "📊 Partition Information for $DEVICE"
        echo "=================================================="
        
        # Basic device info
        log_info "🔧 Device Details:"
        if command -v lsblk >/dev/null 2>&1; then
            lsblk -o NAME,SIZE,TYPE,MOUNTPOINT,FSTYPE,MODEL "$DEVICE" | sed 's/^/  /'
        fi
        
        echo
        
        # Filesystem information
        log_info "💾 Filesystem Information:"
        if command -v blkid >/dev/null 2>&1; then
            blkid "$DEVICE" | sed 's/^/  /'
        fi
        
        echo
        
        # Mount status
        log_info "📌 Mount Status:"
        if mountpoint -q "$DEVICE" 2>/dev/null; then
            log_success "  ✅ Device is mounted"
            mount | grep "$DEVICE" | sed 's/^/  /'
        else
            log_info "  ℹ️  Device is not mounted"
        fi
        
        echo
        
        # Filesystem type
        detected_fs=$(detect_filesystem "$DEVICE")
        if [[ -n "$detected_fs" ]]; then
            log_info "🔍 Detected Filesystem: $detected_fs"
            log_info "💡 Recommended mount options: $(get_mount_options "$detected_fs")"
        else
            log_warning "⚠️  Could not detect filesystem type"
        fi
        
        exit 0
        ;;
    list)
        log_info "📋 Currently Mounted Filesystems"
        echo "=================================================="
        
        if command -v lsblk >/dev/null 2>&1; then
            log_info "Device-based view:"
            lsblk -o NAME,SIZE,TYPE,MOUNTPOINT,FSTYPE | grep -E "(NAME|/dev/)" | sed 's/^/  /'
        fi
        
        echo
        
        log_info "Mount table view:"
        mount | grep -E "^/dev/" | sort | sed 's/^/  /'
        
        echo
        
        log_info "📊 Mount Statistics:"
        df -h | grep -E "^/dev/" | sed 's/^/  /'
        
        exit 0
        ;;
    mount)
        if [[ -z "$DEVICE" ]]; then
            error_exit "Device is required for mounting (--device /dev/sda1)"
        fi
        if [[ -z "$MOUNTPOINT" ]]; then
            error_exit "Mountpoint is required for mounting (--mountpoint /mnt)"
        fi
        
        log_info "🔗 Mounting $DEVICE to $MOUNTPOINT"
        echo "=================================================="
        
        # Check if device exists
        if [[ ! -b "$DEVICE" ]]; then
            error_exit "Device does not exist: $DEVICE"
        fi
        
        # Check if device is already mounted
        if mountpoint -q "$DEVICE" 2>/dev/null; then
            log_warning "⚠️  Device $DEVICE is already mounted"
            mount | grep "$DEVICE" | sed 's/^/  /'
            
            if [[ "$FORCE" == true ]]; then
                log_info "Force mode enabled - unmounting first..."
                umount "$DEVICE" || {
                    if [[ "$LAZY" == true ]]; then
                        umount -l "$DEVICE"
                        log_info "Lazy unmounted $DEVICE"
                    else
                        error_exit "Could not unmount $DEVICE"
                    fi
                }
            else
                log_info "Use --force to unmount and remount"
                exit 0
            fi
        fi
        
        # Check if mountpoint is already in use
        if mountpoint -q "$MOUNTPOINT" 2>/dev/null; then
            log_warning "⚠️  Mountpoint $MOUNTPOINT is already in use"
            mount | grep "$MOUNTPOINT" | sed 's/^/  /'
            
            if [[ "$FORCE" == true ]]; then
                log_info "Force mode enabled - unmounting mountpoint first..."
                umount "$MOUNTPOINT" || {
                    if [[ "$LAZY" == true ]]; then
                        umount -l "$MOUNTPOINT"
                        log_info "Lazy unmounted $MOUNTPOINT"
                    else
                        error_exit "Could not unmount $MOUNTPOINT"
                    fi
                }
            else
                log_info "Use --force to unmount and remount"
                exit 0
            fi
        fi
        
        # Create mountpoint if it doesn't exist
        if [[ ! -d "$MOUNTPOINT" ]]; then
            log_info "📁 Creating mountpoint: $MOUNTPOINT"
            mkdir -p "$MOUNTPOINT"
        fi
        
        # Detect filesystem if not specified
        if [[ -z "$FILESYSTEM" ]]; then
            FILESYSTEM=$(detect_filesystem "$DEVICE")
            if [[ -n "$FILESYSTEM" ]]; then
                log_info "🔍 Auto-detected filesystem: $FILESYSTEM"
            else
                log_warning "⚠️  Could not detect filesystem type, using auto-mount"
            fi
        fi
        
        # Build mount options
        mount_opts=""
        if [[ -n "$OPTIONS" ]]; then
            mount_opts="$OPTIONS"
        elif [[ -n "$FILESYSTEM" ]]; then
            mount_opts=$(get_mount_options "$FILESYSTEM")
        fi
        
        if [[ "$READONLY" == true ]]; then
            mount_opts="${mount_opts},ro"
            log_info "📖 Mounting as read-only"
        fi
        
        # Mount the device
        log_info "🚀 Mounting $DEVICE to $MOUNTPOINT..."
        if [[ -n "$FILESYSTEM" && -n "$mount_opts" ]]; then
            log_info "Command: mount -t $FILESYSTEM -o $mount_opts $DEVICE $MOUNTPOINT"
            mount -t "$FILESYSTEM" -o "$mount_opts" "$DEVICE" "$MOUNTPOINT"
        elif [[ -n "$FILESYSTEM" ]]; then
            log_info "Command: mount -t $FILESYSTEM $DEVICE $MOUNTPOINT"
            mount -t "$FILESYSTEM" "$DEVICE" "$MOUNTPOINT"
        elif [[ -n "$mount_opts" ]]; then
            log_info "Command: mount -o $mount_opts $DEVICE $MOUNTPOINT"
            mount -o "$mount_opts" "$DEVICE" "$MOUNTPOINT"
        else
            log_info "Command: mount $DEVICE $MOUNTPOINT"
            mount "$DEVICE" "$MOUNTPOINT"
        fi
        
        log_success "✅ Successfully mounted $DEVICE to $MOUNTPOINT"
        
        # Show mount information
        log_info "📊 Mount Information:"
        mount | grep "$DEVICE" | sed 's/^/  /'
        df -h "$MOUNTPOINT" | sed 's/^/  /'
        ;;
    umount)
        # Determine what to unmount - device or mountpoint
        TARGET=""
        TARGET_TYPE=""
        
        if [[ -n "$DEVICE" ]]; then
            TARGET="$DEVICE"
            TARGET_TYPE="device"
        elif [[ -n "$MOUNTPOINT" ]]; then
            TARGET="$MOUNTPOINT"
            TARGET_TYPE="mountpoint"
        else
            error_exit "Either device (--device /dev/sda1) or mountpoint (--mountpoint /path) is required for umounting"
        fi
        
        log_info "🔌 Unmounting $TARGET_TYPE: $TARGET"
        echo "=================================================="
        
        # Check if target is mounted
        if ! mountpoint -q "$TARGET" 2>/dev/null; then
            log_warning "⚠️  $TARGET_TYPE $TARGET is not mounted"
            exit 0
        fi
        
        # Show current mount information
        log_info "📌 Current mount information:"
        mount | grep "$TARGET" | sed 's/^/  /'
        
        # Check for busy filesystem
        if lsof "$TARGET" >/dev/null 2>&1; then
            log_warning "⚠️  $TARGET_TYPE $TARGET is in use (files open)"
            lsof "$TARGET" | head -5 | sed 's/^/  /'
            
            if [[ "$LAZY" == true ]]; then
                log_info "Lazy umount enabled - umounting anyway..."
                umount -l "$TARGET"
                log_success "✅ Lazy umounted $TARGET (will umount when not busy)"
            else
                log_info "Use --lazy to force umount (will umount when not busy)"
                exit 1
            fi
        else
            # Use umount -R (recursive) for safety, especially for system repair
            log_info "🚀 Umounting $TARGET (using umount -R for safety)..."
            umount -R "$TARGET"
            log_success "✅ Successfully umounted $TARGET"
        fi
        ;;
    *)
        error_exit "Invalid action: $ACTION. Use mount, umount, list, or info"
        ;;
esac
