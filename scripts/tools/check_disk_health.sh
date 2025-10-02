#!/bin/bash
# check_disk_health.sh - Comprehensive disk health diagnostics using SMART
# Usage: ./check_disk_health.sh --device /dev/sda [--detailed]

set -euo pipefail

# Source common utilities
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$SCRIPT_DIR/../utils.sh"

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
            echo "Check disk health using SMART diagnostics"
            echo "  --device    Device to check (e.g., /dev/sda)"
            echo "  --detailed  Show detailed SMART attributes"
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

# Check if device exists
if [[ ! -b "$DEVICE" ]]; then
    error_exit "Device does not exist: $DEVICE"
fi

# Check if smartctl is available
if ! command -v smartctl >/dev/null 2>&1; then
    log_info "Installing smartmontools for SMART diagnostics..."
    if ! pacman -Sy --noconfirm smartmontools >/dev/null 2>&1; then
        log_error "Could not install smartmontools via pacman"
        log_info "This may be because:"
        log_info "  • Not running as root"
        log_info "  • No internet connection"
        log_info "  • Package repository issues"
        log_info ""
        log_info "Please install smartmontools manually:"
        log_info "  pacman -S smartmontools"
        error_exit "smartctl not available and cannot be installed"
    fi
fi

log_info "🔍 Comprehensive Disk Health Diagnostics"
echo "=================================================="
log_info "Target Device: $DEVICE"

# Get basic disk information
log_info "📊 Disk Information:"
if command -v lsblk >/dev/null 2>&1; then
    lsblk -o NAME,SIZE,TYPE,MOUNTPOINT,MODEL "$DEVICE" 2>/dev/null || {
        log_warning "Could not get detailed disk information for $DEVICE"
        echo "  Device: $DEVICE"
    }
else
    log_warning "lsblk not available"
    echo "  Device: $DEVICE"
fi

# Show device size if available
if [[ -r "$DEVICE" ]]; then
    local size=$(blockdev --getsize64 "$DEVICE" 2>/dev/null | numfmt --to=iec 2>/dev/null || echo "unknown")
    echo "  Size: $size"
fi

echo

# Check if SMART is supported with proper error handling
log_info "🔧 SMART Support Check:"

# First, try basic device info
if ! smartctl -i "$DEVICE" >/dev/null 2>&1; then
    log_error "Cannot access device $DEVICE"
    log_info "This may be due to:"
    log_info "  • Device does not exist"
    log_info "  • Insufficient permissions (try running as root)"
    log_info "  • Device is busy or mounted"
    error_exit "Cannot access device for SMART diagnostics"
fi

# Get device info and check SMART support
DEVICE_INFO=$(smartctl -i "$DEVICE" 2>/dev/null)
if [[ -z "$DEVICE_INFO" ]]; then
    log_error "Could not retrieve device information"
    error_exit "Device information unavailable"
fi

# Check SMART support
SMART_SUPPORT=$(echo "$DEVICE_INFO" | grep -i "SMART support is" | head -1)
if [[ -n "$SMART_SUPPORT" ]]; then
    echo "  $SMART_SUPPORT"
    if [[ "$SMART_SUPPORT" == *"Unavailable"* ]]; then
        log_error "❌ SMART is not supported on this device"
        log_info "This device does not support SMART diagnostics"
        log_info "Common devices without SMART support:"
        log_info "  • Some USB drives"
        log_info "  • Network storage devices"
        log_info "  • Older or specialized storage devices"
        error_exit "SMART not supported on this device"
    fi
else
    log_warning "SMART support status unclear from device info"
fi

# Check if SMART is enabled
SMART_ENABLED=$(echo "$DEVICE_INFO" | grep -i "SMART.*Enabled" | head -1)
if [[ -n "$SMART_ENABLED" ]]; then
    echo "  $SMART_ENABLED"
    if [[ "$SMART_ENABLED" == *"Disabled"* ]]; then
        log_warning "⚠️  SMART is disabled on this device"
        log_info "Attempting to enable SMART..."
        if smartctl -s on "$DEVICE" >/dev/null 2>&1; then
            log_success "✅ SMART enabled successfully"
        else
            log_warning "⚠️  Could not enable SMART (may require permissive mode)"
            log_info "Trying with permissive mode..."
            if smartctl -d sat,12 -s on "$DEVICE" >/dev/null 2>&1; then
                log_success "✅ SMART enabled with permissive mode"
            else
                log_warning "⚠️  Could not enable SMART even with permissive mode"
                log_info "Continuing with limited diagnostics..."
            fi
        fi
    fi
