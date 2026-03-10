//! Type-safe arguments for disk tool scripts.
//!
//! This module provides typed argument structs for disk-related scripts:
//! - `WipeDiskArgs` for `wipe_disk.sh`
//! - `FormatPartitionArgs` for `format_partition.sh`
//! - `MountPartitionArgs` for simple mount operations
//! - `MountPartitionsArgs` for advanced mount/umount operations
//! - `CheckDiskHealthArgs` for `check_disk_health.sh`
//! - `ManualPartitionArgs` for `manual_partition.sh`
//!
//! # Why This Exists
//!
//! The bash script `wipe_disk.sh` expects `--disk`, but early Rust code used `--device`.
//! This caused runtime failures. By using typed structs, the mapping is explicit and
//! verified at compile time.
//!
//! # Filesystem Enum
//!
//! `FormatPartitionArgs` uses the `Filesystem` enum from `types.rs` to prevent
//! string typos like "etx4" instead of "ext4".

use std::path::PathBuf;

use crate::script_traits::ScriptArgs;
use crate::types::Filesystem;

// ============================================================================
// Format Partition
// ============================================================================

/// Type-safe arguments for `scripts/tools/format_partition.sh`.
///
/// # Filesystem Enum
///
/// Uses the `Filesystem` enum to prevent typos. The enum maps to the exact
/// strings expected by the bash script:
///
/// | Enum Variant | String   |
/// |--------------|----------|
/// | `Ext4`       | `"ext4"` |
/// | `Xfs`        | `"xfs"`  |
/// | `Btrfs`      | `"btrfs"`|
/// | `Fat32`      | `"fat32"`|
///
/// # Field to Flag Mapping
///
/// | Rust Field   | CLI Flag       | Required |
/// |--------------|----------------|----------|
/// | `device`     | `--device`     | Yes      |
/// | `filesystem` | `--filesystem` | Yes      |
/// | `label`      | `--label`      | No       |
/// | `force`      | `--force`      | No       |
#[derive(Debug, Clone)]
pub struct FormatPartitionArgs {
    /// Partition device path (e.g., `/dev/sda1`).
    pub device: PathBuf,
    /// Filesystem type - uses strict enum to prevent typos.
    pub filesystem: Filesystem,
    /// Optional filesystem label.
    pub label: Option<String>,
    /// Force formatting even if mounted.
    pub force: bool,
}

impl ScriptArgs for FormatPartitionArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec![
            "--device".to_string(),
            self.device.display().to_string(),
            "--filesystem".to_string(),
            // Enum maps to lowercase string via strum Display
            self.filesystem.to_string(),
        ];
        if let Some(ref label) = self.label {
            args.push("--label".to_string());
            args.push(label.clone());
        }
        if self.force {
            args.push("--force".to_string());
        }
        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "format_partition.sh"
    }

    /// Format is DESTRUCTIVE - erases partition contents.
    fn is_destructive(&self) -> bool {
        true
    }
}

// ============================================================================
// Check Disk Health
// ============================================================================

/// Type-safe arguments for `scripts/tools/check_disk_health.sh`.
///
/// # Field to Flag Mapping
///
/// | Rust Field | CLI Flag   | Required |
/// |------------|------------|----------|
/// | `device`   | `--device` | Yes      |
#[derive(Debug, Clone)]
pub struct CheckDiskHealthArgs {
    /// Disk device path (e.g., `/dev/sda`).
    pub device: PathBuf,
}

impl ScriptArgs for CheckDiskHealthArgs {
    fn to_cli_args(&self) -> Vec<String> {
        vec!["--device".to_string(), self.device.display().to_string()]
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "check_disk_health.sh"
    }

    /// Disk health check is READ-ONLY - not destructive.
    fn is_destructive(&self) -> bool {
        false
    }
}

// ============================================================================
// Mount Partitions
// ============================================================================

/// Type-safe arguments for `scripts/tools/mount_partitions.sh`.
///
/// # Field to Flag Mapping
///
/// | Rust Field   | CLI Flag       | Required |
/// |--------------|----------------|----------|
/// | `action`     | `--action`     | Yes      |
/// | `device`     | `--device`     | Yes      |
/// | `mountpoint` | `--mountpoint` | No       |
/// | `filesystem` | `--filesystem` | No       |
#[derive(Debug, Clone)]
pub struct MountPartitionsArgs {
    /// Action to perform (e.g., `mount`, `unmount`).
    pub action: String,
    /// Device path (e.g., `/dev/sda1`).
    pub device: PathBuf,
    /// Optional mountpoint path.
    pub mountpoint: Option<PathBuf>,
    /// Optional filesystem type.
    pub filesystem: Option<String>,
}

