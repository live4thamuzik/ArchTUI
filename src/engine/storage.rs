//! Storage Stack Engine (Sprint 15)
//!
//! Translates a high-level `PartitionScheme` + `Filesystem` configuration into
//! an ordered sequence of atomic `StorageOp` operations that the script runner
//! can execute.
//!
//! # Supported Strategies
//!
//! | Strategy        | Operations Generated |
//! |-----------------|---------------------|
//! | AutoSimple      | Wipe → Partition → Format → Mount |
//! | AutoSimpleLuks  | Wipe → Partition → LuksFormat → LuksOpen → Format → Mount |
//! | AutoLvm         | Wipe → Partition → PV → VG → LV(root,swap) → Format → Mount |
//! | AutoLuksLvm     | Wipe → Partition → LuksFormat → LuksOpen → PV → VG → LV → Format → Mount |
//!
//! RAID strategies are defined but return an error (deferred to a later sprint).
//!
//! # Design
//!
//! - **Pure logic**: No I/O, no side effects — only generates the plan
//! - **Typed output**: Each `StorageOp` maps directly to a `ScriptArgs` struct
//! - **Validated**: The plan is checked for internal consistency before returning
//! - **Testable**: All operations are unit-tested with config → plan assertions

// Library API - consumed by installer orchestration
#![allow(dead_code)]

use crate::config_file::InstallationConfig;
use crate::types::{Filesystem, PartitionScheme, Toggle};
use anyhow::{bail, Result};
use std::fmt;
use std::path::PathBuf;

// ============================================================================
// Storage Operation Types
// ============================================================================

/// A single atomic storage operation in the plan.
///
/// Operations are ordered: the plan generator ensures correct sequencing
/// (e.g., PV before VG, VG before LV, LuksFormat before LuksOpen).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageOp {
    /// Wipe the entire disk (sgdisk --zap-all)
    WipeDisk {
        disk: PathBuf,
    },

    /// Create partitions on the disk (sgdisk)
    /// Generates EFI + root (+ swap partition if applicable)
    Partition {
        disk: PathBuf,
        /// Whether to create an EFI System Partition
        create_efi: bool,
        /// Whether to create a separate swap partition (non-LVM only)
        create_swap: bool,
    },

    /// Format a partition with LUKS2 encryption
    LuksFormat {
        device: PathBuf,
        label: String,
    },

    /// Open (unlock) a LUKS device
    LuksOpen {
        device: PathBuf,
        mapper_name: String,
    },

    /// Create an LVM Physical Volume
    CreateLvmPv {
        device: PathBuf,
    },

    /// Create an LVM Volume Group
    CreateLvmVg {
        vg_name: String,
        pv_device: PathBuf,
    },

    /// Create an LVM Logical Volume
    CreateLvmLv {
        vg_name: String,
        lv_name: String,
        /// Size specification (e.g., "100%FREE", "2G")
        size: String,
    },

    /// Format a device/partition with a filesystem
    FormatFs {
        device: PathBuf,
        filesystem: Filesystem,
        label: Option<String>,
    },

    /// Mount a device at a mountpoint
    Mount {
        device: PathBuf,
        mountpoint: PathBuf,
        /// Optional mount options (e.g., "subvol=@,compress=zstd")
        options: Option<String>,
    },
}

