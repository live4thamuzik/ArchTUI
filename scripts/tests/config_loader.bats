#!/usr/bin/env bats
# config_loader.bats - Tests for config_loader.sh functions

load 'test_helper'

setup() {
    setup_test_environment
    create_mock_commands

    # Create test config files
    export TEST_CONFIG="$TEST_TMP_DIR/config.json"
    export INVALID_CONFIG="$TEST_TMP_DIR/invalid_config.json"
    export MALFORMED_CONFIG="$TEST_TMP_DIR/malformed.json"

    create_test_config "$TEST_CONFIG"
    create_invalid_config "$INVALID_CONFIG"
    create_malformed_json "$MALFORMED_CONFIG"

    # Source utils first (required by config_loader)
    source_utils
}

teardown() {
    teardown_test_environment
}

# =============================================================================
# JQ Availability Tests
# =============================================================================

@test "check_jq_available succeeds when jq is installed" {
    # Source the script so the function exists!
    source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
    
    run check_jq_available
    [ "$status" -eq 0 ]
}

# =============================================================================
# Configuration Loading Tests
# =============================================================================

@test "load_config_from_json loads boot_mode" {
    # Use real jq if available for this test
    if command -v jq >/dev/null 2>&1; then
        # Source the config loader
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        # Try to load config (may fail due to validation, but should set variables)
        load_config_from_json "$TEST_CONFIG" 2>/dev/null || true
        [ "$BOOT_MODE" = "UEFI" ]
    else
        skip "jq not installed"
    fi
}

@test "load_config_from_json loads install_disk" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        load_config_from_json "$TEST_CONFIG" 2>/dev/null || true
        [ "$INSTALL_DISK" = "/dev/sda" ]
    else
        skip "jq not installed"
    fi
}

@test "load_config_from_json loads partitioning_strategy" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        load_config_from_json "$TEST_CONFIG" 2>/dev/null || true
        [ "$PARTITIONING_STRATEGY" = "auto_simple" ]
    else
        skip "jq not installed"
    fi
}

@test "load_config_from_json loads hostname" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        load_config_from_json "$TEST_CONFIG" 2>/dev/null || true
        [ "$SYSTEM_HOSTNAME" = "testhost" ]
    else
        skip "jq not installed"
    fi
}

@test "load_config_from_json loads username" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        load_config_from_json "$TEST_CONFIG" 2>/dev/null || true
        [ "$MAIN_USERNAME" = "testuser" ]
    else
        skip "jq not installed"
    fi
}

@test "load_config_from_json sets default values for optional fields" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        load_config_from_json "$TEST_CONFIG" 2>/dev/null || true
        # Check default values are set
        [ "$LOCALE" = "en_US.UTF-8" ]
        [ "$KEYMAP" = "us" ]
    else
        skip "jq not installed"
    fi
}

@test "load_config_from_json fails for non-existent file" {
    set +euo pipefail
    source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
    set -euo pipefail

    run load_config_from_json "/nonexistent/config.json"
    [ "$status" -ne 0 ]
}

@test "load_config_from_json fails for malformed JSON" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        run load_config_from_json "$MALFORMED_CONFIG"
        [ "$status" -ne 0 ]
    else
        skip "jq not installed"
    fi
}

# =============================================================================
# Configuration Validation Tests
# =============================================================================

@test "validate_configuration fails when install_disk is empty" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        export INSTALL_DISK=""
        export PARTITIONING_STRATEGY="auto_simple"
        export SYSTEM_HOSTNAME="test"
        export MAIN_USERNAME="user"
        export USER_PASSWORD="pass"
        export ROOT_PASSWORD="root"

        run validate_configuration
        [ "$status" -ne 0 ]
    else
        skip "jq not installed"
    fi
}

@test "validate_configuration fails when hostname is empty" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        export INSTALL_DISK="/dev/sda"
        export PARTITIONING_STRATEGY="auto_simple"
        export SYSTEM_HOSTNAME=""
        export MAIN_USERNAME="user"
        export USER_PASSWORD="pass"
        export ROOT_PASSWORD="root"

        run validate_configuration
        [ "$status" -ne 0 ]
    else
        skip "jq not installed"
    fi
}

@test "validate_configuration fails when username is empty" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        export INSTALL_DISK="/dev/sda"
        export PARTITIONING_STRATEGY="auto_simple"
        export SYSTEM_HOSTNAME="test"
        export MAIN_USERNAME=""
        export USER_PASSWORD="pass"
        export ROOT_PASSWORD="root"

        run validate_configuration
        [ "$status" -ne 0 ]
    else
        skip "jq not installed"
    fi
}

