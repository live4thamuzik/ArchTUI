//! Error handling module for the ArchTUI
//!
//! Provides centralized error handling with proper error types using thiserror.
//! All errors in the application should use these types for consistency.

#![allow(dead_code)] // Error variants and helpers are available for future use

use thiserror::Error;

/// Main error type for the ArchTUI
#[derive(Error, Debug)]
pub enum ArchTuiError {
    /// IO errors (file operations, terminal, etc.)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration errors (loading, parsing, validation)
    #[error("Configuration error: {0}")]
    Config(String),

    /// Script execution errors
    #[error("Script execution failed: {0}")]
    Script(String),

    /// Validation errors (user input, config values)
    #[error("Validation error: {0}")]
    Validation(String),

    /// System errors (commands, processes)
    #[error("System error: {0}")]
    System(String),

    /// Terminal/UI errors
    #[error("Terminal error: {0}")]
    Terminal(String),

    /// State errors (mutex poisoning, invalid state)
    #[error("State error: {0}")]
    State(String),

    /// Install state machine transition errors
    #[error("Install transition error: {0}")]
    InstallTransition(String),

    /// Script manifest validation errors
    #[error("Manifest error: {0}")]
    Manifest(String),

    /// JSON serialization/deserialization errors
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// General errors (catch-all for edge cases)
    #[error("{0}")]
    General(String),
}

/// Result type alias for ArchInstall operations
pub type Result<T> = std::result::Result<T, ArchTuiError>;

// Convenient error constructors
impl ArchTuiError {
    /// Create a configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Self::Config(msg.into())
    }

    /// Create a script execution error
    pub fn script(msg: impl Into<String>) -> Self {
        Self::Script(msg.into())
    }

    /// Create a validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create a system error
    pub fn system(msg: impl Into<String>) -> Self {
        Self::System(msg.into())
    }

    /// Create a terminal error
    pub fn terminal(msg: impl Into<String>) -> Self {
        Self::Terminal(msg.into())
    }

    /// Create a state error
    pub fn state(msg: impl Into<String>) -> Self {
        Self::State(msg.into())
    }

    /// Create an install transition error
    pub fn install_transition(msg: impl Into<String>) -> Self {
        Self::InstallTransition(msg.into())
    }

    /// Create a manifest error
    pub fn manifest(msg: impl Into<String>) -> Self {
        Self::Manifest(msg.into())
    }

    /// Create a general error
    pub fn general(msg: impl Into<String>) -> Self {
        Self::General(msg.into())
    }
}

/// Helper function to create general errors (for backward compatibility)
pub fn general_error(msg: impl Into<String>) -> ArchTuiError {
    ArchTuiError::General(msg.into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ArchTuiError::config("invalid hostname");
        assert_eq!(err.to_string(), "Configuration error: invalid hostname");

        let err = ArchTuiError::validation("password too short");
        assert_eq!(err.to_string(), "Validation error: password too short");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: ArchTuiError = io_err.into();
        assert!(matches!(err, ArchTuiError::Io(_)));
    }

    #[test]
    fn test_error_constructors() {
        let err = ArchTuiError::script("script failed");
        assert!(matches!(err, ArchTuiError::Script(_)));

        let err = ArchTuiError::system("command not found");
        assert!(matches!(err, ArchTuiError::System(_)));
    }
}