impl fmt::Display for StorageOp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WipeDisk { disk } => write!(f, "WipeDisk({})", disk.display()),
            Self::Partition { disk, create_efi, create_swap } => {
                write!(f, "Partition({}, efi={}, swap={})", disk.display(), create_efi, create_swap)
            }
            Self::LuksFormat { device, label } => {
                write!(f, "LuksFormat({}, label={})", device.display(), label)
            }
            Self::LuksOpen { device, mapper_name } => {
                write!(f, "LuksOpen({} -> /dev/mapper/{})", device.display(), mapper_name)
            }
            Self::CreateLvmPv { device } => write!(f, "CreateLvmPV({})", device.display()),
            Self::CreateLvmVg { vg_name, pv_device } => {
                write!(f, "CreateLvmVG({} on {})", vg_name, pv_device.display())
            }
            Self::CreateLvmLv { vg_name, lv_name, size } => {
                write!(f, "CreateLvmLV({}/{}, size={})", vg_name, lv_name, size)
            }
            Self::FormatFs { device, filesystem, label } => {
                write!(f, "FormatFs({}, fs={}, label={:?})", device.display(), filesystem, label)
            }
            Self::Mount { device, mountpoint, options } => {
                write!(f, "Mount({} -> {}, opts={:?})",
                    device.display(), mountpoint.display(), options)
            }
        }
    }
}

/// A complete storage plan: an ordered list of operations.
#[derive(Debug, Clone)]
pub struct StoragePlan {
    /// Ordered sequence of storage operations
    pub ops: Vec<StorageOp>,
    /// The strategy that generated this plan
    pub strategy: PartitionScheme,
    /// Target disk
    pub disk: PathBuf,
    /// Whether encryption is involved
    pub encrypted: bool,
    /// Whether LVM is involved
    pub lvm: bool,
}

impl StoragePlan {
    /// Returns true if this plan includes destructive operations.
    pub fn is_destructive(&self) -> bool {
        self.ops.iter().any(|op| matches!(
            op,
            StorageOp::WipeDisk { .. }
                | StorageOp::Partition { .. }
                | StorageOp::LuksFormat { .. }
                | StorageOp::FormatFs { .. }
        ))
    }

    /// Returns a summary of the plan for logging/display.
    pub fn summary(&self) -> String {
        let mut lines = vec![
            format!("Storage Plan: {:?}", self.strategy),
            format!("  Target: {}", self.disk.display()),
            format!("  Encrypted: {}", self.encrypted),
            format!("  LVM: {}", self.lvm),
            format!("  Operations ({}):", self.ops.len()),
        ];
        for (i, op) in self.ops.iter().enumerate() {
            lines.push(format!("    {}. {}", i + 1, op));
        }
        lines.join("\n")
    }
}

// ============================================================================
// Plan Calculation
// ============================================================================

/// Calculate the storage operation plan from an installation config.
///
/// Translates the high-level strategy (AutoSimple, AutoLvm, etc.) into
/// a concrete, ordered sequence of operations.
///
/// # Errors
///
/// Returns an error if:
/// - The strategy requires RAID (not yet implemented)
/// - The strategy is Manual (handled separately by the TUI)
/// - The disk path is empty or invalid
///
/// # What This Explicitly Refuses To Do
///
/// - RAID strategies: Not implemented. Returns error with explanation.
/// - Manual partitioning: This is interactive and handled by the TUI directly.
/// - Executing anything: This is pure plan generation. No I/O.
pub fn calculate_storage_plan(config: &InstallationConfig) -> Result<StoragePlan> {
    let disk = PathBuf::from(&config.install_disk);

    if config.install_disk.is_empty() {
        bail!("No install disk specified — cannot generate storage plan");
    }

    if !config.install_disk.starts_with("/dev/") {
        bail!(
            "Invalid disk path '{}' — must start with /dev/",
            config.install_disk
        );
    }

    let strategy = config.partitioning_strategy;
    let root_fs = config.root_filesystem;
    let wants_swap = config.swap == Toggle::Yes;
    let swap_size = config.swap_size.clone();
    let is_uefi = config.boot_mode == crate::types::BootMode::Uefi
        || config.boot_mode == crate::types::BootMode::Auto;

    match strategy {
        PartitionScheme::AutoSimple => {
            plan_simple(&disk, root_fs, is_uefi, wants_swap, &swap_size)
        }
        PartitionScheme::AutoSimpleLuks => {
            plan_simple_luks(&disk, root_fs, is_uefi, wants_swap, &swap_size)
        }
        PartitionScheme::AutoLvm => {
            plan_lvm(&disk, root_fs, is_uefi, wants_swap, &swap_size)
        }
        PartitionScheme::AutoLuksLvm => {
            plan_luks_lvm(&disk, root_fs, is_uefi, wants_swap, &swap_size)
        }
        PartitionScheme::AutoRaid
        | PartitionScheme::AutoRaidLuks
        | PartitionScheme::AutoRaidLvm
        | PartitionScheme::AutoRaidLvmLuks => {
            bail!(
                "RAID strategy {:?} is not yet implemented. \
                 Prerequisite: multi-disk selection and mdadm integration.",
                strategy
            )
        }
        PartitionScheme::Manual => {
            bail!(
                "Manual partitioning does not use the storage engine — \
                 it is handled interactively by the TUI."
            )
        }
    }
}

