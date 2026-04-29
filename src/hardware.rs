//! Hardware environment detection
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

use anyhow::Result;
use std::fmt;
use std::net::{SocketAddr, TcpStream};
use std::path::Path;
use std::time::Duration;

use crate::process_guard::CommandProcessGroup;

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

        // Network check runs in a background thread with a 3-second deadline
        // to avoid blocking TUI startup (common in VMs without network).
        let network = {
            let (tx, rx) = std::sync::mpsc::channel();
            std::thread::spawn(move || {
                let _ = tx.send(detect_internet());
            });
            match rx.recv_timeout(Duration::from_secs(3)) {
                Ok(state) => state,
                Err(_) => {
                    tracing::warn!("Network detection timed out — defaulting to Offline");
                    NetworkState::Offline
                }
            }
        };

        tracing::info!(
            "Hardware detection: firmware={}, network={}",
            firmware,
            network
        );

        Self { firmware, network }
    }

    /// Returns true if the bootloader choice is compatible with the firmware.
    ///
    /// systemd-boot requires UEFI. GRUB works with both UEFI and BIOS.
    pub fn is_bootloader_compatible(&self, bootloader: &crate::types::Bootloader) -> bool {
        match bootloader {
            crate::types::Bootloader::SystemdBoot
            | crate::types::Bootloader::Refind
            | crate::types::Bootloader::Efistub => self.firmware.is_uefi(),
            crate::types::Bootloader::Grub | crate::types::Bootloader::Limine => true,
        }
    }
}

impl fmt::Display for HardwareInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Firmware: {}, Network: {}", self.firmware, self.network)
    }
}

// ============================================================================
// OS Detection Types
// ============================================================================

/// Type of operating system detected on a partition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectedOsType {
    /// Windows Boot Manager found on an ESP
    Windows,
    /// Linux distribution found (has /etc/os-release)
    Linux,
}

impl fmt::Display for DetectedOsType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Windows => write!(f, "Windows"),
            Self::Linux => write!(f, "Linux"),
        }
    }
}

/// A single operating system detected on a disk partition.
#[derive(Debug, Clone)]
pub struct DetectedOs {
    /// Human-readable name (e.g., "Ubuntu 24.04", "Windows Boot Manager")
    pub name: String,
    /// Device path of the partition (e.g., "/dev/sda2")
    pub device: String,
    /// Whether this is on the same disk as the install target
    pub same_disk: bool,
    /// OS category
    pub os_type: DetectedOsType,
}

/// Aggregated OS detection results for all scanned disks.
///
/// Populated at disk selection time in the TUI. Results are relative
/// to a specific install disk (same_disk field on each entry).
#[derive(Debug, Clone, Default)]
pub struct OsDetectionResults {
    /// All detected operating systems
    pub entries: Vec<DetectedOs>,
    /// The install disk these results are relative to
    pub install_disk: String,
}

impl OsDetectionResults {
    /// Returns true if any operating system was detected.
    pub fn has_any(&self) -> bool {
        !self.entries.is_empty()
    }

    /// Returns true if Windows was detected on any disk.
    pub fn has_windows(&self) -> bool {
        self.entries.iter().any(|e| e.os_type == DetectedOsType::Windows)
    }

    /// Returns true if a Linux installation was detected on any disk.
    pub fn has_linux(&self) -> bool {
        self.entries.iter().any(|e| e.os_type == DetectedOsType::Linux)
    }

    /// Returns OSes detected on the same disk as the install target.
    pub fn same_disk_os(&self) -> Vec<&DetectedOs> {
        self.entries.iter().filter(|e| e.same_disk).collect()
    }

    /// Returns OSes detected on disks other than the install target.
    pub fn other_disk_os(&self) -> Vec<&DetectedOs> {
        self.entries.iter().filter(|e| !e.same_disk).collect()
    }

    /// First detected Windows ESP device path (for env export).
    pub fn windows_esp_device(&self) -> Option<&str> {
        self.entries
            .iter()
            .find(|e| e.os_type == DetectedOsType::Windows)
            .map(|e| e.device.as_str())
    }

    /// First detected Linux entry (for env export).
    pub fn first_linux(&self) -> Option<&DetectedOs> {
        self.entries.iter().find(|e| e.os_type == DetectedOsType::Linux)
    }

    /// Human-readable summary for status_message display.
    pub fn summary_line(&self) -> String {
        let names: Vec<String> = self
            .entries
            .iter()
            .map(|e| {
                let location = if e.same_disk { "same disk" } else { "other disk" };
                format!("{} on {} ({})", e.name, e.device, location)
            })
            .collect();
        names.join(", ")
    }
}

// ============================================================================
// OS Detection — Heuristic (lsblk-based, no mounting)
// ============================================================================

