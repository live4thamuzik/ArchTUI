//! Type-safe arguments for network tool scripts.
//!
//! This module provides typed argument structs for network-related scripts:
//! - `ConfigureNetworkArgs` for `configure_network.sh`
//! - `TestNetworkArgs` for `test_network.sh`
//! - `FirewallArgs` for `configure_firewall.sh`
//! - `NetworkDiagnosticsArgs` for `network_diagnostics.sh`

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
}
