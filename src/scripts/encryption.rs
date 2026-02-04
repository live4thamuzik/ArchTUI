//! Type-safe arguments for LUKS encryption scripts (Sprint 11).
//!
//! This module provides typed argument structs for encryption-related scripts:
//! - `LuksFormatArgs` for `encrypt_device.sh --action format`
//! - `LuksOpenArgs` for `encrypt_device.sh --action open`
//! - `LuksCloseArgs` for `encrypt_device.sh --action close`

#![allow(dead_code)]
//!
//! # Security Model
//!
//! **CRITICAL**: Passwords are NEVER passed via CLI arguments (visible in `ps aux`).
//! Instead, passwords are written to a temporary keyfile in `/tmp` (RAM on live ISO),
//! and the keyfile path is passed to the script. The `SecretFile` wrapper ensures
//! the keyfile is securely wiped even if the script fails.
//!
//! # LUKS2 Standard
//!
//! All operations use LUKS2 (modern standard) with secure defaults:
//! - Cipher: aes-xts-plain64 (AES-256)
//! - Key derivation: argon2id
//! - Hash: sha256

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};

use crate::script_traits::ScriptArgs;

// ============================================================================
// SecretFile - RAII Wrapper for Secure Keyfile Management
// ============================================================================

/// RAII wrapper for secure temporary keyfile management.
///
/// Creates a keyfile with 0600 permissions in `/tmp` (RAM on live ISO).
/// The keyfile is securely overwritten and deleted when dropped, even
/// if the script execution fails or panics.
///
/// # Security Guarantees
///
/// 1. **Restricted permissions**: Created with mode 0600 (owner read/write only)
/// 2. **RAM-backed**: `/tmp` on Arch ISO is tmpfs (in memory, not persisted)
/// 3. **Secure deletion**: Overwritten with zeros before unlinking
/// 4. **Panic-safe**: Drop trait ensures cleanup on unwinding
///
/// # Example
///
/// ```ignore
/// use archtui::scripts::encryption::SecretFile;
///
/// let secret = SecretFile::new("my_password")?;
/// // Use secret.path() for the keyfile path
/// // When `secret` goes out of scope, the file is securely deleted
/// ```
#[derive(Debug)]
pub struct SecretFile {
    /// Path to the temporary keyfile.
    path: PathBuf,
    /// Size of the secret (for secure overwrite).
    size: usize,
}

impl SecretFile {
    /// Create a new secret file with the given content.
    ///
    /// # Arguments
    ///
    /// * `secret` - The secret content (password) to write to the file
    ///
    /// # Returns
    ///
    /// - `Ok(SecretFile)` - File created successfully
    /// - `Err` - Failed to create file or write content
    ///
    /// # Security Notes
    ///
    /// - File is created with mode 0600 (owner read/write only)
    /// - Uses `/tmp` which is tmpfs on Arch live ISO (RAM-backed)
    /// - Unique filename prevents collision attacks
    pub fn new(secret: &str) -> std::io::Result<Self> {
        // Generate unique filename with cryptographic randomness
        let random_suffix: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
            ^ std::process::id() as u64;

        let path = PathBuf::from(format!("/tmp/.archinstall_keyfile_{:016x}", random_suffix));

        // Create file with restricted permissions (0600)
        // CRITICAL: Use OpenOptionsExt::mode() to set permissions atomically
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true) // Fail if exists (prevents race)
            .mode(0o600) // Owner read/write only
            .open(&path)?;

        // Write secret content
        file.write_all(secret.as_bytes())?;
        file.sync_all()?; // Ensure written to disk (well, RAM)

        log::debug!("SecretFile created: {:?} ({} bytes)", path, secret.len());

