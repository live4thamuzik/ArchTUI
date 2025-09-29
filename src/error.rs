//! Error handling module for the ArchInstall TUI
//!
//! Provides centralized error handling with proper error types and messages.

use std::fmt;

/// Main error type for the ArchInstall TUI
#[derive(Debug)]
pub enum ArchInstallError {
    /// IO errors
    IoError(std::io::Error),
    /// General errors
    GeneralError(String),
}

impl fmt::Display for ArchInstallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArchInstallError::IoError(err) => write!(f, "IO Error: {}", err),
            ArchInstallError::GeneralError(msg) => write!(f, "Error: {}", msg),
        }
    }
}

impl std::error::Error for ArchInstallError {}

impl From<std::io::Error> for ArchInstallError {
    fn from(err: std::io::Error) -> Self {
        ArchInstallError::IoError(err)
    }
}

/// Helper function to create general errors
pub fn general_error(msg: impl Into<String>) -> ArchInstallError {
    ArchInstallError::GeneralError(msg.into())
}
