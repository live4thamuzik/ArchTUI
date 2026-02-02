//! ArchInstall TUI Library
//!
//! This library provides the core functionality for the Arch Linux TUI installer.

pub mod app;
pub mod cli;
pub mod components;
pub mod config;
pub mod config_file;
pub mod error;
pub mod input;
pub mod install_state;
pub mod installer;
#[cfg(feature = "alpm")]
pub mod package_manager;
pub mod package_utils;
pub mod process_guard;
pub mod script_manifest;
pub mod script_runner;
pub mod script_traits;
pub mod scripts;
pub mod scrolling;
pub mod theme;
pub mod types;
pub mod ui;

// Re-export UI wizard types (Sprint 7)
pub use ui::{WizardData, WizardState};

// Re-export main types for convenience
pub use config::{ConfigOption, Configuration, Package};
pub use config_file::InstallationConfig;
pub use error::ArchInstallError;
pub use install_state::{InstallStage, InstallTransitionError, InstallerContext};
pub use process_guard::{ChildRegistry, CommandProcessGroup, ProcessGuard};
pub use script_manifest::{
    EnvRequirement, ManifestError, ManifestRegistry, OptionalEnv, ScriptManifest,
    ValidatedExecution,
};
#[cfg(feature = "alpm")]
pub use package_manager::PackageManager;
pub use script_runner::{run_script_safe, ScriptOutput};
pub use script_traits::{disable_dry_run, enable_dry_run, is_dry_run, ScriptArgs};
pub use scripts::disk::{
    FormatPartitionArgs, MountPartitionArgs, WipeDiskArgs, WipeMethod, WipeMethodError,
};
pub use scripts::config::{GenFstabArgs, LocaleArgs, UserAddArgs};
pub use installer::{configure_system, prepare_disks, DiskLayout, SystemConfig};
#[cfg(feature = "alpm")]
pub use installer::{install_base_system, install_base_system_with_extras};
pub use types::{
    AurHelper, AutoToggle, Bootloader, BootMode, DesktopEnvironment, DisplayManager, Filesystem,
    GpuDriver, GrubTheme, Kernel, PartitionScheme, PlymouthTheme, SnapshotFrequency, Toggle,
};