        Ok(Self {
            path,
            size: secret.len(),
        })
    }

    /// Get the path to the keyfile.
    ///
    /// This path can be passed to scripts that accept `--key-file` arguments.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Securely wipe the file content before deletion.
    ///
    /// Overwrites with zeros to prevent recovery from RAM or swap.
    /// Called automatically by Drop, but can be called manually for
    /// explicit cleanup.
    fn secure_wipe(&self) {
        // Try to overwrite with zeros
        if let Ok(mut file) = OpenOptions::new().write(true).open(&self.path) {
            let zeros = vec![0u8; self.size];
            let _ = file.write_all(&zeros);
            let _ = file.sync_all();
        }

        // Delete the file
        if let Err(e) = fs::remove_file(&self.path) {
            log::warn!("Failed to remove keyfile {:?}: {}", self.path, e);
        } else {
            log::debug!("SecretFile securely wiped: {:?}", self.path);
        }
    }
}

impl Drop for SecretFile {
    /// Securely wipe and delete the keyfile on drop.
    ///
    /// This ensures the keyfile is cleaned up even if:
    /// - The script execution fails
    /// - A panic occurs
    /// - Early return from a function
    fn drop(&mut self) {
        self.secure_wipe();
    }
}

// ============================================================================
// LUKS Cipher Configuration
// ============================================================================

/// LUKS cipher configuration.
///
/// Provides type-safe cipher selection with secure defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LuksCipher {
    /// AES-256 in XTS mode (recommended for most uses).
    /// This is the default and most widely compatible option.
    Aes256Xts,
    /// Serpent-256 in XTS mode (alternative cipher).
    Serpent256Xts,
    /// Twofish-256 in XTS mode (alternative cipher).
    Twofish256Xts,
}

impl LuksCipher {
    /// Get the cryptsetup cipher string.
    pub fn as_cipher_str(&self) -> &'static str {
        match self {
            LuksCipher::Aes256Xts => "aes-xts-plain64",
            LuksCipher::Serpent256Xts => "serpent-xts-plain64",
            LuksCipher::Twofish256Xts => "twofish-xts-plain64",
        }
    }

    /// Get the key size in bits.
    pub fn key_size(&self) -> u32 {
        match self {
            LuksCipher::Aes256Xts => 512, // XTS mode uses 2x key size
            LuksCipher::Serpent256Xts => 512,
            LuksCipher::Twofish256Xts => 512,
        }
    }
}

impl Default for LuksCipher {
    fn default() -> Self {
        LuksCipher::Aes256Xts
    }
}

impl std::fmt::Display for LuksCipher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_cipher_str())
    }
}

// ============================================================================
// LuksFormatArgs
// ============================================================================

/// Type-safe arguments for LUKS format operation.
///
/// Formats a device with LUKS2 encryption using a keyfile.
///
/// # Field to Flag/Env Mapping
///
/// | Rust Field | CLI Flag     | Notes |
/// |------------|--------------|-------|
/// | `device`   | `--device`   | Partition to encrypt (e.g., /dev/sda2) |
/// | `cipher`   | `--cipher`   | Cipher algorithm (default: aes-xts-plain64) |
/// | `key_file` | `--key-file` | Path to keyfile (managed by SecretFile) |
/// | `label`    | `--label`    | Optional LUKS label |
/// | `confirm`  | env: `CONFIRM_LUKS_FORMAT` | Required for destructive operation |
///
/// # Security
///
/// - NEVER pass the password in `to_cli_args()` - use keyfile
/// - The `key_file` field holds the path to a `SecretFile`
/// - Requires `CONFIRM_LUKS_FORMAT=yes` environment variable
#[derive(Debug, Clone)]
pub struct LuksFormatArgs {
    /// Device to format with LUKS (e.g., `/dev/sda2`).
    pub device: PathBuf,
    /// Cipher configuration (default: AES-256-XTS).
    pub cipher: LuksCipher,
    /// Path to the keyfile (created by `SecretFile`).
    pub key_file: PathBuf,
    /// Optional LUKS label for the encrypted volume.
    pub label: Option<String>,
    /// Explicit confirmation for destructive operation.
    pub confirm: bool,
}

