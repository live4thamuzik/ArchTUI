//! Pre-install orchestration (Sprint 17)
//!
//! Handles steps that must complete *before* disk preparation:
//! - Mirror ranking via reflector (network-dependent, skippable)
//!
//! # Design
//!
//! - Uses `detect_internet()` from Sprint 14 (pure Rust, no shelling out)
//! - Skips automatically when offline — never blocks the installation
//! - Timeout-aware: reflector gets 30 seconds max
//! - Skippable by user (some users maintain custom mirrorlists)

// Library API — consumed by installer orchestration
#![allow(dead_code)]

use crate::hardware::{self, NetworkState};
use crate::script_runner::run_script_safe;
use crate::scripts::network::{MirrorSortMethod, UpdateMirrorsArgs};

use std::fmt;

// ============================================================================
// Mirror Ranking Result
// ============================================================================

/// Outcome of mirror ranking attempt.
#[derive(Debug, Clone)]
pub enum MirrorRankResult {
    /// Mirrors were ranked and saved successfully.
    Ranked { mirror_count: u32 },
    /// Mirror ranking was skipped (with reason).
    Skipped(SkipReason),
    /// Mirror ranking failed (non-fatal — installation continues).
    Failed(String),
}

/// Reason mirror ranking was skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkipReason {
    /// No network connectivity detected.
    Offline,
    /// User explicitly opted out of mirror ranking.
    UserSkipped,
}

impl fmt::Display for MirrorRankResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Ranked { mirror_count } => {
                write!(f, "Ranked {} mirrors", mirror_count)
            }
            Self::Skipped(reason) => write!(f, "Skipped: {}", reason),
            Self::Failed(err) => write!(f, "Failed: {}", err),
        }
    }
}

impl fmt::Display for SkipReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Offline => write!(f, "no network connectivity"),
            Self::UserSkipped => write!(f, "user opted out"),
        }
    }
}

// ============================================================================
// Pre-install Configuration
// ============================================================================

/// Configuration for pre-install steps.
#[derive(Debug, Clone)]
pub struct PreinstallConfig {
    /// Whether the user wants to skip mirror ranking.
    pub skip_mirrors: bool,
    /// Optional country filter for reflector (ISO 3166-1 alpha-2).
    pub mirror_country: Option<String>,
    /// Number of mirrors to keep (default: 20).
    pub mirror_limit: u32,
    /// Mirror sort method (default: Rate).
    pub mirror_sort: MirrorSortMethod,
    /// Timeout in seconds for reflector (default: 30).
    pub mirror_timeout: u32,
}

impl Default for PreinstallConfig {
    fn default() -> Self {
        Self {
            skip_mirrors: false,
            mirror_country: None,
            mirror_limit: 20,
            mirror_sort: MirrorSortMethod::Rate,
            mirror_timeout: 30,
        }
    }
}

// ============================================================================
// Mirror Ranking
// ============================================================================