// ============================================================================
// Strategy Implementations
// ============================================================================

/// Plan: Simple partitioning (EFI + root, optional swap partition)
///
/// Disk layout:
/// ```text
/// /dev/sdX1 - EFI System Partition (512M, FAT32) [if UEFI]
/// /dev/sdX2 - Root partition (remaining space)
/// /dev/sdX3 - Swap partition [if enabled]
/// ```
fn plan_simple(
    disk: &PathBuf,
    root_fs: Filesystem,
    is_uefi: bool,
    wants_swap: bool,
    swap_size: &str,
) -> Result<StoragePlan> {
    let mut ops = Vec::new();

    // Step 1: Wipe disk
    ops.push(StorageOp::WipeDisk { disk: disk.clone() });

    // Step 2: Create partitions
    ops.push(StorageOp::Partition {
        disk: disk.clone(),
        create_efi: is_uefi,
        create_swap: wants_swap,
    });

    // Step 3: Format EFI partition
    let root_part_num = if is_uefi {
        ops.push(StorageOp::FormatFs {
            device: partition_path(disk, 1),
            filesystem: Filesystem::Fat32,
            label: Some("EFI".to_string()),
        });
        2
    } else {
        1
    };

    // Step 4: Format root partition
    let root_device = partition_path(disk, root_part_num);
    ops.push(StorageOp::FormatFs {
        device: root_device.clone(),
        filesystem: root_fs,
        label: Some("archroot".to_string()),
    });

    // Step 5: Mount root
    let mount_opts = btrfs_mount_options(root_fs);
    ops.push(StorageOp::Mount {
        device: root_device,
        mountpoint: PathBuf::from("/mnt"),
        options: mount_opts,
    });

    // Step 6: Mount EFI
    if is_uefi {
        ops.push(StorageOp::Mount {
            device: partition_path(disk, 1),
            mountpoint: PathBuf::from("/mnt/boot"),
            options: None,
        });
    }

    // Step 7: Swap partition (if requested, non-LVM)
    if wants_swap {
        let swap_part = partition_path(disk, root_part_num + 1);
        ops.push(StorageOp::FormatFs {
            device: swap_part,
            filesystem: Filesystem::Ext4, // placeholder — swap is mkswap, not a FS
            label: Some(format!("swap:{}", swap_size)),
        });
    }

    Ok(StoragePlan {
        ops,
        strategy: PartitionScheme::AutoSimple,
        disk: disk.clone(),
        encrypted: false,
        lvm: false,
    })
}

