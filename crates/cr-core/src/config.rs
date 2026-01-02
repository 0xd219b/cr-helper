//! Configuration management for cr-helper

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Review settings
    pub review: ReviewConfig,
    /// Export settings
    pub export: ExportConfig,
    /// Diff settings
    pub diff: DiffConfig,
    /// UI settings
    pub ui: UiConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            review: ReviewConfig::default(),
            export: ExportConfig::default(),
            diff: DiffConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

/// Review-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ReviewConfig {
    /// Check categories to enable
    pub checks: Vec<String>,
    /// Maximum comment content length
    pub max_comment_length: usize,
    /// Auto-save interval in seconds
    pub auto_save_interval: u64,
}

impl Default for ReviewConfig {
    fn default() -> Self {
        Self {
            checks: vec![
                "security".to_string(),
                "error-handling".to_string(),
                "performance".to_string(),
                "best-practices".to_string(),
            ],
            max_comment_length: 2000,
            auto_save_interval: 30,
        }
    }
}

/// Export-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ExportConfig {
    /// Default export format
    pub default_format: String,
    /// Include code context in export
    pub include_code_context: bool,
    /// Number of context lines (before and after)
    pub context_lines: usize,
    /// Include statistics in export
    pub include_stats: bool,
    /// Include suggested fixes
    pub include_suggestions: bool,
}

impl Default for ExportConfig {
    fn default() -> Self {
        Self {
            default_format: "markdown-enhanced".to_string(),
            include_code_context: true,
            context_lines: 2,
            include_stats: true,
            include_suggestions: true,
        }
    }
}

/// Diff-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DiffConfig {
    /// File patterns to include
    pub include_patterns: Vec<String>,
    /// File patterns to exclude
    pub exclude_patterns: Vec<String>,
    /// Delta theme
    pub delta_theme: Option<String>,
    /// Show line numbers
    pub line_numbers: bool,
    /// Side by side view
    pub side_by_side: bool,
}

impl Default for DiffConfig {
    fn default() -> Self {
        Self {
            include_patterns: vec!["*".to_string()],
            exclude_patterns: vec![
                "*.lock".to_string(),
                "target/".to_string(),
                "node_modules/".to_string(),
                ".git/".to_string(),
            ],
            delta_theme: None,
            line_numbers: true,
            side_by_side: false,
        }
    }
}

/// UI-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Show file tree panel
    pub show_file_tree: bool,
    /// Color theme
    pub theme: String,
    /// Key bindings (vim/default)
    pub key_bindings: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_file_tree: true,
            theme: "default".to_string(),
            key_bindings: "default".to_string(),
        }
    }
}

/// Claude Code integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ClaudeCodeConfig {
    /// Automatically trigger review on stop
    pub auto_review_on_stop: bool,
    /// Minimum file changes to trigger review
    pub min_changes_for_review: usize,
    /// Block on critical issues
    pub block_on_critical: bool,
    /// Output directory for review files
    pub output_dir: PathBuf,
}

impl Default for ClaudeCodeConfig {
    fn default() -> Self {
        Self {
            auto_review_on_stop: true,
            min_changes_for_review: 3,
            block_on_critical: true,
            output_dir: PathBuf::from(".claude/cr-helper"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.export.include_code_context);
        assert_eq!(config.export.context_lines, 2);
        assert!(config.ui.show_file_tree);
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml = toml::to_string_pretty(&config).unwrap();
        assert!(toml.contains("[review]"));
        assert!(toml.contains("[export]"));

        let config2: Config = toml::from_str(&toml).unwrap();
        assert_eq!(config.export.context_lines, config2.export.context_lines);
    }

    #[test]
    fn test_claude_code_config() {
        let config = ClaudeCodeConfig::default();
        assert!(config.auto_review_on_stop);
        assert_eq!(config.min_changes_for_review, 3);
        assert!(config.block_on_critical);
    }
}
