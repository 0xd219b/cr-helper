//! Delta integration for syntax-highlighted diff rendering

use crate::error::{CrHelperError, Result};
use std::process::Command;

/// Configuration for Delta renderer
#[derive(Debug, Clone)]
pub struct DeltaConfig {
    /// Color theme
    pub theme: Option<String>,
    /// Show line numbers
    pub line_numbers: bool,
    /// Side by side view
    pub side_by_side: bool,
    /// Additional arguments to pass to delta
    pub extra_args: Vec<String>,
}

impl Default for DeltaConfig {
    fn default() -> Self {
        Self {
            theme: None,
            line_numbers: true,
            side_by_side: false,
            extra_args: Vec::new(),
        }
    }
}

/// Delta renderer for syntax-highlighted diff output
pub struct DeltaRenderer {
    config: DeltaConfig,
}

impl DeltaRenderer {
    /// Create a new renderer with default config
    pub fn new() -> Self {
        Self {
            config: DeltaConfig::default(),
        }
    }

    /// Create a new renderer with custom config
    pub fn with_config(config: DeltaConfig) -> Self {
        Self { config }
    }

    /// Check if delta is available
    pub fn is_available() -> bool {
        Command::new("delta")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Get delta version
    pub fn get_version() -> Option<String> {
        Command::new("delta")
            .arg("--version")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    }

    /// Render diff using delta
    pub fn render(&self, diff: &str) -> Result<String> {
        if !Self::is_available() {
            return Err(CrHelperError::DeltaNotInstalled);
        }

        let mut cmd = Command::new("delta");

        // Add configuration options
        if self.config.line_numbers {
            cmd.arg("--line-numbers");
        }

        if self.config.side_by_side {
            cmd.arg("--side-by-side");
        }

        if let Some(ref theme) = self.config.theme {
            cmd.arg("--syntax-theme").arg(theme);
        }

        for arg in &self.config.extra_args {
            cmd.arg(arg);
        }

        // Pipe diff through delta
        use std::io::Write;
        use std::process::Stdio;

        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| CrHelperError::Command {
                command: "delta".to_string(),
                message: e.to_string(),
            })?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(diff.as_bytes()).map_err(|e| {
                CrHelperError::Command {
                    command: "delta".to_string(),
                    message: format!("Failed to write to stdin: {}", e),
                }
            })?;
        }

        let output = child.wait_with_output().map_err(|e| CrHelperError::Command {
            command: "delta".to_string(),
            message: e.to_string(),
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CrHelperError::Command {
                command: "delta".to_string(),
                message: stderr.to_string(),
            });
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Render diff without delta (fallback)
    pub fn render_fallback(diff: &str) -> String {
        diff.to_string()
    }

    /// Render with fallback if delta is not available
    pub fn render_or_fallback(&self, diff: &str) -> String {
        self.render(diff).unwrap_or_else(|_| Self::render_fallback(diff))
    }
}

impl Default for DeltaRenderer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_delta_config_default() {
        let config = DeltaConfig::default();
        assert!(config.line_numbers);
        assert!(!config.side_by_side);
        assert!(config.theme.is_none());
    }

    #[test]
    fn test_render_fallback() {
        let diff = "+added line\n-removed line";
        let result = DeltaRenderer::render_fallback(diff);
        assert_eq!(result, diff);
    }

    #[test]
    fn test_is_available() {
        // This test will pass or fail depending on system setup
        let _ = DeltaRenderer::is_available();
    }

    #[test]
    fn test_render_or_fallback() {
        let renderer = DeltaRenderer::new();
        let diff = "+added line\n-removed line";
        let result = renderer.render_or_fallback(diff);
        // Should return something (either delta output or fallback)
        assert!(!result.is_empty() || diff.is_empty());
    }
}
