//! ArchInstall TUI Library
//!
//! This library provides the core functionality for the Arch Linux TUI installer.

pub mod app;
pub mod config;
pub mod error;
pub mod input;
pub mod installer;
pub mod package_utils;
pub mod scrolling;
pub mod ui;

// Re-export main types for convenience
pub use config::{ConfigOption, Configuration, Package};
pub use error::ArchInstallError;
