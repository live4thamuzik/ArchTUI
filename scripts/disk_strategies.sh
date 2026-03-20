#!/bin/bash
# disk_strategies.sh - Main dispatcher for disk partitioning strategies

set -euo pipefail

# --- Signal Handling for Destructive Operations ---
cleanup_disk_strategies() {
    local sig="$1"
    echo "DISK_STRATEGIES: Received $sig, aborting disk operations..." >&2
    [[ "$sig" == "SIGTERM" ]] && exit 143 || exit 130
}
trap 'cleanup_disk_strategies SIGTERM' SIGTERM
trap 'cleanup_disk_strategies SIGINT' SIGINT

# Source utility functions via bootstrap
_STRAT_DIR="$(dirname "${BASH_SOURCE[0]}")"

# shellcheck source=bootstrap.sh
source "$_STRAT_DIR/bootstrap.sh" || { echo "FATAL: Cannot source bootstrap.sh" >&2; exit 1; }

source_or_die "$_STRAT_DIR/utils.sh"
source_or_die "$_STRAT_DIR/disk_utils.sh"

# Constants are defined in disk_utils.sh

# --- Main Strategy Dispatcher ---
execute_disk_strategy() {
    local strategy_func="$1"
    local base_strategy="$PARTITION_SCHEME"
    
    log_info "Executing disk strategy: $PARTITION_SCHEME"
    
    # Handle RAID strategies
    if [[ "$PARTITION_SCHEME" =~ ^(.+)_(raid[0-9]+)$ ]]; then
        local base_strategy="${BASH_REMATCH[1]}"
        local raid_level="${BASH_REMATCH[2]}"
        log_info "Detected RAID strategy: $base_strategy with RAID level: $raid_level"
        export RAID_LEVEL="$raid_level"
    fi
    
    # Execute the strategy
    if declare -f "$strategy_func" >/dev/null 2>&1; then
        "$strategy_func" || error_exit "Disk strategy '$PARTITION_SCHEME' failed."
    else
        error_exit "Unknown partitioning scheme: $PARTITION_SCHEME."
    fi
    
    log_info "Disk strategy execution complete."
}

# --- Strategy Functions (Load and Execute) ---

# Simple partitioning
do_auto_simple_partitioning_efi_xbootldr() {
    source_or_die "$_STRAT_DIR/strategies/simple.sh"
    execute_simple_partitioning
}

# Simple LUKS partitioning
do_auto_simple_luks_partitioning() {
    source_or_die "$_STRAT_DIR/strategies/simple_luks.sh"
    execute_simple_luks_partitioning
}

# LVM partitioning
do_auto_lvm_partitioning_efi_xbootldr() {
    source_or_die "$_STRAT_DIR/strategies/lvm.sh"
    execute_lvm_partitioning
}

# LVM + LUKS partitioning
do_auto_luks_lvm_partitioning() {
    source_or_die "$_STRAT_DIR/strategies/lvm_luks.sh"
    execute_lvm_luks_partitioning
}

# RAID partitioning
do_auto_raid_partitioning() {
    source_or_die "$_STRAT_DIR/strategies/raid.sh"
    execute_raid_partitioning
}

# RAID + LUKS partitioning
do_auto_raid_luks_partitioning() {
    source_or_die "$_STRAT_DIR/strategies/raid_luks.sh"
    execute_raid_luks_partitioning
}

# RAID + LVM partitioning
do_auto_raid_lvm_partitioning() {
    source_or_die "$_STRAT_DIR/strategies/raid_lvm.sh"
    execute_raid_lvm_partitioning
}

# RAID + LVM + LUKS partitioning
do_auto_raid_lvm_luks_partitioning() {
    source_or_die "$_STRAT_DIR/strategies/raid_lvm_luks.sh"
    execute_raid_lvm_luks_partitioning
}

# Manual partitioning
do_manual_partitioning_guided() {
    source_or_die "$_STRAT_DIR/strategies/manual.sh"
    execute_manual_partitioning
}

# Pre-mounted partitioning (detect existing mounts)
do_pre_mounted_partitioning() {
    source_or_die "$_STRAT_DIR/strategies/pre_mounted.sh"
    execute_pre_mounted_partitioning
}

# --- Legacy Functions (for backward compatibility) ---
# These map to the new modular functions

# Legacy simple partitioning
do_auto_simple_partitioning() {
    do_auto_simple_partitioning_efi_xbootldr
}

# Legacy LVM partitioning  
do_auto_lvm_partitioning() {
    do_auto_lvm_partitioning_efi_xbootldr
}

# Legacy manual partitioning
do_manual_partitioning() {
    do_manual_partitioning_guided
}
