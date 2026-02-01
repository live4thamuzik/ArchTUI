//! Type-safe arguments for user tool scripts.
//!
//! This module provides typed argument structs for user-related scripts:
//! - `AddUserArgs` for `add_user.sh`
//! - `ResetPasswordArgs` for `reset_password.sh`
//! - `GroupsArgs` for `manage_groups.sh`
//! - `SshArgs` for `configure_ssh.sh`
//! - `SecurityAuditArgs` for `security_audit.sh`

use crate::script_traits::ScriptArgs;

// ============================================================================
// Add User
// ============================================================================

/// Type-safe arguments for `scripts/tools/add_user.sh`.
#[derive(Debug, Clone)]
pub struct AddUserArgs {
    /// Username to create.
    pub username: String,
    /// Shell for the user.
    pub shell: String,
    /// Optional full name.
    pub full_name: Option<String>,
    /// Optional comma-separated groups.
    pub groups: Option<String>,
}

impl ScriptArgs for AddUserArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec![
            "--username".to_string(),
            self.username.clone(),
            "--shell".to_string(),
            self.shell.clone(),
        ];
        if let Some(ref name) = self.full_name {
            args.push("--full-name".to_string());
            args.push(name.clone());
        }
        if let Some(ref grps) = self.groups {
            args.push("--groups".to_string());
            args.push(grps.clone());
        }
        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "add_user.sh"
    }
}

// ============================================================================
// Reset Password
// ============================================================================

/// Type-safe arguments for `scripts/tools/reset_password.sh`.
#[derive(Debug, Clone)]
pub struct ResetPasswordArgs {
    /// Username to reset password for.
    pub username: String,
}

impl ScriptArgs for ResetPasswordArgs {
    fn to_cli_args(&self) -> Vec<String> {
        vec!["--username".to_string(), self.username.clone()]
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "reset_password.sh"
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
}
