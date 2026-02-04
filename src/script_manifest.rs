//! Script Execution Contracts
//!
//! This module defines the contract between Rust and Bash scripts, ensuring:
//! - Scripts declare their requirements explicitly
//! - Rust validates all requirements before execution
//! - Bash scripts refuse to run without required environment variables
//!
//! # Design Principles
//!
//! 1. **Explicit Contracts**: Every script must have a manifest declaring its requirements
//! 2. **Fail Fast**: Rust refuses to execute scripts with unsatisfied requirements
//! 3. **Defense in Depth**: Bash scripts also validate their own requirements
//! 4. **No Implicit Dependencies**: All environment variables must be declared
//!
//! # Manifest Format
//!
//! Manifests are JSON files with the following structure:
//! ```json
//! {
//!   "script": "scripts/tools/wipe_disk.sh",
//!   "description": "Securely wipe a disk",
//!   "destructive": true,
//!   "required_confirmation": "CONFIRM_WIPE_DISK",
//!   "required_env": [
//!     { "name": "INSTALL_DISK", "description": "Target disk device path", "pattern": "^/dev/" }
//!   ],
//!   "optional_env": [
//!     { "name": "WIPE_METHOD", "description": "Wipe method", "default": "quick" }
//!   ]
//! }
//! ```

// Library API - these types are exported for external use but not yet consumed by the binary
#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Errors that can occur during manifest operations
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ManifestError {
    /// Manifest file not found
    #[error("Manifest not found for script: {script}")]
    NotFound { script: String },

    /// Invalid manifest format
    #[error("Invalid manifest format: {reason}")]
    InvalidFormat { reason: String },

    /// Required environment variable missing
    #[error("Required environment variable '{name}' is not set (script: {script})")]
    MissingEnvVar { script: String, name: String },

    /// Environment variable value doesn't match required pattern
    #[error("Environment variable '{name}' has invalid value '{value}': {reason}")]
    InvalidEnvValue {
        name: String,
        value: String,
        reason: String,
    },

    /// Destructive script missing confirmation
    #[error(
        "Destructive script '{script}' requires confirmation: set {confirmation}=yes in environment"
    )]
    MissingConfirmation { script: String, confirmation: String },

    /// Script file not found
    #[error("Script file not found: {path}")]
    ScriptNotFound { path: String },

    /// IO error reading manifest
    #[error("Failed to read manifest: {reason}")]
    IoError { reason: String },
}

impl From<std::io::Error> for ManifestError {
    fn from(err: std::io::Error) -> Self {
        ManifestError::IoError {
            reason: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for ManifestError {
    fn from(err: serde_json::Error) -> Self {
        ManifestError::InvalidFormat {
            reason: err.to_string(),
        }
    }
}

/// Environment variable requirement
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EnvRequirement {
    /// Variable name (e.g., "INSTALL_DISK")
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Optional regex pattern for validation (e.g., "^/dev/")
    #[serde(default)]
    pub pattern: Option<String>,

    /// Whether the variable can be empty
    #[serde(default)]
    pub allow_empty: bool,
}

impl EnvRequirement {
    /// Create a new required environment variable
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            pattern: None,
            allow_empty: false,
        }
    }

    /// Add a validation pattern
    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.pattern = Some(pattern.into());
        self
    }

    /// Allow empty values
    pub fn allow_empty(mut self) -> Self {
        self.allow_empty = true;
        self
    }

    /// Validate a value against this requirement
    pub fn validate(&self, value: &str) -> Result<(), ManifestError> {
        // Check for empty values
        if value.is_empty() && !self.allow_empty {
            return Err(ManifestError::InvalidEnvValue {
                name: self.name.clone(),
                value: value.to_string(),
                reason: "value cannot be empty".to_string(),
            });
        }

        // Check pattern if specified
        if let Some(ref pattern) = self.pattern {
            // Simple pattern matching (prefix/suffix/contains)
            // Full regex would require the regex crate
            let matches = if pattern.starts_with('^') && pattern.ends_with('$') {
                // Exact match (minus anchors)
                let inner = &pattern[1..pattern.len() - 1];
                value == inner
            } else if pattern.starts_with('^') {
                // Prefix match
                value.starts_with(&pattern[1..])
            } else if pattern.ends_with('$') {
                // Suffix match
                value.ends_with(&pattern[..pattern.len() - 1])
            } else {
                // Contains match
                value.contains(pattern)
            };

            if !matches {
                return Err(ManifestError::InvalidEnvValue {
                    name: self.name.clone(),
                    value: value.to_string(),
                    reason: format!("value must match pattern: {}", pattern),
                });
            }
        }

        Ok(())
    }
}

