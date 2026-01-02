//! Claude Code adapter
//!
//! Implementation of AgentAdapter for Claude Code.

use super::{AgentAdapter, AgentInfo, AgentType, InstallScope};
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// Claude Code adapter
pub struct ClaudeCodeAdapter {
    /// Project directory (current directory)
    project_dir: PathBuf,
}

impl ClaudeCodeAdapter {
    /// Create a new Claude Code adapter
    pub fn new() -> Self {
        Self {
            project_dir: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }

    /// Create with a specific project directory
    pub fn with_project_dir(project_dir: PathBuf) -> Self {
        Self { project_dir }
    }

    /// Get the project .claude directory
    fn project_claude_dir(&self) -> PathBuf {
        self.project_dir.join(".claude")
    }

    /// Get the global .claude directory
    fn global_claude_dir(&self) -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".claude"))
    }

    /// Format a comment for Claude Code context
    fn format_comment(&self, comment: &cr_core::comment::Comment) -> String {
        let severity_icon = match comment.severity {
            cr_core::comment::Severity::Critical => "🔴",
            cr_core::comment::Severity::Warning => "🟡",
            cr_core::comment::Severity::Info => "🔵",
        };

        let mut result = format!(
            "{} **{}**: {}",
            severity_icon,
            comment.severity.to_string().to_uppercase(),
            comment.content
        );

        if let Some(path) = &comment.metadata.file_path {
            if let Some(line) = comment.metadata.line_number {
                result.push_str(&format!("\n   📍 `{}:{}`", path, line));
            } else {
                result.push_str(&format!("\n   📍 `{}`", path));
            }
        }

        result
    }

    /// Format location for Claude Code
    fn format_location(&self, file: &cr_core::diff::FileDiff) -> String {
        let mode_icon = match file.mode {
            cr_core::diff::FileMode::Added => "➕",
            cr_core::diff::FileMode::Deleted => "➖",
            cr_core::diff::FileMode::Modified => "📝",
            cr_core::diff::FileMode::Renamed => "📛",
            cr_core::diff::FileMode::Copied => "📋",
            cr_core::diff::FileMode::Binary => "🔢",
        };
        format!("{} {}", mode_icon, file.display_path().to_string_lossy())
    }
}

impl Default for ClaudeCodeAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentAdapter for ClaudeCodeAdapter {
    fn agent_type(&self) -> AgentType {
        AgentType::ClaudeCode
    }

    fn detect(&self) -> Result<Option<AgentInfo>> {
        let project_dir = self.project_claude_dir();
        let global_dir = self.global_claude_dir();

        let has_project = project_dir.exists();
        let has_global = global_dir.as_ref().map(|d| d.exists()).unwrap_or(false);

        if !has_project && !has_global {
            return Ok(None);
        }

        Ok(Some(AgentInfo {
            agent_type: AgentType::ClaudeCode,
            name: "Claude Code".to_string(),
            version: None, // Could detect from claude CLI if available
            project_dir: if has_project {
                Some(project_dir)
            } else {
                None
            },
            global_dir: if has_global { global_dir } else { None },
        }))
    }

