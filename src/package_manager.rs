//! Transparent ALPM Integration for Package Management
//!
//! This module provides direct access to libalpm through Rust bindings,
//! eliminating the need to shell out to pacman and parse stdout.
//!
//! # Transparency
//!
//! All ALPM operations are logged through the Rust `log` crate via callbacks.
//! This ensures full visibility of "Downloading...", "Installing...", etc.
//! in the TUI without parsing process output.
//!
//! # Architecture
//!
//! - `PackageManager`: Main struct that owns the ALPM handle
//! - `log_cb`: Routes ALPM log messages to `log::*` macros
//! - `install_packages`: Runs a sync transaction on target packages

// Library API - will be used for Sprint 5 base system installation
#![allow(dead_code)]

use alpm::{Alpm, LogLevel, SigLevel, TransFlag};
use anyhow::{Context, Result};
use std::path::Path;

/// Package manager wrapping libalpm with full logging transparency.
///
/// Initializes ALPM on a target root (e.g., `/mnt` for installation),
/// not the live system, ensuring packages are installed to the correct location.
pub struct PackageManager {
    handle: Alpm,
}

impl PackageManager {
    /// Create a new PackageManager targeting the specified root.
    ///
    /// # Arguments
    ///
    /// * `root` - Target root directory (e.g., `/mnt` during installation)
    /// * `db_path` - Package database path (e.g., `/mnt/var/lib/pacman`)
    ///
    /// # Transparency
    ///
    /// The ALPM handle is configured with `log_cb` to route all library
    /// messages to the Rust logging system.
    pub fn new<P: AsRef<Path>>(root: P, db_path: P) -> Result<Self> {
        let root = root.as_ref();
        let db_path = db_path.as_ref();

        // ALPM requires paths as strings
        let root_str = root
            .to_str()
            .context("Root path contains invalid UTF-8")?;
        let db_path_str = db_path
            .to_str()
            .context("DB path contains invalid UTF-8")?;

        let handle = Alpm::new(root_str, db_path_str).with_context(|| {
            format!(
                "Failed to initialize ALPM with root={}, db_path={}",
                root.display(),
                db_path.display()
            )
        })?;

        // TRANSPARENCY REQUIREMENT: Wire up log callback
        handle.set_log_cb((), log_cb);

        log::info!(
            "ALPM initialized: root={}, db_path={}",
            root.display(),
            db_path.display()
        );

        Ok(Self { handle })
    }

    /// Create a PackageManager from pacman.conf on the target system.
    ///
    /// Reads `/mnt/etc/pacman.conf` (or specified path) to configure repos.
    pub fn from_pacman_conf<P: AsRef<Path>>(root: P, conf_path: P) -> Result<Self> {
        let root = root.as_ref();
        let conf_path = conf_path.as_ref();

        let conf = pacmanconf::Config::from_file(conf_path).with_context(|| {
            format!("Failed to parse pacman.conf at {}", conf_path.display())
        })?;

        let db_path = root.join("var/lib/pacman");
        let mut pm = Self::new(root, &db_path)?;

        // Register sync databases from pacman.conf
        for repo in &conf.repos {
            let db = pm
                .handle
                .register_syncdb_mut(repo.name.clone(), SigLevel::USE_DEFAULT)
                .with_context(|| format!("Failed to register sync db: {}", repo.name))?;

            // Add servers from the repo configuration
            for server in &repo.servers {
                db.add_server(server.clone())
                    .with_context(|| format!("Failed to add server {} to {}", server, repo.name))?;
            }

            log::debug!(
                "Registered sync db: {} with {} servers",
                repo.name,
                repo.servers.len()
            );
        }

        Ok(pm)
    }

