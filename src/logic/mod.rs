//! Logic modules — translates high-level user choices into concrete actions.
//!
//! The logic layer resolves abstract selections (e.g., "KDE", "Nvidia") into
//! specific package names and service lists.
//!
//! # Modules
//!
//! - `resolver` — Package and service name resolution
//! - `preinstall` — Pre-install orchestration: mirror ranking
//! - `postinstall` — Post-install orchestration: AUR, dotfiles

pub mod postinstall;
pub mod preinstall;
pub mod resolver;