/// Optional environment variable with default
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OptionalEnv {
    /// Variable name
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Default value if not set
    pub default: String,
}

impl OptionalEnv {
    /// Create a new optional environment variable
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        default: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            default: default.into(),
        }
    }

    /// Get the value, using default if not set in environment
    pub fn get_value(&self, env: &HashMap<String, String>) -> String {
        env.get(&self.name)
            .cloned()
            .unwrap_or_else(|| self.default.clone())
    }
}

/// Script manifest defining the execution contract
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScriptManifest {
    /// Path to the script (relative to scripts directory)
    pub script: String,

    /// Human-readable description of what the script does
    pub description: String,

    /// Whether this script performs destructive operations (disk writes, etc.)
    #[serde(default)]
    pub destructive: bool,

    /// Required confirmation environment variable for destructive scripts
    /// Must be set to "yes" for the script to execute
    #[serde(default)]
    pub required_confirmation: Option<String>,

    /// Required environment variables (script will fail without these)
    #[serde(default)]
    pub required_env: Vec<EnvRequirement>,

    /// Optional environment variables with defaults
    #[serde(default)]
    pub optional_env: Vec<OptionalEnv>,

    /// Expected exit codes (default: [0] for success only)
    #[serde(default = "default_exit_codes")]
    pub valid_exit_codes: Vec<i32>,

    /// Whether the script needs stdin (for password passing, etc.)
    #[serde(default)]
    pub needs_stdin: bool,

    /// Script version for compatibility tracking
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_exit_codes() -> Vec<i32> {
    vec![0]
}

fn default_version() -> String {
    "1.0".to_string()
}

impl ScriptManifest {
    /// Create a new manifest builder
    pub fn builder(script: impl Into<String>, description: impl Into<String>) -> ManifestBuilder {
        ManifestBuilder::new(script, description)
    }

    /// Load a manifest from a JSON file
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let content = std::fs::read_to_string(path)?;
        let manifest: Self = serde_json::from_str(&content)?;
        manifest.validate_structure()?;
        Ok(manifest)
    }

    /// Load a manifest from a JSON string
    pub fn from_json(json: &str) -> Result<Self, ManifestError> {
        let manifest: Self = serde_json::from_str(json)?;
        manifest.validate_structure()?;
        Ok(manifest)
    }

    /// Validate the manifest structure itself
    fn validate_structure(&self) -> Result<(), ManifestError> {
        // Destructive scripts must have a confirmation requirement
        if self.destructive && self.required_confirmation.is_none() {
            return Err(ManifestError::InvalidFormat {
                reason: format!(
                    "Destructive script '{}' must specify required_confirmation",
                    self.script
                ),
            });
        }

        // Check for duplicate environment variable names
        let mut seen = std::collections::HashSet::new();
        for req in &self.required_env {
            if !seen.insert(&req.name) {
                return Err(ManifestError::InvalidFormat {
                    reason: format!("Duplicate required_env: {}", req.name),
                });
            }
        }
        for opt in &self.optional_env {
            if !seen.insert(&opt.name) {
                return Err(ManifestError::InvalidFormat {
                    reason: format!("Duplicate optional_env: {}", opt.name),
                });
            }
        }

        Ok(())
    }

    /// Validate that all requirements are met for execution
    ///
    /// # Arguments
    /// * `env` - Environment variables that will be passed to the script
    /// * `scripts_dir` - Base directory for scripts (to verify script exists)
    ///
    /// # Returns
    /// * `Ok(ValidatedExecution)` if all requirements are met
    /// * `Err(ManifestError)` describing what's missing
    pub fn validate_execution(
        &self,
        env: &HashMap<String, String>,
        scripts_dir: Option<&Path>,
    ) -> Result<ValidatedExecution, ManifestError> {
        // Check script file exists
        if let Some(base_dir) = scripts_dir {
            let script_path = base_dir.join(&self.script);
            if !script_path.exists() {
                return Err(ManifestError::ScriptNotFound {
                    path: script_path.display().to_string(),
                });
            }
        }

        // Check confirmation for destructive scripts
        if self.destructive {
            if let Some(ref confirmation_var) = self.required_confirmation {
                let confirmation_value = env.get(confirmation_var).map(|s| s.as_str()).unwrap_or("");
                if confirmation_value != "yes" {
                    return Err(ManifestError::MissingConfirmation {
                        script: self.script.clone(),
                        confirmation: confirmation_var.clone(),
                    });
                }
            }
        }

        // Check required environment variables
        for req in &self.required_env {
            match env.get(&req.name) {
                None => {
                    return Err(ManifestError::MissingEnvVar {
                        script: self.script.clone(),
                        name: req.name.clone(),
                    });
                }
                Some(value) => {
                    req.validate(value)?;
                }
            }
        }

        // Build final environment with defaults applied
        let mut final_env = env.clone();
        for opt in &self.optional_env {
            if !final_env.contains_key(&opt.name) {
                final_env.insert(opt.name.clone(), opt.default.clone());
            }
        }

        Ok(ValidatedExecution {
            script_path: PathBuf::from(&self.script),
            environment: final_env,
            needs_stdin: self.needs_stdin,
            valid_exit_codes: self.valid_exit_codes.clone(),
        })
    }

    /// Generate a Bash header comment documenting the contract
    pub fn to_bash_header(&self) -> String {
        let mut lines = Vec::new();

        lines.push(format!("# {}", self.script));
        lines.push(format!("# {}", self.description));
        lines.push("#".to_string());
        lines.push("# ENVIRONMENT CONTRACT:".to_string());

        if self.destructive {
            if let Some(ref conf) = self.required_confirmation {
                lines.push(format!("#   {}=yes   Required for execution", conf));
            }
        }

        if !self.required_env.is_empty() {
            lines.push("#".to_string());
            lines.push("# REQUIRED ENVIRONMENT VARIABLES:".to_string());
            for req in &self.required_env {
                let pattern_note = req
                    .pattern
                    .as_ref()
                    .map(|p| format!(" (pattern: {})", p))
                    .unwrap_or_default();
                lines.push(format!("#   {} - {}{}", req.name, req.description, pattern_note));
            }
        }

        if !self.optional_env.is_empty() {
            lines.push("#".to_string());
            lines.push("# OPTIONAL ENVIRONMENT VARIABLES:".to_string());
            for opt in &self.optional_env {
                lines.push(format!(
                    "#   {} - {} (default: {})",
                    opt.name, opt.description, opt.default
                ));
            }
        }

        lines.push("#".to_string());
        lines.push("# This script is NON-INTERACTIVE.".to_string());

        lines.join("\n")
    }
}

