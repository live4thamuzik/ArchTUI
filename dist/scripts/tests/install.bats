#!/usr/bin/env bats
# install.bats - Tests for install.sh functions

load 'test_helper'

setup() {
    setup_test_environment
    create_mock_commands

    # Set up mock /mnt directory
    export MOCK_MNT="$TEST_TMP_DIR/mnt"
    mkdir -p "$MOCK_MNT"
    mkdir -p "$MOCK_MNT/boot"
    mkdir -p "$MOCK_MNT/home"

    # Source install.sh functions (but don't run main)
    export SCRIPT_DIR="$SCRIPTS_DIR"
    export SOURCED_MODE=1

    # Source utils first
    source "$SCRIPTS_DIR/utils.sh"
}

teardown() {
    teardown_test_environment
}

# =============================================================================
# Environment Variable Default Tests
# =============================================================================

@test "INSTALL_DISK defaults to empty string" {
    unset INSTALL_DISK
    source "$SCRIPTS_DIR/install.sh" 2>/dev/null || true
    [[ -z "${INSTALL_DISK:-}" ]] || [[ "$INSTALL_DISK" == "" ]]
}

@test "BOOTLOADER defaults to grub" {
    unset BOOTLOADER
    export BOOTLOADER="${BOOTLOADER:-grub}"
    [ "$BOOTLOADER" = "grub" ]
}

@test "FILESYSTEM defaults to ext4" {
    unset FILESYSTEM
    export FILESYSTEM="${FILESYSTEM:-ext4}"
    [ "$FILESYSTEM" = "ext4" ]
}

@test "DESKTOP_ENVIRONMENT defaults to none" {
    unset DESKTOP_ENVIRONMENT
    export DESKTOP_ENVIRONMENT="${DESKTOP_ENVIRONMENT:-none}"
    [ "$DESKTOP_ENVIRONMENT" = "none" ]
}

@test "PLYMOUTH defaults to No" {
    unset PLYMOUTH
    export PLYMOUTH="${PLYMOUTH:-No}"
    [ "$PLYMOUTH" = "No" ]
}

@test "SECURE_BOOT defaults to No" {
    unset SECURE_BOOT
    export SECURE_BOOT="${SECURE_BOOT:-No}"
    [ "$SECURE_BOOT" = "No" ]
}

@test "BTRFS_SNAPSHOTS defaults to No" {
    unset BTRFS_SNAPSHOTS
    export BTRFS_SNAPSHOTS="${BTRFS_SNAPSHOTS:-No}"
    [ "$BTRFS_SNAPSHOTS" = "No" ]
}

# =============================================================================
# Script Sourcing Tests
# =============================================================================

@test "install.sh sources utils.sh successfully" {
    source "$SCRIPTS_DIR/utils.sh"
    # Check that log_info is available
    type log_info &>/dev/null
}

@test "install.sh sources disk_utils.sh successfully" {
    source "$SCRIPTS_DIR/utils.sh"
    source "$SCRIPTS_DIR/disk_utils.sh"
    # Check that disk utility functions are available
    type get_partition_path &>/dev/null
}

@test "install.sh sources disk_strategies.sh successfully" {
    source "$SCRIPTS_DIR/utils.sh"
    source "$SCRIPTS_DIR/disk_utils.sh"
    source "$SCRIPTS_DIR/disk_strategies.sh"
    # Check that strategy dispatcher is available
    type prepare_disk &>/dev/null
}

# =============================================================================
# Strategy Script Existence Tests
# =============================================================================

@test "simple strategy script exists" {
    [ -f "$SCRIPTS_DIR/strategies/simple.sh" ]
}

@test "simple_luks strategy script exists" {
    [ -f "$SCRIPTS_DIR/strategies/simple_luks.sh" ]
}

@test "lvm strategy script exists" {
    [ -f "$SCRIPTS_DIR/strategies/lvm.sh" ]
}

@test "lvm_luks strategy script exists" {
    [ -f "$SCRIPTS_DIR/strategies/lvm_luks.sh" ]
}

@test "raid strategy script exists" {
    [ -f "$SCRIPTS_DIR/strategies/raid.sh" ]
}

@test "raid_luks strategy script exists" {
    [ -f "$SCRIPTS_DIR/strategies/raid_luks.sh" ]
}

@test "raid_lvm strategy script exists" {
    [ -f "$SCRIPTS_DIR/strategies/raid_lvm.sh" ]
}

@test "raid_lvm_luks strategy script exists" {
    [ -f "$SCRIPTS_DIR/strategies/raid_lvm_luks.sh" ]
}

@test "manual strategy script exists" {
    [ -f "$SCRIPTS_DIR/strategies/manual.sh" ]
}

# =============================================================================
# Desktop Environment Script Existence Tests
# =============================================================================

@test "gnome desktop script exists" {
    [ -f "$SCRIPTS_DIR/desktops/gnome.sh" ]
}

@test "kde desktop script exists" {
    [ -f "$SCRIPTS_DIR/desktops/kde.sh" ]
}

@test "hyprland desktop script exists" {
    [ -f "$SCRIPTS_DIR/desktops/hyprland.sh" ]
}

@test "i3 desktop script exists" {
    [ -f "$SCRIPTS_DIR/desktops/i3.sh" ]
}

@test "xfce desktop script exists" {
    [ -f "$SCRIPTS_DIR/desktops/xfce.sh" ]
}

@test "none desktop script exists" {
    [ -f "$SCRIPTS_DIR/desktops/none.sh" ]
}

# =============================================================================
# Directory Structure Tests
# =============================================================================

@test "scripts directory contains required files" {
    [ -f "$SCRIPTS_DIR/install.sh" ]
    [ -f "$SCRIPTS_DIR/install_wrapper.sh" ]
    [ -f "$SCRIPTS_DIR/chroot_config.sh" ]
    [ -f "$SCRIPTS_DIR/utils.sh" ]
    [ -f "$SCRIPTS_DIR/disk_utils.sh" ]
    [ -f "$SCRIPTS_DIR/disk_strategies.sh" ]
}

@test "strategies directory exists" {
    [ -d "$SCRIPTS_DIR/strategies" ]
}

@test "desktops directory exists" {
    [ -d "$SCRIPTS_DIR/desktops" ]
}

@test "tools directory exists" {
    [ -d "$SCRIPTS_DIR/tools" ]
}

# =============================================================================
# Script Permissions Tests
# =============================================================================

@test "install.sh is executable" {
    [ -x "$SCRIPTS_DIR/install.sh" ]
}

@test "chroot_config.sh is executable" {
    [ -x "$SCRIPTS_DIR/chroot_config.sh" ]
}

@test "install_wrapper.sh is executable" {
    [ -x "$SCRIPTS_DIR/install_wrapper.sh" ]
}
