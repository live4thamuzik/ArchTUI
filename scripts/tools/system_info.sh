#!/bin/bash
# system_info.sh - Display comprehensive system information using ISO tools
# Usage: ./system_info.sh [--detailed] [--json]

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
DETAILED=false
JSON_OUTPUT=false

# Parse arguments
while [[ $# -gt 0 ]]; do
    case "$1" in
        --detailed)
            DETAILED=true
            shift
            ;;
        --json)
            JSON_OUTPUT=true
            shift
            ;;
        --help)
            echo "Usage: $0 [--detailed] [--json]"
            echo ""
            echo "Options:"
            echo "  --detailed    Show detailed system information"
            echo "  --json        Output in JSON format"
            echo "  --help        Show this help message"
            echo ""
            echo "Examples:"
            echo "  $0                    # Basic system info"
            echo "  $0 --detailed         # Detailed system info"
            echo "  $0 --json             # JSON output"
            echo "  $0 --detailed --json  # Detailed JSON output"
            echo ""
            echo "Note: Uses tools available on Arch ISO (no additional packages required)"
            exit 0
            ;;
        *)
            log_error "Unknown option: $1"
            exit 1
            ;;
    esac
done

# Function to get system information
get_system_info() {
    local info_type="$1"
    
    case "$info_type" in
        "hostname")
            hostname 2>/dev/null || echo "unknown"
            ;;
        "kernel")
            uname -r 2>/dev/null || echo "unknown"
            ;;
        "architecture")
            uname -m 2>/dev/null || echo "unknown"
            ;;
        "uptime")
            uptime 2>/dev/null | awk '{print $3,$4}' | sed 's/,//' || echo "unknown"
            ;;
        "load_average")
            uptime 2>/dev/null | awk -F'load average:' '{print $2}' | sed 's/^ *//' || echo "unknown"
            ;;
        "memory_total")
            if command -v free >/dev/null 2>&1; then
                free -h 2>/dev/null | awk 'NR==2{print $2}' || echo "unknown"
            else
                echo "unknown"
            fi
            ;;
        "memory_used")
            if command -v free >/dev/null 2>&1; then
                free -h 2>/dev/null | awk 'NR==2{print $3}' || echo "unknown"
            else
                echo "unknown"
            fi
            ;;
        "memory_available")
            if command -v free >/dev/null 2>&1; then
                free -h 2>/dev/null | awk 'NR==2{print $7}' || echo "unknown"
            else
                echo "unknown"
            fi
            ;;
        "cpu_model")
            if [[ -r /proc/cpuinfo ]]; then
                grep "model name" /proc/cpuinfo | head -1 | cut -d: -f2 | sed 's/^ *//' || echo "unknown"
            else
                echo "unknown"
            fi
            ;;
        "cpu_cores")
            if [[ -r /proc/cpuinfo ]]; then
                grep "processor" /proc/cpuinfo | wc -l || echo "unknown"
            else
                echo "unknown"
            fi
            ;;
        "cpu_frequency")
            if command -v lscpu >/dev/null 2>&1; then
                lscpu | grep "CPU MHz" | awk '{print $3 " MHz"}' || echo "unknown"
            elif [[ -r /proc/cpuinfo ]]; then
                grep "cpu MHz" /proc/cpuinfo | head -1 | awk '{print $4 " MHz"}' || echo "unknown"
            else
                echo "unknown"
            fi
            ;;
        "boot_mode")
            if [[ -d /sys/firmware/efi ]]; then
                echo "UEFI"
            else
                echo "BIOS"
            fi
            ;;
        "boot_disk")
            if command -v lsblk >/dev/null 2>&1; then
                lsblk -d -o NAME,TYPE | grep disk | head -1 | awk '{print "/dev/" $1}' || echo "unknown"
            else
                echo "unknown"
            fi
            ;;
        "network_interfaces")
            if command -v ip >/dev/null 2>&1; then
                ip link show | grep -E "^[0-9]+:" | awk -F: '{print $2}' | sed 's/^ *//' | tr '\n' ' ' || echo "none"
            else
                echo "unknown"
            fi
            ;;
        "mounted_filesystems")
            if command -v mount >/dev/null 2>&1; then
                mount | grep -E "(ext4|xfs|btrfs|vfat|ntfs)" | wc -l || echo "0"
            else
                echo "unknown"
            fi
            ;;
        "swap_usage")
            if command -v free >/dev/null 2>&1; then
                free -h 2>/dev/null | awk 'NR==3{print $3 "/" $2}' || echo "unknown"
            else
                echo "unknown"
            fi
            ;;
        "temperature")
            if [[ -r /sys/class/thermal/thermal_zone0/temp ]]; then
                local temp=$(cat /sys/class/thermal/thermal_zone0/temp 2>/dev/null)
                if [[ -n "$temp" ]]; then
                    echo "$((temp / 1000))°C"
                else
                    echo "unknown"
                fi
            else
                echo "unknown"
            fi
            ;;
        "distribution")
            if [[ -r /etc/os-release ]]; then
                grep "PRETTY_NAME" /etc/os-release | cut -d'"' -f2 || echo "unknown"
            else
                echo "unknown"
            fi
            ;;
        "shell")
            echo "${SHELL:-unknown}"
            ;;
        "date")
            date 2>/dev/null || echo "unknown"
            ;;
        "timezone")
            if command -v timedatectl >/dev/null 2>&1; then
                timedatectl | grep "Time zone" | awk '{print $3}' || echo "unknown"
            elif [[ -r /etc/timezone ]]; then
                cat /etc/timezone 2>/dev/null || echo "unknown"
            else
                echo "unknown"
            fi
            ;;
        *)
            echo "unknown"
            ;;
    esac
}

