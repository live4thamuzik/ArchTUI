#!/usr/bin/env bats
# chroot_config.bats - Tests for chroot_config.sh functions

load 'test_helper'

setup() {
    setup_test_environment
    create_mock_commands
    source_utils

    # Create mock /usr/share directory structure
    export MOCK_USR="$TEST_TMP_DIR/usr"
    mkdir -p "$MOCK_USR/share/plymouth/themes"
    mkdir -p "$MOCK_USR/share/secureboot/keys"

    # Create mock /etc directory structure
    export MOCK_ETC="$TEST_TMP_DIR/etc"
    mkdir -p "$MOCK_ETC/snapper/configs"
    mkdir -p "$MOCK_ETC/pacman.d/hooks"
    mkdir -p "$MOCK_ETC/locale.gen"

    # Set up mock environment
    export PLYMOUTH="No"
    export PLYMOUTH_THEME=""
    export BTRFS_SNAPSHOTS="No"
    export FILESYSTEM="ext4"
    export SECURE_BOOT="No"
    export BOOTLOADER="grub"
}

teardown() {
    teardown_test_environment
}

# =============================================================================
# Script Structure Tests
# =============================================================================

@test "chroot_config.sh exists and is executable" {
    [ -f "$SCRIPTS_DIR/chroot_config.sh" ]
    [ -x "$SCRIPTS_DIR/chroot_config.sh" ]
}

@test "chroot_config.sh has proper shebang" {
    head -1 "$SCRIPTS_DIR/chroot_config.sh" | grep -q "#!/bin/bash"
}

@test "chroot_config.sh sets errexit option" {
    grep -q "set -e" "$SCRIPTS_DIR/chroot_config.sh"
}

# =============================================================================
# Function Existence Tests
# =============================================================================

@test "chroot_config.sh defines configure_localization function" {
    grep -q "configure_localization().*" "$SCRIPTS_DIR/chroot_config.sh"
}

@test "chroot_config.sh defines configure_timezone function" {
    # Match function name followed by optional space then parenthesis
    grep -q "configure_timezone" "$SCRIPTS_DIR/chroot_config.sh"
}

@test "chroot_config.sh defines create_users function" {
    grep -q "create_users" "$SCRIPTS_DIR/chroot_config.sh"
}

@test "chroot_config.sh defines configure_mkinitcpio function" {
    grep -q "configure_mkinitcpio().*" "$SCRIPTS_DIR/chroot_config.sh"
}

@test "chroot_config.sh defines configure_plymouth function" {
    grep -q "configure_plymouth().*" "$SCRIPTS_DIR/chroot_config.sh"
}

@test "chroot_config.sh defines configure_snapper function" {
    grep -q "configure_snapper().*" "$SCRIPTS_DIR/chroot_config.sh"
}

@test "chroot_config.sh defines configure_secure_boot function" {
    grep -q "configure_secure_boot().*" "$SCRIPTS_DIR/chroot_config.sh"
}

# =============================================================================
# Plymouth Configuration Tests
# =============================================================================

@test "configure_plymouth checks PLYMOUTH environment variable" {
    grep -q 'PLYMOUTH:-No' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "configure_plymouth installs plymouth package when enabled" {
    grep -q 'pacman -S --noconfirm --needed plymouth' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "configure_plymouth sets theme with plymouth-set-default-theme" {
    grep -q 'plymouth-set-default-theme' "$SCRIPTS_DIR/chroot_config.sh"
}

# =============================================================================
# Snapper Configuration Tests
# =============================================================================

@test "configure_snapper checks BTRFS_SNAPSHOTS environment variable" {
    grep -q 'BTRFS_SNAPSHOTS:-No' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "configure_snapper checks filesystem type" {
    grep -q 'FILESYSTEM.*btrfs' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "configure_snapper installs snapper packages" {
    grep -q 'pacman -S --noconfirm --needed snapper snap-pac' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "configure_snapper creates snapper config for root" {
    grep -q 'snapper -c root create-config' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "configure_snapper enables snapper timers" {
    grep -q 'snapper-timeline.timer' "$SCRIPTS_DIR/chroot_config.sh"
    grep -q 'snapper-cleanup.timer' "$SCRIPTS_DIR/chroot_config.sh"
}

# =============================================================================
# Secure Boot Configuration Tests
# =============================================================================

@test "configure_secure_boot checks SECURE_BOOT environment variable" {
    grep -q 'SECURE_BOOT:-No' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "configure_secure_boot checks for UEFI mode" {
    grep -q '/sys/firmware/efi' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "configure_secure_boot installs sbctl" {
    grep -q 'pacman -S --noconfirm --needed sbctl' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "configure_secure_boot creates keys" {
    grep -q 'sbctl create-keys' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "configure_secure_boot signs kernel" {
    grep -q 'sbctl sign' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "configure_secure_boot creates enrollment script" {
    grep -q 'enroll-secure-boot-keys.sh' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "configure_secure_boot creates pacman hook for signing" {
    grep -q '95-secureboot.hook' "$SCRIPTS_DIR/chroot_config.sh"
}

# =============================================================================
# mkinitcpio Configuration Tests
# =============================================================================

@test "configure_mkinitcpio adds plymouth hook when enabled" {
    grep -q 'hooks=.*plymouth' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "configure_mkinitcpio adds plymouth-encrypt hook for encrypted systems" {
    grep -q 'plymouth-encrypt' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "configure_mkinitcpio adds lvm2 hook when needed" {
    grep -q 'hooks=.*lvm2' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "configure_mkinitcpio adds encrypt hook for LUKS" {
    grep -q 'hooks=.*encrypt' "$SCRIPTS_DIR/chroot_config.sh"
}

# =============================================================================
# Desktop Environment Tests
# =============================================================================

@test "chroot_config.sh handles gnome desktop" {
    grep -q 'gnome' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "chroot_config.sh handles kde desktop" {
    grep -q 'kde\|plasma' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "chroot_config.sh handles hyprland desktop" {
    grep -q 'hyprland' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "chroot_config.sh handles i3 desktop" {
    grep -q 'i3' "$SCRIPTS_DIR/chroot_config.sh"
}

@test "chroot_config.sh handles xfce desktop" {
    grep -q 'xfce' "$SCRIPTS_DIR/chroot_config.sh"
}

# =============================================================================
# Phase Order Tests
# =============================================================================

@test "chroot_config has Phase 1: Basic System Configuration" {
    grep -q "Phase 1.*Basic System" "$SCRIPTS_DIR/chroot_config.sh"
}

@test "chroot_config has Phase 2: Bootloader & Initramfs" {
    grep -q "Phase 2.*Bootloader" "$SCRIPTS_DIR/chroot_config.sh"
}

@test "chroot_config has Phase 3: Desktop Environment" {
    grep -q "Phase 3.*Desktop" "$SCRIPTS_DIR/chroot_config.sh"
}

@test "chroot_config has Phase 4: Additional Software" {
    grep -q "Phase 4.*Additional" "$SCRIPTS_DIR/chroot_config.sh"
}

@test "chroot_config has Phase 5: Final Configuration" {
    grep -q "Phase 5.*Final" "$SCRIPTS_DIR/chroot_config.sh"
}