impl ScriptArgs for MountPartitionsArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec![
            "--action".to_string(),
            self.action.clone(),
            "--device".to_string(),
            self.device.display().to_string(),
        ];
        if let Some(ref mp) = self.mountpoint {
            args.push("--mountpoint".to_string());
            args.push(mp.display().to_string());
        }
        if let Some(ref fs) = self.filesystem {
            args.push("--filesystem".to_string());
            args.push(fs.clone());
        }
        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "mount_partitions.sh"
    }

    /// Mount operations are not destructive (don't erase data).
    fn is_destructive(&self) -> bool {
        false
    }
}

// ============================================================================
// Mount Partition (Simple)
// ============================================================================

/// Simplified type-safe arguments for mounting a single partition.
///
/// This is a convenience wrapper for common mount operations during installation.
/// For advanced operations (umount, list, info), use `MountPartitionsArgs`.
///
/// # Field to Flag Mapping
///
/// | Rust Field   | CLI Flag       | Required |
/// |--------------|----------------|----------|
/// | `device`     | `--device`     | Yes      |
/// | `mountpoint` | `--mountpoint` | Yes      |
/// | `options`    | `--options`    | No       |
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use archtui::scripts::disk::MountPartitionArgs;
/// use archtui::script_traits::ScriptArgs;
///
/// // Mount root partition
/// let args = MountPartitionArgs {
///     device: PathBuf::from("/dev/sda2"),
///     mountpoint: PathBuf::from("/mnt"),
///     options: None,
/// };
///
/// assert_eq!(args.to_cli_args(), vec![
///     "--action", "mount",
///     "--device", "/dev/sda2",
///     "--mountpoint", "/mnt",
/// ]);
/// ```
#[derive(Debug, Clone)]
#[allow(dead_code)] // Used by installer::prepare_disks
pub struct MountPartitionArgs {
    /// Partition device path (e.g., `/dev/sda2`).
    pub device: PathBuf,
    /// Target mountpoint (e.g., `/mnt` or `/mnt/boot`).
    pub mountpoint: PathBuf,
    /// Optional mount options (e.g., `noatime,compress=zstd`).
    pub options: Option<String>,
}

impl ScriptArgs for MountPartitionArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec![
            "--action".to_string(),
            "mount".to_string(),
            "--device".to_string(),
            self.device.display().to_string(),
            "--mountpoint".to_string(),
            self.mountpoint.display().to_string(),
        ];
        if let Some(ref opts) = self.options {
            args.push("--options".to_string());
            args.push(opts.clone());
        }
        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "mount_partitions.sh"
    }

    /// Mount operations are not destructive (don't erase data).
    fn is_destructive(&self) -> bool {
        false
    }
}

// ============================================================================
// Table Type (GPT / MBR)
// ============================================================================

/// Partition table type for `create-table` action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableType {
    /// GUID Partition Table (modern, supports > 2TB, required for UEFI)
    Gpt,
    /// Master Boot Record (legacy BIOS)
    Mbr,
}

impl TableType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TableType::Gpt => "gpt",
            TableType::Mbr => "mbr",
        }
    }
}

impl std::fmt::Display for TableType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for TableType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "gpt" => Ok(TableType::Gpt),
            "mbr" | "dos" | "msdos" => Ok(TableType::Mbr),
            _ => Err(format!("Invalid table type '{}'. Valid: gpt, mbr", s)),
        }
    }
}

// ============================================================================
// Partition Type (sgdisk type codes)
// ============================================================================

/// Partition type codes for `add-partition` action.
///
/// Maps to sgdisk type codes (GPT) and sfdisk type IDs (MBR).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartitionType {
    /// EFI System Partition (sgdisk EF00, MBR ef)
    Efi,
    /// BIOS Boot Partition (sgdisk EF02, MBR — N/A, GPT only)
    BiosBoot,
    /// Linux filesystem (sgdisk 8300, MBR 83)
    Linux,
    /// Linux swap (sgdisk 8200, MBR 82)
    Swap,
    /// Linux LVM (sgdisk 8E00, MBR 8e)
    Lvm,
    /// Linux LUKS (sgdisk 8309, MBR 83)
    Luks,
}

#[allow(dead_code)] // Used by bash script mapping + tests
impl PartitionType {
    /// sgdisk type code for GPT tables.
    pub fn as_sgdisk_code(&self) -> &'static str {
        match self {
            PartitionType::Efi => "EF00",
            PartitionType::BiosBoot => "EF02",
            PartitionType::Linux => "8300",
            PartitionType::Swap => "8200",
            PartitionType::Lvm => "8E00",
            PartitionType::Luks => "8309",
        }
    }

    /// sfdisk type ID for MBR tables.
    pub fn as_mbr_id(&self) -> &'static str {
        match self {
            PartitionType::Efi => "ef",
            PartitionType::BiosBoot => "ef", // BIOS boot is GPT-only; best MBR approximation
            PartitionType::Linux => "83",
            PartitionType::Swap => "82",
            PartitionType::Lvm => "8e",
            PartitionType::Luks => "83",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            PartitionType::Efi => "EFI System",
            PartitionType::BiosBoot => "BIOS Boot",
            PartitionType::Linux => "Linux",
            PartitionType::Swap => "Linux Swap",
            PartitionType::Lvm => "Linux LVM",
            PartitionType::Luks => "Linux LUKS",
        }
    }
}

