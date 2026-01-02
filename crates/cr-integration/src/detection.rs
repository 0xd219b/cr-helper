//! Agent detection utilities
//!
//! Utilities for detecting installed Agent CLIs.

use crate::adapter::{AgentAdapter, AgentInfo, AgentType};
use crate::adapter::claude_code::ClaudeCodeAdapter;
use anyhow::Result;

/// Detected agents result
pub struct DetectedAgents {
    /// All detected agents
    pub agents: Vec<AgentInfo>,
}

impl DetectedAgents {
    /// Check if any agents were detected
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// Get the first detected agent
    pub fn first(&self) -> Option<&AgentInfo> {
        self.agents.first()
    }

    /// Check if a specific agent type is detected
    pub fn has_agent(&self, agent_type: AgentType) -> bool {
        self.agents.iter().any(|a| a.agent_type == agent_type)
    }
}

/// Detect all installed agents
pub fn detect_agents() -> Result<DetectedAgents> {
    let mut agents = Vec::new();

    // Try Claude Code
    let claude_adapter = ClaudeCodeAdapter::new();
    if let Ok(Some(info)) = claude_adapter.detect() {
        agents.push(info);
    }

    // Add more agent detectors here as they are implemented

    Ok(DetectedAgents { agents })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_agents() {
        let result = detect_agents();
        assert!(result.is_ok());
    }

    #[test]
    fn test_detected_agents_is_empty() {
        let detected = DetectedAgents { agents: vec![] };
        assert!(detected.is_empty());
    }

    #[test]
    fn test_detected_agents_has_agent() {
        let detected = DetectedAgents {
            agents: vec![AgentInfo {
                agent_type: AgentType::ClaudeCode,
                name: "Claude Code".to_string(),
                version: None,
                project_dir: None,
                global_dir: None,
            }],
        };
        assert!(detected.has_agent(AgentType::ClaudeCode));
        assert!(!detected.has_agent(AgentType::Other));
    }
}
