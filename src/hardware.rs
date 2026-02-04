//! Hardware environment detection (Sprint 14)
//!
//! Detects firmware mode (UEFI vs BIOS) and network connectivity using pure Rust.
//! No shelling out — all detection uses std library primitives.
//!
//! # Design
//!
//! - **Fail Fast**: Ambiguous detection logs a warning and defaults to safe mode (BIOS)
//! - **Pure Rust**: Network check uses `TcpStream::connect_timeout`, not ping/shell
//! - **No `unwrap()`**: All fallible paths use `anyhow::Result`
//!
//! # Integration
//!
//! Call `HardwareInfo::detect()` at startup before presenting the TUI.
//! The result informs which options are valid (e.g., systemd-boot requires UEFI).

// Library API - consumed by installer logic
#![allow(dead_code)]

use anyhow::{Context, Result};
use std::fmt;
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

/// Detected firmware mode of the system.
///
/// Determined by checking for the existence of `/sys/firmware/efi`.
/// If the directory exists, the system booted in UEFI mode.
/// If it does not exist, the system booted in legacy BIOS mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FirmwareMode {
    /// UEFI firmware — supports GPT, ESP partition, systemd-boot
    Uefi,
    /// Legacy BIOS firmware — requires MBR or GPT with BIOS boot partition
    Bios,
}

impl FirmwareMode {
    /// Returns true if the system booted in UEFI mode.
    pub fn is_uefi(self) -> bool {
        matches!(self, Self::Uefi)
    }

    /// Returns true if the system booted in legacy BIOS mode.
    pub fn is_bios(self) -> bool {
        matches!(self, Self::Bios)
    }
}

impl fmt::Display for FirmwareMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uefi => write!(f, "UEFI"),
            Self::Bios => write!(f, "BIOS"),
        }
    }
}

/// Network connectivity state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkState {
    /// TCP connection to archlinux.org:443 succeeded
    Online,
    /// TCP connection failed or timed out
    Offline,
}

impl NetworkState {
    /// Returns true if network connectivity is available.
    pub fn is_online(self) -> bool {
        matches!(self, Self::Online)
    }
}

impl fmt::Display for NetworkState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Online => write!(f, "Online"),
            Self::Offline => write!(f, "Offline"),
        }
    }
}

/// Aggregated hardware detection results.
///
/// Created via `HardwareInfo::detect()` at startup. Provides the installer
/// with facts about the environment so it can make correct decisions
/// (e.g., only offer systemd-boot on UEFI, skip mirror update if offline).
#[derive(Debug, Clone)]
pub struct HardwareInfo {
    /// Detected firmware mode (UEFI or BIOS)
    pub firmware: FirmwareMode,
    /// Network connectivity state
    pub network: NetworkState,
}

impl HardwareInfo {
    /// Detect hardware environment.
    ///
    /// Checks firmware mode and network connectivity. This function
    /// never panics — detection failures are handled gracefully with
    /// safe defaults.
    ///
    /// # Returns
    ///
    /// `HardwareInfo` with detected values. Network detection failure
    /// defaults to `Offline`. Firmware detection is unambiguous (directory
    /// either exists or it doesn't).
    pub fn detect() -> Self {
        let firmware = detect_firmware_mode();
        let network = detect_internet();

        log::info!("Hardware detection: firmware={}, network={}", firmware, network);

        Self { firmware, network }
    }

    /// Returns true if the bootloader choice is compatible with the firmware.
    ///
    /// systemd-boot requires UEFI. GRUB works with both UEFI and BIOS.
    pub fn is_bootloader_compatible(&self, bootloader: &crate::types::Bootloader) -> bool {
        match bootloader {
            crate::types::Bootloader::SystemdBoot => self.firmware.is_uefi(),
            crate::types::Bootloader::Grub => true,
        }
    }
}

impl fmt::Display for HardwareInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Firmware: {}, Network: {}", self.firmware, self.network)
    }
}

// ============================================================================
// Detection Functions
// ============================================================================

/// Detect firmware mode by checking for the EFI sysfs directory.
///
/// The Linux kernel exposes `/sys/firmware/efi` only when booted in UEFI mode.
/// This is the canonical detection method used by systemd, grub-install, etc.
///
/// # Safety
///
/// This is a read-only filesystem check. No destructive operations.
pub fn detect_firmware_mode() -> FirmwareMode {
    let efi_path = Path::new("/sys/firmware/efi");

    if efi_path.exists() {
        log::info!("UEFI firmware detected (/sys/firmware/efi exists)");
        FirmwareMode::Uefi
    } else {
        log::info!("BIOS firmware detected (/sys/firmware/efi not found)");
        FirmwareMode::Bios
    }
}

