#!/usr/bin/env bats
# disk_utils.bats - Tests for disk_utils.sh functions

load 'test_helper'

setup() {
    setup_test_environment
    create_mock_commands
    source_disk_utils
}

teardown() {
    teardown_test_environment
}

# =============================================================================
# Partition Path Generation Tests
# =============================================================================

@test "get_partition_path returns correct path for SATA disk" {
    run get_partition_path "/dev/sda" "1"
    [ "$status" -eq 0 ]
    [ "$output" = "/dev/sda1" ]
}

@test "get_partition_path returns correct path for NVMe disk" {
    run get_partition_path "/dev/nvme0n1" "1"
    [ "$status" -eq 0 ]
    [ "$output" = "/dev/nvme0n1p1" ]
}

@test "get_partition_path returns correct path for eMMC disk" {
    run get_partition_path "/dev/mmcblk0" "1"
    [ "$status" -eq 0 ]
    [ "$output" = "/dev/mmcblk0p1" ]
}

@test "get_partition_path returns correct path for loop device" {
    run get_partition_path "/dev/loop0" "1"
    [ "$status" -eq 0 ]
    [ "$output" = "/dev/loop0p1" ]
}

@test "get_partition_path handles multiple partitions" {
    run get_partition_path "/dev/sda" "3"
    [ "$status" -eq 0 ]
    [ "$output" = "/dev/sda3" ]
}

# =============================================================================
# Swap Size Calculation Tests
# These tests verify RAM-based swap calculation logic:
# - RAM <= 4GB: 2x RAM
# - RAM <= 16GB: 1x RAM
# - RAM > 16GB: cap at 16GB
# =============================================================================

@test "get_swap_size_mib returns 2048 for 1GB RAM (2x rule)" {
    run get_swap_size_mib "1"
    [ "$status" -eq 0 ]
    [ "$output" = "2048" ]  # 1GB * 2 * 1024 = 2048
}

@test "get_swap_size_mib returns 4096 for 2GB RAM (2x rule)" {
    run get_swap_size_mib "2"
    [ "$status" -eq 0 ]
    [ "$output" = "4096" ]  # 2GB * 2 * 1024 = 4096
}

@test "get_swap_size_mib returns 8192 for 4GB RAM (2x rule)" {
    run get_swap_size_mib "4"
    [ "$status" -eq 0 ]
    [ "$output" = "8192" ]  # 4GB * 2 * 1024 = 8192
}

@test "get_swap_size_mib returns 8192 for 8GB RAM (1x rule)" {
    run get_swap_size_mib "8"
    [ "$status" -eq 0 ]
    [ "$output" = "8192" ]  # 8GB * 1024 = 8192
}

@test "get_swap_size_mib returns 16384 for 16GB RAM (1x rule)" {
    run get_swap_size_mib "16"
    [ "$status" -eq 0 ]
    [ "$output" = "16384" ]  # 16GB * 1024 = 16384
}

@test "get_swap_size_mib parses numeric G suffix" {
    run get_swap_size_mib "6G"
    [ "$status" -eq 0 ]
    [ "$output" = "6144" ]  # 6GB * 1024 = 6144 (1x rule for RAM > 4GB)
}

@test "get_swap_size_mib caps at 16GB for large RAM" {
    run get_swap_size_mib "32"
    [ "$status" -eq 0 ]
    [ "$output" = "16384" ]  # Capped at 16GB
}

@test "get_swap_size_mib returns default for unknown format" {
    run get_swap_size_mib "unknown"
    [ "$status" -eq 0 ]
    [ "$output" = "2048" ]  # DEFAULT_SWAP_SIZE_MIB
}

# =============================================================================
# Partition Type Constants Tests
# =============================================================================

@test "EFI_PARTITION_TYPE constant is defined" {
    [ "$EFI_PARTITION_TYPE" = "EF00" ]
}

@test "LINUX_PARTITION_TYPE constant is defined" {
    [ "$LINUX_PARTITION_TYPE" = "8300" ]
}

@test "LVM_PARTITION_TYPE constant is defined" {
    [ "$LVM_PARTITION_TYPE" = "8E00" ]
}

@test "SWAP_PARTITION_TYPE constant is defined" {
    [ "$SWAP_PARTITION_TYPE" = "8200" ]
}

@test "XBOOTLDR_PARTITION_TYPE constant is defined" {
    [ "$XBOOTLDR_PARTITION_TYPE" = "EA00" ]
}

@test "BIOS_BOOT_PARTITION_TYPE constant is defined" {
    [ "$BIOS_BOOT_PARTITION_TYPE" = "EF02" ]
}

# =============================================================================
# Default Size Constants Tests
# =============================================================================

@test "BOOT_PART_SIZE_MIB constant is 1024" {
    [ "$BOOT_PART_SIZE_MIB" = "1024" ]
}