/// Result of successful validation, ready for execution
#[derive(Debug, Clone)]
pub struct ValidatedExecution {
    /// Path to the script
    pub script_path: PathBuf,

    /// Environment variables to pass (with defaults applied)
    pub environment: HashMap<String, String>,

    /// Whether the script needs stdin input
    pub needs_stdin: bool,

    /// Valid exit codes for this script
    pub valid_exit_codes: Vec<i32>,
}

impl ValidatedExecution {
    /// Check if an exit code is valid for this script
    pub fn is_valid_exit_code(&self, code: i32) -> bool {
        self.valid_exit_codes.contains(&code)
    }
}

/// Builder for creating ScriptManifest instances
#[derive(Debug, Clone)]
pub struct ManifestBuilder {
    script: String,
    description: String,
    destructive: bool,
    required_confirmation: Option<String>,
    required_env: Vec<EnvRequirement>,
    optional_env: Vec<OptionalEnv>,
    valid_exit_codes: Vec<i32>,
    needs_stdin: bool,
    version: String,
}

impl ManifestBuilder {
    /// Create a new builder
    pub fn new(script: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            script: script.into(),
            description: description.into(),
            destructive: false,
            required_confirmation: None,
            required_env: Vec::new(),
            optional_env: Vec::new(),
            valid_exit_codes: vec![0],
            needs_stdin: false,
            version: "1.0".to_string(),
        }
    }

    /// Mark as destructive with required confirmation variable
    pub fn destructive(mut self, confirmation_var: impl Into<String>) -> Self {
        self.destructive = true;
        self.required_confirmation = Some(confirmation_var.into());
        self
    }

    /// Add a required environment variable
    pub fn require_env(mut self, req: EnvRequirement) -> Self {
        self.required_env.push(req);
        self
    }

    /// Add an optional environment variable with default
    pub fn optional_env(mut self, opt: OptionalEnv) -> Self {
        self.optional_env.push(opt);
        self
    }

    /// Set valid exit codes
    pub fn valid_exit_codes(mut self, codes: Vec<i32>) -> Self {
        self.valid_exit_codes = codes;
        self
    }

    /// Mark as needing stdin
    pub fn needs_stdin(mut self) -> Self {
        self.needs_stdin = true;
        self
    }

    /// Set version
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Build the manifest
    pub fn build(self) -> Result<ScriptManifest, ManifestError> {
        let manifest = ScriptManifest {
            script: self.script,
            description: self.description,
            destructive: self.destructive,
            required_confirmation: self.required_confirmation,
            required_env: self.required_env,
            optional_env: self.optional_env,
            valid_exit_codes: self.valid_exit_codes,
            needs_stdin: self.needs_stdin,
            version: self.version,
        };
        manifest.validate_structure()?;
        Ok(manifest)
    }
}