@test "validate_configuration fails for invalid partitioning strategy" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        export INSTALL_DISK="/dev/sda"
        export PARTITIONING_STRATEGY="invalid_strategy"
        export SYSTEM_HOSTNAME="test"
        export MAIN_USERNAME="user"
        export USER_PASSWORD="pass"
        export ROOT_PASSWORD="root"

        run validate_configuration
        [ "$status" -ne 0 ]
    else
        skip "jq not installed"
    fi
}

@test "validate_configuration accepts all valid strategies" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        local strategies=("auto_simple" "auto_simple_luks" "auto_lvm" "auto_luks_lvm" "auto_raid" "auto_raid_luks" "auto_raid_lvm" "auto_raid_lvm_luks" "manual")

        for strategy in "${strategies[@]}"; do
            export INSTALL_DISK="/dev/sda"
            export PARTITIONING_STRATEGY="$strategy"
            export SYSTEM_HOSTNAME="test"
            export MAIN_USERNAME="user"
            export USER_PASSWORD="pass"
            export ROOT_PASSWORD="root"
            export ENCRYPTION="no"

            run validate_configuration
            [ "$status" -eq 0 ]
        done
    else
        skip "jq not installed"
    fi
}

@test "validate_configuration fails when encryption enabled without password" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        export INSTALL_DISK="/dev/sda"
        export PARTITIONING_STRATEGY="auto_simple"
        export SYSTEM_HOSTNAME="test"
        export MAIN_USERNAME="user"
        export USER_PASSWORD="pass"
        export ROOT_PASSWORD="root"
        export ENCRYPTION="yes"
        export ENCRYPTION_PASSWORD=""

        run validate_configuration
        [ "$status" -ne 0 ]
    else
        skip "jq not installed"
    fi
}

@test "validate_configuration succeeds with valid complete config" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        export INSTALL_DISK="/dev/sda"
        export PARTITIONING_STRATEGY="auto_simple"
        export SYSTEM_HOSTNAME="testhost"
        export MAIN_USERNAME="testuser"
        export USER_PASSWORD="testpass"
        export ROOT_PASSWORD="rootpass"
        export ENCRYPTION="no"

        run validate_configuration
        [ "$status" -eq 0 ]
    else
        skip "jq not installed"
    fi
}

@test "validate_configuration fails for invalid disk path" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        export INSTALL_DISK="sda"  # Missing /dev/ prefix
        export PARTITIONING_STRATEGY="auto_simple"
        export SYSTEM_HOSTNAME="test"
        export MAIN_USERNAME="user"
        export USER_PASSWORD="pass"
        export ROOT_PASSWORD="root"
        export ENCRYPTION="no"

        run validate_configuration
        [ "$status" -ne 0 ]
    else
        skip "jq not installed"
    fi
}

# =============================================================================
# Variable Conversion Tests
# =============================================================================

@test "config sets ROOT_FILESYSTEM_TYPE from ROOT_FILESYSTEM" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        load_config_from_json "$TEST_CONFIG" 2>/dev/null || true
        [ "$ROOT_FILESYSTEM_TYPE" = "$ROOT_FILESYSTEM" ]
    else
        skip "jq not installed"
    fi
}

@test "config sets WANT_SWAP from SWAP" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        load_config_from_json "$TEST_CONFIG" 2>/dev/null || true
        [ "$WANT_SWAP" = "$SWAP" ]
    else
        skip "jq not installed"
    fi
}

@test "config sets WANT_HOME_PARTITION from SEPARATE_HOME" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        load_config_from_json "$TEST_CONFIG" 2>/dev/null || true
        [ "$WANT_HOME_PARTITION" = "$SEPARATE_HOME" ]
    else
        skip "jq not installed"
    fi
}

# =============================================================================
# Display Configuration Tests
# =============================================================================

@test "display_configuration outputs configuration values" {
    if command -v jq >/dev/null 2>&1; then
        set +euo pipefail
        source "$SCRIPTS_DIR/config_loader.sh" 2>/dev/null || true
        set -euo pipefail

        export BOOT_MODE="UEFI"
        export INSTALL_DISK="/dev/sda"
        export PARTITIONING_STRATEGY="auto_simple"
        export KERNEL="linux"
        export ROOT_FILESYSTEM="ext4"
        export HOME_FILESYSTEM="ext4"
        export SEPARATE_HOME="no"
        export ENCRYPTION="no"
        export SWAP="yes"
        export SYSTEM_HOSTNAME="testhost"
        export MAIN_USERNAME="testuser"
        export DESKTOP_ENVIRONMENT="none"
        export DISPLAY_MANAGER="none"
        export BOOTLOADER="grub"
        export AUR_HELPER="paru"

        run display_configuration
        [ "$status" -eq 0 ]
        [[ "$output" =~ "UEFI" ]]
        [[ "$output" =~ "/dev/sda" ]]
        [[ "$output" =~ "testhost" ]]
    else
        skip "jq not installed"
    fi
}