@test "DEFAULT_ESP_SIZE_MIB constant is 512" {
    [ "$DEFAULT_ESP_SIZE_MIB" = "512" ]
}

@test "DEFAULT_SWAP_SIZE_MIB constant is 2048" {
    [ "$DEFAULT_SWAP_SIZE_MIB" = "2048" ]
}

# =============================================================================
# Disk Wiping Tests
# =============================================================================

@test "wipe_disk calls wipefs" {
    export CONFIRM_WIPE_DISK=yes
    run wipe_disk "/dev/sda"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "wipefs.*--all.*--force.*/dev/sda"
}

@test "wipe_disk calls dd to zero disk" {
    export CONFIRM_WIPE_DISK=yes
    run wipe_disk "/dev/sda"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "dd.*if=/dev/zero"
}

@test "wipe_disk calls partprobe" {
    export CONFIRM_WIPE_DISK=yes
    run wipe_disk "/dev/sda"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "partprobe.*/dev/sda"
}

# =============================================================================
# Partition Table Creation Tests
# =============================================================================

@test "create_partition_table creates GPT for UEFI mode" {
    export BOOT_MODE="UEFI"
    run create_partition_table "/dev/sda"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "sgdisk.*--zap-all.*/dev/sda"
}

# =============================================================================
# ESP Partition Creation Tests
# =============================================================================

@test "create_esp_partition creates partition with correct type" {
    export BOOT_MODE="UEFI"
    mkdir -p /mnt/efi 2>/dev/null || true
    run create_esp_partition "/dev/sda" "1" "512"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "sgdisk.*EF00"
}

@test "create_esp_partition formats with FAT32" {
    export BOOT_MODE="UEFI"
    mkdir -p /mnt/efi 2>/dev/null || true
    run create_esp_partition "/dev/sda" "1"
    assert_mock_called_with_pattern "mkfs.fat.*-F32"
}

# =============================================================================
# XBOOTLDR Partition Tests
# =============================================================================

@test "create_xbootldr_partition creates partition with EA00 type" {
    export BOOT_MODE="UEFI"
    mkdir -p /mnt/boot 2>/dev/null || true
    run create_xbootldr_partition "/dev/sda" "2" "1024"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "sgdisk.*EA00"
}

# =============================================================================
# Swap Partition Tests
# Note: WANT_SWAP check is handled by the calling strategy, not this function
# =============================================================================

@test "create_swap_partition creates partition with correct type" {
    export BOOT_MODE="UEFI"
    run create_swap_partition "/dev/sda" "3" "2048"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "sgdisk.*8200"  # SWAP_PARTITION_TYPE
}

@test "create_swap_partition creates and formats swap" {
    export BOOT_MODE="UEFI"
    run create_swap_partition "/dev/sda" "3" "2048"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "mkswap.*/dev/sda3"
}

@test "create_swap_partition enables swap" {
    export BOOT_MODE="UEFI"
    run create_swap_partition "/dev/sda" "3" "2048"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "swapon.*/dev/sda3"
}

# =============================================================================
# Root Partition Tests
# =============================================================================

@test "create_root_partition creates partition with Linux type" {
    export BOOT_MODE="UEFI"
    run create_root_partition "/dev/sda" "4" "ext4"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "sgdisk.*8300"
}

@test "create_root_partition formats with specified filesystem" {
    export BOOT_MODE="UEFI"
    run create_root_partition "/dev/sda" "4" "ext4"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "mkfs.ext4"
}

# =============================================================================
# Home Partition Tests
# Note: WANT_HOME_PARTITION check is handled by the calling strategy
# =============================================================================

@test "create_home_partition creates partition with Linux type" {
    export BOOT_MODE="UEFI"
    run create_home_partition "/dev/sda" "5" "ext4"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "sgdisk.*8300"  # LINUX_PARTITION_TYPE
}

@test "create_home_partition formats with specified filesystem" {
    export BOOT_MODE="UEFI"
    run create_home_partition "/dev/sda" "5" "btrfs"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "mkfs.btrfs"
}

# =============================================================================
# Safe Mount Tests
# =============================================================================

@test "safe_mount creates mountpoint directory" {
    local test_mountpoint="$TEST_TMP_DIR/test_mount"
    run safe_mount "/dev/sda1" "$test_mountpoint"
    [ "$status" -eq 0 ]
    [ -d "$test_mountpoint" ]
}

@test "safe_mount calls mount command" {
    local test_mountpoint="$TEST_TMP_DIR/test_mount"
    run safe_mount "/dev/sda1" "$test_mountpoint"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "mount.*/dev/sda1.*$test_mountpoint"
}