impl std::fmt::Display for PartitionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_sgdisk_code())
    }
}

impl std::str::FromStr for PartitionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "EF00" | "EFI" | "EFI SYSTEM" => Ok(PartitionType::Efi),
            "EF02" | "BIOS BOOT" => Ok(PartitionType::BiosBoot),
            "8300" | "LINUX" => Ok(PartitionType::Linux),
            "8200" | "LINUX SWAP" | "SWAP" => Ok(PartitionType::Swap),
            "8E00" | "LINUX LVM" | "LVM" => Ok(PartitionType::Lvm),
            "8309" | "LINUX LUKS" | "LUKS" => Ok(PartitionType::Luks),
            _ => Err(format!(
                "Invalid partition type '{}'. Valid: EF00/EFI, EF02/BIOS Boot, 8300/Linux, 8200/Swap, 8E00/LVM, 8309/LUKS",
                s
            )),
        }
    }
}

// ============================================================================
// Manual Partition (cfdisk mode)
// ============================================================================

/// Type-safe arguments for `scripts/tools/manual_partition.sh --action cfdisk`.
///
/// # Field to Flag Mapping
///
/// | Rust Field | CLI Flag     | Required |
/// |------------|--------------|----------|
/// | `device`   | `--device`   | Yes      |
/// | (action)   | `--action`   | Yes (always "cfdisk") |
#[derive(Debug, Clone)]
pub struct ManualPartitionArgs {
    /// Disk device path (e.g., `/dev/sda`).
    pub device: PathBuf,
}

impl ScriptArgs for ManualPartitionArgs {
    fn to_cli_args(&self) -> Vec<String> {
        vec![
            "--device".to_string(),
            self.device.display().to_string(),
            "--action".to_string(),
            "cfdisk".to_string(),
        ]
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![("CONFIRM_MANUAL_PARTITION".to_string(), "yes".to_string())]
    }

    fn script_name(&self) -> &'static str {
        "manual_partition.sh"
    }

    /// Manual partitioning is DESTRUCTIVE - modifies partition table.
    fn is_destructive(&self) -> bool {
        true
    }
}

// ============================================================================
// Create Partition Table
// ============================================================================

/// Type-safe arguments for `scripts/tools/manual_partition.sh --action create-table`.
///
/// # Field to Flag Mapping
///
/// | Rust Field    | CLI Flag       | Required |
/// |---------------|----------------|----------|
/// | `device`      | `--device`     | Yes      |
/// | `table_type`  | `--table-type` | Yes      |
/// | (action)      | `--action`     | Yes (always "create-table") |
/// | `confirm`     | N/A (env)      | Sets `CONFIRM_MANUAL_PARTITION=yes` |
#[derive(Debug, Clone)]
pub struct CreateTableArgs {
    /// Disk device path (e.g., `/dev/sda`).
    pub device: PathBuf,
    /// Partition table type (GPT or MBR).
    pub table_type: TableType,
    /// Whether to set `CONFIRM_MANUAL_PARTITION=yes`.
    pub confirm: bool,
}

impl ScriptArgs for CreateTableArgs {
    fn to_cli_args(&self) -> Vec<String> {
        vec![
            "--device".to_string(),
            self.device.display().to_string(),
            "--action".to_string(),
            "create-table".to_string(),
            "--table-type".to_string(),
            self.table_type.as_str().to_string(),
        ]
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        if self.confirm {
            vec![("CONFIRM_MANUAL_PARTITION".to_string(), "yes".to_string())]
        } else {
            vec![]
        }
    }

    fn script_name(&self) -> &'static str {
        "manual_partition.sh"
    }

    fn is_destructive(&self) -> bool {
        true
    }
}

// ============================================================================
// Add Partition
// ============================================================================

/// Type-safe arguments for `scripts/tools/manual_partition.sh --action add-partition`.
///
/// # Field to Flag Mapping
///
/// | Rust Field       | CLI Flag    | Required |
/// |------------------|-------------|----------|
/// | `device`         | `--device`  | Yes      |
/// | `number`         | `--number`  | Yes      |
/// | `size`           | `--size`    | Yes      |
/// | `partition_type` | `--type`    | Yes      |
/// | `label`          | `--label`   | No       |
/// | `confirm`        | N/A (env)   | Sets `CONFIRM_MANUAL_PARTITION=yes` |
#[derive(Debug, Clone)]
pub struct AddPartitionArgs {
    /// Disk device path (e.g., `/dev/sda`).
    pub device: PathBuf,
    /// Partition number (1-128 for GPT, 1-4 for MBR primary).
    pub number: u8,
    /// Partition size (e.g., "512M", "50G", "remaining").
    pub size: String,
    /// Partition type code.
    pub partition_type: PartitionType,
    /// Optional partition label (GPT only).
    pub label: Option<String>,
    /// Whether to set `CONFIRM_MANUAL_PARTITION=yes`.
    pub confirm: bool,
}