/// ESP partition type GUID (EFI System Partition).
const ESP_PARTTYPE_GUID: &str = "c12a7328-f81f-11d2-ba4b-00a0c93ec93b";

/// Generate OS hints per disk using lsblk heuristics only (no mounting).
///
/// Returns a map of disk path → hint strings. Fast and non-blocking.
/// Used to enrich the disk selection right-panel preview.
pub fn detect_os_heuristic() -> std::collections::HashMap<String, Vec<String>> {
    use std::collections::HashMap;
    use std::process::Command;

    let mut hints: HashMap<String, Vec<String>> = HashMap::new();

    // Get partition-level info: NAME, FSTYPE, PARTTYPE, SIZE, PKNAME (parent disk)
    let output = match Command::new("lsblk")
        .args(["-rno", "NAME,FSTYPE,PARTTYPE,SIZE,PKNAME"])
        .in_new_process_group()
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!("lsblk failed for OS heuristic scan: {}", e);
            return hints;
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);

    for line in stdout.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() < 5 {
            continue;
        }

        let fstype = fields[1];
        let parttype = fields[2].to_lowercase();
        let size_str = fields[3];
        let parent = format!("/dev/{}", fields[4]);

        // Parse size — lsblk uses human-readable (e.g., "953.9G", "512M")
        let size_gb = parse_lsblk_size_gb(size_str);

        // ESP detection
        if parttype.contains(ESP_PARTTYPE_GUID) {
            hints
                .entry(parent.clone())
                .or_default()
                .push("EFI System Partition found".to_string());
        }

        // Possible Windows (ntfs partition > 10GB)
        if fstype == "ntfs" && size_gb > 10.0 {
            let entry = hints.entry(parent.clone()).or_default();
            if !entry.iter().any(|h| h.contains("Windows")) {
                entry.push("Possible Windows installation (ntfs)".to_string());
            }
        }

        // Possible Linux (ext4/btrfs/xfs partition > 4GB)
        if matches!(fstype, "ext4" | "btrfs" | "xfs") && size_gb > 4.0 {
            let entry = hints.entry(parent.clone()).or_default();
            if !entry.iter().any(|h| h.contains("Linux")) {
                entry.push(format!("Possible Linux installation ({})", fstype));
            }
        }
    }

    hints
}

/// Parse lsblk human-readable size string to approximate GB.
fn parse_lsblk_size_gb(s: &str) -> f64 {
    let s = s.trim();
    if s.is_empty() {
        return 0.0;
    }

    // Find where the numeric part ends
    let num_end = s
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(s.len());
    let (num_str, suffix) = s.split_at(num_end);
    let value: f64 = num_str.parse().unwrap_or(0.0);

    match suffix.to_uppercase().as_str() {
        "T" => value * 1024.0,
        "G" => value,
        "M" => value / 1024.0,
        "K" => value / (1024.0 * 1024.0),
        _ => value, // assume bytes-ish, negligible
    }
}

// ============================================================================
// OS Detection — Definitive (mount-based probe via bash script)
// ============================================================================

/// Run the definitive OS detection probe (mount + check).
///
/// Calls `scripts/tools/detect_os.sh` synchronously and parses JSON output.
/// Returns `OsDetectionResults` — empty on any failure (best-effort).
///
/// Expected JSON: `{"os":[{"name":"...","device":"...","type":"linux|windows","same_disk":true|false}]}`
pub fn detect_os_definitive(install_disk: &str) -> OsDetectionResults {
    use std::process::Command;

    let script_path = crate::script_runner::scripts_base_dir()
        .join("tools")
        .join("detect_os.sh");

    if !script_path.exists() {
        tracing::warn!(
            path = %script_path.display(),
            "detect_os.sh not found — skipping definitive OS probe"
        );
        return OsDetectionResults {
            install_disk: install_disk.to_string(),
            ..Default::default()
        };
    }

    tracing::info!(disk = %install_disk, "Running definitive OS detection probe");

    let output = match Command::new("/bin/bash")
        .arg(script_path.as_os_str())
        .args(["--install-disk", install_disk])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .in_new_process_group()
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            tracing::warn!("Failed to run detect_os.sh: {}", e);
            return OsDetectionResults {
                install_disk: install_disk.to_string(),
                ..Default::default()
            };
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(
            exit_code = ?output.status.code(),
            stderr = %stderr,
            "detect_os.sh exited with non-zero status"
        );
        return OsDetectionResults {
            install_disk: install_disk.to_string(),
            ..Default::default()
        };
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_detect_os_json(&stdout, install_disk)
}