/// Registry of known script manifests
#[derive(Debug, Default)]
pub struct ManifestRegistry {
    manifests: HashMap<String, ScriptManifest>,
}

impl ManifestRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a manifest
    pub fn register(&mut self, manifest: ScriptManifest) {
        self.manifests.insert(manifest.script.clone(), manifest);
    }

    /// Get a manifest by script path
    pub fn get(&self, script: &str) -> Option<&ScriptManifest> {
        self.manifests.get(script)
    }

    /// Load all manifests from a directory
    pub fn load_from_directory(&mut self, dir: impl AsRef<Path>) -> Result<usize, ManifestError> {
        let dir = dir.as_ref();
        let mut count = 0;

        if !dir.exists() {
            return Ok(0);
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                let manifest = ScriptManifest::from_file(&path)?;
                self.register(manifest);
                count += 1;
            }
        }

        Ok(count)
    }

    /// Validate and prepare execution for a script
    pub fn validate_execution(
        &self,
        script: &str,
        env: &HashMap<String, String>,
        scripts_dir: Option<&Path>,
    ) -> Result<ValidatedExecution, ManifestError> {
        let manifest = self.get(script).ok_or_else(|| ManifestError::NotFound {
            script: script.to_string(),
        })?;

        manifest.validate_execution(env, scripts_dir)
    }

    /// Get all registered manifests
    pub fn all(&self) -> impl Iterator<Item = &ScriptManifest> {
        self.manifests.values()
    }

    /// Create a registry with built-in manifests for core scripts
    pub fn with_core_manifests() -> Self {
        let mut registry = Self::new();

        // Install script manifest
        registry.register(
            ScriptManifest::builder("scripts/install.sh", "Main Arch Linux installation script")
                .destructive("CONFIRM_INSTALL")
                .require_env(
                    EnvRequirement::new("INSTALL_DISK", "Target disk for installation")
                        .with_pattern("^/dev/"),
                )
                .require_env(EnvRequirement::new(
                    "PARTITIONING_STRATEGY",
                    "Disk partitioning strategy",
                ))
                .require_env(EnvRequirement::new("SYSTEM_HOSTNAME", "System hostname"))
                .require_env(EnvRequirement::new("MAIN_USERNAME", "Primary user account name"))
                .require_env(EnvRequirement::new("BOOT_MODE", "Boot mode (UEFI or BIOS)"))
                .optional_env(OptionalEnv::new("KERNEL", "Linux kernel variant", "linux"))
                .optional_env(OptionalEnv::new("LOCALE", "System locale", "en_US.UTF-8"))
                .optional_env(OptionalEnv::new("KEYMAP", "Keyboard layout", "us"))
                .optional_env(OptionalEnv::new(
                    "TIMEZONE_REGION",
                    "Timezone region",
                    "America",
                ))
                .optional_env(OptionalEnv::new("TIMEZONE", "Timezone city", "New_York"))
                .optional_env(OptionalEnv::new("BOOTLOADER", "Bootloader", "grub"))
                .optional_env(OptionalEnv::new(
                    "DESKTOP_ENVIRONMENT",
                    "Desktop environment",
                    "none",
                ))
                .needs_stdin() // For password passing
                .build()
                .expect("Core manifest should be valid"), // Safe: hardcoded valid manifest
        );

        // Wipe disk manifest
        registry.register(
            ScriptManifest::builder("scripts/tools/wipe_disk.sh", "Securely wipe a disk")
                .destructive("CONFIRM_WIPE_DISK")
                .require_env(
                    EnvRequirement::new("INSTALL_DISK", "Disk to wipe").with_pattern("^/dev/"),
                )
                .optional_env(OptionalEnv::new(
                    "WIPE_METHOD",
                    "Wipe method (quick or zero)",
                    "quick",
                ))
                .build()
                .expect("Core manifest should be valid"), // Safe: hardcoded valid manifest
        );

        // Manual partition manifest
        registry.register(
            ScriptManifest::builder(
                "scripts/tools/manual_partition.sh",
                "Manual disk partitioning with cfdisk",
            )
            .destructive("CONFIRM_MANUAL_PARTITION")
            .require_env(
                EnvRequirement::new("INSTALL_DISK", "Disk to partition").with_pattern("^/dev/"),
            )
            .build()
            .expect("Core manifest should be valid"), // Safe: hardcoded valid manifest
        );

        // Chroot config manifest
        registry.register(
            ScriptManifest::builder(
                "scripts/chroot_config.sh",
                "Configure system inside chroot environment",
            )
            .require_env(EnvRequirement::new("MAIN_USERNAME", "Primary user account"))
            .require_env(EnvRequirement::new("SYSTEM_HOSTNAME", "System hostname"))
            .optional_env(OptionalEnv::new("LOCALE", "System locale", "en_US.UTF-8"))
            .optional_env(OptionalEnv::new("KEYMAP", "Keyboard layout", "us"))
            .optional_env(OptionalEnv::new("BOOTLOADER", "Bootloader", "grub"))
            .build()
            .expect("Core manifest should be valid"), // Safe: hardcoded valid manifest
        );

        registry
    }
}