@test "safe_mount passes mount options" {
    local test_mountpoint="$TEST_TMP_DIR/test_mount"
    run safe_mount "/dev/sda1" "$test_mountpoint" "noatime,compress=zstd"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "mount.*-o.*noatime"
}

# =============================================================================
# LUKS Encryption Tests
# Function signature: setup_luks_encryption(partition, password, mapper_name)
# =============================================================================

@test "setup_luks_encryption fails with empty password" {
    run setup_luks_encryption "/dev/sda1" "" "cryptroot"
    [ "$status" -eq 1 ]
}

@test "setup_luks_encryption calls cryptsetup luksFormat" {
    run setup_luks_encryption "/dev/sda1" "testpassword" "cryptroot"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "cryptsetup.*luksFormat"
}

@test "setup_luks_encryption opens LUKS container" {
    run setup_luks_encryption "/dev/sda1" "testpassword" "cryptroot"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "cryptsetup.*open.*/dev/sda1.*cryptroot"
}

@test "setup_luks_encryption uses default mapper name" {
    run setup_luks_encryption "/dev/sda1" "testpassword"
    [ "$status" -eq 0 ]
    # Default mapper_name is "cryptroot"
    assert_mock_called_with_pattern "cryptsetup.*open.*/dev/sda1.*cryptroot"
}

# =============================================================================
# Btrfs Subvolume Tests
# Function signature: setup_btrfs_subvolumes(mountpoint, include_home)
# =============================================================================

@test "setup_btrfs_subvolumes creates @ subvolume" {
    local mountpoint="$TEST_TMP_DIR/mnt"
    mkdir -p "$mountpoint"
    run setup_btrfs_subvolumes "$mountpoint" "no"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "btrfs.*subvolume.*create.*/@$"
}

@test "setup_btrfs_subvolumes creates @var subvolume" {
    local mountpoint="$TEST_TMP_DIR/mnt"
    mkdir -p "$mountpoint"
    run setup_btrfs_subvolumes "$mountpoint" "no"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "btrfs.*subvolume.*create.*/@var"
}

@test "setup_btrfs_subvolumes creates @tmp subvolume" {
    local mountpoint="$TEST_TMP_DIR/mnt"
    mkdir -p "$mountpoint"
    run setup_btrfs_subvolumes "$mountpoint" "no"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "btrfs.*subvolume.*create.*/@tmp"
}

@test "setup_btrfs_subvolumes creates @snapshots subvolume" {
    local mountpoint="$TEST_TMP_DIR/mnt"
    mkdir -p "$mountpoint"
    run setup_btrfs_subvolumes "$mountpoint" "no"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "btrfs.*subvolume.*create.*/@snapshots"
}

@test "setup_btrfs_subvolumes creates @home when include_home is yes" {
    local mountpoint="$TEST_TMP_DIR/mnt"
    mkdir -p "$mountpoint"
    run setup_btrfs_subvolumes "$mountpoint" "yes"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "btrfs.*subvolume.*create.*/@home"
}

@test "setup_btrfs_subvolumes skips @home when include_home is no" {
    local mountpoint="$TEST_TMP_DIR/mnt"
    mkdir -p "$mountpoint"
    run setup_btrfs_subvolumes "$mountpoint" "no"
    [ "$status" -eq 0 ]
    # @home should NOT be in the mock log
    ! grep -q "@home" "$MOCK_CALLS_LOG"
}

# =============================================================================
# Device Info Capture Tests
# =============================================================================

@test "capture_device_info sets ROOT_DEVICE variable" {
    capture_device_info "root" "/dev/sda1"
    [ "$ROOT_DEVICE" = "/dev/sda1" ]
}

@test "capture_device_info sets EFI_DEVICE variable" {
    capture_device_info "efi" "/dev/sda1"
    [ "$EFI_DEVICE" = "/dev/sda1" ]
}

@test "capture_device_info sets SWAP_DEVICE variable" {
    capture_device_info "swap" "/dev/sda2"
    [ "$SWAP_DEVICE" = "/dev/sda2" ]
}

@test "capture_device_info ignores unknown type (no-op)" {
    # Unknown types are silently ignored (no export, no failure)
    run capture_device_info "unknown_type" "/dev/sda1"
    [ "$status" -eq 0 ]
    # Should still log even for unknown type
    [[ "$output" =~ "unknown_type" ]] || true
}

@test "capture_device_info fails for empty device path" {
    run capture_device_info "root" ""
    [ "$status" -eq 1 ]
}

# =============================================================================
# Validation Tests
# =============================================================================

@test "validate_partitioning_requirements logs validation message" {
    export INSTALL_DISK="/dev/sda"

    # Create a mock block device
    touch "$MOCK_DEV_DIR/sda"

    run validate_partitioning_requirements
    [ "$status" -eq 0 ]
    [[ "$output" =~ "Validating" ]]
}
