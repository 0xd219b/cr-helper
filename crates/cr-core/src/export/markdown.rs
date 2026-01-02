//! Markdown exporters for sessions

use super::context::ContextExtractor;
use super::exporter::Exporter;
use crate::comment::model::{Comment, Severity};
use crate::error::Result;
use crate::session::Session;

/// Markdown exporter
pub struct MarkdownExporter {
    /// Include diff snippets
    include_diff: bool,
    /// Include statistics section
    include_stats: bool,
    /// Include suggestions section
    include_suggestions: bool,
    /// Context extractor
    context: ContextExtractor,
}

impl MarkdownExporter {
    /// Create a new Markdown exporter with default settings
    pub fn new() -> Self {
        Self {
            include_diff: true,
            include_stats: true,
            include_suggestions: true,
            context: ContextExtractor::new(2),
        }
    }

    /// Set whether to include diff snippets
    pub fn with_diff(mut self, include: bool) -> Self {
        self.include_diff = include;
        self
    }

    /// Set whether to include statistics
    pub fn with_stats(mut self, include: bool) -> Self {
        self.include_stats = include;
        self
    }

    /// Set whether to include suggestions
    pub fn with_suggestions(mut self, include: bool) -> Self {
        self.include_suggestions = include;
        self
    }

    /// Render the report header
    fn render_header(&self, session: &Session) -> String {
        let mut header = String::new();
        header.push_str("# Code Review Report\n\n");

        header.push_str(&format!(
            "**Session:** `{}`\n",
            session.id
        ));
        header.push_str(&format!(
            "**Date:** {}\n",
            session.created_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        header.push_str(&format!(
            "**Source:** {}\n",
            session.diff_source.description()
        ));

        if let Some(ref repo) = session.metadata.repository {
            header.push_str(&format!(
                "**Repository:** `{}`\n",
                repo.display()
            ));
        }

        if let Some(ref name) = session.metadata.name {
            header.push_str(&format!("**Name:** {}\n", name));
        }

        header.push('\n');
        header
    }

    /// Render the statistics section
    fn render_stats(&self, session: &Session) -> String {
        if !self.include_stats {
            return String::new();
        }

        let counts = session.comments.count_by_severity();
        let critical = counts.get(&Severity::Critical).unwrap_or(&0);
        let warning = counts.get(&Severity::Warning).unwrap_or(&0);
        let info = counts.get(&Severity::Info).unwrap_or(&0);

        let mut stats = String::new();
        stats.push_str("## Summary\n\n");
        stats.push_str(&format!(
            "- **Total Comments:** {}\n",
            session.comment_count()
        ));
        stats.push_str(&format!(
            "- **Files Reviewed:** {}\n",
            session.file_count()
        ));
        stats.push_str(&format!("- {} Critical Issues\n", critical));
        stats.push_str(&format!("- {} Warnings\n", warning));
        stats.push_str(&format!("- {} Info\n", info));
        stats.push('\n');

        stats
    }

    /// Render comments grouped by severity
    fn render_comments(&self, session: &Session) -> String {
        let mut output = String::new();

        // Group comments by severity
        let critical: Vec<_> = session
            .comments
            .get_by_severity(Severity::Critical)
            .into_iter()
            .collect();
        let warnings: Vec<_> = session
            .comments
            .get_by_severity(Severity::Warning)
            .into_iter()
            .collect();
        let info: Vec<_> = session
            .comments
            .get_by_severity(Severity::Info)
            .into_iter()
            .collect();

        if !critical.is_empty() {
            output.push_str("## Critical Issues\n\n");
            for comment in critical {
                output.push_str(&self.render_comment(comment, session));
            }
        }

        if !warnings.is_empty() {
            output.push_str("## Warnings\n\n");
            for comment in warnings {
                output.push_str(&self.render_comment(comment, session));
            }
        }

        if !info.is_empty() {
            output.push_str("## Info\n\n");
            for comment in info {
                output.push_str(&self.render_comment(comment, session));
            }
        }

        output
    }

    /// Render a single comment
    fn render_comment(&self, comment: &Comment, session: &Session) -> String {
        let mut output = String::new();

        // Location header
        let file_path = comment
            .metadata
            .file_path
            .clone()
            .unwrap_or_else(|| comment.file_id().to_string());

        let line_info = comment
            .metadata
            .line_number
            .map(|n| format!(":{}", n))
            .unwrap_or_default();

        let tags = if comment.tags.is_empty() {
            String::new()
        } else {
            format!(" [{}]", comment.tags.join(", "))
        };

        output.push_str(&format!(
            "### `{}{}`{}\n\n",
            file_path, line_info, tags
        ));

        // Comment content
        output.push_str(&comment.content);
        output.push_str("\n\n");

        // Code context
        if self.include_diff {
            if let Some(ctx) = self.context.extract(comment, &session.diff_data) {
                output.push_str(&ContextExtractor::format_code_block(&ctx, &file_path));
                output.push_str("\n\n");
            }
        }

        // Suggested fix
        if self.include_suggestions {
            if let Some(fix) = comment.extensions.suggested_fix() {
                output.push_str("**Suggested Fix:**\n\n");
                output.push_str(fix);
                output.push_str("\n\n");
            }
        }

        output.push_str("---\n\n");
        output
    }
}

impl Default for MarkdownExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Exporter for MarkdownExporter {
    fn export(&self, session: &Session) -> Result<String> {
        let mut output = String::new();

        output.push_str(&self.render_header(session));
        output.push_str(&self.render_stats(session));
        output.push_str(&self.render_comments(session));

        Ok(output)
    }

    fn format_name(&self) -> &str {
        "markdown"
    }

    fn file_extension(&self) -> &str {
        "md"
    }
}

/// Enhanced Markdown exporter with YAML frontmatter
pub struct MarkdownEnhancedExporter {
    /// Base Markdown exporter
    base: MarkdownExporter,
}

impl MarkdownEnhancedExporter {
    /// Create a new enhanced Markdown exporter
    pub fn new() -> Self {
        Self {
            base: MarkdownExporter::new(),
        }
    }

    /// Render YAML frontmatter
    fn render_frontmatter(&self, session: &Session) -> String {
        let counts = session.comments.count_by_severity();

        let mut fm = String::new();
        fm.push_str("---\n");
        fm.push_str("cr-helper-version: \"1.0\"\n");
        fm.push_str(&format!("session-id: \"{}\"\n", session.id));
        fm.push_str(&format!(
            "timestamp: \"{}\"\n",
            session.created_at.to_rfc3339()
        ));
        fm.push_str("stats:\n");
        fm.push_str(&format!("  files: {}\n", session.file_count()));
        fm.push_str(&format!("  comments: {}\n", session.comment_count()));
        fm.push_str(&format!(
            "  critical: {}\n",
            counts.get(&Severity::Critical).unwrap_or(&0)
        ));
        fm.push_str(&format!(
            "  warnings: {}\n",
            counts.get(&Severity::Warning).unwrap_or(&0)
        ));
        fm.push_str(&format!(
            "  info: {}\n",
            counts.get(&Severity::Info).unwrap_or(&0)
        ));
        fm.push_str("---\n\n");

        fm
    }

    /// Render enhanced comment with anchor
    fn render_enhanced_comment(&self, comment: &Comment, session: &Session) -> String {
        let mut output = String::new();

        // Location header with anchor
        let file_path = comment
            .metadata
            .file_path
            .clone()
            .unwrap_or_else(|| comment.file_id().to_string());

        let line_info = comment
            .metadata
            .line_number
            .map(|n| format!(":{}", n))
            .unwrap_or_default();

        let short_id = &comment.id.to_string()[..8.min(comment.id.to_string().len())];

        output.push_str(&format!(
            "### `{}{}`  {{#{}}}\n\n",
            file_path, line_info, short_id
        ));

        // Severity badge
        let badge = match comment.severity {
            Severity::Critical => "> **CRITICAL**",
            Severity::Warning => "> **WARNING**",
            Severity::Info => "> **INFO**",
        };
        output.push_str(badge);
        output.push('\n');

        if !comment.tags.is_empty() {
            output.push_str(&format!("> Tags: {}\n", comment.tags.join(", ")));
        }
        output.push('\n');

        // Comment content
        output.push_str(&comment.content);
        output.push_str("\n\n");

        // Code context
        if let Some(ctx) = self.base.context.extract(comment, &session.diff_data) {
            output.push_str("#### Code Context\n\n");
            output.push_str(&ContextExtractor::format_code_block(&ctx, &file_path));
            output.push_str("\n\n");
        }

        // Suggested fix with approach
        if let Some(fix) = comment.extensions.suggested_fix() {
            output.push_str("#### Suggested Approach\n\n");
            output.push_str(fix);
            output.push_str("\n\n");
        }

        output.push_str("---\n\n");
        output
    }

    /// Render enhanced comments section
    fn render_enhanced_comments(&self, session: &Session) -> String {
        let mut output = String::new();

        let critical: Vec<_> = session
            .comments
            .get_by_severity(Severity::Critical)
            .into_iter()
            .collect();
        let warnings: Vec<_> = session
            .comments
            .get_by_severity(Severity::Warning)
            .into_iter()
            .collect();
        let info: Vec<_> = session
            .comments
            .get_by_severity(Severity::Info)
            .into_iter()
            .collect();

        if !critical.is_empty() {
            output.push_str("## Critical Issues\n\n");
            for comment in critical {
                output.push_str(&self.render_enhanced_comment(comment, session));
            }
        }

        if !warnings.is_empty() {
            output.push_str("## Warnings\n\n");
            for comment in warnings {
                output.push_str(&self.render_enhanced_comment(comment, session));
            }
        }

        if !info.is_empty() {
            output.push_str("## Info\n\n");
            for comment in info {
                output.push_str(&self.render_enhanced_comment(comment, session));
            }
        }

        output
    }
}

impl Default for MarkdownEnhancedExporter {
    fn default() -> Self {
        Self::new()
    }
}

impl Exporter for MarkdownEnhancedExporter {
    fn export(&self, session: &Session) -> Result<String> {
        let mut output = String::new();

        output.push_str(&self.render_frontmatter(session));
        output.push_str(&self.base.render_header(session));
        output.push_str(&self.base.render_stats(session));
        output.push_str(&self.render_enhanced_comments(session));

        Ok(output)
    }

    fn format_name(&self) -> &str {
        "markdown-enhanced"
    }

    fn file_extension(&self) -> &str {
        "md"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comment::builder::CommentBuilder;
    use crate::comment::model::DiffSide;
    use crate::diff::DiffData;
    use crate::session::DiffSource;
    use crate::types::{FileId, LineId};

    fn create_test_session() -> Session {
        Session::new(DiffSource::WorkingTree, DiffData::empty())
    }

    fn create_session_with_comments() -> Session {
        let mut session = create_test_session();

        let comment1 = CommentBuilder::new(
            FileId::from_string("file1"),
            LineId::from_string("line1"),
            DiffSide::New,
        )
        .content("Critical issue: SQL injection vulnerability")
        .critical()
        .tag("security")
        .tag("sql")
        .line_number(42)
        .file_path("src/database.rs")
        .suggested_fix("Use parameterized queries instead")
        .build()
        .unwrap();

        let comment2 = CommentBuilder::new(
            FileId::from_string("file2"),
            LineId::from_string("line2"),
            DiffSide::New,
        )
        .content("Consider using a more descriptive variable name")
        .info()
        .line_number(15)
        .file_path("src/utils.rs")
        .build()
        .unwrap();

        session.comments.add(comment1).unwrap();
        session.comments.add(comment2).unwrap();

        session
    }

    #[test]
    fn test_markdown_exporter_creation() {
        let exporter = MarkdownExporter::new();
        assert_eq!(exporter.format_name(), "markdown");
        assert_eq!(exporter.file_extension(), "md");
    }

    #[test]
    fn test_export_empty_session() {
        let exporter = MarkdownExporter::new();
        let session = create_test_session();

        let result = exporter.export(&session);
        assert!(result.is_ok());

        let md = result.unwrap();
        assert!(md.contains("# Code Review Report"));
        assert!(md.contains("**Session:**"));
        assert!(md.contains("## Summary"));
    }

    #[test]
    fn test_export_session_with_comments() {
        let exporter = MarkdownExporter::new();
        let session = create_session_with_comments();

        let result = exporter.export(&session);
        assert!(result.is_ok());

        let md = result.unwrap();
        assert!(md.contains("## Critical Issues"));
        assert!(md.contains("SQL injection vulnerability"));
        assert!(md.contains("[security, sql]"));
        assert!(md.contains("**Suggested Fix:**"));
    }

    #[test]
    fn test_markdown_stats() {
        let exporter = MarkdownExporter::new();
        let session = create_session_with_comments();

        let md = exporter.export(&session).unwrap();
        assert!(md.contains("**Total Comments:** 2"));
        assert!(md.contains("1 Critical Issues"));
        assert!(md.contains("1 Info"));
    }

    #[test]
    fn test_markdown_without_stats() {
        let exporter = MarkdownExporter::new().with_stats(false);
        let session = create_test_session();

        let md = exporter.export(&session).unwrap();
        assert!(!md.contains("## Summary"));
    }

    #[test]
    fn test_enhanced_markdown_exporter() {
        let exporter = MarkdownEnhancedExporter::new();
        assert_eq!(exporter.format_name(), "markdown-enhanced");
    }

    #[test]
    fn test_enhanced_export_has_frontmatter() {
        let exporter = MarkdownEnhancedExporter::new();
        let session = create_session_with_comments();

        let md = exporter.export(&session).unwrap();
        assert!(md.starts_with("---\n"));
        assert!(md.contains("cr-helper-version:"));
        assert!(md.contains("session-id:"));
        assert!(md.contains("stats:"));
    }

    #[test]
    fn test_enhanced_export_has_anchors() {
        let exporter = MarkdownEnhancedExporter::new();
        let session = create_session_with_comments();

        let md = exporter.export(&session).unwrap();
        // Check for anchor syntax
        assert!(md.contains("{#"));
    }

    #[test]
    fn test_enhanced_export_has_severity_badges() {
        let exporter = MarkdownEnhancedExporter::new();
        let session = create_session_with_comments();

        let md = exporter.export(&session).unwrap();
        assert!(md.contains("> **CRITICAL**"));
        assert!(md.contains("> **INFO**"));
    }
}