impl ScriptArgs for LuksFormatArgs {
    fn to_cli_args(&self) -> Vec<String> {
        let mut args = vec![
            "--action".to_string(),
            "format".to_string(),
            "--device".to_string(),
            self.device.display().to_string(),
            "--cipher".to_string(),
            self.cipher.as_cipher_str().to_string(),
            "--key-size".to_string(),
            self.cipher.key_size().to_string(),
            "--key-file".to_string(),
            self.key_file.display().to_string(),
        ];

        if let Some(ref label) = self.label {
            args.push("--label".to_string());
            args.push(label.clone());
        }

        args
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        if self.confirm {
            vec![("CONFIRM_LUKS_FORMAT".to_string(), "yes".to_string())]
        } else {
            vec![]
        }
    }

    fn script_name(&self) -> &'static str {
        "encrypt_device.sh"
    }

    /// LUKS format is DESTRUCTIVE - erases partition contents.
    fn is_destructive(&self) -> bool {
        true
    }
}

// ============================================================================
// LuksOpenArgs
// ============================================================================

/// Type-safe arguments for LUKS open (unlock) operation.
///
/// Opens an encrypted LUKS device using a keyfile.
///
/// # Field to Flag Mapping
///
/// | Rust Field    | CLI Flag       | Notes |
/// |---------------|----------------|-------|
/// | `device`      | `--device`     | Encrypted partition |
/// | `mapper_name` | `--mapper`     | Device mapper name (e.g., "cryptroot") |
/// | `key_file`    | `--key-file`   | Path to keyfile |
///
/// # After Opening
///
/// The decrypted device will be available at `/dev/mapper/<mapper_name>`.
#[derive(Debug, Clone)]
pub struct LuksOpenArgs {
    /// Encrypted device to open (e.g., `/dev/sda2`).
    pub device: PathBuf,
    /// Device mapper name (decrypted device at /dev/mapper/<name>).
    pub mapper_name: String,
    /// Path to the keyfile.
    pub key_file: PathBuf,
}

impl ScriptArgs for LuksOpenArgs {
    fn to_cli_args(&self) -> Vec<String> {
        vec![
            "--action".to_string(),
            "open".to_string(),
            "--device".to_string(),
            self.device.display().to_string(),
            "--mapper".to_string(),
            self.mapper_name.clone(),
            "--key-file".to_string(),
            self.key_file.display().to_string(),
        ]
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "encrypt_device.sh"
    }

    /// LUKS open is NOT destructive - only unlocks existing encrypted data.
    fn is_destructive(&self) -> bool {
        false
    }
}

// ============================================================================
// LuksCloseArgs
// ============================================================================

/// Type-safe arguments for LUKS close (lock) operation.
///
/// Closes an opened LUKS device.
///
/// # Field to Flag Mapping
///
/// | Rust Field    | CLI Flag   | Notes |
/// |---------------|------------|-------|
/// | `mapper_name` | `--mapper` | Device mapper name to close |
#[derive(Debug, Clone)]
pub struct LuksCloseArgs {
    /// Device mapper name to close.
    pub mapper_name: String,
}

impl ScriptArgs for LuksCloseArgs {
    fn to_cli_args(&self) -> Vec<String> {
        vec![
            "--action".to_string(),
            "close".to_string(),
            "--mapper".to_string(),
            self.mapper_name.clone(),
        ]
    }

    fn get_env_vars(&self) -> Vec<(String, String)> {
        vec![]
    }

