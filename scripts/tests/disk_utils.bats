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
# =============================================================================

@test "get_swap_size_mib returns 1024 for 1GB" {
    export SWAP_SIZE="1GB"
    run get_swap_size_mib
    [ "$status" -eq 0 ]
    [ "$output" = "1024" ]
}

@test "get_swap_size_mib returns 2048 for 2GB" {
    export SWAP_SIZE="2GB"
    run get_swap_size_mib
    [ "$status" -eq 0 ]
    [ "$output" = "2048" ]
}

@test "get_swap_size_mib returns 4096 for 4GB" {
    export SWAP_SIZE="4GB"
    run get_swap_size_mib
    [ "$status" -eq 0 ]
    [ "$output" = "4096" ]
}

@test "get_swap_size_mib returns 8192 for 8GB" {
    export SWAP_SIZE="8GB"
    run get_swap_size_mib
    [ "$status" -eq 0 ]
    [ "$output" = "8192" ]
}

@test "get_swap_size_mib returns 16384 for 16GB" {
    export SWAP_SIZE="16GB"
    run get_swap_size_mib
    [ "$status" -eq 0 ]
    [ "$output" = "16384" ]
}

@test "get_swap_size_mib parses numeric GB value" {
    export SWAP_SIZE="6G"
    run get_swap_size_mib
    [ "$status" -eq 0 ]
    [ "$output" = "6144" ]
}

@test "get_swap_size_mib parses MB value" {
    export SWAP_SIZE="512M"
    run get_swap_size_mib
    [ "$status" -eq 0 ]
    [ "$output" = "512" ]
}

@test "get_swap_size_mib returns default for unknown format" {
    export SWAP_SIZE="unknown"
    run get_swap_size_mib
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
    export INSTALL_DISK="/dev/sda"
    run wipe_disk "/dev/sda"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "wipefs.*-af.*/dev/sda"
}

@test "wipe_disk calls dd to zero disk" {
    run wipe_disk "/dev/sda"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "dd.*if=/dev/zero"
}

@test "wipe_disk calls partprobe" {
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
# =============================================================================

@test "create_swap_partition skips when WANT_SWAP is no" {
    export WANT_SWAP="no"
    run create_swap_partition "/dev/sda" "3" "2048"
    [ "$status" -eq 0 ]
    [[ "$output" =~ "not requested" ]]
}

@test "create_swap_partition creates partition when WANT_SWAP is yes" {
    export WANT_SWAP="yes"
    export BOOT_MODE="UEFI"
    run create_swap_partition "/dev/sda" "3" "2048"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "mkswap"
}

@test "create_swap_partition enables swap" {
    export WANT_SWAP="yes"
    export BOOT_MODE="UEFI"
    run create_swap_partition "/dev/sda" "3" "2048"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "swapon"
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
# =============================================================================

@test "create_home_partition skips when WANT_HOME_PARTITION is no" {
    export WANT_HOME_PARTITION="no"
    run create_home_partition "/dev/sda" "5" "ext4"
    [ "$status" -eq 0 ]
    [[ "$output" =~ "not requested" ]]
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
# =============================================================================

@test "setup_luks_encryption fails without password" {
    unset ENCRYPTION_PASSWORD
    run setup_luks_encryption "/dev/sda1" "cryptroot"
    [ "$status" -eq 1 ]
}

@test "setup_luks_encryption calls cryptsetup luksFormat" {
    export ENCRYPTION_PASSWORD="testpassword"
    run setup_luks_encryption "/dev/sda1" "cryptroot"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "cryptsetup.*luksFormat"
}

@test "setup_luks_encryption opens LUKS container" {
    export ENCRYPTION_PASSWORD="testpassword"
    run setup_luks_encryption "/dev/sda1" "cryptroot"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "cryptsetup.*open.*/dev/sda1.*cryptroot"
}

# =============================================================================
# Btrfs Subvolume Tests
# =============================================================================

@test "setup_btrfs_subvolumes creates @ subvolume" {
    mkdir -p /mnt
    run setup_btrfs_subvolumes "/dev/sda1" "no"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "btrfs.*subvolume.*create.*/mnt/@"
}

@test "setup_btrfs_subvolumes creates @var subvolume" {
    mkdir -p /mnt
    run setup_btrfs_subvolumes "/dev/sda1" "no"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "btrfs.*subvolume.*create.*/mnt/@var"
}

@test "setup_btrfs_subvolumes creates @tmp subvolume" {
    mkdir -p /mnt
    run setup_btrfs_subvolumes "/dev/sda1" "no"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "btrfs.*subvolume.*create.*/mnt/@tmp"
}

@test "setup_btrfs_subvolumes creates @snapshots subvolume" {
    mkdir -p /mnt
    run setup_btrfs_subvolumes "/dev/sda1" "no"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "btrfs.*subvolume.*create.*/mnt/@snapshots"
}

@test "setup_btrfs_subvolumes creates @home when include_home is yes" {
    mkdir -p /mnt
    run setup_btrfs_subvolumes "/dev/sda1" "yes"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "btrfs.*subvolume.*create.*/mnt/@home"
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

@test "capture_device_info fails for unknown type" {
    run capture_device_info "unknown_type" "/dev/sda1"
    [ "$status" -eq 1 ]
}

@test "capture_device_info fails for empty device path" {
    run capture_device_info "root" ""
    [ "$status" -eq 1 ]
}

# =============================================================================
# Validation Tests
# =============================================================================

@test "validate_partitioning_requirements sets default filesystem types" {
    unset ROOT_FILESYSTEM_TYPE
    unset HOME_FILESYSTEM_TYPE
    export ROOT_FILESYSTEM="ext4"
    export HOME_FILESYSTEM="ext4"
    export INSTALL_DISK="/dev/sda"

    # Create a mock block device
    touch "$MOCK_DEV_DIR/sda"

    validate_partitioning_requirements
    [ "$ROOT_FILESYSTEM_TYPE" = "ext4" ]
    [ "$HOME_FILESYSTEM_TYPE" = "ext4" ]
}
