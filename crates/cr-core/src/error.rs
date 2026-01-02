//! Error types for cr-helper

use std::path::PathBuf;
use thiserror::Error;

/// Main error type for cr-helper
#[derive(Debug, Error)]
pub enum CrHelperError {
    /// Git operation error
    #[error("Git error: {0}")]
    Git(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    /// TOML parsing error
    #[error("TOML error: {0}")]
    Toml(String),

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Comment not found
    #[error("Comment not found: {0}")]
    CommentNotFound(String),

    /// Invalid diff format
    #[error("Invalid diff format: {0}")]
    InvalidDiff(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// File not found
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    /// Unsupported schema version
    #[error("Unsupported schema version: {0}")]
    UnsupportedSchemaVersion(String),

    /// Delta not installed
    #[error("Delta is not installed. Please install delta: https://github.com/dandavison/delta")]
    DeltaNotInstalled,

    /// External command error
    #[error("Command '{command}' failed: {message}")]
    Command { command: String, message: String },

    /// Generic error with context
    #[error("{context}: {source}")]
    WithContext {
        context: String,
        #[source]
        source: Box<CrHelperError>,
    },
}

impl CrHelperError {
    /// Add context to an error
    pub fn with_context(self, context: impl Into<String>) -> Self {
        CrHelperError::WithContext {
            context: context.into(),
            source: Box::new(self),
        }
    }
}

/// Result type alias for cr-helper
pub type Result<T> = std::result::Result<T, CrHelperError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = CrHelperError::SessionNotFound("test-123".to_string());
        assert_eq!(err.to_string(), "Session not found: test-123");
    }

    #[test]
    fn test_error_with_context() {
        let err = CrHelperError::Validation("invalid content".to_string());
        let err = err.with_context("Failed to create comment");
        assert!(err.to_string().contains("Failed to create comment"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err: CrHelperError = io_err.into();
        assert!(matches!(err, CrHelperError::Io(_)));
    }
}