    fn format_context(&self, session: &cr_core::session::Session) -> Result<String> {
        let mut context = String::new();

        // Header
        context.push_str("# Code Review Results\n\n");

        // Stats
        let stats = session.diff_data.stats.clone();
        context.push_str(&format!(
            "**Summary**: {} files changed, {} insertions(+), {} deletions(-), {} comments\n\n",
            stats.files_changed,
            stats.insertions,
            stats.deletions,
            session.comments.count()
        ));

        // Count by severity
        let mut critical = 0;
        let mut warning = 0;
        let mut info = 0;
        for comment in session.comments.all_sorted() {
            match comment.severity {
                cr_core::comment::Severity::Critical => critical += 1,
                cr_core::comment::Severity::Warning => warning += 1,
                cr_core::comment::Severity::Info => info += 1,
            }
        }

        if critical > 0 || warning > 0 || info > 0 {
            context.push_str("**Issues**:\n");
            if critical > 0 {
                context.push_str(&format!("- 🔴 Critical: {}\n", critical));
            }
            if warning > 0 {
                context.push_str(&format!("- 🟡 Warning: {}\n", warning));
            }
            if info > 0 {
                context.push_str(&format!("- 🔵 Info: {}\n", info));
            }
            context.push('\n');
        }

        // Files changed
        if !session.diff_data.files.is_empty() {
            context.push_str("## Files Changed\n\n");
            for file in &session.diff_data.files {
                context.push_str(&format!("- {}\n", self.format_location(file)));
            }
            context.push('\n');
        }

        // Comments
        if session.comments.count() > 0 {
            context.push_str("## Review Comments\n\n");

            // Group by severity
            if critical > 0 {
                context.push_str("### 🔴 Critical Issues\n\n");
                for comment in session.comments.all_sorted() {
                    if matches!(comment.severity, cr_core::comment::Severity::Critical) {
                        context.push_str(&self.format_comment(comment));
                        context.push_str("\n\n");
                    }
                }
            }

            if warning > 0 {
                context.push_str("### 🟡 Warnings\n\n");
                for comment in session.comments.all_sorted() {
                    if matches!(comment.severity, cr_core::comment::Severity::Warning) {
                        context.push_str(&self.format_comment(comment));
                        context.push_str("\n\n");
                    }
                }
            }

            if info > 0 {
                context.push_str("### 🔵 Information\n\n");
                for comment in session.comments.all_sorted() {
                    if matches!(comment.severity, cr_core::comment::Severity::Info) {
                        context.push_str(&self.format_comment(comment));
                        context.push_str("\n\n");
                    }
                }
            }
        }

        Ok(context)
    }

    fn export_to_file(&self, session: &cr_core::session::Session, path: &Path) -> Result<()> {
        let context = self.format_context(session)?;
        fs::write(path, context)?;
        Ok(())
    }

    fn settings_path(&self, scope: InstallScope) -> Option<PathBuf> {
        match scope {
            InstallScope::Project => Some(self.project_claude_dir().join("settings.json")),
            InstallScope::Local => Some(self.project_claude_dir().join("settings.local.json")),
            InstallScope::Global => self.global_claude_dir().map(|d| d.join("settings.json")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_creation() {
        let adapter = ClaudeCodeAdapter::new();
        assert_eq!(adapter.agent_type(), AgentType::ClaudeCode);
    }

    #[test]
    fn test_settings_path() {
        let adapter = ClaudeCodeAdapter::new();

        let project_path = adapter.settings_path(InstallScope::Project);
        assert!(project_path.is_some());
        assert!(project_path.unwrap().ends_with("settings.json"));

        let local_path = adapter.settings_path(InstallScope::Local);
        assert!(local_path.is_some());
        assert!(local_path.unwrap().ends_with("settings.local.json"));
    }

    #[test]
    fn test_format_location() {
        use cr_core::diff::{FileDiff, FileMode};
        use cr_core::types::FileId;

        let adapter = ClaudeCodeAdapter::new();

        let file = FileDiff {
            id: FileId::from_string("test"),
            old_path: None,
            new_path: Some("src/main.rs".into()),
            mode: FileMode::Modified,
            hunks: vec![],
            lazy: false,
        };

        let formatted = adapter.format_location(&file);
        assert!(formatted.contains("src/main.rs"));
        assert!(formatted.contains("📝"));
    }

    #[test]
    fn test_format_comment() {
        use cr_core::comment::builder::CommentBuilder;
        use cr_core::comment::model::{DiffSide, Severity};
        use cr_core::types::{FileId, LineId};

        let adapter = ClaudeCodeAdapter::new();

        let comment = CommentBuilder::new(
            FileId::from_string("f1"),
            LineId::from_string("l1"),
            DiffSide::New,
        )
        .content("This is a test comment")
        .file_path("src/main.rs")
        .line_number(42)
        .severity(Severity::Warning)
        .build()
        .unwrap();

        let formatted = adapter.format_comment(&comment);
        assert!(formatted.contains("🟡"));
        assert!(formatted.contains("WARNING"));
        assert!(formatted.contains("src/main.rs:42"));
    }
}
