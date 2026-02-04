//! Type-safe arguments for network tool scripts.
//!
//! This module provides typed argument structs for network-related scripts:
//! - `ConfigureNetworkArgs` for `configure_network.sh`
//! - `TestNetworkArgs` for `test_network.sh`
//! - `FirewallArgs` for `configure_firewall.sh`
//! - `NetworkDiagnosticsArgs` for `network_diagnostics.sh`
//! - `UpdateMirrorsArgs` for `update_mirrors.sh` (Sprint 13)

use crate::script_traits::ScriptArgs;

// ============================================================================
// Configure Network
// ============================================================================

/// Type-safe arguments for `scripts/tools/configure_network.sh`.
#[derive(Debug, Clone)]
pub struct ConfigureNetworkArgs {
    /// Network interface name.
    pub interface: String,
    /// Optional IP address.
    pub ip: Option<String>,
    /// Optional gateway address.
    pub gateway: Option<String>,
}

impl ScriptArgs for ConfigureNetworkArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec!["--interface".to_string(), self.interface.clone()];
        if let Some(ref ip) = self.ip {
            args.push("--ip".to_string());
            args.push(ip.clone());
        }
        if let Some(ref gw) = self.gateway {
            args.push("--gateway".to_string());
            args.push(gw.clone());
        }
        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "configure_network.sh"
    }

    /// Network configuration is DESTRUCTIVE - modifies network settings.
    fn is_destructive(&self) -> bool {
        true
    }
}

// ============================================================================
// Test Network
// ============================================================================

/// Type-safe arguments for `scripts/tools/test_network.sh`.
#[derive(Debug, Clone)]
pub struct TestNetworkArgs {
    /// Action to perform (e.g., `ping`, `dns`, `connectivity`).
    pub action: String,
    /// Optional host to test.
    pub host: Option<String>,
    /// Timeout in seconds.
    pub timeout: u32,
}

impl ScriptArgs for TestNetworkArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec!["--action".to_string(), self.action.clone()];
        if let Some(ref h) = self.host {
            args.push("--host".to_string());
            args.push(h.clone());
        }
        args.push("--timeout".to_string());
        args.push(self.timeout.to_string());
        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "test_network.sh"
    }

    /// Network test is READ-ONLY - not destructive.
    fn is_destructive(&self) -> bool {
        false
    }
}

// ============================================================================
// Configure Firewall
// ============================================================================

/// Type-safe arguments for `scripts/tools/configure_firewall.sh`.
#[derive(Debug, Clone)]
pub struct FirewallArgs {
    /// Action to perform.
    pub action: String,
    /// Firewall type.
    pub firewall_type: String,
    /// Optional port number.
    pub port: Option<u16>,
    /// Protocol (tcp/udp).
    pub protocol: String,
    /// Allow flag.
    pub allow: bool,
    /// Deny flag.
    pub deny: bool,
}

impl ScriptArgs for FirewallArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec![
            "--action".to_string(),
            self.action.clone(),
            "--type".to_string(),
            self.firewall_type.clone(),
        ];
        if let Some(p) = self.port {
            args.push("--port".to_string());
            args.push(p.to_string());
        }
        args.push("--protocol".to_string());
        args.push(self.protocol.clone());
        if self.allow {
            args.push("--allow".to_string());
        }
        if self.deny {
            args.push("--deny".to_string());
        }
        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "configure_firewall.sh"
    }

    /// Firewall configuration is DESTRUCTIVE - modifies firewall rules.
    fn is_destructive(&self) -> bool {
        true
    }
}

// ============================================================================
// Network Diagnostics
// ============================================================================

/// Type-safe arguments for `scripts/tools/network_diagnostics.sh`.
#[derive(Debug, Clone)]
pub struct NetworkDiagnosticsArgs {
    /// Action to perform.
    pub action: String,
}

impl ScriptArgs for NetworkDiagnosticsArgs {
    fn to_cli_args(&self) -> Vec<String> {
        vec!["--action".to_string(), self.action.clone()]
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "network_diagnostics.sh"
    }

    /// Network diagnostics is READ-ONLY - not destructive.
    fn is_destructive(&self) -> bool {
        false
    }
}

// ============================================================================
// Update Mirrors (Sprint 13)
// ============================================================================

/// Mirror sort method for reflector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Library API - all variants used by consumer code
pub enum MirrorSortMethod {
    /// Sort by download rate (fastest first).
    Rate,
    /// Sort by last synchronization time.
    Age,
    /// Sort by country.
    Country,
    /// Sort by mirror score.
    Score,
}

impl MirrorSortMethod {
    /// Get the reflector argument value.
    pub fn as_str(&self) -> &'static str {
        match self {
            MirrorSortMethod::Rate => "rate",
            MirrorSortMethod::Age => "age",
            MirrorSortMethod::Country => "country",
            MirrorSortMethod::Score => "score",
        }
    }
}

impl Default for MirrorSortMethod {
    fn default() -> Self {
        MirrorSortMethod::Rate
    }
}

