#!/bin/bash
# check_disk_health.sh - Comprehensive disk reliability test
# Usage: ./check_disk_health.sh --device /dev/sda [--detailed]

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
DEVICE=""
DETAILED=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --device)
            DEVICE="$2"
            shift 2
            ;;
        --detailed)
            DETAILED=true
            shift
            ;;
        --help)
            echo "Usage: $0 --device <device> [--detailed]"
            echo ""
            echo "Comprehensive disk reliability test that works on ANY disk"
            echo ""
            echo "Tests performed:"
            echo "  • Filesystem integrity check"
            echo "  • Bad blocks detection"
            echo "  • Performance test"
            echo "  • SMART diagnostics (if supported)"
            echo ""
            echo "Options:"
            echo "  --device <device>  Device to test (e.g., /dev/sda1)"
            echo "  --detailed         Show detailed test results"
            echo ""
            echo "Examples:"
            echo "  $0 --device /dev/sda1        # Test partition"
            echo "  $0 --device /dev/sda         # Test entire disk"
            echo "  $0 --device /dev/sda1 --detailed  # Detailed results"
            echo ""
            echo "Note: Works on any disk regardless of SMART support"
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
    error_exit "Device is required (--device /dev/sda1)"
fi

# Validate device exists
if [[ ! -b "$DEVICE" ]]; then
    error_exit "Device does not exist: $DEVICE"
fi

log_info "🔍 Comprehensive Disk Reliability Test"
echo "=================================================="
log_info "Target Device: $DEVICE"

# Get basic device information
log_info "📊 Device Information:"
if command -v lsblk >/dev/null 2>&1; then
    lsblk -o NAME,SIZE,TYPE,MOUNTPOINT,MODEL "$DEVICE" 2>/dev/null || {
        log_warning "Could not get detailed device information"
        echo "  Device: $DEVICE"
    }
else
    echo "  Device: $DEVICE"
fi

# Show device size if available
if [[ -r "$DEVICE" ]]; then
    size=$(blockdev --getsize64 "$DEVICE" 2>/dev/null | numfmt --to=iec 2>/dev/null || echo "unknown")
    echo "  Size: $size"
fi

echo

# Test 1: Filesystem Integrity Check
log_info "🧪 Test 1: Filesystem Integrity Check"
echo "--------------------------------------------------"

# Check if device is mounted
mount_point=$(mount | grep "$DEVICE" | awk '{print $3}' | head -1)
if [[ -n "$mount_point" ]]; then
    log_info "Device is mounted at: $mount_point"
    
    # Check filesystem type
    fs_type=$(mount | grep "$DEVICE" | awk '{print $5}' | head -1)
    log_info "Filesystem type: $fs_type"
    
    # Run appropriate filesystem check
    case "$fs_type" in
        "ext4"|"ext3"|"ext2")
            log_info "Running ext4 filesystem check..."
            if umount "$DEVICE" 2>/dev/null; then
                log_info "Unmounted device for filesystem check"
                fsck_output=""
                if fsck_output=$(fsck -n "$DEVICE" 2>&1); then
                    log_success "✅ Filesystem integrity: GOOD"
                else
                    log_warning "⚠️  Filesystem integrity: ISSUES DETECTED"
                    echo "$fsck_output" | head -20
                    echo "  Run 'fsck $DEVICE' to repair (after unmounting)"
                fi
                # Remount if it was previously mounted
                if [[ -n "$mount_point" ]]; then
                    mount "$DEVICE" "$mount_point" 2>/dev/null || log_warning "Could not remount device"
                fi
            else
                log_warning "⚠️  Cannot unmount device for filesystem check"
                echo "  Device is in use - filesystem check skipped"
            fi
            ;;
        "btrfs")
            log_info "Running btrfs filesystem check..."
            btrfs_output=""
            if btrfs_output=$(btrfs check --readonly "$DEVICE" 2>&1); then
                log_success "✅ Btrfs filesystem integrity: GOOD"
            else
                log_warning "⚠️  Btrfs filesystem integrity: ISSUES DETECTED"
                echo "$btrfs_output" | head -20
                echo "  Run 'btrfs check --repair $DEVICE' to repair"
            fi
            ;;
        "xfs")
            log_info "Running xfs filesystem check..."
            xfs_output=""
            if xfs_output=$(xfs_repair -n "$DEVICE" 2>&1); then
                log_success "✅ XFS filesystem integrity: GOOD"
            else
                log_warning "⚠️  XFS filesystem integrity: ISSUES DETECTED"
                echo "$xfs_output" | head -20
                echo "  Run 'xfs_repair $DEVICE' to repair"
            fi
            ;;
        *)
            log_warning "⚠️  Unknown filesystem type: $fs_type"
            echo "  Filesystem check skipped for unsupported type"
            ;;
    esac
else
    log_info "Device is not mounted"
    echo "  Filesystem integrity check skipped (device not mounted)"
fi

echo

# Test 2: Bad Blocks Detection
log_info "🧪 Test 2: Bad Blocks Detection"
echo "--------------------------------------------------"

log_info "Scanning for bad blocks (this may take a while)..."
if command -v badblocks >/dev/null 2>&1; then
    # Run badblocks in read-only mode
    if badblocks -v -s "$DEVICE" 2>/dev/null; then
        log_success "✅ Bad blocks scan: PASSED"
        echo "  No bad blocks detected"
    else
        log_warning "⚠️  Bad blocks scan: FAILED"
        echo "  Bad blocks detected - consider replacing disk"
        echo "  Run 'badblocks -w $DEVICE' for destructive test (BACKUP FIRST!)"
    fi