// Convert ManifestError to the main ArchTuiError type
impl From<ManifestError> for crate::error::ArchTuiError {
    fn from(err: ManifestError) -> Self {
        crate::error::ArchTuiError::Manifest(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // EnvRequirement Tests
    // =========================================================================

    #[test]
    fn test_env_requirement_validates_non_empty() {
        let req = EnvRequirement::new("TEST_VAR", "A test variable");

        assert!(req.validate("some_value").is_ok());
        assert!(req.validate("").is_err());
    }

    #[test]
    fn test_env_requirement_allows_empty_when_configured() {
        let req = EnvRequirement::new("TEST_VAR", "A test variable").allow_empty();

        assert!(req.validate("").is_ok());
        assert!(req.validate("some_value").is_ok());
    }

    #[test]
    fn test_env_requirement_validates_prefix_pattern() {
        let req =
            EnvRequirement::new("DISK", "A disk device").with_pattern("^/dev/");

        assert!(req.validate("/dev/sda").is_ok());
        assert!(req.validate("/dev/nvme0n1").is_ok());
        assert!(req.validate("/home/user").is_err());
        assert!(req.validate("sda").is_err());
    }

    #[test]
    fn test_env_requirement_validates_suffix_pattern() {
        let req = EnvRequirement::new("FILE", "A shell script").with_pattern(".sh$");

        assert!(req.validate("install.sh").is_ok());
        assert!(req.validate("path/to/script.sh").is_ok());
        assert!(req.validate("install.py").is_err());
    }

    #[test]
    fn test_env_requirement_validates_contains_pattern() {
        let req = EnvRequirement::new("CONFIG", "Contains linux").with_pattern("linux");

        assert!(req.validate("archlinux").is_ok());
        assert!(req.validate("linux-kernel").is_ok());
        assert!(req.validate("windows").is_err());
    }

    // =========================================================================
    // ScriptManifest Tests
    // =========================================================================

    #[test]
    fn test_manifest_builder_basic() {
        let manifest = ScriptManifest::builder("scripts/test.sh", "A test script")
            .build()
            .unwrap();

        assert_eq!(manifest.script, "scripts/test.sh");
        assert_eq!(manifest.description, "A test script");
        assert!(!manifest.destructive);
        assert!(manifest.required_confirmation.is_none());
    }

    #[test]
    fn test_manifest_builder_destructive_requires_confirmation() {
        let manifest = ScriptManifest::builder("scripts/dangerous.sh", "A dangerous script")
            .destructive("CONFIRM_DANGER")
            .build()
            .unwrap();

        assert!(manifest.destructive);
        assert_eq!(
            manifest.required_confirmation,
            Some("CONFIRM_DANGER".to_string())
        );
    }

    #[test]
    fn test_manifest_destructive_without_confirmation_fails() {
        // Manually create an invalid manifest
        let manifest = ScriptManifest {
            script: "test.sh".to_string(),
            description: "test".to_string(),
            destructive: true,
            required_confirmation: None, // This is invalid!
            required_env: vec![],
            optional_env: vec![],
            valid_exit_codes: vec![0],
            needs_stdin: false,
            version: "1.0".to_string(),
        };

        let result = manifest.validate_structure();
        assert!(result.is_err());
        assert!(matches!(result, Err(ManifestError::InvalidFormat { .. })));
    }

    #[test]
    fn test_manifest_from_json() {
        let json = r#"{
            "script": "scripts/tools/wipe_disk.sh",
            "description": "Wipe a disk",
            "destructive": true,
            "required_confirmation": "CONFIRM_WIPE",
            "required_env": [
                {"name": "DISK", "description": "Target disk", "pattern": "^/dev/"}
            ],
            "optional_env": [
                {"name": "METHOD", "description": "Wipe method", "default": "quick"}
            ]
        }"#;

        let manifest = ScriptManifest::from_json(json).unwrap();
        assert_eq!(manifest.script, "scripts/tools/wipe_disk.sh");
        assert!(manifest.destructive);
        assert_eq!(manifest.required_env.len(), 1);
        assert_eq!(manifest.optional_env.len(), 1);
    }

    #[test]
    fn test_manifest_duplicate_env_vars_rejected() {
        let json = r#"{
            "script": "test.sh",
            "description": "test",
            "required_env": [
                {"name": "SAME_VAR", "description": "First"},
                {"name": "SAME_VAR", "description": "Duplicate"}
            ]
        }"#;

        let result = ScriptManifest::from_json(json);
        assert!(result.is_err());
    }

