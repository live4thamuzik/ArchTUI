//! Type-safe script argument modules.
//!
//! This module contains structs that implement `ScriptArgs` for each tool script.
//! Each struct maps Rust fields to the exact CLI flags and environment variables
//! expected by the corresponding bash script.
//!
//! # Categories
//!
//! - `disk`: Disk operations (wipe, format, mount, health)
//! - `system`: System configuration (bootloader, fstab, chroot, services)
//! - `user`: User management (add, password, groups, ssh, security)
//! - `network`: Network configuration (configure, test, firewall, diagnostics)

pub mod disk;
pub mod network;
pub mod system;
pub mod user;