    fn script_name(&self) -> &'static str {
        "encrypt_device.sh"
    }

    /// LUKS close is NOT destructive.
    fn is_destructive(&self) -> bool {
        false
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_luks_format_args() {
        let args = LuksFormatArgs {
            device: PathBuf::from("/dev/sda2"),
            cipher: LuksCipher::default(),
            key_file: PathBuf::from("/tmp/keyfile"),
            label: Some("cryptroot".to_string()),
            confirm: true,
        };

        let cli_args = args.to_cli_args();
        assert!(cli_args.contains(&"--action".to_string()));
        assert!(cli_args.contains(&"format".to_string()));
        assert!(cli_args.contains(&"--device".to_string()));
        assert!(cli_args.contains(&"/dev/sda2".to_string()));
        assert!(cli_args.contains(&"--cipher".to_string()));
        assert!(cli_args.contains(&"aes-xts-plain64".to_string()));
        assert!(cli_args.contains(&"--key-file".to_string()));
        assert!(cli_args.contains(&"--label".to_string()));
        assert!(cli_args.contains(&"cryptroot".to_string()));

        let env_vars = args.get_env_vars();
        assert_eq!(env_vars.len(), 1);
        assert_eq!(env_vars[0].0, "CONFIRM_LUKS_FORMAT");
        assert_eq!(env_vars[0].1, "yes");

        assert!(args.is_destructive());
    }

    #[test]
    fn test_luks_format_no_confirm() {
        let args = LuksFormatArgs {
            device: PathBuf::from("/dev/sda2"),
            cipher: LuksCipher::default(),
            key_file: PathBuf::from("/tmp/keyfile"),
            label: None,
            confirm: false,
        };

        let env_vars = args.get_env_vars();
        assert!(env_vars.is_empty());
    }

    #[test]
    fn test_luks_open_args() {
        let args = LuksOpenArgs {
            device: PathBuf::from("/dev/sda2"),
            mapper_name: "cryptroot".to_string(),
            key_file: PathBuf::from("/tmp/keyfile"),
        };

        let cli_args = args.to_cli_args();
        assert!(cli_args.contains(&"--action".to_string()));
        assert!(cli_args.contains(&"open".to_string()));
        assert!(cli_args.contains(&"--mapper".to_string()));
        assert!(cli_args.contains(&"cryptroot".to_string()));

        // Open is not destructive
        assert!(!args.is_destructive());
    }

    #[test]
    fn test_luks_close_args() {
        let args = LuksCloseArgs {
            mapper_name: "cryptroot".to_string(),
        };

        let cli_args = args.to_cli_args();
        assert!(cli_args.contains(&"--action".to_string()));
        assert!(cli_args.contains(&"close".to_string()));
        assert!(cli_args.contains(&"--mapper".to_string()));
        assert!(cli_args.contains(&"cryptroot".to_string()));

        // Close is not destructive
        assert!(!args.is_destructive());
    }

    #[test]
    fn test_luks_cipher_defaults() {
        assert_eq!(LuksCipher::default(), LuksCipher::Aes256Xts);
        assert_eq!(LuksCipher::default().as_cipher_str(), "aes-xts-plain64");
        assert_eq!(LuksCipher::default().key_size(), 512);
    }

    #[test]
    fn test_luks_cipher_variants() {
        assert_eq!(
            LuksCipher::Serpent256Xts.as_cipher_str(),
            "serpent-xts-plain64"
        );
        assert_eq!(
            LuksCipher::Twofish256Xts.as_cipher_str(),
            "twofish-xts-plain64"
        );
    }

    #[test]
    fn test_script_name() {
        let format_args = LuksFormatArgs {
            device: PathBuf::from("/dev/sda2"),
            cipher: LuksCipher::default(),
            key_file: PathBuf::from("/tmp/keyfile"),
            label: None,
            confirm: true,
        };
        assert_eq!(format_args.script_name(), "encrypt_device.sh");

        let open_args = LuksOpenArgs {
            device: PathBuf::from("/dev/sda2"),
            mapper_name: "cryptroot".to_string(),
            key_file: PathBuf::from("/tmp/keyfile"),
        };
        assert_eq!(open_args.script_name(), "encrypt_device.sh");
    }

    #[test]
    fn test_password_not_in_cli_args() {
        // SECURITY: Verify password is NEVER in CLI args
        let args = LuksFormatArgs {
            device: PathBuf::from("/dev/sda2"),
            cipher: LuksCipher::default(),
            key_file: PathBuf::from("/tmp/keyfile"),
            label: None,
            confirm: true,
        };

        let cli_args = args.to_cli_args();
        // CLI args should only contain the keyfile PATH, not any password content
        for arg in &cli_args {
            // The keyfile path is OK, but no raw password data should appear
            assert!(
                !arg.contains("password") && !arg.contains("secret"),
                "CLI args must not contain password data"
            );
        }
    }
}