    // =========================================================================
    // Validation Tests
    // =========================================================================

    #[test]
    fn test_validate_execution_missing_required_env() {
        let manifest = ScriptManifest::builder("scripts/test.sh", "Test")
            .require_env(EnvRequirement::new("REQUIRED_VAR", "A required var"))
            .build()
            .unwrap();

        let env = HashMap::new();
        let result = manifest.validate_execution(&env, None);

        assert!(matches!(result, Err(ManifestError::MissingEnvVar { .. })));
    }

    #[test]
    fn test_validate_execution_missing_confirmation() {
        let manifest = ScriptManifest::builder("scripts/dangerous.sh", "Dangerous")
            .destructive("CONFIRM_DANGER")
            .build()
            .unwrap();

        let env = HashMap::new();
        let result = manifest.validate_execution(&env, None);

        assert!(matches!(
            result,
            Err(ManifestError::MissingConfirmation { .. })
        ));
    }

    #[test]
    fn test_validate_execution_wrong_confirmation_value() {
        let manifest = ScriptManifest::builder("scripts/dangerous.sh", "Dangerous")
            .destructive("CONFIRM_DANGER")
            .build()
            .unwrap();

        let mut env = HashMap::new();
        env.insert("CONFIRM_DANGER".to_string(), "no".to_string());

        let result = manifest.validate_execution(&env, None);
        assert!(matches!(
            result,
            Err(ManifestError::MissingConfirmation { .. })
        ));
    }