    /// Install packages via a sync transaction.
    ///
    /// # Transparency
    ///
    /// Progress and status messages are routed through `log_cb` and appear
    /// in the TUI logs as "Downloading...", "Installing...", etc.
    ///
    /// # Arguments
    ///
    /// * `targets` - Package names to install
    pub fn install_packages(&mut self, targets: &[&str]) -> Result<()> {
        if targets.is_empty() {
            log::warn!("install_packages called with empty target list");
            return Ok(());
        }

        log::info!("Starting package installation: {:?}", targets);

        // Refresh databases
        self.handle
            .syncdbs_mut()
            .update(false)
            .context("Failed to update sync databases")?;

        log::info!("Sync databases updated");

        // Verify all packages exist before starting transaction
        // Store (pkg_name, db_name) pairs as owned data to avoid borrow issues
        let mut pkg_locations: Vec<(String, String)> = Vec::new();
        for target in targets {
            let mut found = false;
            for db in self.handle.syncdbs() {
                if db.pkg(*target).is_ok() {
                    pkg_locations.push(((*target).to_string(), db.name().to_string()));
                    found = true;
                    log::debug!("Found {} in {}", target, db.name());
                    break;
                }
            }
            if !found {
                anyhow::bail!("Package not found in any sync database: {}", target);
            }
        }

        // Initialize transaction
        let flags = TransFlag::empty();
        self.handle
            .trans_init(flags)
            .context("Failed to initialize transaction")?;

        // Add packages to transaction - look up each one fresh to avoid borrow issues
        for (pkg_name, db_name) in &pkg_locations {
            // Find the db again and get the package
            let pkg = self
                .handle
                .syncdbs()
                .iter()
                .find(|db| db.name() == db_name)
                .and_then(|db| db.pkg(pkg_name.as_str()).ok());

            let pkg = match pkg {
                Some(p) => p,
                None => {
                    let _ = self.handle.trans_release();
                    anyhow::bail!("Package {} disappeared from {}", pkg_name, db_name);
                }
            };

            let add_result = self.handle.trans_add_pkg(pkg);
            if let Err(e) = add_result {
                let err_msg = e.to_string();
                let _ = self.handle.trans_release();
                anyhow::bail!("Failed to add package to transaction: {}: {}", pkg_name, err_msg);
            }
            log::info!("Queued for installation: {}", pkg_name);
        }

        // Prepare transaction (resolve dependencies)
        // Note: Error types borrow handle, so convert fully before trans_release
        let prepare_err = self.handle.trans_prepare().err().map(|e| e.to_string());
        if let Some(err_msg) = prepare_err {
            let _ = self.handle.trans_release();
            anyhow::bail!("Transaction prepare failed: {}", err_msg);
        }

        log::info!("Transaction prepared, resolving dependencies...");

        // Log what will be installed
        for pkg in self.handle.trans_add() {
            log::info!(
                "Will install: {}-{} ({})",
                pkg.name(),
                pkg.version(),
                humanize_size(pkg.isize())
            );
        }

        // Commit transaction (download and install)
        let commit_err = self.handle.trans_commit().err().map(|e| e.to_string());
        if let Some(err_msg) = commit_err {
            let _ = self.handle.trans_release();
            anyhow::bail!("Transaction commit failed: {}", err_msg);
        }

        log::info!("Transaction committed successfully");

        // Release transaction
        self.handle
            .trans_release()
            .context("Failed to release transaction")?;

        log::info!("Package installation complete: {:?}", targets);

        Ok(())
    }

    /// Get a reference to the underlying ALPM handle.
    ///
    /// Use this for advanced operations not covered by this wrapper.
    pub fn handle(&self) -> &Alpm {
        &self.handle
    }

    /// Get a mutable reference to the underlying ALPM handle.
    pub fn handle_mut(&mut self) -> &mut Alpm {
        &mut self.handle
    }
}

/// ALPM log callback that routes messages to Rust's `log` crate.
///
/// # Transparency
///
/// This callback ensures all ALPM output (downloading, installing, warnings,
/// errors) is visible in the TUI through the standard logging infrastructure.
fn log_cb(level: LogLevel, msg: &str, _: &mut ()) {
    // ALPM messages often have trailing newlines - strip them
    let msg = msg.trim_end();

    if level.contains(LogLevel::ERROR) {
        log::error!("[ALPM] {}", msg);
    } else if level.contains(LogLevel::WARNING) {
        log::warn!("[ALPM] {}", msg);
    } else if level.contains(LogLevel::DEBUG) {
        log::debug!("[ALPM] {}", msg);
    } else if level.contains(LogLevel::FUNCTION) {
        log::trace!("[ALPM] {}", msg);
    } else {
        // Default to info for any other level
        log::info!("[ALPM] {}", msg);
    }
}

/// Convert bytes to human-readable size string.
fn humanize_size(bytes: i64) -> String {
    const KB: i64 = 1024;
    const MB: i64 = KB * 1024;
    const GB: i64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GiB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MiB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KiB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_humanize_size() {
        assert_eq!(humanize_size(512), "512 B");
        assert_eq!(humanize_size(1024), "1.00 KiB");
        assert_eq!(humanize_size(1536), "1.50 KiB");
        assert_eq!(humanize_size(1048576), "1.00 MiB");
        assert_eq!(humanize_size(1073741824), "1.00 GiB");
    }
}