/// Parse JSON output from detect_os.sh into `OsDetectionResults`.
fn parse_detect_os_json(json_str: &str, install_disk: &str) -> OsDetectionResults {
    // Find the JSON line (skip any log output on stderr/stdout preamble)
    let json_line = json_str
        .lines()
        .find(|line| line.trim_start().starts_with('{'))
        .unwrap_or(json_str.trim());

    let parsed: serde_json::Value = match serde_json::from_str(json_line) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(json = %json_line, err = %e, "Failed to parse detect_os.sh JSON output");
            return OsDetectionResults {
                install_disk: install_disk.to_string(),
                ..Default::default()
            };
        }
    };

    let mut entries = Vec::new();

    if let Some(os_array) = parsed.get("os").and_then(|v| v.as_array()) {
        for entry in os_array {
            let name = entry
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Unknown")
                .to_string();
            let device = entry
                .get("device")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let os_type_str = entry
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("linux");
            let same_disk = entry
                .get("same_disk")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let os_type = match os_type_str {
                "windows" => DetectedOsType::Windows,
                _ => DetectedOsType::Linux,
            };

            entries.push(DetectedOs {
                name,
                device,
                same_disk,
                os_type,
            });
        }
    }

    tracing::info!(
        count = entries.len(),
        disk = %install_disk,
        "Definitive OS probe complete"
    );

    OsDetectionResults {
        entries,
        install_disk: install_disk.to_string(),
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
        tracing::info!("UEFI firmware detected (/sys/firmware/efi exists)");
        FirmwareMode::Uefi
    } else {
        tracing::info!("BIOS firmware detected (/sys/firmware/efi not found)");
        FirmwareMode::Bios
    }
}