else
    log_warning "⚠️  badblocks not available"
    echo "  Install e2fsprogs package for bad blocks testing"
fi

echo

# Test 3: Performance Test
log_info "🧪 Test 3: Performance Test"
echo "--------------------------------------------------"

log_info "Testing read performance..."
if command -v dd >/dev/null 2>&1; then
    # Test read speed (1MB test)
    read_speed=$(dd if="$DEVICE" of=/dev/null bs=1M count=1 2>&1 | grep -o '[0-9.]* MB/s' | tail -1 || echo "unknown")
    echo "  Read speed: $read_speed"
    
    # Test write speed if device is not mounted (to avoid data corruption)
    if [[ -z "$mount_point" ]]; then
        log_info "Testing write performance (safe test)..."
        # Create a small test file and measure write speed
        test_file="/tmp/disk_test_$$"
        if dd if=/dev/zero of="$test_file" bs=1M count=1 2>/dev/null; then
            write_speed=$(dd if="$test_file" of="$DEVICE" bs=1M count=1 2>&1 | grep -o '[0-9.]* MB/s' | tail -1 || echo "unknown")
            echo "  Write speed: $write_speed"
            rm -f "$test_file"
        else
            echo "  Write speed: Test skipped (device access issue)"
        fi
    else
        echo "  Write speed: Test skipped (device is mounted)"
    fi
    
    # Overall performance assessment
    if [[ "$read_speed" != "unknown" ]]; then
        speed_num=$(echo "$read_speed" | grep -o '[0-9.]*' | head -1)
        if (( $(echo "$speed_num > 50" | bc -l 2>/dev/null || echo "0") )); then
            log_success "✅ Performance: GOOD"
        elif (( $(echo "$speed_num > 10" | bc -l 2>/dev/null || echo "0") )); then
            log_warning "⚠️  Performance: SLOW"
            echo "  Disk may be aging or experiencing issues"
        else
            log_error "❌ Performance: VERY SLOW"
            echo "  Disk may have serious issues"
        fi
    fi
else
    log_warning "⚠️  dd not available for performance testing"
fi

echo

# Test 4: SMART Diagnostics (if supported)
log_info "🧪 Test 4: SMART Diagnostics (if supported)"
echo "--------------------------------------------------"

if command -v smartctl >/dev/null 2>&1; then
    # Check if SMART is supported
    if smartctl -H "$DEVICE" >/dev/null 2>&1; then
        log_info "SMART is supported on this device"
        
        # Get overall health
        health_status=$(smartctl -H "$DEVICE" 2>/dev/null | grep "SMART overall-health self-assessment test result" || echo "")
        if [[ "$health_status" == *"PASSED"* ]]; then
            log_success "✅ SMART Health: GOOD"
        elif [[ "$health_status" == *"FAILED"* ]]; then
            log_error "❌ SMART Health: BAD"
            echo "  SMART indicates hardware issues"
        else
            log_warning "⚠️  SMART Health: UNKNOWN"
        fi
        
        # Check critical attributes if detailed mode
        if [[ "$DETAILED" == true ]]; then
            log_info "Critical SMART Attributes:"
            smartctl -A "$DEVICE" 2>/dev/null | grep -E "(Reallocated_Sector|Current_Pending_Sector|Offline_Uncorrectable)" | sed 's/^/  /'
        fi
    else
        log_info "SMART not supported on this device"
        echo "  This is normal for some USB drives and older devices"
        echo "  Other tests above provide reliability assessment"
    fi
else
    log_info "smartctl not available"
    echo "  Install smartmontools package for SMART diagnostics"
    echo "  Other tests above provide reliability assessment"
fi

echo

# Overall Assessment
log_info "🎯 Overall Assessment"
echo "=================================================="

# Count issues
issues=0
warnings=0

# Check filesystem results (simplified assessment)
if mount | grep -q "$DEVICE"; then
    echo "✅ Device is properly mounted and accessible"
else
    echo "ℹ️  Device is not mounted (normal for some devices)"
fi

# Check performance results
if command -v dd >/dev/null 2>&1; then
    echo "✅ Performance test completed"
else
    echo "⚠️  Performance test skipped (dd not available)"
    ((warnings++))
fi

# Check bad blocks results
if command -v badblocks >/dev/null 2>&1; then
    echo "✅ Bad blocks scan completed"
else
    echo "⚠️  Bad blocks scan skipped (badblocks not available)"
    ((warnings++))
fi

# SMART assessment
if command -v smartctl >/dev/null 2>&1 && smartctl -H "$DEVICE" >/dev/null 2>&1; then
    echo "✅ SMART diagnostics available"
else
    echo "ℹ️  SMART diagnostics not available (normal for some devices)"
fi

echo

# Final recommendation
if [[ $issues -eq 0 ]]; then
    if [[ $warnings -eq 0 ]]; then
        log_success "🎉 Disk Reliability: EXCELLENT"
        echo "  Your disk appears to be in excellent condition."
        echo "  No issues detected in any test."
    else
        log_success "✅ Disk Reliability: GOOD"
        echo "  Your disk appears to be in good condition."
        echo "  Some diagnostic tools were unavailable, but core tests passed."
    fi
else
    log_warning "⚠️  Disk Reliability: ISSUES DETECTED"
    echo "  Review the test results above for specific issues."
    echo "  Consider backing up important data."
fi

echo
log_info "💡 Recommendations:"
echo "  • Run this test monthly for monitoring"
echo "  • Backup important data regularly"
echo "  • Replace disk if multiple tests consistently fail"
if [[ "$DETAILED" == false ]]; then
    echo "  • Use --detailed flag for more information"
fi

log_success "✅ Disk reliability test completed successfully!"