/// Rank pacman mirrors if online. Skips gracefully if offline or user opted out.
///
/// This function is designed to be called *before* `prepare_disks()`. It:
/// 1. Checks network connectivity via `detect_internet()` (pure Rust)
/// 2. If offline, returns `Skipped(Offline)` immediately
/// 3. If user opted out, returns `Skipped(UserSkipped)`
/// 4. Otherwise, runs reflector with a timeout
///
/// # Failure Policy
///
/// Mirror ranking failure is **non-fatal**. The default mirrorlist from the
/// Arch ISO is usually acceptable. A failed ranking logs a warning and
/// returns `Failed(reason)` — the caller should continue with installation.
///
/// # Timeout
///
/// Reflector is given `config.mirror_timeout` seconds (default 30). If it
/// hangs (slow DNS, unresponsive mirrors), the script's internal timeout
/// or the process group cleanup will handle it.
pub fn rank_mirrors(config: &PreinstallConfig) -> MirrorRankResult {
    // 1. User opt-out check
    if config.skip_mirrors {
        log::info!("Mirror ranking skipped by user");
        return MirrorRankResult::Skipped(SkipReason::UserSkipped);
    }

    // 2. Network connectivity check (pure Rust — Sprint 14)
    let network = hardware::detect_internet();
    if network != NetworkState::Online {
        log::warn!("Mirror ranking skipped: no network connectivity");
        return MirrorRankResult::Skipped(SkipReason::Offline);
    }

    log::info!(
        "Ranking mirrors (country={:?}, limit={}, sort={}, timeout={}s)",
        config.mirror_country,
        config.mirror_limit,
        config.mirror_sort,
        config.mirror_timeout,
    );

    // 3. Run reflector via update_mirrors.sh
    let args = UpdateMirrorsArgs {
        country: config.mirror_country.clone(),
        limit: config.mirror_limit,
        sort: config.mirror_sort,
        protocol: Some("https".to_string()),
        save: true,
    };

    match run_script_safe(&args) {
        Ok(output) => {
            if output.success {
                log::info!("Mirror ranking completed successfully");
                MirrorRankResult::Ranked {
                    mirror_count: config.mirror_limit,
                }
            } else {
                let msg = format!(
                    "reflector exited with code {} — using default mirrorlist",
                    output.exit_code.unwrap_or(-1)
                );
                log::warn!("{}", msg);
                MirrorRankResult::Failed(msg)
            }
        }
        Err(e) => {
            let msg = format!("reflector failed to execute: {} — using default mirrorlist", e);
            log::warn!("{}", msg);
            MirrorRankResult::Failed(msg)
        }
    }
}

/// Build a `PreinstallConfig` from an `InstallationConfig`.
///
/// Maps the high-level installation config to pre-install parameters.
pub fn preinstall_config_from(config: &crate::config_file::InstallationConfig) -> PreinstallConfig {
    PreinstallConfig {
        skip_mirrors: false, // TUI sets this if user opts out
        mirror_country: if config.mirror_country.is_empty() {
            None
        } else {
            Some(config.mirror_country.clone())
        },
        mirror_limit: 20,
        mirror_sort: MirrorSortMethod::Rate,
        mirror_timeout: 30,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preinstall_config_defaults() {
        let config = PreinstallConfig::default();
        assert!(!config.skip_mirrors);
        assert!(config.mirror_country.is_none());
        assert_eq!(config.mirror_limit, 20);
        assert_eq!(config.mirror_sort, MirrorSortMethod::Rate);
        assert_eq!(config.mirror_timeout, 30);
    }

    #[test]
    fn test_rank_mirrors_user_skipped() {
        let config = PreinstallConfig {
            skip_mirrors: true,
            ..PreinstallConfig::default()
        };
        let result = rank_mirrors(&config);
        assert!(matches!(
            result,
            MirrorRankResult::Skipped(SkipReason::UserSkipped)
        ));
    }

    #[test]
    fn test_mirror_rank_result_display() {
        let ranked = MirrorRankResult::Ranked { mirror_count: 20 };
        assert_eq!(ranked.to_string(), "Ranked 20 mirrors");

        let skipped = MirrorRankResult::Skipped(SkipReason::Offline);
        assert_eq!(skipped.to_string(), "Skipped: no network connectivity");

        let user_skip = MirrorRankResult::Skipped(SkipReason::UserSkipped);
        assert_eq!(user_skip.to_string(), "Skipped: user opted out");

        let failed = MirrorRankResult::Failed("timeout".to_string());
        assert_eq!(failed.to_string(), "Failed: timeout");
    }

    #[test]
    fn test_skip_reason_display() {
        assert_eq!(SkipReason::Offline.to_string(), "no network connectivity");
        assert_eq!(SkipReason::UserSkipped.to_string(), "user opted out");
    }

    #[test]
    fn test_preinstall_config_from_installation_config() {
        let mut install_config = crate::config_file::InstallationConfig::new();
        install_config.mirror_country = "US".to_string();

        let pre = preinstall_config_from(&install_config);
        assert_eq!(pre.mirror_country, Some("US".to_string()));
        assert!(!pre.skip_mirrors);
    }

    #[test]
    fn test_preinstall_config_from_empty_country() {
        let mut install_config = crate::config_file::InstallationConfig::new();
        install_config.mirror_country = String::new();

        let pre = preinstall_config_from(&install_config);
        assert!(pre.mirror_country.is_none());
    }
}