# Function to get detailed information
get_detailed_info() {
    echo "=================================================="
    echo "🔍 DETAILED SYSTEM INFORMATION"
    echo "=================================================="
    
    echo ""
    echo "💻 HARDWARE INFORMATION"
    echo "--------------------------------------------------"
    
    if [[ -r /proc/cpuinfo ]]; then
        echo "CPU Information:"
        grep "model name" /proc/cpuinfo | head -1 | cut -d: -f2 | sed 's/^ *//' | sed 's/^/  /'
        echo "  Cores: $(grep "processor" /proc/cpuinfo | wc -l)"
        if command -v lscpu >/dev/null 2>&1; then
            echo "  Architecture: $(lscpu | grep "Architecture" | awk '{print $2}')"
            echo "  Threads per core: $(lscpu | grep "Thread(s) per core" | awk '{print $4}')"
        fi
    fi
    
    echo ""
    if command -v free >/dev/null 2>&1; then
        echo "Memory Information:"
        free -h | sed 's/^/  /'
    fi
    
    echo ""
    echo "Storage Information:"
    if command -v lsblk >/dev/null 2>&1; then
        lsblk -f | sed 's/^/  /'
    else
        echo "  lsblk not available"
    fi
    
    echo ""
    echo "🌐 NETWORK INFORMATION"
    echo "--------------------------------------------------"
    
    if command -v ip >/dev/null 2>&1; then
        echo "Network Interfaces:"
        ip addr show | grep -E "(^[0-9]+:|inet )" | sed 's/^/  /'
    else
        echo "  ip command not available"
    fi
    
    echo ""
    echo "📁 MOUNTED FILESYSTEMS"
    echo "--------------------------------------------------"
    
    if command -v mount >/dev/null 2>&1; then
        mount | grep -E "(ext4|xfs|btrfs|vfat|ntfs)" | sed 's/^/  /'
    else
        echo "  mount command not available"
    fi
    
    echo ""
    echo "⚙️  SYSTEM CONFIGURATION"
    echo "--------------------------------------------------"
    
    echo "Boot Mode: $(get_system_info boot_mode)"
    echo "Kernel: $(get_system_info kernel)"
    echo "Architecture: $(get_system_info architecture)"
    
    if [[ -r /etc/os-release ]]; then
        echo "Distribution: $(get_system_info distribution)"
    fi
    
    echo ""
    echo "🔧 SYSTEM STATUS"
    echo "--------------------------------------------------"
    
    echo "Uptime: $(get_system_info uptime)"
    echo "Load Average: $(get_system_info load_average)"
    
    if [[ -r /sys/class/thermal/thermal_zone0/temp ]]; then
        echo "Temperature: $(get_system_info temperature)"
    fi
    
    echo ""
    echo "🌍 ENVIRONMENT"
    echo "--------------------------------------------------"
    
    echo "Date: $(get_system_info date)"
    echo "Timezone: $(get_system_info timezone)"
    echo "Shell: $(get_system_info shell)"
    
    if command -v env >/dev/null 2>&1; then
        echo ""
        echo "Environment Variables:"
        env | grep -E "(PATH|HOME|USER|SHELL)" | head -10 | sed 's/^/  /'
    fi
}