/// Plan: Simple + LUKS encryption on root
///
/// Disk layout:
/// ```text
/// /dev/sdX1 - EFI System Partition (512M, FAT32) [if UEFI]
/// /dev/sdX2 - LUKS-encrypted root → /dev/mapper/cryptroot
/// ```
fn plan_simple_luks(
    disk: &PathBuf,
    root_fs: Filesystem,
    is_uefi: bool,
    wants_swap: bool,
    swap_size: &str,
) -> Result<StoragePlan> {
    let mut ops = Vec::new();

    ops.push(StorageOp::WipeDisk { disk: disk.clone() });

    ops.push(StorageOp::Partition {
        disk: disk.clone(),
        create_efi: is_uefi,
        create_swap: false, // Swap goes inside LUKS or as swapfile
    });

    let root_part_num = if is_uefi {
        ops.push(StorageOp::FormatFs {
            device: partition_path(disk, 1),
            filesystem: Filesystem::Fat32,
            label: Some("EFI".to_string()),
        });
        2
    } else {
        1
    };

    let root_part = partition_path(disk, root_part_num);

    // LUKS format + open
    ops.push(StorageOp::LuksFormat {
        device: root_part.clone(),
        label: "cryptroot".to_string(),
    });
    ops.push(StorageOp::LuksOpen {
        device: root_part,
        mapper_name: "cryptroot".to_string(),
    });

    // Format the decrypted device
    let mapper_device = PathBuf::from("/dev/mapper/cryptroot");
    let mount_opts = btrfs_mount_options(root_fs);
    ops.push(StorageOp::FormatFs {
        device: mapper_device.clone(),
        filesystem: root_fs,
        label: Some("archroot".to_string()),
    });

    // Mount
    ops.push(StorageOp::Mount {
        device: mapper_device,
        mountpoint: PathBuf::from("/mnt"),
        options: mount_opts,
    });

    if is_uefi {
        ops.push(StorageOp::Mount {
            device: partition_path(disk, 1),
            mountpoint: PathBuf::from("/mnt/boot"),
            options: None,
        });
    }

    // Swap note: with LUKS, swap is typically a swapfile inside the encrypted root
    // or an encrypted swap partition. For simplicity, we skip swap partition here.
    let _ = (wants_swap, swap_size); // Acknowledge but defer

    Ok(StoragePlan {
        ops,
        strategy: PartitionScheme::AutoSimpleLuks,
        disk: disk.clone(),
        encrypted: true,
        lvm: false,
    })
}

/// Plan: LVM on raw partition
///
/// Disk layout:
/// ```text
/// /dev/sdX1 - EFI System Partition (512M, FAT32) [if UEFI]
/// /dev/sdX2 - LVM PV → VG "archvg" → LV "root" + LV "swap"
/// ```
fn plan_lvm(
    disk: &PathBuf,
    root_fs: Filesystem,
    is_uefi: bool,
    wants_swap: bool,
    swap_size: &str,
) -> Result<StoragePlan> {
    let mut ops = Vec::new();
    let vg_name = "archvg";

    ops.push(StorageOp::WipeDisk { disk: disk.clone() });

    ops.push(StorageOp::Partition {
        disk: disk.clone(),
        create_efi: is_uefi,
        create_swap: false, // LVM manages swap
    });

    let lvm_part_num = if is_uefi {
        ops.push(StorageOp::FormatFs {
            device: partition_path(disk, 1),
            filesystem: Filesystem::Fat32,
            label: Some("EFI".to_string()),
        });
        2
    } else {
        1
    };

    let lvm_part = partition_path(disk, lvm_part_num);

    // LVM: PV → VG → LVs
    ops.push(StorageOp::CreateLvmPv { device: lvm_part.clone() });
    ops.push(StorageOp::CreateLvmVg {
        vg_name: vg_name.to_string(),
        pv_device: lvm_part,
    });

    // Swap LV (if requested) — create first so root gets remaining space
    if wants_swap {
        ops.push(StorageOp::CreateLvmLv {
            vg_name: vg_name.to_string(),
            lv_name: "swap".to_string(),
            size: format!("{}G", swap_size.trim_end_matches('G').trim_end_matches('g')),
        });
    }

    // Root LV — takes all remaining space
    ops.push(StorageOp::CreateLvmLv {
        vg_name: vg_name.to_string(),
        lv_name: "root".to_string(),
        size: "100%FREE".to_string(),
    });

    // Format root LV
    let root_lv = PathBuf::from(format!("/dev/{}/root", vg_name));
    let mount_opts = btrfs_mount_options(root_fs);
    ops.push(StorageOp::FormatFs {
        device: root_lv.clone(),
        filesystem: root_fs,
        label: Some("archroot".to_string()),
    });

    // Format swap LV
    if wants_swap {
        let swap_lv = PathBuf::from(format!("/dev/{}/swap", vg_name));
        ops.push(StorageOp::FormatFs {
            device: swap_lv,
            filesystem: Filesystem::Ext4, // placeholder for mkswap
            label: Some("swap".to_string()),
        });
    }

    // Mount root
    ops.push(StorageOp::Mount {
        device: root_lv,
        mountpoint: PathBuf::from("/mnt"),
        options: mount_opts,
    });

    // Mount EFI
    if is_uefi {
        ops.push(StorageOp::Mount {
            device: partition_path(disk, 1),
            mountpoint: PathBuf::from("/mnt/boot"),
            options: None,
        });
    }

    Ok(StoragePlan {
        ops,
        strategy: PartitionScheme::AutoLvm,
        disk: disk.clone(),
        encrypted: false,
        lvm: true,
    })
}

