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
//! - `encryption`: LUKS encryption (format, open, close) - Sprint 11
//! - `network`: Network configuration (configure, test, firewall, mirrors)
//! - `profiles`: Desktop profiles and dotfiles - Sprint 12
//! - `system`: System configuration (bootloader, chroot, services)
//! - `user`: User management (password, groups, ssh, security)
//!
//! # Security Note
//!
//! Password handling is done via environment variables, NOT CLI flags.
//! See `config::UserAddArgs` for the secure pattern.
//!
//! LUKS encryption uses a `SecretFile` wrapper that securely manages
//! temporary keyfiles. See `encryption::SecretFile` for details.

pub mod config;
pub mod disk;
pub mod encryption;
pub mod network;
pub mod profiles;
pub mod system;
pub mod user;
pub mod user_ops;
