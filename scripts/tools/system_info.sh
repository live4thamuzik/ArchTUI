#!/bin/bash
# system_info.sh - Display system information
# Usage: ./system_info.sh [--detailed]

set -euo pipefail

# Source common utilities
SCRIPT_DIR="$(dirname "${BASH_SOURCE[0]}")"
source "$SCRIPT_DIR/../utils.sh"

# Default values
DETAILED=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --detailed)
            DETAILED=true
            shift
            ;;
        --help)
            echo "Usage: $0 [--detailed]"
            echo "Display system information"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

log_info "System Information"
echo "===================="

# Basic system info
echo
echo "Operating System:"
if [[ -f /etc/os-release ]]; then
    grep PRETTY_NAME /etc/os-release | cut -d'"' -f2
else
    echo "Unknown"
fi

echo
echo "Kernel:"
uname -r

echo
echo "Architecture:"
uname -m

# CPU information
echo
echo "CPU Information:"
if [[ -f /proc/cpuinfo ]]; then
    echo "Model: $(grep 'model name' /proc/cpuinfo | head -1 | cut -d: -f2 | xargs)"
    echo "Cores: $(nproc)"
    echo "Threads: $(grep -c processor /proc/cpuinfo)"
else
    echo "CPU info not available"
fi

# Memory information
echo
echo "Memory Information:"
if command -v free >/dev/null 2>&1; then
    free -h
else
    echo "Memory info not available"
fi

# Disk information
echo
echo "Disk Information:"
if command -v df >/dev/null 2>&1; then
    df -h | grep -E '^/dev/'
else
    echo "Disk info not available"
fi

# Network information
echo
echo "Network Interfaces:"
if command -v ip >/dev/null 2>&1; then
    ip addr show | grep -E '^[0-9]+:' | cut -d: -f2 | xargs
else
    echo "Network info not available"
fi

# Boot information
echo
echo "Boot Information:"
if [[ -d /sys/firmware/efi ]]; then
    echo "Boot Mode: UEFI"
else
    echo "Boot Mode: BIOS"
fi

# Bootloader
echo
echo "Bootloader:"
if [[ -f /sys/firmware/efi/efivars ]]; then
    echo "EFI Bootloader detected"
elif [[ -f /boot/grub/grub.cfg ]]; then
    echo "GRUB detected"
elif [[ -f /boot/loader/loader.conf ]]; then
    echo "systemd-boot detected"
else
    echo "Bootloader: Unknown"
fi

if [[ "$DETAILED" == true ]]; then
    echo
    echo "Detailed Information"
    echo "==================="
    
    # Detailed CPU info
    echo
    echo "Detailed CPU Information:"
    if [[ -f /proc/cpuinfo ]]; then
        grep -E 'processor|model name|cpu MHz|cache size' /proc/cpuinfo | head -20
    fi
    
    # Detailed memory info
    echo
    echo "Detailed Memory Information:"
    if [[ -f /proc/meminfo ]]; then
        head -10 /proc/meminfo
    fi
    
    # Mounted filesystems
    echo
    echo "Mounted Filesystems:"
    mount | grep -E '^/dev/' | sort
    
    # Running services
    echo
    echo "Running Services:"
    if command -v systemctl >/dev/null 2>&1; then
        systemctl list-units --state=running --type=service | head -10
    else
        echo "systemctl not available"
    fi
    
    # Loaded modules
    echo
    echo "Loaded Kernel Modules:"
    if [[ -f /proc/modules ]]; then
        head -10 /proc/modules
    fi
fi

log_success "System information displayed"
