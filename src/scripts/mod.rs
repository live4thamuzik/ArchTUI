//! Type-safe script argument modules.
//!
//! This module contains structs that implement `ScriptArgs` for each tool script.
//! Each struct maps Rust fields to the exact CLI flags and environment variables
//! expected by the corresponding bash script.
//!
//! # Categories
//!
//! - `config`: Post-install configuration (fstab, users, locale)
//! - `disk`: Disk operations (wipe, format, mount, health)
//! - `network`: Network configuration (configure, test, firewall, diagnostics)
//! - `system`: System configuration (bootloader, chroot, services)
//! - `user`: User management (password, groups, ssh, security)
//!
//! # Security Note
//!
//! Password handling is done via environment variables, NOT CLI flags.
//! See `config::UserAddArgs` for the secure pattern.

pub mod config;
pub mod disk;
pub mod network;
pub mod system;
pub mod user;
