//! Agent adapter module
//!
//! Provides traits and implementations for integrating with various Agent CLIs.

pub mod claude_code;

use std::path::Path;
use anyhow::Result;

/// Agent type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentType {
    /// Claude Code
    ClaudeCode,
    /// Other/Unknown agent
    Other,
}

/// Information about a detected agent
#[derive(Debug, Clone)]
pub struct AgentInfo {
    /// Agent type
    pub agent_type: AgentType,
    /// Agent name
    pub name: String,
    /// Version (if detectable)
    pub version: Option<String>,
    /// Project directory
    pub project_dir: Option<std::path::PathBuf>,
    /// Global config directory
    pub global_dir: Option<std::path::PathBuf>,
}

/// Trait for agent adapters
pub trait AgentAdapter: Send + Sync {
    /// Get the agent type
    fn agent_type(&self) -> AgentType;

    /// Detect if this agent is present
    fn detect(&self) -> Result<Option<AgentInfo>>;

    /// Check if the agent is installed
    fn is_installed(&self) -> bool {
        self.detect().map(|info| info.is_some()).unwrap_or(false)
    }

    /// Format a session for this agent's context
    fn format_context(&self, session: &cr_core::session::Session) -> Result<String>;

    /// Export session to a file in agent-compatible format
    fn export_to_file(&self, session: &cr_core::session::Session, path: &Path) -> Result<()>;

    /// Get the settings path for this agent
    fn settings_path(&self, scope: InstallScope) -> Option<std::path::PathBuf>;
}

/// Installation scope
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallScope {
    /// Project-level configuration
    Project,
    /// Local configuration (not committed)
    Local,
    /// Global user configuration
    Global,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_type() {
        assert_eq!(AgentType::ClaudeCode, AgentType::ClaudeCode);
        assert_ne!(AgentType::ClaudeCode, AgentType::Other);
    }

    #[test]
    fn test_agent_info() {
        let info = AgentInfo {
            agent_type: AgentType::ClaudeCode,
            name: "Claude Code".to_string(),
            version: Some("1.0.0".to_string()),
            project_dir: None,
            global_dir: None,
        };
        assert_eq!(info.agent_type, AgentType::ClaudeCode);
        assert!(info.version.is_some());
    }
}