impl ScriptArgs for AddPartitionArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec![
            "--device".to_string(),
            self.device.display().to_string(),
            "--action".to_string(),
            "add-partition".to_string(),
            "--number".to_string(),
            self.number.to_string(),
            "--size".to_string(),
            self.size.clone(),
            "--type".to_string(),
            self.partition_type.as_sgdisk_code().to_string(),
        ];
        if let Some(ref label) = self.label {
            args.push("--label".to_string());
            args.push(label.clone());
        }
        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        if self.confirm {
            vec![("CONFIRM_MANUAL_PARTITION".to_string(), "yes".to_string())]
        } else {
            vec![]
        }
    }

    fn script_name(&self) -> &'static str {
        "manual_partition.sh"
    }

    fn is_destructive(&self) -> bool {
        true
    }
}

// ============================================================================
// Delete Partition
// ============================================================================

/// Type-safe arguments for `scripts/tools/manual_partition.sh --action delete-partition`.
///
/// # Field to Flag Mapping
///
/// | Rust Field | CLI Flag    | Required |
/// |------------|-------------|----------|
/// | `device`   | `--device`  | Yes      |
/// | `number`   | `--number`  | Yes      |
/// | `confirm`  | N/A (env)   | Sets `CONFIRM_MANUAL_PARTITION=yes` |
#[derive(Debug, Clone)]
pub struct DeletePartitionArgs {
    /// Disk device path (e.g., `/dev/sda`).
    pub device: PathBuf,
    /// Partition number to delete.
    pub number: u8,
    /// Whether to set `CONFIRM_MANUAL_PARTITION=yes`.
    pub confirm: bool,
}

impl ScriptArgs for DeletePartitionArgs {
    fn to_cli_args(&self) -> Vec<String> {
        vec![
            "--device".to_string(),
            self.device.display().to_string(),
            "--action".to_string(),
            "delete-partition".to_string(),
            "--number".to_string(),
            self.number.to_string(),
        ]
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        if self.confirm {
            vec![("CONFIRM_MANUAL_PARTITION".to_string(), "yes".to_string())]
        } else {
            vec![]
        }
    }

    fn script_name(&self) -> &'static str {
        "manual_partition.sh"
    }

    fn is_destructive(&self) -> bool {
        true
    }
}

// ============================================================================
// Wipe Disk (existing)
// ============================================================================

/// Wipe method supported by wipe_disk.sh.
///
/// These map 1:1 to the bash script's `--method` argument.
/// Using an enum prevents typos like "zeros" instead of "zero".
///
/// # Bash Script Reference
///
/// From `scripts/tools/wipe_disk.sh`:
/// - `quick`: Remove partition table and filesystem signatures only (wipefs)
/// - `secure`: Full device wipe (blkdiscard for SSD, zeros for HDD)
/// - `auto`: Auto-detect device type and use appropriate secure wipe
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WipeMethod {
    /// Remove partition table and filesystem signatures only (wipefs).
    /// Fast, suitable for re-partitioning.
    Quick,

    /// Full device wipe appropriate for device type:
    /// - SSD: Uses blkdiscard (TRIM) - fast, preserves SSD lifespan
    /// - HDD: Overwrites with zeros - thorough for magnetic storage
    Secure,

    /// Auto-detect device type (SSD/HDD) and use appropriate secure wipe.
    Auto,
}

impl WipeMethod {
    /// Convert to the string expected by wipe_disk.sh.
    ///
    /// # Mapping
    ///
    /// | Enum Variant | String |
    /// |--------------|--------|
    /// | `Quick`      | `"quick"` |
    /// | `Secure`     | `"secure"` |
    /// | `Auto`       | `"auto"` |
    pub fn as_str(&self) -> &'static str {
        match self {
            WipeMethod::Quick => "quick",
            WipeMethod::Secure => "secure",
            WipeMethod::Auto => "auto",
        }
    }
}

impl std::fmt::Display for WipeMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for WipeMethod {
    type Err = WipeMethodError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "quick" => Ok(WipeMethod::Quick),
            "secure" => Ok(WipeMethod::Secure),
            "auto" => Ok(WipeMethod::Auto),
            _ => Err(WipeMethodError::InvalidMethod(s.to_string())),
        }
    }
}

/// Error for invalid wipe method strings.
#[derive(Debug, Clone)]
pub enum WipeMethodError {
    /// The provided string is not a valid wipe method.
    InvalidMethod(String),
}

impl std::fmt::Display for WipeMethodError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WipeMethodError::InvalidMethod(s) => {
                write!(f, "Invalid wipe method '{}'. Valid: quick, secure, auto", s)
            }
        }
    }
}

