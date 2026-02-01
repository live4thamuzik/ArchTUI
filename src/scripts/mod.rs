//! Type-safe script argument modules.
//!
//! This module contains structs that implement `ScriptArgs` for each tool script.
//! Each struct maps Rust fields to the exact CLI flags and environment variables
//! expected by the corresponding bash script.

pub mod disk;
