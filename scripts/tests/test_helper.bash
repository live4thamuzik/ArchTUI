#!/bin/bash
# test_helper.bash - Common test utilities and mock functions for bats tests

# Get the directory of the test helper
TESTS_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SCRIPTS_DIR="$(dirname "$TESTS_DIR")"

# Create temporary test directories
setup_test_environment() {
    export TEST_TMP_DIR=$(mktemp -d)
    export LOG_FILE="$TEST_TMP_DIR/test.log"
    export MOCK_CALLS_LOG="$TEST_TMP_DIR/mock_calls.log"
    touch "$MOCK_CALLS_LOG"

    # Create mock /dev directory
    export MOCK_DEV_DIR="$TEST_TMP_DIR/dev"
    mkdir -p "$MOCK_DEV_DIR"

    # Create mock block devices (as files for testing)
    touch "$MOCK_DEV_DIR/sda"
    touch "$MOCK_DEV_DIR/sdb"
    touch "$MOCK_DEV_DIR/nvme0n1"
}

teardown_test_environment() {
    if [[ -d "$TEST_TMP_DIR" ]]; then
        rm -rf "$TEST_TMP_DIR"
    fi
}

# Log mock function calls for verification
log_mock_call() {
    local func_name="$1"
    shift
    echo "$func_name: $*" >> "$MOCK_CALLS_LOG"
}

# Check if a mock was called with specific arguments
assert_mock_called() {
    local func_name="$1"
    shift
    local expected_args="$*"

    if grep -q "^${func_name}: ${expected_args}$" "$MOCK_CALLS_LOG"; then
        return 0
    else
        echo "Expected mock call not found: ${func_name}: ${expected_args}"
        echo "Actual mock calls:"
        cat "$MOCK_CALLS_LOG"
        return 1
    fi
}

# Check if a mock was called (with any arguments)
assert_mock_called_with_pattern() {
    local pattern="$1"

    if grep -q "$pattern" "$MOCK_CALLS_LOG"; then
        return 0
    else
        echo "Expected mock call pattern not found: $pattern"
        echo "Actual mock calls:"
        cat "$MOCK_CALLS_LOG"
        return 1
    fi
}

# Mock system commands
# These create shell functions that override system commands

create_mock_commands() {
    # Mock blkid
    blkid() {
        log_mock_call "blkid" "$@"
        case "$1" in
            "-s")
                if [[ "$2" == "UUID" && "$4" == "/dev/sda1" ]]; then
                    echo "test-uuid-1234"
                elif [[ "$2" == "PARTUUID" && "$4" == "/dev/sda1" ]]; then
                    echo "test-partuuid-5678"
                else
                    echo "mock-uuid-0000"
                fi
                ;;
            *)
                echo "/dev/sda1: UUID=\"test-uuid-1234\" TYPE=\"ext4\""
                ;;
        esac
        return 0
    }
    export -f blkid

    # Mock sgdisk
    sgdisk() {
        log_mock_call "sgdisk" "$@"
        return 0
    }
    export -f sgdisk

    # Mock mkfs.ext4
    mkfs.ext4() {
        log_mock_call "mkfs.ext4" "$@"
        return 0
    }
    export -f mkfs.ext4

    # Mock mkfs.btrfs
    mkfs.btrfs() {
        log_mock_call "mkfs.btrfs" "$@"
        return 0
    }
    export -f mkfs.btrfs

    # Mock mkfs.xfs
    mkfs.xfs() {
        log_mock_call "mkfs.xfs" "$@"
        return 0
    }
    export -f mkfs.xfs

    # Mock mkfs.fat
    mkfs.fat() {
        log_mock_call "mkfs.fat" "$@"
        return 0
    }
    export -f mkfs.fat

    # Mock mkswap
    mkswap() {
        log_mock_call "mkswap" "$@"
        return 0
    }
    export -f mkswap

    # Mock mount
    mount() {
        log_mock_call "mount" "$@"
        return 0
    }
    export -f mount

    # Mock umount
    umount() {
        log_mock_call "umount" "$@"
        return 0
    }
    export -f umount

    # Mock partprobe
    partprobe() {
        log_mock_call "partprobe" "$@"
        return 0
    }
    export -f partprobe

    # Mock wipefs
    wipefs() {
        log_mock_call "wipefs" "$@"
        return 0
    }
    export -f wipefs

    # Mock dd
    dd() {
        log_mock_call "dd" "$@"
        return 0
    }
    export -f dd

    # Mock blockdev
    blockdev() {
        log_mock_call "blockdev" "$@"
        # Return mock size in 512-byte sectors (100GB)
        echo "209715200"
        return 0
    }
    export -f blockdev

    # Mock lsblk
    lsblk() {
        log_mock_call "lsblk" "$@"
        case "$*" in
            *"-b"*)
                # Return size in bytes (100GB)
                echo "107374182400"
                ;;
            *"-n -o MOUNTPOINT"*)
                # Return empty mountpoint (not mounted)
                echo ""
                ;;
            *)
                echo "NAME   SIZE TYPE MOUNTPOINT"
                echo "sda    100G disk"
                echo "sda1    99G part /"
                ;;
        esac
        return 0
    }
    export -f lsblk

    # Mock ping
    ping() {
        log_mock_call "ping" "$@"
        return 0
    }
    export -f ping

    # Mock timedatectl
    timedatectl() {
        log_mock_call "timedatectl" "$@"
        if [[ "$*" == *"NTPSynchronized"* ]]; then
            echo "yes"
        fi
        return 0
    }
    export -f timedatectl

    # Mock pacman
    pacman() {
        log_mock_call "pacman" "$@"
        if [[ "$1" == "-Qi" ]]; then
            # Simulate package is installed
            return 0
        fi
        return 0
    }
    export -f pacman

    # Mock cryptsetup
    cryptsetup() {
        log_mock_call "cryptsetup" "$@"
        return 0
    }
    export -f cryptsetup

    # Mock btrfs
    btrfs() {
        log_mock_call "btrfs" "$@"
        return 0
    }
    export -f btrfs

    # Mock swapon
    swapon() {
        log_mock_call "swapon" "$@"
        return 0
    }
    export -f swapon

    # Mock mountpoint - simulate not mounted by default
    mountpoint() {
        log_mock_call "mountpoint" "$@"
        return 1  # Not a mountpoint
    }
    export -f mountpoint

    # Mock mkdir (but actually create directories)
    # We don't mock mkdir as we need it for real test operations

    # Mock jq
    jq() {
        log_mock_call "jq" "$@"
        case "$1" in
            "-r")
                local query="$2"
                local file="$3"
                if [[ -f "$file" ]]; then
                    # Use real jq if available, otherwise mock
                    if command -v /usr/bin/jq >/dev/null 2>&1; then
                        /usr/bin/jq "$@"
                    else
                        echo "mock_value"
                    fi
                else
                    echo "mock_value"
                fi
                ;;
            "empty")
                return 0
                ;;
            *)
                echo "mock_jq_output"
                ;;
        esac
        return 0
    }
    export -f jq
}