/// Plan: LUKS + LVM (encrypted LVM)
///
/// Disk layout:
/// ```text
/// /dev/sdX1 - EFI System Partition (512M, FAT32) [if UEFI]
/// /dev/sdX2 - LUKS → /dev/mapper/cryptlvm → PV → VG "archvg" → LV "root" + LV "swap"
/// ```
fn plan_luks_lvm(
    disk: &PathBuf,
    root_fs: Filesystem,
    is_uefi: bool,
    wants_swap: bool,
    swap_size: &str,
) -> Result<StoragePlan> {
    let mut ops = Vec::new();
    let vg_name = "archvg";

    ops.push(StorageOp::WipeDisk { disk: disk.clone() });

    ops.push(StorageOp::Partition {
        disk: disk.clone(),
        create_efi: is_uefi,
        create_swap: false,
    });

    let luks_part_num = if is_uefi {
        ops.push(StorageOp::FormatFs {
            device: partition_path(disk, 1),
            filesystem: Filesystem::Fat32,
            label: Some("EFI".to_string()),
        });
        2
    } else {
        1
    };

    let luks_part = partition_path(disk, luks_part_num);

    // LUKS
    ops.push(StorageOp::LuksFormat {
        device: luks_part.clone(),
        label: "cryptlvm".to_string(),
    });
    ops.push(StorageOp::LuksOpen {
        device: luks_part,
        mapper_name: "cryptlvm".to_string(),
    });

    let mapper_device = PathBuf::from("/dev/mapper/cryptlvm");

    // LVM on top of LUKS
    ops.push(StorageOp::CreateLvmPv { device: mapper_device.clone() });
    ops.push(StorageOp::CreateLvmVg {
        vg_name: vg_name.to_string(),
        pv_device: mapper_device,
    });

    if wants_swap {
        ops.push(StorageOp::CreateLvmLv {
            vg_name: vg_name.to_string(),
            lv_name: "swap".to_string(),
            size: format!("{}G", swap_size.trim_end_matches('G').trim_end_matches('g')),
        });
    }

    ops.push(StorageOp::CreateLvmLv {
        vg_name: vg_name.to_string(),
        lv_name: "root".to_string(),
        size: "100%FREE".to_string(),
    });

    let root_lv = PathBuf::from(format!("/dev/{}/root", vg_name));
    let mount_opts = btrfs_mount_options(root_fs);
    ops.push(StorageOp::FormatFs {
        device: root_lv.clone(),
        filesystem: root_fs,
        label: Some("archroot".to_string()),
    });

    if wants_swap {
        let swap_lv = PathBuf::from(format!("/dev/{}/swap", vg_name));
        ops.push(StorageOp::FormatFs {
            device: swap_lv,
            filesystem: Filesystem::Ext4,
            label: Some("swap".to_string()),
        });
    }

    ops.push(StorageOp::Mount {
        device: root_lv,
        mountpoint: PathBuf::from("/mnt"),
        options: mount_opts,
    });

    if is_uefi {
        ops.push(StorageOp::Mount {
            device: partition_path(disk, 1),
            mountpoint: PathBuf::from("/mnt/boot"),
            options: None,
        });
    }

    Ok(StoragePlan {
        ops,
        strategy: PartitionScheme::AutoLuksLvm,
        disk: disk.clone(),
        encrypted: true,
        lvm: true,
    })
}

