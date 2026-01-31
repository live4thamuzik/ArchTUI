#!/usr/bin/env bats
# utils.bats - Tests for utils.sh functions

load 'test_helper'

setup() {
    setup_test_environment
    create_mock_commands
    # Order matters: utils first, then disk_utils which depends on utils
    source "$SCRIPTS_DIR/utils.sh"
    source "$SCRIPTS_DIR/disk_utils.sh"
}

teardown() {
    teardown_test_environment
}

# =============================================================================
# Logging Function Tests
# =============================================================================

@test "log_info outputs timestamped info message" {
    run log_info "Test message"
    [ "$status" -eq 0 ]
    [[ "$output" =~ "INFO: Test message" ]]
}

@test "log_warn outputs timestamped warning message to stderr" {
    run log_warn "Warning message"
    [ "$status" -eq 0 ]
    [[ "$output" =~ "WARN: Warning message" ]]
}

@test "log_error outputs timestamped error message to stderr" {
    run log_error "Error message"
    [ "$status" -eq 0 ]
    [[ "$output" =~ "ERROR: Error message" ]]
}

@test "log_success outputs timestamped success message" {
    run log_success "Success message"
    [ "$status" -eq 0 ]
    [[ "$output" =~ "SUCCESS: Success message" ]]
}

@test "log_debug outputs message when LOG_LEVEL is DEBUG" {
    export LOG_LEVEL="DEBUG"
    run log_debug "Debug message"
    [ "$status" -eq 0 ]
    [[ "$output" =~ "DEBUG: Debug message" ]]
}

@test "log_debug does not output when LOG_LEVEL is INFO" {
    export LOG_LEVEL="INFO"
    run log_debug "Hidden message"
    [ "$status" -eq 0 ]
    [[ ! "$output" =~ "Hidden message" ]]
}

# =============================================================================
# Validation Helper Tests
# =============================================================================

@test "validate_username accepts valid username" {
    run validate_username "valid_user"
    [ "$status" -eq 0 ]
}

@test "validate_username accepts username with underscore" {
    run validate_username "_valid_user"
    [ "$status" -eq 0 ]
}

@test "validate_username accepts username with numbers" {
    run validate_username "user123"
    [ "$status" -eq 0 ]
}

@test "validate_username rejects empty username" {
    run validate_username ""
    [ "$status" -eq 1 ]
}

@test "validate_username rejects username with spaces" {
    run validate_username "invalid user"
    [ "$status" -eq 1 ]
}

@test "validate_username rejects username with special characters" {
    run validate_username "user@name"
    [ "$status" -eq 1 ]
}

@test "validate_hostname accepts valid hostname" {
    run validate_hostname "my-host"
    [ "$status" -eq 0 ]
}

@test "validate_hostname accepts hostname with dashes" {
    run validate_hostname "my-cool-host"
    [ "$status" -eq 0 ]
}

@test "validate_hostname accepts hostname with dots" {
    run validate_hostname "host.domain"
    [ "$status" -eq 0 ]
}

@test "validate_hostname rejects empty hostname" {
    run validate_hostname ""
    [ "$status" -eq 1 ]
}

@test "validate_hostname rejects hostname with spaces" {
    run validate_hostname "my host"
    [ "$status" -eq 1 ]
}

@test "validate_hostname rejects hostname with special characters" {
    run validate_hostname "host!"
    [ "$status" -eq 1 ]
}

# =============================================================================
# Input Sanitization Tests (Sprint 6 Step 2)
# =============================================================================

@test "shell_escape escapes single quotes" {
    run shell_escape "it's a test"
    [ "$status" -eq 0 ]
    # Output should be escaped so it's safe for shell
    [[ "$output" != "it's a test" ]]
}

@test "shell_escape escapes special characters" {
    run shell_escape 'test$VAR'
    [ "$status" -eq 0 ]
    # Dollar sign should be escaped
    [[ "$output" =~ '\$' || "$output" =~ "'" ]]
}

@test "shell_escape handles backticks" {
    run shell_escape 'test`cmd`'
    [ "$status" -eq 0 ]
    # Backticks should be escaped
    [[ "$output" != 'test`cmd`' ]]
}

@test "shell_escape preserves safe strings" {
    run shell_escape "safe_string123"
    [ "$status" -eq 0 ]
    [ "$output" = "safe_string123" ]
}

@test "validate_safe_string accepts alphanumeric with dash underscore dot" {
    run validate_safe_string "file-name_v1.2"
    [ "$status" -eq 0 ]
}