impl std::error::Error for WipeMethodError {}

/// Type-safe arguments for `scripts/tools/wipe_disk.sh`.
///
/// # Field to Flag Mapping
///
/// | Rust Field | CLI Flag     | Notes |
/// |------------|--------------|-------|
/// | `device`   | `--disk`     | NOT `--device` (bash script uses `--disk`) |
/// | `method`   | `--method`   | Valid: quick, secure, auto |
/// | `confirm`  | N/A (env)    | Sets `CONFIRM_WIPE_DISK=yes` |
///
/// # Environment Contract
///
/// The bash script requires `CONFIRM_WIPE_DISK=yes` to execute.
/// This is passed via `get_env_vars()` when `confirm` is true.
///
/// # Example
///
/// ```
/// use std::path::PathBuf;
/// use archtui::scripts::disk::{WipeDiskArgs, WipeMethod};
/// use archtui::script_traits::ScriptArgs;
///
/// let args = WipeDiskArgs {
///     device: PathBuf::from("/dev/sda"),
///     method: WipeMethod::Quick,
///     confirm: true,
/// };
///
/// assert_eq!(args.to_cli_args(), vec!["--disk", "/dev/sda", "--method", "quick"]);
/// assert_eq!(args.get_env_vars(), vec![("CONFIRM_WIPE_DISK".to_string(), "yes".to_string())]);
/// ```
#[derive(Debug, Clone)]
pub struct WipeDiskArgs {
    /// Target disk device path (e.g., `/dev/sda`).
    ///
    /// # CLI Mapping
    ///
    /// Maps to `--disk` flag, NOT `--device`.
    /// The bash script `wipe_disk.sh:254` parses `--disk`.
    pub device: PathBuf,

    /// Wipe method to use.
    ///
    /// # CLI Mapping
    ///
    /// Maps to `--method` flag with values: quick, secure, auto.
    pub method: WipeMethod,

    /// Whether to set `CONFIRM_WIPE_DISK=yes` environment variable.
    ///
    /// # Environment Contract
    ///
    /// The script refuses to run without `CONFIRM_WIPE_DISK=yes`.
    /// Setting this to `false` will cause the script to fail.
    pub confirm: bool,
}

impl ScriptArgs for WipeDiskArgs {
    /// Convert to CLI arguments for wipe_disk.sh.
    ///
    /// # Output Format
    ///
    /// `["--disk", "<device>", "--method", "<method>"]`
    ///
    /// # Critical Note
    ///
    /// Uses `--disk` NOT `--device`. This is the exact flag expected by
    /// `wipe_disk.sh` at line 254.
    fn to_cli_args(&self) -> Vec<String> {
        // CRITICAL: Use "--disk" NOT "--device"
        // wipe_disk.sh:254 expects "--disk"
        vec![
            "--disk".to_string(),
            self.device.display().to_string(),
            "--method".to_string(),
            self.method.as_str().to_string(),
        ]
    }

    /// Get environment variables required by wipe_disk.sh.
    ///
    /// # Returns
    ///
    /// - If `confirm` is true: `[("CONFIRM_WIPE_DISK", "yes")]`
    /// - If `confirm` is false: `[]`
    fn get_env_vars(&self) -> Vec<(String, String)> {
        if self.confirm {
            vec![("CONFIRM_WIPE_DISK".to_string(), "yes".to_string())]
        } else {
            vec![]
        }
    }