# Create a mock block device for testing
create_mock_block_device() {
    local device_name="$1"
    local device_path="$MOCK_DEV_DIR/$device_name"
    touch "$device_path"

    # Override the -b test to return true for our mock devices
    # This is done by creating a function that tests for our mock paths
}

# Create test configuration file
create_test_config() {
    local config_file="$1"
    cat > "$config_file" << 'EOF'
{
    "boot_mode": "UEFI",
    "install_disk": "/dev/sda",
    "partitioning_strategy": "auto_simple",
    "root_filesystem": "ext4",
    "home_filesystem": "ext4",
    "separate_home": "no",
    "encryption": "no",
    "swap": "yes",
    "swap_size": "2GB",
    "timezone_region": "America",
    "timezone": "New_York",
    "locale": "en_US.UTF-8",
    "keymap": "us",
    "kernel": "linux",
    "hostname": "testhost",
    "username": "testuser",
    "user_password": "testpass123",
    "root_password": "rootpass123",
    "bootloader": "grub",
    "desktop_environment": "none"
}
EOF
}

# Create invalid configuration file (missing required fields)
create_invalid_config() {
    local config_file="$1"
    cat > "$config_file" << 'EOF'
{
    "boot_mode": "UEFI",
    "partitioning_strategy": "auto_simple"
}
EOF
}

# Create malformed JSON file
create_malformed_json() {
    local config_file="$1"
    cat > "$config_file" << 'EOF'
{
    "boot_mode": "UEFI",
    "install_disk": "/dev/sda"
    "missing_comma": true
}
EOF
}

# Source the scripts being tested (with mocks active)
source_utils() {
    # Override EUID for root check tests
    export EUID=0

    # Temporarily disable strict mode for sourcing
    set +euo pipefail
    source "$SCRIPTS_DIR/utils.sh" 2>/dev/null || true
    set -euo pipefail
}

source_disk_utils() {
    set +euo pipefail
    source "$SCRIPTS_DIR/disk_utils.sh" 2>/dev/null || true
    set -euo pipefail
}

# Override test function for block devices
# Since we can't create real block devices in tests, we override the -b test
test_is_block_device() {
    local device="$1"
    # Return true for mock devices
    if [[ "$device" =~ ^/dev/(sda|sdb|nvme|loop) ]] || [[ -f "$MOCK_DEV_DIR/${device##*/}" ]]; then
        return 0
    fi
    return 1
}