else
    log_warning "SMART enablement status unclear"
fi

# Try different device types if basic SMART fails
if ! smartctl -H "$DEVICE" >/dev/null 2>&1; then
    log_warning "Basic SMART check failed, trying alternative device types..."
    
    # Try common device types that might need specific flags
    DEVICE_TYPES=("sat" "sat,12" "auto" "usb" "nvme")
    SMART_WORKING=false
    
    for dev_type in "${DEVICE_TYPES[@]}"; do
        if smartctl -d "$dev_type" -H "$DEVICE" >/dev/null 2>&1; then
            log_info "✅ SMART working with device type: $dev_type"
            SMART_WORKING=true
            # Set environment variable for subsequent smartctl calls
            export SMARTCTL_DEVICE_TYPE="$dev_type"
            break
        fi
    done
    
    if [[ "$SMART_WORKING" == false ]]; then
        log_error "❌ SMART diagnostics not available for this device"
        log_info "This device may not support SMART or requires special handling"
        log_info ""
        log_info "Common devices without SMART support:"
        log_info "  • Some USB drives"
        log_info "  • Network storage devices"
        log_info "  • Older or specialized storage devices"
        log_info ""
        log_info "Available alternatives:"
        log_info "  • Use 'badblocks' to check for bad sectors"
        log_info "  • Use 'fsck' to check filesystem integrity"
        log_info "  • Check device manufacturer's diagnostic tools"
        log_info ""
        log_info "Note: This is normal for certain device types and doesn't indicate a problem."
        exit 0  # Exit gracefully instead of error
    fi
fi

echo

# Helper function to run smartctl with appropriate device type
run_smartctl() {
    local args=("$@")
    if [[ -n "${SMARTCTL_DEVICE_TYPE:-}" ]]; then
        smartctl -d "$SMARTCTL_DEVICE_TYPE" "${args[@]}"
    else
        smartctl "${args[@]}"
    fi
}

# Check SMART overall health
log_info "🏥 SMART Health Status:"
if run_smartctl -H "$DEVICE" >/dev/null 2>&1; then
    HEALTH_STATUS=$(run_smartctl -H "$DEVICE" | grep "SMART overall-health self-assessment test result")
    if [[ "$HEALTH_STATUS" == *"PASSED"* ]]; then
        log_success "✅ SMART Health: PASSED"
    elif [[ "$HEALTH_STATUS" == *"FAILED"* ]]; then
        log_error "❌ SMART Health: FAILED"
        log_error "⚠️  This disk may have hardware issues!"
    else
        log_warning "⚠️  SMART Health: Status unclear"
        echo "    $HEALTH_STATUS"
    fi
else
    log_warning "⚠️  Could not retrieve SMART health status"
fi

echo

# Display critical SMART attributes
log_info "📈 Critical SMART Attributes:"
if run_smartctl -A "$DEVICE" >/dev/null 2>&1; then
    # Show critical attributes in a readable format
    echo "  Reallocated Sectors:"
    run_smartctl -A "$DEVICE" | grep -E "(Reallocated_Sector|Reallocated_Sectors)" | head -1 | sed 's/^/    /'
    
    echo "  Current Pending Sectors:"
    run_smartctl -A "$DEVICE" | grep -E "(Current_Pending_Sector|Pending_Sector)" | head -1 | sed 's/^/    /'
    
    echo "  Uncorrectable Errors:"
    run_smartctl -A "$DEVICE" | grep -E "(Offline_Uncorrectable|Uncorrectable_Error)" | head -1 | sed 's/^/    /'
    
    echo "  Power-On Hours:"
    run_smartctl -A "$DEVICE" | grep -E "(Power_On_Hours|Power_On_Time)" | head -1 | sed 's/^/    /'
    
    echo "  Power Cycle Count:"
    run_smartctl -A "$DEVICE" | grep -E "(Power_Cycle_Count|Start_Stop_Count)" | head -1 | sed 's/^/    /'