    #[test]
    fn test_validate_execution_success() {
        let manifest = ScriptManifest::builder("scripts/test.sh", "Test")
            .require_env(EnvRequirement::new("REQUIRED_VAR", "A required var"))
            .optional_env(OptionalEnv::new("OPTIONAL_VAR", "Optional", "default_value"))
            .build()
            .unwrap();

        let mut env = HashMap::new();
        env.insert("REQUIRED_VAR".to_string(), "some_value".to_string());

        let result = manifest.validate_execution(&env, None).unwrap();

        // Required var should be in final environment
        assert_eq!(result.environment.get("REQUIRED_VAR").unwrap(), "some_value");

        // Optional var should have default
        assert_eq!(
            result.environment.get("OPTIONAL_VAR").unwrap(),
            "default_value"
        );
    }

    #[test]
    fn test_validate_execution_destructive_with_confirmation() {
        let manifest = ScriptManifest::builder("scripts/dangerous.sh", "Dangerous")
            .destructive("CONFIRM_DANGER")
            .build()
            .unwrap();

        let mut env = HashMap::new();
        env.insert("CONFIRM_DANGER".to_string(), "yes".to_string());

        let result = manifest.validate_execution(&env, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_execution_invalid_pattern() {
        let manifest = ScriptManifest::builder("scripts/test.sh", "Test")
            .require_env(
                EnvRequirement::new("DISK", "Target disk").with_pattern("^/dev/"),
            )
            .build()
            .unwrap();

        let mut env = HashMap::new();
        env.insert("DISK".to_string(), "/home/user".to_string());

        let result = manifest.validate_execution(&env, None);
        assert!(matches!(result, Err(ManifestError::InvalidEnvValue { .. })));
    }

    // =========================================================================
    // Registry Tests
    // =========================================================================

    #[test]
    fn test_registry_register_and_get() {
        let mut registry = ManifestRegistry::new();

        let manifest = ScriptManifest::builder("scripts/test.sh", "Test")
            .build()
            .unwrap();

        registry.register(manifest);

        assert!(registry.get("scripts/test.sh").is_some());
        assert!(registry.get("nonexistent.sh").is_none());
    }

    #[test]
    fn test_registry_with_core_manifests() {
        let registry = ManifestRegistry::with_core_manifests();

        // Core manifests should be registered
        assert!(registry.get("scripts/install.sh").is_some());
        assert!(registry.get("scripts/tools/wipe_disk.sh").is_some());
        assert!(registry.get("scripts/tools/manual_partition.sh").is_some());
    }

    #[test]
    fn test_registry_validate_execution() {
        let registry = ManifestRegistry::with_core_manifests();

        let mut env = HashMap::new();
        env.insert("CONFIRM_WIPE_DISK".to_string(), "yes".to_string());
        env.insert("INSTALL_DISK".to_string(), "/dev/sda".to_string());

        let result = registry.validate_execution("scripts/tools/wipe_disk.sh", &env, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validated_execution_exit_code() {
        let manifest = ScriptManifest::builder("scripts/test.sh", "Test")
            .valid_exit_codes(vec![0, 1, 2])
            .build()
            .unwrap();

        let env = HashMap::new();
        let validated = manifest.validate_execution(&env, None).unwrap();

        assert!(validated.is_valid_exit_code(0));
        assert!(validated.is_valid_exit_code(1));
        assert!(validated.is_valid_exit_code(2));
        assert!(!validated.is_valid_exit_code(3));
        assert!(!validated.is_valid_exit_code(-1));
    }

    // =========================================================================
    // Bash Header Generation Tests
    // =========================================================================

    #[test]
    fn test_bash_header_generation() {
        let manifest = ScriptManifest::builder("scripts/tools/wipe_disk.sh", "Securely wipe a disk")
            .destructive("CONFIRM_WIPE_DISK")
            .require_env(
                EnvRequirement::new("INSTALL_DISK", "Target disk").with_pattern("^/dev/"),
            )
            .optional_env(OptionalEnv::new("WIPE_METHOD", "Wipe method", "quick"))
            .build()
            .unwrap();

        let header = manifest.to_bash_header();

        assert!(header.contains("ENVIRONMENT CONTRACT:"));
        assert!(header.contains("CONFIRM_WIPE_DISK=yes"));
        assert!(header.contains("REQUIRED ENVIRONMENT VARIABLES:"));
        assert!(header.contains("INSTALL_DISK"));
        assert!(header.contains("OPTIONAL ENVIRONMENT VARIABLES:"));
        assert!(header.contains("WIPE_METHOD"));
        assert!(header.contains("NON-INTERACTIVE"));
    }
}
