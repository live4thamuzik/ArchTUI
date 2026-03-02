//! Type-safe arguments for user tool scripts.
//!
//! This module provides typed argument structs for user-related scripts:
//! - `ResetPasswordArgs` for `reset_password.sh`
//! - `GroupsArgs` for `manage_groups.sh`
//! - `SshArgs` for `configure_ssh.sh`
//! - `SecurityAuditArgs` for `security_audit.sh`
//!
//! Note: `add_user.sh` uses `UserAddArgs` from `scripts::config` (secure password via env var).

use crate::script_traits::ScriptArgs;

// ============================================================================
// Reset Password
// ============================================================================

/// Type-safe arguments for `scripts/tools/reset_password.sh`.
#[derive(Clone)]
pub struct ResetPasswordArgs {
    /// Username to reset password for.
    pub username: String,
    /// New password (passed via env var, never on CLI).
    pub password: String,
}

// ROE §8.1: Custom Debug impl redacts password field
impl std::fmt::Debug for ResetPasswordArgs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ResetPasswordArgs")
            .field("username", &self.username)
            .field("password", &"********")
            .finish()
    }
}

impl ScriptArgs for ResetPasswordArgs {
    fn to_cli_args(&self) -> Vec<String> {
        vec!["--username".to_string(), self.username.clone()]
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![("USER_PASSWORD".to_string(), self.password.clone())]
    }

    fn script_name(&self) -> &'static str {
        "reset_password.sh"
    }

    /// Password reset is DESTRUCTIVE - modifies /etc/shadow.
    fn is_destructive(&self) -> bool {
        true
    }
}

// ============================================================================
// Manage Groups
// ============================================================================

/// Type-safe arguments for `scripts/tools/manage_groups.sh`.
#[derive(Debug, Clone)]
pub struct GroupsArgs {
    /// Action to perform (e.g., `add`, `remove`, `list`).
    pub action: String,
    /// Optional user to operate on.
    pub user: Option<String>,
    /// Optional group to operate on.
    pub group: Option<String>,
}

impl ScriptArgs for GroupsArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec!["--action".to_string(), self.action.clone()];
        if let Some(ref u) = self.user {
            args.push("--user".to_string());
            args.push(u.clone());
        }
        if let Some(ref g) = self.group {
            args.push("--group".to_string());
            args.push(g.clone());
        }
        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "manage_groups.sh"
    }

    /// Group management is DESTRUCTIVE - modifies /etc/group.
    fn is_destructive(&self) -> bool {
        true
    }
}

// ============================================================================
// Configure SSH
// ============================================================================

/// Type-safe arguments for `scripts/tools/configure_ssh.sh`.
#[derive(Debug, Clone)]
pub struct SshArgs {
    /// Action to perform.
    pub action: String,
    /// Optional port number.
    pub port: Option<u16>,
    /// Enable root login.
    pub enable_root_login: Option<bool>,
    /// Enable password authentication.
    pub enable_password_auth: Option<bool>,
}

impl ScriptArgs for SshArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec!["--action".to_string(), self.action.clone()];
        if let Some(p) = self.port {
            args.push("--port".to_string());
            args.push(p.to_string());
        }
        if let Some(enable) = self.enable_root_login {
            if enable {
                args.push("--enable-root-login".to_string());
            } else {
                args.push("--disable-root-login".to_string());
            }
        }
        if let Some(enable) = self.enable_password_auth {
            if enable {
                args.push("--enable-password-auth".to_string());
            } else {
                args.push("--disable-password-auth".to_string());
            }
        }
        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "configure_ssh.sh"
    }

    /// SSH configuration is DESTRUCTIVE - modifies sshd_config.
    fn is_destructive(&self) -> bool {
        true
    }
}

// ============================================================================
// Security Audit
// ============================================================================

/// Type-safe arguments for `scripts/tools/security_audit.sh`.
#[derive(Debug, Clone)]
pub struct SecurityAuditArgs {
    /// Action to perform.
    pub action: String,
}

impl ScriptArgs for SecurityAuditArgs {
    fn to_cli_args(&self) -> Vec<String> {
        vec!["--action".to_string(), self.action.clone()]
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "security_audit.sh"
    }

    /// Security audit is READ-ONLY - not destructive.
    fn is_destructive(&self) -> bool {
        false
    }
}
