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
        log_warning "Could not install smartmontools via pacman"
        log_info "Trying to use existing system tools..."
    fi
fi

log_info "🔍 Comprehensive Disk Health Diagnostics for $DEVICE"
echo "=================================================="

# Get basic disk information
log_info "📊 Disk Information:"
if command -v lsblk >/dev/null 2>&1; then
    lsblk -o NAME,SIZE,TYPE,MOUNTPOINT,MODEL "$DEVICE" 2>/dev/null || log_info "Basic disk info not available"
fi

echo

# Check if SMART is supported
log_info "🔧 SMART Support Check:"
if smartctl -i "$DEVICE" >/dev/null 2>&1; then
    SMART_SUPPORT=$(smartctl -i "$DEVICE" | grep -i "SMART support is" | head -1)
    if [[ -n "$SMART_SUPPORT" ]]; then
        echo "  $SMART_SUPPORT"
    else
        log_warning "SMART support status unclear"
    fi
    
    # Check if SMART is enabled
    SMART_ENABLED=$(smartctl -i "$DEVICE" | grep -i "SMART.*Enabled" | head -1)
    if [[ -n "$SMART_ENABLED" ]]; then
        echo "  $SMART_ENABLED"
    fi
else
    log_error "Cannot access SMART information for $DEVICE"
    log_info "This may be due to:"
    log_info "  • Device does not support SMART"
    log_info "  • Insufficient permissions (try running as root)"
    log_info "  • Device is busy or mounted"
    exit 1
fi

echo

# Check SMART overall health
log_info "🏥 SMART Health Status:"
if smartctl -H "$DEVICE" >/dev/null 2>&1; then
    HEALTH_STATUS=$(smartctl -H "$DEVICE" | grep "SMART overall-health self-assessment test result")
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
if smartctl -A "$DEVICE" >/dev/null 2>&1; then
    # Show critical attributes in a readable format
    echo "  Reallocated Sectors:"
    smartctl -A "$DEVICE" | grep -E "(Reallocated_Sector|Reallocated_Sectors)" | head -1 | sed 's/^/    /'
    
    echo "  Current Pending Sectors:"
    smartctl -A "$DEVICE" | grep -E "(Current_Pending_Sector|Pending_Sector)" | head -1 | sed 's/^/    /'
    
    echo "  Uncorrectable Errors:"
    smartctl -A "$DEVICE" | grep -E "(Offline_Uncorrectable|Uncorrectable_Error)" | head -1 | sed 's/^/    /'
    
    echo "  Power-On Hours:"
    smartctl -A "$DEVICE" | grep -E "(Power_On_Hours|Power_On_Time)" | head -1 | sed 's/^/    /'
    
    echo "  Power Cycle Count:"
    smartctl -A "$DEVICE" | grep -E "(Power_Cycle_Count|Start_Stop_Count)" | head -1 | sed 's/^/    /'
else
    log_warning "⚠️  Could not retrieve SMART attributes"
fi

echo

# Display disk temperature
log_info "🌡️  Disk Temperature:"
if smartctl -A "$DEVICE" >/dev/null 2>&1; then
    TEMP=$(smartctl -A "$DEVICE" | grep -i temperature | head -1)
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
if smartctl -l error "$DEVICE" >/dev/null 2>&1; then
    ERROR_COUNT=$(smartctl -l error "$DEVICE" | grep -c "Error" || true)
    if [[ "$ERROR_COUNT" -gt 0 ]]; then
        log_warning "  ⚠️  Found $ERROR_COUNT error log entries"
        if [[ "$DETAILED" == true ]]; then
            echo
            smartctl -l error "$DEVICE" | head -20 | sed 's/^/    /'
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
if smartctl -l selftest "$DEVICE" >/dev/null 2>&1; then
    SELFTEST_RESULTS=$(smartctl -l selftest "$DEVICE" | grep -E "(Completed|Failed|Interrupted)" | head -3)
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
    smartctl -A "$DEVICE" | head -30 | sed 's/^/  /'
    echo "=================================================="
    log_info "Use 'smartctl -A $DEVICE' for complete attribute list"
fi

echo
log_info "💡 Recommendations:"
log_info "  • Run regular self-tests: smartctl -t short $DEVICE"
log_info "  • Monitor temperature and error counts regularly"
log_info "  • Consider replacing disk if health status is FAILED"
log_info "  • Keep backups of important data"

echo
log_success "✅ Disk health diagnostics completed for $DEVICE"
