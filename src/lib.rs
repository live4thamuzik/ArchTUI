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
pub mod installer;
pub mod package_utils;
pub mod process_guard;
pub mod scrolling;
pub mod theme;
pub mod types;
pub mod ui;

// Re-export main types for convenience
pub use config::{ConfigOption, Configuration, Package};
pub use config_file::InstallationConfig;
pub use error::ArchInstallError;
pub use process_guard::{ChildRegistry, CommandProcessGroup, ProcessGuard};
pub use types::{
    AurHelper, AutoToggle, Bootloader, BootMode, DesktopEnvironment, DisplayManager, Filesystem,
    GpuDriver, GrubTheme, Kernel, PartitionScheme, PlymouthTheme, SnapshotFrequency, Toggle,
};