# Function to output JSON
output_json() {
    echo "{"
    echo "  \"system\": {"
    echo "    \"hostname\": \"$(get_system_info hostname)\","
    echo "    \"kernel\": \"$(get_system_info kernel)\","
    echo "    \"architecture\": \"$(get_system_info architecture)\","
    echo "    \"distribution\": \"$(get_system_info distribution)\","
    echo "    \"boot_mode\": \"$(get_system_info boot_mode)\","
    echo "    \"uptime\": \"$(get_system_info uptime)\","
    echo "    \"load_average\": \"$(get_system_info load_average)\","
    echo "    \"date\": \"$(get_system_info date)\","
    echo "    \"timezone\": \"$(get_system_info timezone)\","
    echo "    \"shell\": \"$(get_system_info shell)\""
    echo "  },"
    echo "  \"hardware\": {"
    echo "    \"cpu_model\": \"$(get_system_info cpu_model)\","
    echo "    \"cpu_cores\": \"$(get_system_info cpu_cores)\","
    echo "    \"cpu_frequency\": \"$(get_system_info cpu_frequency)\","
    echo "    \"memory_total\": \"$(get_system_info memory_total)\","
    echo "    \"memory_used\": \"$(get_system_info memory_used)\","
    echo "    \"memory_available\": \"$(get_system_info memory_available)\","
    echo "    \"swap_usage\": \"$(get_system_info swap_usage)\","
    echo "    \"temperature\": \"$(get_system_info temperature)\","
    echo "    \"boot_disk\": \"$(get_system_info boot_disk)\""
    echo "  },"
    echo "  \"network\": {"
    echo "    \"interfaces\": \"$(get_system_info network_interfaces)\""
    echo "  },"
    echo "  \"storage\": {"
    echo "    \"mounted_filesystems\": \"$(get_system_info mounted_filesystems)\""
    echo "  }"
    echo "}"
}

# Main execution
log_info "🔍 System Information Tool (ISO Compatible)"
echo "=================================================="

if [[ "$JSON_OUTPUT" == true ]]; then
    output_json
elif [[ "$DETAILED" == true ]]; then
    get_detailed_info
else
    # Basic system information
    echo "💻 BASIC SYSTEM INFORMATION"
    echo "--------------------------------------------------"
    echo "Hostname: $(get_system_info hostname)"
    echo "Distribution: $(get_system_info distribution)"
    echo "Kernel: $(get_system_info kernel)"
    echo "Architecture: $(get_system_info architecture)"
    echo "Boot Mode: $(get_system_info boot_mode)"
    echo "Uptime: $(get_system_info uptime)"
    echo "Load Average: $(get_system_info load_average)"
    echo ""
    echo "🖥️  HARDWARE SUMMARY"
    echo "--------------------------------------------------"
    echo "CPU: $(get_system_info cpu_model)"
    echo "Cores: $(get_system_info cpu_cores)"
    echo "Memory: $(get_system_info memory_used)/$(get_system_info memory_total) (Available: $(get_system_info memory_available))"
    echo "Swap: $(get_system_info swap_usage)"
    if [[ "$(get_system_info temperature)" != "unknown" ]]; then
        echo "Temperature: $(get_system_info temperature)"
    fi
    echo ""
    echo "🌐 NETWORK & STORAGE"
    echo "--------------------------------------------------"
    echo "Network Interfaces: $(get_system_info network_interfaces)"
    echo "Mounted Filesystems: $(get_system_info mounted_filesystems)"
    echo "Boot Disk: $(get_system_info boot_disk)"
    echo ""
    echo "🌍 ENVIRONMENT"
    echo "--------------------------------------------------"
    echo "Date: $(get_system_info date)"
    echo "Timezone: $(get_system_info timezone)"
    echo "Shell: $(get_system_info shell)"
    
    echo ""
    echo "💡 Use --detailed for comprehensive information"
    echo "💡 Use --json for machine-readable output"
fi

log_success "✅ System information retrieved successfully!"