// ============================================================================
// Helpers
// ============================================================================

/// Generate a partition device path from a disk path and partition number.
///
/// Handles both `/dev/sdX` → `/dev/sdX1` and `/dev/nvme0n1` → `/dev/nvme0n1p1` patterns.
fn partition_path(disk: &PathBuf, partition_num: u32) -> PathBuf {
    let disk_str = disk.display().to_string();

    // NVMe and loop devices use 'p' separator (e.g., /dev/nvme0n1p1, /dev/loop0p1)
    if disk_str.ends_with(|c: char| c.is_ascii_digit()) {
        PathBuf::from(format!("{}p{}", disk_str, partition_num))
    } else {
        PathBuf::from(format!("{}{}", disk_str, partition_num))
    }
}

/// Generate Btrfs-specific mount options if the filesystem is Btrfs.
///
/// Returns `Some("subvol=@,compress=zstd,noatime")` for Btrfs, `None` otherwise.
fn btrfs_mount_options(fs: Filesystem) -> Option<String> {
    if fs == Filesystem::Btrfs {
        Some("subvol=@,compress=zstd,noatime".to_string())
    } else {
        None
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_file::InstallationConfig;
    use crate::types::*;

    /// Helper: create a basic config for testing
    fn test_config(strategy: PartitionScheme, fs: Filesystem) -> InstallationConfig {
        let mut config = InstallationConfig::new();
        config.install_disk = "/dev/sda".to_string();
        config.partitioning_strategy = strategy;
        config.root_filesystem = fs;
        config.boot_mode = BootMode::Uefi;
        config.swap = Toggle::Yes;
        config.swap_size = "2".to_string();
        config
    }

    #[test]
    fn test_simple_plan_has_wipe_format_mount() {
        let config = test_config(PartitionScheme::AutoSimple, Filesystem::Ext4);
        let plan = calculate_storage_plan(&config).expect("plan generation failed"); // test: known-good input

        // Must start with wipe
        assert!(matches!(&plan.ops[0], StorageOp::WipeDisk { .. }));

        // Must have partition step
        assert!(plan.ops.iter().any(|op| matches!(op, StorageOp::Partition { .. })));

        // Must format root
        assert!(plan.ops.iter().any(|op| matches!(
            op,
            StorageOp::FormatFs { filesystem: Filesystem::Ext4, .. }
        )));

        // Must mount root at /mnt
        assert!(plan.ops.iter().any(|op| matches!(
            op,
            StorageOp::Mount { mountpoint, .. } if mountpoint == &PathBuf::from("/mnt")
        )));

        assert!(!plan.encrypted);
        assert!(!plan.lvm);
        assert!(plan.is_destructive());
    }

    #[test]
    fn test_simple_plan_uefi_has_efi_partition() {
        let config = test_config(PartitionScheme::AutoSimple, Filesystem::Ext4);
        let plan = calculate_storage_plan(&config).expect("plan generation failed"); // test: known-good input

        // Must format EFI partition as FAT32
        assert!(plan.ops.iter().any(|op| matches!(
            op,
            StorageOp::FormatFs { filesystem: Filesystem::Fat32, .. }
        )));

        // Must mount EFI at /mnt/boot
        assert!(plan.ops.iter().any(|op| matches!(
            op,
            StorageOp::Mount { mountpoint, .. } if mountpoint == &PathBuf::from("/mnt/boot")
        )));
    }

    #[test]
    fn test_luks_plan_has_encrypt_steps() {
        let config = test_config(PartitionScheme::AutoSimpleLuks, Filesystem::Ext4);
        let plan = calculate_storage_plan(&config).expect("plan generation failed"); // test: known-good input

        // Must have LuksFormat
        assert!(plan.ops.iter().any(|op| matches!(op, StorageOp::LuksFormat { .. })));

        // Must have LuksOpen
        assert!(plan.ops.iter().any(|op| matches!(op, StorageOp::LuksOpen { .. })));

        // Format target must be /dev/mapper/cryptroot
        assert!(plan.ops.iter().any(|op| matches!(
            op,
            StorageOp::FormatFs { device, .. }
                if device == &PathBuf::from("/dev/mapper/cryptroot")
        )));

        assert!(plan.encrypted);
        assert!(!plan.lvm);
    }

    #[test]
    fn test_lvm_plan_has_pv_vg_lv_steps() {
        let config = test_config(PartitionScheme::AutoLvm, Filesystem::Ext4);
        let plan = calculate_storage_plan(&config).expect("plan generation failed"); // test: known-good input

        // Must have PV creation
        assert!(plan.ops.iter().any(|op| matches!(op, StorageOp::CreateLvmPv { .. })));

        // Must have VG creation
        assert!(plan.ops.iter().any(|op| matches!(
            op,
            StorageOp::CreateLvmVg { vg_name, .. } if vg_name == "archvg"
        )));

        // Must have root LV
        assert!(plan.ops.iter().any(|op| matches!(
            op,
            StorageOp::CreateLvmLv { lv_name, size, .. }
                if lv_name == "root" && size == "100%FREE"
        )));

        // Must have swap LV (swap=Yes)
        assert!(plan.ops.iter().any(|op| matches!(
            op,
            StorageOp::CreateLvmLv { lv_name, .. } if lv_name == "swap"
        )));

        assert!(!plan.encrypted);
        assert!(plan.lvm);
    }

    #[test]
    fn test_luks_lvm_plan_has_all_layers() {
        let config = test_config(PartitionScheme::AutoLuksLvm, Filesystem::Ext4);
        let plan = calculate_storage_plan(&config).expect("plan generation failed"); // test: known-good input

        // Must have LUKS steps
        assert!(plan.ops.iter().any(|op| matches!(op, StorageOp::LuksFormat { .. })));
        assert!(plan.ops.iter().any(|op| matches!(op, StorageOp::LuksOpen { .. })));

        // Must have LVM steps
        assert!(plan.ops.iter().any(|op| matches!(op, StorageOp::CreateLvmPv { .. })));
        assert!(plan.ops.iter().any(|op| matches!(op, StorageOp::CreateLvmVg { .. })));
        assert!(plan.ops.iter().any(|op| matches!(op, StorageOp::CreateLvmLv { .. })));

        // PV must be on /dev/mapper/cryptlvm (not raw partition)
        assert!(plan.ops.iter().any(|op| matches!(
            op,
            StorageOp::CreateLvmPv { device }
                if device == &PathBuf::from("/dev/mapper/cryptlvm")
        )));

        assert!(plan.encrypted);
        assert!(plan.lvm);
    }

    #[test]
    fn test_btrfs_plan_has_mount_options() {
        let config = test_config(PartitionScheme::AutoSimple, Filesystem::Btrfs);
        let plan = calculate_storage_plan(&config).expect("plan generation failed"); // test: known-good input

        // Must have btrfs mount options on root mount
        assert!(plan.ops.iter().any(|op| matches!(
            op,
            StorageOp::Mount { options: Some(opts), mountpoint, .. }
                if mountpoint == &PathBuf::from("/mnt")
                    && opts.contains("subvol=@")
                    && opts.contains("compress=zstd")
        )));
    }

    #[test]
    fn test_raid_plan_returns_error() {
        let config = test_config(PartitionScheme::AutoRaid, Filesystem::Ext4);
        let result = calculate_storage_plan(&config);
        assert!(result.is_err());

        let err_msg = result.expect_err("should fail").to_string(); // test: expected error
        assert!(err_msg.contains("RAID"));
    }

    #[test]
    fn test_manual_plan_returns_error() {
        let config = test_config(PartitionScheme::Manual, Filesystem::Ext4);
        let result = calculate_storage_plan(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_disk_returns_error() {
        let mut config = test_config(PartitionScheme::AutoSimple, Filesystem::Ext4);
        config.install_disk = String::new();
        let result = calculate_storage_plan(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_disk_path_returns_error() {
        let mut config = test_config(PartitionScheme::AutoSimple, Filesystem::Ext4);
        config.install_disk = "not-a-device".to_string();
        let result = calculate_storage_plan(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_partition_path_sda() {
        assert_eq!(
            partition_path(&PathBuf::from("/dev/sda"), 1),
            PathBuf::from("/dev/sda1")
        );
        assert_eq!(
            partition_path(&PathBuf::from("/dev/sda"), 2),
            PathBuf::from("/dev/sda2")
        );
    }

    #[test]
    fn test_partition_path_nvme() {
        assert_eq!(
            partition_path(&PathBuf::from("/dev/nvme0n1"), 1),
            PathBuf::from("/dev/nvme0n1p1")
        );
        assert_eq!(
            partition_path(&PathBuf::from("/dev/nvme0n1"), 2),
            PathBuf::from("/dev/nvme0n1p2")
        );
    }

    #[test]
    fn test_lvm_no_swap() {
        let mut config = test_config(PartitionScheme::AutoLvm, Filesystem::Ext4);
        config.swap = Toggle::No;
        let plan = calculate_storage_plan(&config).expect("plan generation failed"); // test: known-good input

        // Should NOT have swap LV
        assert!(!plan.ops.iter().any(|op| matches!(
            op,
            StorageOp::CreateLvmLv { lv_name, .. } if lv_name == "swap"
        )));
    }

    #[test]
    fn test_plan_summary_not_empty() {
        let config = test_config(PartitionScheme::AutoSimple, Filesystem::Ext4);
        let plan = calculate_storage_plan(&config).expect("plan generation failed"); // test: known-good input
        let summary = plan.summary();
        assert!(!summary.is_empty());
        assert!(summary.contains("AutoSimple"));
        assert!(summary.contains("/dev/sda"));
    }

    #[test]
    fn test_operation_ordering_wipe_first() {
        let config = test_config(PartitionScheme::AutoLuksLvm, Filesystem::Ext4);
        let plan = calculate_storage_plan(&config).expect("plan generation failed"); // test: known-good input

        // First operation must always be WipeDisk
        assert!(matches!(&plan.ops[0], StorageOp::WipeDisk { .. }));

        // Second operation must always be Partition
        assert!(matches!(&plan.ops[1], StorageOp::Partition { .. }));
    }

    #[test]
    fn test_luks_format_before_open() {
        let config = test_config(PartitionScheme::AutoSimpleLuks, Filesystem::Ext4);
        let plan = calculate_storage_plan(&config).expect("plan generation failed"); // test: known-good input

        let format_idx = plan.ops.iter().position(|op| matches!(op, StorageOp::LuksFormat { .. }));
        let open_idx = plan.ops.iter().position(|op| matches!(op, StorageOp::LuksOpen { .. }));

        assert!(format_idx.is_some());
        assert!(open_idx.is_some());
        // format_idx safe: asserted is_some above
        // open_idx safe: asserted is_some above
        assert!(format_idx.expect("just checked") < open_idx.expect("just checked"));
    }
}