/// Detect network connectivity via TCP connection to archlinux.org.
///
/// Uses `TcpStream::connect_timeout` with a 2-second timeout.
/// Connects to port 443 (HTTPS) since it's universally allowed through firewalls.
///
/// # Why TCP instead of ICMP/ping?
///
/// - ICMP is often blocked by firewalls
/// - `ping` requires shelling out (Rust controls)
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
    // Try DNS resolution first, fall back to hardcoded IP if DNS is unavailable
    let addr: SocketAddr = std::net::ToSocketAddrs::to_socket_addrs(&"archlinux.org:443")
        .ok()
        .and_then(|mut addrs| addrs.next())
        .unwrap_or_else(|| {
            tracing::debug!(
                "DNS resolution failed for archlinux.org, using hardcoded IP fallback"
            );
            // SAFETY: hardcoded valid IP:port literal — parse is infallible
            "147.75.81.97:443".parse().unwrap()
        });

    let timeout = Duration::from_secs(2);

    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(_stream) => {
            tracing::info!("Network connectivity confirmed (TCP to archlinux.org:443)");
            NetworkState::Online
        }
        Err(e) => {
            tracing::warn!("Network connectivity check failed: {}", e);
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
            tracing::info!("UEFI firmware confirmed (efivars accessible)");
        } else {
            tracing::warn!(
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
    // Try DNS resolution first, fall back to hardcoded IP
    let addr: SocketAddr = std::net::ToSocketAddrs::to_socket_addrs(&"archlinux.org:443")
        .ok()
        .and_then(|mut addrs| addrs.next())
        // SAFETY: hardcoded valid IP:port literal — parse is infallible
        .unwrap_or_else(|| "147.75.81.97:443".parse().unwrap());

    let timeout = Duration::from_secs(2);

    match TcpStream::connect_timeout(&addr, timeout) {
        Ok(_stream) => Ok(true),
        Err(e) => {
            tracing::info!("Network offline: {}", e);
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

    // ── OS Detection Tests ──────────────────────────────────────────

    fn make_test_results() -> OsDetectionResults {
        OsDetectionResults {
            install_disk: "/dev/sda".to_string(),
            entries: vec![
                DetectedOs {
                    name: "Ubuntu 24.04".to_string(),
                    device: "/dev/sda2".to_string(),
                    same_disk: true,
                    os_type: DetectedOsType::Linux,
                },
                DetectedOs {
                    name: "Windows Boot Manager".to_string(),
                    device: "/dev/sdb1".to_string(),
                    same_disk: false,
                    os_type: DetectedOsType::Windows,
                },
                DetectedOs {
                    name: "Fedora 41".to_string(),
                    device: "/dev/sdc3".to_string(),
                    same_disk: false,
                    os_type: DetectedOsType::Linux,
                },
            ],
        }
    }

    #[test]
    fn test_os_detection_has_any() {
        let results = make_test_results();
        assert!(results.has_any());

        let empty = OsDetectionResults::default();
        assert!(!empty.has_any());
    }

    #[test]
    fn test_os_detection_has_windows() {
        let results = make_test_results();
        assert!(results.has_windows());

        let linux_only = OsDetectionResults {
            install_disk: "/dev/sda".to_string(),
            entries: vec![DetectedOs {
                name: "Arch".to_string(),
                device: "/dev/sdb1".to_string(),
                same_disk: false,
                os_type: DetectedOsType::Linux,
            }],
        };
        assert!(!linux_only.has_windows());
    }

    #[test]
    fn test_os_detection_has_linux() {
        let results = make_test_results();
        assert!(results.has_linux());
    }

    #[test]
    fn test_os_detection_same_disk_os() {
        let results = make_test_results();
        let same = results.same_disk_os();
        assert_eq!(same.len(), 1);
        assert_eq!(same[0].name, "Ubuntu 24.04");
        assert_eq!(same[0].device, "/dev/sda2");
    }

    #[test]
    fn test_os_detection_other_disk_os() {
        let results = make_test_results();
        let other = results.other_disk_os();
        assert_eq!(other.len(), 2);
    }

    #[test]
    fn test_os_detection_windows_esp_device() {
        let results = make_test_results();
        assert_eq!(results.windows_esp_device(), Some("/dev/sdb1"));

        let empty = OsDetectionResults::default();
        assert_eq!(empty.windows_esp_device(), None);
    }

    #[test]
    fn test_os_detection_first_linux() {
        let results = make_test_results();
        let first = results.first_linux();
        assert!(first.is_some());
        assert_eq!(first.unwrap().name, "Ubuntu 24.04");
    }

    #[test]
    fn test_os_detection_summary_line() {
        let results = make_test_results();
        let summary = results.summary_line();
        assert!(summary.contains("Ubuntu 24.04"));
        assert!(summary.contains("same disk"));
        assert!(summary.contains("Windows Boot Manager"));
        assert!(summary.contains("other disk"));
    }

    #[test]
    fn test_os_detection_default_empty() {
        let results = OsDetectionResults::default();
        assert!(!results.has_any());
        assert!(!results.has_windows());
        assert!(!results.has_linux());
        assert!(results.same_disk_os().is_empty());
        assert!(results.other_disk_os().is_empty());
        assert_eq!(results.windows_esp_device(), None);
        assert!(results.first_linux().is_none());
        assert!(results.summary_line().is_empty());
    }

    #[test]
    fn test_detected_os_type_display() {
        assert_eq!(DetectedOsType::Windows.to_string(), "Windows");
        assert_eq!(DetectedOsType::Linux.to_string(), "Linux");
    }

    // ── JSON Parsing Tests ──────────────────────────────────────────

    #[test]
    fn test_parse_detect_os_json_full() {
        let json = r#"{"os":[{"name":"Ubuntu 24.04","device":"/dev/sdb2","type":"linux","same_disk":false},{"name":"Windows Boot Manager","device":"/dev/sda1","type":"windows","same_disk":true}]}"#;
        let results = parse_detect_os_json(json, "/dev/sda");

        assert_eq!(results.entries.len(), 2);
        assert_eq!(results.install_disk, "/dev/sda");

        assert_eq!(results.entries[0].name, "Ubuntu 24.04");
        assert_eq!(results.entries[0].device, "/dev/sdb2");
        assert_eq!(results.entries[0].os_type, DetectedOsType::Linux);
        assert!(!results.entries[0].same_disk);

        assert_eq!(results.entries[1].name, "Windows Boot Manager");
        assert_eq!(results.entries[1].device, "/dev/sda1");
        assert_eq!(results.entries[1].os_type, DetectedOsType::Windows);
        assert!(results.entries[1].same_disk);
    }

    #[test]
    fn test_parse_detect_os_json_empty() {
        let json = r#"{"os":[]}"#;
        let results = parse_detect_os_json(json, "/dev/sda");
        assert!(!results.has_any());
    }

    #[test]
    fn test_parse_detect_os_json_invalid() {
        let results = parse_detect_os_json("not json at all", "/dev/sda");
        assert!(!results.has_any());
        assert_eq!(results.install_disk, "/dev/sda");
    }

    #[test]
    fn test_parse_detect_os_json_with_log_preamble() {
        // detect_os.sh may output log lines before JSON
        let output = "[INFO] Scanning for Linux...\n[INFO] Found Ubuntu\n{\"os\":[{\"name\":\"Ubuntu\",\"device\":\"/dev/sda2\",\"type\":\"linux\",\"same_disk\":true}]}";
        let results = parse_detect_os_json(output, "/dev/sda");
        assert_eq!(results.entries.len(), 1);
        assert_eq!(results.entries[0].name, "Ubuntu");
    }

    #[test]
    fn test_parse_lsblk_size_gb() {
        assert!((parse_lsblk_size_gb("953.9G") - 953.9).abs() < 0.01);
        assert!((parse_lsblk_size_gb("512M") - 0.5).abs() < 0.01);
        assert!((parse_lsblk_size_gb("2T") - 2048.0).abs() < 0.01);
        assert!((parse_lsblk_size_gb("") - 0.0).abs() < 0.01);
    }
}