@test "validate_safe_string rejects spaces" {
    run validate_safe_string "file name"
    [ "$status" -eq 1 ]
}

@test "validate_safe_string rejects shell metacharacters" {
    run validate_safe_string 'file;rm'
    [ "$status" -eq 1 ]
}

@test "validate_safe_string rejects empty string" {
    run validate_safe_string ""
    [ "$status" -eq 1 ]
}

@test "validate_device_path accepts standard block devices" {
    run validate_device_path "/dev/sda"
    [ "$status" -eq 0 ]
}

@test "validate_device_path accepts nvme devices" {
    run validate_device_path "/dev/nvme0n1p1"
    [ "$status" -eq 0 ]
}

@test "validate_device_path accepts mapper devices" {
    run validate_device_path "/dev/mapper/cryptroot"
    [ "$status" -eq 0 ]
}

@test "validate_device_path rejects paths without /dev/" {
    run validate_device_path "/tmp/fake"
    [ "$status" -eq 1 ]
}

@test "validate_device_path rejects shell injection attempts" {
    run validate_device_path '/dev/sda;rm -rf /'
    [ "$status" -eq 1 ]
}

# =============================================================================
# Device Helper Tests (Requires disk_utils.sh)
# =============================================================================

@test "get_device_uuid returns UUID for valid device" {
    run get_device_uuid "/dev/sda1"
    [ "$status" -eq 0 ]
    [ "$output" = "mock-uuid-sda1" ]
}

@test "get_device_uuid fails for empty device path" {
    run get_device_uuid ""
    [ "$status" -eq 1 ]
}

@test "capture_device_info captures root device correctly" {
    # Can't use 'run' here because export doesn't propagate from subshell
    capture_device_info "root" "/dev/sda1"
    local status=$?
    [ "$status" -eq 0 ]
    [ "$ROOT_DEVICE" = "/dev/sda1" ]
}

# =============================================================================
# Package Helper Tests
# =============================================================================

@test "check_package_available succeeds for installed package" {
    # Mock pacman -Si to succeed for e2fsprogs
    function pacman() {
        if [[ "$1" == "-Si" ]]; then
            return 0
        fi
        return 1
    }
    export -f pacman
    
    run check_package_available "e2fsprogs"
    [ "$status" -eq 0 ]
}

# =============================================================================
# Filesystem Helper Tests
# =============================================================================

@test "format_filesystem calls mkfs.ext4 for ext4" {
    run format_filesystem "/dev/sda1" "ext4"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "mkfs.ext4.*/dev/sda1"
}

@test "format_filesystem calls mkfs.btrfs for btrfs" {
    run format_filesystem "/dev/sda1" "btrfs"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "mkfs.btrfs.*/dev/sda1"
}

@test "format_filesystem calls mkfs.xfs for xfs" {
    run format_filesystem "/dev/sda1" "xfs"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "mkfs.xfs.*/dev/sda1"
}

@test "format_filesystem calls mkfs.fat for vfat" {
    run format_filesystem "/dev/sda1" "vfat"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "mkfs.fat.*/dev/sda1"
}

@test "format_filesystem calls mkswap for swap" {
    run format_filesystem "/dev/sda1" "swap"
    [ "$status" -eq 0 ]
    assert_mock_called_with_pattern "mkswap.*/dev/sda1"
}

@test "format_filesystem fails for unknown filesystem type" {
    run format_filesystem "/dev/sda1" "unknown_fs"
    [ "$status" -eq 1 ]
}

# =============================================================================
# Error Handling Tests
# =============================================================================

@test "error_exit terminates with exit code 1" {
    # Wrap in subshell to catch exit
    run bash -c 'source '"$SCRIPTS_DIR"'/utils.sh 2>/dev/null; error_exit "Test error"'
    [ "$status" -eq 1 ]
}

@test "log_and_continue does not exit" {
    run log_and_continue "Non-critical error"
    [ "$status" -eq 0 ]
    [[ "$output" =~ "WARN: Non-critical error" ]]
}

# =============================================================================
# Command Execution Tests
# =============================================================================

@test "execute_non_critical runs command and logs success" {
    run execute_non_critical "Test operation" echo "hello"
    [ "$status" -eq 0 ]
    [[ "$output" =~ "INFO: Test operation" ]]
}

@test "execute_non_critical returns 1 on failure but continues" {
    run execute_non_critical "Failing op" false
    [ "$status" -eq 1 ]
    [[ "$output" =~ "NON-CRITICAL: Failing op failed" ]]
}