    /// Returns "wipe_disk.sh".
    fn script_name(&self) -> &'static str {
        "wipe_disk.sh"
    }

    /// Wipe is DESTRUCTIVE - erases entire disk.
    fn is_destructive(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wipe_disk_args_uses_disk_flag_not_device() {
        let args = WipeDiskArgs {
            device: PathBuf::from("/dev/sda"),
            method: WipeMethod::Quick,
            confirm: true,
        };

        let cli_args = args.to_cli_args();

        // CRITICAL: Must be "--disk" not "--device"
        assert_eq!(
            cli_args[0], "--disk",
            "First arg must be --disk, not --device"
        );
        assert_eq!(cli_args[1], "/dev/sda");
        assert_eq!(cli_args[2], "--method");
        assert_eq!(cli_args[3], "quick");
    }

    #[test]
    fn test_wipe_disk_args_confirm_sets_env_var() {
        let args = WipeDiskArgs {
            device: PathBuf::from("/dev/sda"),
            method: WipeMethod::Secure,
            confirm: true,
        };

        let env_vars = args.get_env_vars();
        assert_eq!(env_vars.len(), 1);
        assert_eq!(env_vars[0].0, "CONFIRM_WIPE_DISK");
        assert_eq!(env_vars[0].1, "yes");
    }

    #[test]
    fn test_wipe_disk_args_no_confirm_empty_env() {
        let args = WipeDiskArgs {
            device: PathBuf::from("/dev/sda"),
            method: WipeMethod::Quick,
            confirm: false,
        };

        let env_vars = args.get_env_vars();
        assert!(env_vars.is_empty(), "No env vars when confirm=false");
    }

    #[test]
    fn test_wipe_method_from_str() {
        assert_eq!(
            "quick".parse::<WipeMethod>().expect("should parse"),
            WipeMethod::Quick
        );
        assert_eq!(
            "secure".parse::<WipeMethod>().expect("should parse"),
            WipeMethod::Secure
        );
        assert_eq!(
            "auto".parse::<WipeMethod>().expect("should parse"),
            WipeMethod::Auto
        );
        // Case insensitive
        assert_eq!(
            "QUICK".parse::<WipeMethod>().expect("case insensitive"),
            WipeMethod::Quick
        );
    }

    #[test]
    fn test_wipe_method_invalid() {
        let result = "invalid".parse::<WipeMethod>();
        assert!(result.is_err());

        // Old invalid values that were in the TUI
        assert!("zero".parse::<WipeMethod>().is_err(), "zero is not valid");
        assert!(
            "random".parse::<WipeMethod>().is_err(),
            "random is not valid"
        );
    }

    #[test]
    fn test_script_name() {
        let args = WipeDiskArgs {
            device: PathBuf::from("/dev/sda"),
            method: WipeMethod::Quick,
            confirm: true,
        };
        assert_eq!(args.script_name(), "wipe_disk.sh");
    }

    #[test]
    fn test_nvme_device_path() {
        let args = WipeDiskArgs {
            device: PathBuf::from("/dev/nvme0n1"),
            method: WipeMethod::Auto,
            confirm: true,
        };

        let cli_args = args.to_cli_args();
        assert_eq!(cli_args[1], "/dev/nvme0n1");
    }

    // ========================================================================
    // FormatPartitionArgs Tests
    // ========================================================================

    #[test]
    fn test_format_partition_args_ext4() {
        let args = FormatPartitionArgs {
            device: PathBuf::from("/dev/sda2"),
            filesystem: Filesystem::Ext4,
            label: None,
            force: false,
        };

        let cli_args = args.to_cli_args();
        assert_eq!(cli_args[0], "--device");
        assert_eq!(cli_args[1], "/dev/sda2");
        assert_eq!(cli_args[2], "--filesystem");
        assert_eq!(cli_args[3], "ext4", "Filesystem::Ext4 must map to 'ext4'");
    }

    #[test]
    fn test_format_partition_args_fat32_for_efi() {
        let args = FormatPartitionArgs {
            device: PathBuf::from("/dev/sda1"),
            filesystem: Filesystem::Fat32,
            label: Some("EFI".to_string()),
            force: false,
        };

        let cli_args = args.to_cli_args();
        assert_eq!(
            cli_args[3], "fat32",
            "Filesystem::Fat32 must map to 'fat32'"
        );
        assert_eq!(cli_args[4], "--label");
        assert_eq!(cli_args[5], "EFI");
    }

    #[test]
    fn test_format_partition_args_btrfs_with_force() {
        let args = FormatPartitionArgs {
            device: PathBuf::from("/dev/sda2"),
            filesystem: Filesystem::Btrfs,
            label: Some("archroot".to_string()),
            force: true,
        };

        let cli_args = args.to_cli_args();
        assert_eq!(cli_args[3], "btrfs");
        assert!(cli_args.contains(&"--force".to_string()));
        assert!(cli_args.contains(&"--label".to_string()));
    }

    #[test]
    fn test_format_partition_script_name() {
        let args = FormatPartitionArgs {
            device: PathBuf::from("/dev/sda1"),
            filesystem: Filesystem::Ext4,
            label: None,
            force: false,
        };
        assert_eq!(args.script_name(), "format_partition.sh");
    }

    // ========================================================================
    // MountPartitionArgs Tests
    // ========================================================================

    #[test]
    fn test_mount_partition_args_root() {
        let args = MountPartitionArgs {
            device: PathBuf::from("/dev/sda2"),
            mountpoint: PathBuf::from("/mnt"),
            options: None,
        };

        let cli_args = args.to_cli_args();
        assert_eq!(cli_args[0], "--action");
        assert_eq!(cli_args[1], "mount");
        assert_eq!(cli_args[2], "--device");
        assert_eq!(cli_args[3], "/dev/sda2");
        assert_eq!(cli_args[4], "--mountpoint");
        assert_eq!(cli_args[5], "/mnt");
    }

    #[test]
    fn test_mount_partition_args_boot() {
        let args = MountPartitionArgs {
            device: PathBuf::from("/dev/sda1"),
            mountpoint: PathBuf::from("/mnt/boot"),
            options: None,
        };

        let cli_args = args.to_cli_args();
        assert_eq!(cli_args[5], "/mnt/boot");
    }

    #[test]
    fn test_mount_partition_args_with_options() {
        let args = MountPartitionArgs {
            device: PathBuf::from("/dev/sda2"),
            mountpoint: PathBuf::from("/mnt"),
            options: Some("noatime,compress=zstd".to_string()),
        };

        let cli_args = args.to_cli_args();
        assert!(cli_args.contains(&"--options".to_string()));
        assert!(cli_args.contains(&"noatime,compress=zstd".to_string()));
    }

    #[test]
    fn test_mount_partition_script_name() {
        let args = MountPartitionArgs {
            device: PathBuf::from("/dev/sda2"),
            mountpoint: PathBuf::from("/mnt"),
            options: None,
        };
        assert_eq!(args.script_name(), "mount_partitions.sh");
    }

    // ========================================================================
    // TableType Tests
    // ========================================================================

    #[test]
    fn test_table_type_from_str() {
        assert_eq!("gpt".parse::<TableType>().unwrap(), TableType::Gpt);
        assert_eq!("GPT".parse::<TableType>().unwrap(), TableType::Gpt);
        assert_eq!("mbr".parse::<TableType>().unwrap(), TableType::Mbr);
        assert_eq!("MBR".parse::<TableType>().unwrap(), TableType::Mbr);
        assert_eq!("dos".parse::<TableType>().unwrap(), TableType::Mbr);
        assert_eq!("msdos".parse::<TableType>().unwrap(), TableType::Mbr);
    }

    #[test]
    fn test_table_type_from_str_invalid() {
        assert!("invalid".parse::<TableType>().is_err());
    }

    #[test]
    fn test_table_type_display() {
        assert_eq!(TableType::Gpt.to_string(), "gpt");
        assert_eq!(TableType::Mbr.to_string(), "mbr");
    }

    // ========================================================================
    // PartitionType Tests
    // ========================================================================

    #[test]
    fn test_partition_type_sgdisk_codes() {
        assert_eq!(PartitionType::Efi.as_sgdisk_code(), "EF00");
        assert_eq!(PartitionType::BiosBoot.as_sgdisk_code(), "EF02");
        assert_eq!(PartitionType::Linux.as_sgdisk_code(), "8300");
        assert_eq!(PartitionType::Swap.as_sgdisk_code(), "8200");
        assert_eq!(PartitionType::Lvm.as_sgdisk_code(), "8E00");
        assert_eq!(PartitionType::Luks.as_sgdisk_code(), "8309");
    }

    #[test]
    fn test_partition_type_mbr_ids() {
        assert_eq!(PartitionType::Efi.as_mbr_id(), "ef");
        assert_eq!(PartitionType::Linux.as_mbr_id(), "83");
        assert_eq!(PartitionType::Swap.as_mbr_id(), "82");
        assert_eq!(PartitionType::Lvm.as_mbr_id(), "8e");
    }

    #[test]
    fn test_partition_type_from_str() {
        assert_eq!("EF00".parse::<PartitionType>().unwrap(), PartitionType::Efi);
        assert_eq!("EFI".parse::<PartitionType>().unwrap(), PartitionType::Efi);
        assert_eq!(
            "EFI System".parse::<PartitionType>().unwrap(),
            PartitionType::Efi
        );
        assert_eq!(
            "8300".parse::<PartitionType>().unwrap(),
            PartitionType::Linux
        );
        assert_eq!(
            "Linux".parse::<PartitionType>().unwrap(),
            PartitionType::Linux
        );
        assert_eq!(
            "Swap".parse::<PartitionType>().unwrap(),
            PartitionType::Swap
        );
        assert_eq!("LVM".parse::<PartitionType>().unwrap(), PartitionType::Lvm);
        assert_eq!(
            "LUKS".parse::<PartitionType>().unwrap(),
            PartitionType::Luks
        );
        assert_eq!(
            "BIOS Boot".parse::<PartitionType>().unwrap(),
            PartitionType::BiosBoot
        );
    }

    #[test]
    fn test_partition_type_from_str_invalid() {
        assert!("invalid".parse::<PartitionType>().is_err());
    }

    #[test]
    fn test_partition_type_display_name() {
        assert_eq!(PartitionType::Efi.display_name(), "EFI System");
        assert_eq!(PartitionType::BiosBoot.display_name(), "BIOS Boot");
        assert_eq!(PartitionType::Linux.display_name(), "Linux");
        assert_eq!(PartitionType::Swap.display_name(), "Linux Swap");
        assert_eq!(PartitionType::Lvm.display_name(), "Linux LVM");
        assert_eq!(PartitionType::Luks.display_name(), "Linux LUKS");
    }

    // ========================================================================
    // ManualPartitionArgs Tests (cfdisk mode)
    // ========================================================================

    #[test]
    fn test_manual_partition_args_includes_action_cfdisk() {
        let args = ManualPartitionArgs {
            device: PathBuf::from("/dev/sda"),
        };
        let cli = args.to_cli_args();
        assert_eq!(cli, vec!["--device", "/dev/sda", "--action", "cfdisk"]);
    }

    #[test]
    fn test_manual_partition_args_sets_confirm_env() {
        let args = ManualPartitionArgs {
            device: PathBuf::from("/dev/sda"),
        };
        let env = args.get_env_vars();
        assert_eq!(env.len(), 1);
        assert_eq!(env[0].0, "CONFIRM_MANUAL_PARTITION");
        assert_eq!(env[0].1, "yes");
    }

    // ========================================================================
    // CreateTableArgs Tests
    // ========================================================================

    #[test]
    fn test_create_table_args_gpt() {
        let args = CreateTableArgs {
            device: PathBuf::from("/dev/sda"),
            table_type: TableType::Gpt,
            confirm: true,
        };
        let cli = args.to_cli_args();
        assert_eq!(
            cli,
            vec![
                "--device",
                "/dev/sda",
                "--action",
                "create-table",
                "--table-type",
                "gpt"
            ]
        );
        let env = args.get_env_vars();
        assert_eq!(env[0].0, "CONFIRM_MANUAL_PARTITION");
    }

    #[test]
    fn test_create_table_args_mbr() {
        let args = CreateTableArgs {
            device: PathBuf::from("/dev/sda"),
            table_type: TableType::Mbr,
            confirm: false,
        };
        let cli = args.to_cli_args();
        assert!(cli.contains(&"mbr".to_string()));
        assert!(args.get_env_vars().is_empty());
    }

    #[test]
    fn test_create_table_script_name() {
        let args = CreateTableArgs {
            device: PathBuf::from("/dev/sda"),
            table_type: TableType::Gpt,
            confirm: true,
        };
        assert_eq!(args.script_name(), "manual_partition.sh");
        assert!(args.is_destructive());
    }

    // ========================================================================
    // AddPartitionArgs Tests
    // ========================================================================

    #[test]
    fn test_add_partition_args_efi() {
        let args = AddPartitionArgs {
            device: PathBuf::from("/dev/sda"),
            number: 1,
            size: "512M".to_string(),
            partition_type: PartitionType::Efi,
            label: Some("EFI".to_string()),
            confirm: true,
        };
        let cli = args.to_cli_args();
        assert_eq!(cli[0], "--device");
        assert_eq!(cli[1], "/dev/sda");
        assert_eq!(cli[2], "--action");
        assert_eq!(cli[3], "add-partition");
        assert_eq!(cli[4], "--number");
        assert_eq!(cli[5], "1");
        assert_eq!(cli[6], "--size");
        assert_eq!(cli[7], "512M");
        assert_eq!(cli[8], "--type");
        assert_eq!(cli[9], "EF00");
        assert_eq!(cli[10], "--label");
        assert_eq!(cli[11], "EFI");
    }

    #[test]
    fn test_add_partition_args_no_label() {
        let args = AddPartitionArgs {
            device: PathBuf::from("/dev/nvme0n1"),
            number: 2,
            size: "50G".to_string(),
            partition_type: PartitionType::Linux,
            label: None,
            confirm: true,
        };
        let cli = args.to_cli_args();
        assert_eq!(cli.len(), 10); // No --label pair
        assert_eq!(cli[1], "/dev/nvme0n1");
        assert_eq!(cli[9], "8300");
    }

    #[test]
    fn test_add_partition_script_name() {
        let args = AddPartitionArgs {
            device: PathBuf::from("/dev/sda"),
            number: 1,
            size: "512M".to_string(),
            partition_type: PartitionType::Efi,
            label: None,
            confirm: true,
        };
        assert_eq!(args.script_name(), "manual_partition.sh");
        assert!(args.is_destructive());
    }

    // ========================================================================
    // DeletePartitionArgs Tests
    // ========================================================================

    #[test]
    fn test_delete_partition_args() {
        let args = DeletePartitionArgs {
            device: PathBuf::from("/dev/sda"),
            number: 3,
            confirm: true,
        };
        let cli = args.to_cli_args();
        assert_eq!(
            cli,
            vec![
                "--device",
                "/dev/sda",
                "--action",
                "delete-partition",
                "--number",
                "3"
            ]
        );
        let env = args.get_env_vars();
        assert_eq!(env[0].0, "CONFIRM_MANUAL_PARTITION");
    }

    #[test]
    fn test_delete_partition_no_confirm() {
        let args = DeletePartitionArgs {
            device: PathBuf::from("/dev/sda"),
            number: 1,
            confirm: false,
        };
        assert!(args.get_env_vars().is_empty());
    }

    #[test]
    fn test_delete_partition_script_name() {
        let args = DeletePartitionArgs {
            device: PathBuf::from("/dev/sda"),
            number: 1,
            confirm: true,
        };
        assert_eq!(args.script_name(), "manual_partition.sh");
        assert!(args.is_destructive());
    }
}