impl std::fmt::Display for MirrorSortMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Type-safe arguments for `scripts/tools/update_mirrors.sh`.
///
/// Updates the pacman mirrorlist using reflector for faster downloads.
///
/// # Field to Flag Mapping
///
/// | Rust Field | CLI Flag    | Notes |
/// |------------|-------------|-------|
/// | `country`  | `--country` | Optional country filter (ISO 3166-1) |
/// | `limit`    | `--limit`   | Number of mirrors to keep (default: 20) |
/// | `sort`     | `--sort`    | Sort method (default: rate) |
/// | `protocol` | `--protocol`| Protocol filter (https) |
/// | `save`     | `--save`    | Whether to save to mirrorlist |
///
/// # Network Requirement
///
/// This script requires network connectivity. It will fail gracefully
/// if the network is down and return a non-zero exit code.
///
/// # Example
///
/// ```ignore
/// use archtui::scripts::network::{UpdateMirrorsArgs, MirrorSortMethod};
///
/// let args = UpdateMirrorsArgs {
///     country: Some("US".to_string()),
///     limit: 20,
///     sort: MirrorSortMethod::Rate,
///     protocol: Some("https".to_string()),
///     save: true,
/// };
/// ```
#[derive(Debug, Clone)]
pub struct UpdateMirrorsArgs {
    /// Optional country filter (ISO 3166-1 alpha-2 code, e.g., "US", "DE").
    /// If None, uses all countries.
    pub country: Option<String>,
    /// Number of mirrors to keep in the list.
    pub limit: u32,
    /// Sort method (rate, age, country, score).
    pub sort: MirrorSortMethod,
    /// Protocol filter (default: https).
    pub protocol: Option<String>,
    /// Whether to save the result to /etc/pacman.d/mirrorlist.
    pub save: bool,
}

impl Default for UpdateMirrorsArgs {
    fn default() -> Self {
        Self {
            country: None,
            limit: 20,
            sort: MirrorSortMethod::Rate,
            protocol: Some("https".to_string()),
            save: true,
        }
    }
}

impl ScriptArgs for UpdateMirrorsArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec![
            "--limit".to_string(),
            self.limit.to_string(),
            "--sort".to_string(),
            self.sort.as_str().to_string(),
        ];

        if let Some(ref country) = self.country {
            args.push("--country".to_string());
            args.push(country.clone());
        }

        if let Some(ref protocol) = self.protocol {
            args.push("--protocol".to_string());
            args.push(protocol.clone());
        }

        if self.save {
            args.push("--save".to_string());
        }

        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "update_mirrors.sh"
    }

    /// Mirror update is DESTRUCTIVE - modifies /etc/pacman.d/mirrorlist.
    fn is_destructive(&self) -> bool {
        self.save
    }
}

// ============================================================================
// Check Network Connectivity
// ============================================================================

/// Type-safe arguments for network connectivity check.
///
/// Simple connectivity test before operations that require network.
#[derive(Debug, Clone)]
pub struct CheckConnectivityArgs {
    /// Host to ping (default: archlinux.org).
    pub host: String,
    /// Timeout in seconds.
    pub timeout: u32,
}

impl Default for CheckConnectivityArgs {
    fn default() -> Self {
        Self {
            host: "archlinux.org".to_string(),
            timeout: 5,
        }
    }
}

impl ScriptArgs for CheckConnectivityArgs {
    fn to_cli_args(&self) -> Vec<String> {
        vec![
            "--action".to_string(),
            "connectivity".to_string(),
            "--host".to_string(),
            self.host.clone(),
            "--timeout".to_string(),
            self.timeout.to_string(),
        ]
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "test_network.sh"
    }

    /// Connectivity check is READ-ONLY.
    fn is_destructive(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_mirrors_args_default() {
        let args = UpdateMirrorsArgs::default();
        assert_eq!(args.limit, 20);
        assert_eq!(args.sort, MirrorSortMethod::Rate);
        assert_eq!(args.protocol, Some("https".to_string()));
        assert!(args.save);
        assert!(args.country.is_none());
    }

    #[test]
    fn test_update_mirrors_args_with_country() {
        let args = UpdateMirrorsArgs {
            country: Some("US".to_string()),
            limit: 10,
            sort: MirrorSortMethod::Score,
            protocol: Some("https".to_string()),
            save: true,
        };

        let cli_args = args.to_cli_args();
        assert!(cli_args.contains(&"--country".to_string()));
        assert!(cli_args.contains(&"US".to_string()));
        assert!(cli_args.contains(&"--limit".to_string()));
        assert!(cli_args.contains(&"10".to_string()));
        assert!(cli_args.contains(&"--sort".to_string()));
        assert!(cli_args.contains(&"score".to_string()));
        assert!(cli_args.contains(&"--save".to_string()));

        assert!(args.is_destructive()); // save=true
        assert_eq!(args.script_name(), "update_mirrors.sh");
    }

    #[test]
    fn test_update_mirrors_args_no_save() {
        let args = UpdateMirrorsArgs {
            country: None,
            limit: 20,
            sort: MirrorSortMethod::Rate,
            protocol: None,
            save: false,
        };

        let cli_args = args.to_cli_args();
        assert!(!cli_args.contains(&"--save".to_string()));
        assert!(!cli_args.contains(&"--country".to_string()));
        assert!(!cli_args.contains(&"--protocol".to_string()));

        // Not destructive if not saving
        assert!(!args.is_destructive());
    }

    #[test]
    fn test_mirror_sort_methods() {
        assert_eq!(MirrorSortMethod::Rate.as_str(), "rate");
        assert_eq!(MirrorSortMethod::Age.as_str(), "age");
        assert_eq!(MirrorSortMethod::Country.as_str(), "country");
        assert_eq!(MirrorSortMethod::Score.as_str(), "score");
    }

    #[test]
    fn test_check_connectivity_args() {
        let args = CheckConnectivityArgs::default();
        assert_eq!(args.host, "archlinux.org");
        assert_eq!(args.timeout, 5);

        let cli_args = args.to_cli_args();
        assert!(cli_args.contains(&"--action".to_string()));
        assert!(cli_args.contains(&"connectivity".to_string()));

        assert!(!args.is_destructive());
    }
}
