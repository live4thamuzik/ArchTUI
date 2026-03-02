//! Logic modules — translates high-level user choices into concrete actions.
//!
//! The logic layer resolves abstract selections (e.g., "KDE", "Nvidia") into
//! specific package names and service lists.
//!
//! # Modules
//!
//! - `resolver` — Package and service name resolution (Sprint 16)
//! - `preinstall` — Pre-install orchestration: mirror ranking (Sprint 17)
//! - `postinstall` — Post-install orchestration: AUR, dotfiles (Sprint 18)

pub mod postinstall;
pub mod preinstall;
pub mod resolver;