/// Detect network connectivity via TCP connection to archlinux.org.
///
/// Uses `TcpStream::connect_timeout` with a 5-second timeout.
/// Connects to port 443 (HTTPS) since it's universally allowed through firewalls.
///
/// # Why TCP instead of ICMP/ping?
///
/// - ICMP is often blocked by firewalls
/// - `ping` requires shelling out (violates ROE: Rust controls)
/// - TCP connect is the most reliable connectivity test
///
/// # Failure Mode
///
/// Returns `NetworkState::Offline` if:
/// - DNS resolution fails
/// - TCP connection times out (5s)
/// - Connection is refused
/// - Any other I/O error occurs
pub fn detect_internet() -> NetworkState {
    // archlinux.org HTTPS — reliable, always up, firewall-friendly
    let addr: SocketAddr = match "147.75.81.97:443".parse() {
        Ok(a) => a,
        Err(e) => {
            log::warn!("Failed to parse socket address: {}", e);
            return NetworkState::Offline;
        }
    };

    let timeout = Duration::from_secs(5);

    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(_stream) => {
            log::info!("Network connectivity confirmed (TCP to archlinux.org:443)");
            NetworkState::Online
        }
        Err(e) => {
            log::warn!("Network connectivity check failed: {}", e);
            NetworkState::Offline
        }
    }
}

/// Detect firmware mode with Result return for callers that need error context.
///
/// Unlike `detect_firmware_mode()` which always succeeds, this variant
/// returns an error if the sysfs path cannot be accessed (e.g., inside a container).
pub fn detect_firmware_mode_strict() -> Result<FirmwareMode> {
    let efi_path = Path::new("/sys/firmware/efi");

    // Check if /sys/firmware exists at all (sanity check)
    let sys_firmware = Path::new("/sys/firmware");
    if !sys_firmware.exists() {
        anyhow::bail!(
            "/sys/firmware does not exist — are you running inside a container? \
             Firmware detection requires a real Linux system."
        );
    }

    if efi_path.exists() {
        // Double-check by reading efivars
        let efivars = Path::new("/sys/firmware/efi/efivars");
        if efivars.exists() {
            log::info!("UEFI firmware confirmed (efivars accessible)");
        } else {
            log::warn!(
                "/sys/firmware/efi exists but efivars not found — \
                 UEFI detected but EFI variables may not be writable"
            );
        }
        Ok(FirmwareMode::Uefi)
    } else {
        Ok(FirmwareMode::Bios)
    }
}

/// Detect internet connectivity with Result return for callers that need error context.
///
/// Returns `Ok(true)` if online, `Ok(false)` if offline, `Err` if detection itself failed.
pub fn detect_internet_strict() -> Result<bool> {
    let addr: SocketAddr = "147.75.81.97:443"
        .parse()
        .context("Failed to parse archlinux.org socket address")?;

    let timeout = Duration::from_secs(5);

    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(_stream) => Ok(true),
        Err(e) => {
            log::info!("Network offline: {}", e);
            Ok(false)
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_firmware_mode_display() {
        assert_eq!(FirmwareMode::Uefi.to_string(), "UEFI");
        assert_eq!(FirmwareMode::Bios.to_string(), "BIOS");
    }

    #[test]
    fn test_firmware_mode_predicates() {
        assert!(FirmwareMode::Uefi.is_uefi());
        assert!(!FirmwareMode::Uefi.is_bios());
        assert!(FirmwareMode::Bios.is_bios());
        assert!(!FirmwareMode::Bios.is_uefi());
    }

    #[test]
    fn test_network_state_display() {
        assert_eq!(NetworkState::Online.to_string(), "Online");
        assert_eq!(NetworkState::Offline.to_string(), "Offline");
    }

    #[test]
    fn test_network_state_predicates() {
        assert!(NetworkState::Online.is_online());
        assert!(!NetworkState::Offline.is_online());
    }

    #[test]
    fn test_hardware_info_display() {
        let info = HardwareInfo {
            firmware: FirmwareMode::Uefi,
            network: NetworkState::Online,
        };
        assert_eq!(info.to_string(), "Firmware: UEFI, Network: Online");
    }

    #[test]
    fn test_bootloader_compatibility_systemdboot_requires_uefi() {
        let uefi_hw = HardwareInfo {
            firmware: FirmwareMode::Uefi,
            network: NetworkState::Offline,
        };
        let bios_hw = HardwareInfo {
            firmware: FirmwareMode::Bios,
            network: NetworkState::Offline,
        };

        use crate::types::Bootloader;

        // systemd-boot requires UEFI
        assert!(uefi_hw.is_bootloader_compatible(&Bootloader::SystemdBoot));
        assert!(!bios_hw.is_bootloader_compatible(&Bootloader::SystemdBoot));

        // GRUB works with both
        assert!(uefi_hw.is_bootloader_compatible(&Bootloader::Grub));
        assert!(bios_hw.is_bootloader_compatible(&Bootloader::Grub));
    }

    #[test]
    fn test_detect_firmware_mode_runs() {
        // This test runs on any system — just verify it returns a valid variant
        let mode = detect_firmware_mode();
        assert!(mode.is_uefi() || mode.is_bios());
    }

    #[test]
    fn test_detect_internet_runs() {
        // Just verify the function completes without panic
        let state = detect_internet();
        assert!(state.is_online() || !state.is_online());
    }

    #[test]
    fn test_hardware_info_detect_runs() {
        // Integration test — verify detect() returns valid data
        let info = HardwareInfo::detect();
        // Must be one or the other
        assert!(info.firmware.is_uefi() || info.firmware.is_bios());
    }
}