else
    log_warning "⚠️  Could not retrieve SMART attributes"
fi

echo

# Display disk temperature
log_info "🌡️  Disk Temperature:"
if run_smartctl -A "$DEVICE" >/dev/null 2>&1; then
    TEMP=$(run_smartctl -A "$DEVICE" | grep -i temperature | head -1)
    if [[ -n "$TEMP" ]]; then
        echo "  $TEMP"
        # Extract temperature value and provide guidance
        TEMP_VALUE=$(echo "$TEMP" | grep -o '[0-9]\+' | head -1)
        if [[ -n "$TEMP_VALUE" ]]; then
            if [[ "$TEMP_VALUE" -lt 40 ]]; then
                log_success "  ✅ Temperature is good (< 40°C)"
            elif [[ "$TEMP_VALUE" -lt 50 ]]; then
                log_info "  ℹ️  Temperature is acceptable (40-50°C)"
            elif [[ "$TEMP_VALUE" -lt 60 ]]; then
                log_warning "  ⚠️  Temperature is warm (50-60°C)"
            else
                log_error "  🔥 Temperature is hot (> 60°C) - consider cooling!"
            fi
        fi
    else
        log_info "  Temperature monitoring not available"
    fi
else
    log_warning "  Could not retrieve temperature information"
fi

echo

# Display error logs
log_info "📋 Error Log Summary:"
if run_smartctl -l error "$DEVICE" >/dev/null 2>&1; then
    ERROR_COUNT=$(run_smartctl -l error "$DEVICE" | grep -c "Error" || true)
    if [[ "$ERROR_COUNT" -gt 0 ]]; then
        log_warning "  ⚠️  Found $ERROR_COUNT error log entries"
        if [[ "$DETAILED" == true ]]; then
            echo
            run_smartctl -l error "$DEVICE" | head -20 | sed 's/^/    /'
        else
            log_info "  Use --detailed flag to see full error log"
        fi
    else
        log_success "  ✅ No error log entries found"
    fi
else
    log_warning "  Could not retrieve error log"
fi

echo

# Display self-test results
log_info "🧪 Self-Test Results:"
if run_smartctl -l selftest "$DEVICE" >/dev/null 2>&1; then
    SELFTEST_RESULTS=$(run_smartctl -l selftest "$DEVICE" | grep -E "(Completed|Failed|Interrupted)" | head -3)
    if [[ -n "$SELFTEST_RESULTS" ]]; then
        echo "$SELFTEST_RESULTS" | sed 's/^/  /'
    else
        log_info "  No recent self-test results found"
        log_info "  Consider running a self-test with: smartctl -t short $DEVICE"
    fi
else
    log_warning "  Could not retrieve self-test results"
fi

echo

# Detailed SMART attributes if requested
if [[ "$DETAILED" == true ]]; then
    log_info "📊 Detailed SMART Attributes:"
    echo "=================================================="
    run_smartctl -A "$DEVICE" | head -30 | sed 's/^/  /'
    echo "=================================================="
    if [[ -n "${SMARTCTL_DEVICE_TYPE:-}" ]]; then
        log_info "Use 'smartctl -d $SMARTCTL_DEVICE_TYPE -A $DEVICE' for complete attribute list"
    else
        log_info "Use 'smartctl -A $DEVICE' for complete attribute list"
    fi
fi

echo
log_info "💡 Recommendations:"
if [[ -n "${SMARTCTL_DEVICE_TYPE:-}" ]]; then
    log_info "  • Run regular self-tests: smartctl -d $SMARTCTL_DEVICE_TYPE -t short $DEVICE"
else
    log_info "  • Run regular self-tests: smartctl -t short $DEVICE"
fi
log_info "  • Monitor temperature and error counts regularly"
log_info "  • Consider replacing disk if health status is FAILED"
log_info "  • Keep backups of important data"
log_info "  • For devices without SMART support, use 'badblocks' and 'fsck'"

echo
log_success "✅ Disk health diagnostics completed for $DEVICE"
