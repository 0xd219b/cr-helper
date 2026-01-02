//! JSON exporter for sessions

use super::context::ContextExtractor;
use super::exporter::Exporter;
use crate::comment::model::Severity;
use crate::error::Result;
use crate::session::Session;
use crate::types::ProtocolVersion;
use serde::{Deserialize, Serialize};

/// JSON exporter with compact mode support
pub struct JsonExporter {
    /// Whether to use pretty-print formatting
    pretty: bool,
    /// Whether to use compact field names
    compact: bool,
    /// Format name
    name: String,
    /// Context extractor
    context: ContextExtractor,
}

impl JsonExporter {
    /// Create a new JSON exporter
    pub fn new(compact: bool) -> Self {
        Self {
            pretty: !compact,
            compact,
            name: if compact {
                "json-compact".to_string()
            } else {
                "json".to_string()
            },
            context: ContextExtractor::new(2),
        }
    }

    /// Create a compact JSON exporter
    pub fn compact() -> Self {
        Self::new(true)
    }

    /// Create a pretty-printed JSON exporter
    pub fn pretty() -> Self {
        Self::new(false)
    }

    /// Set the context lines
    pub fn with_context_lines(mut self, lines: usize) -> Self {
        self.context = ContextExtractor::new(lines);
        self
    }
}

impl Exporter for JsonExporter {
    fn export(&self, session: &Session) -> Result<String> {
        let data = ExportData::from_session(session, &self.context);

        let json = if self.pretty {
            serde_json::to_string_pretty(&data)?
        } else {
            serde_json::to_string(&data)?
        };

        Ok(json)
    }

    fn format_name(&self) -> &str {
        &self.name
    }

    fn file_extension(&self) -> &str {
        "json"
    }
}

/// Exported data structure (compact field names for token optimization)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportData {
    /// Protocol version
    pub v: String,
    /// Session ID
    pub sid: String,
    /// Unix timestamp
    pub ts: i64,
    /// Repository path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    /// Statistics
    pub stats: ExportStats,
    /// Reviews (comments)
    pub reviews: Vec<ExportReview>,
}

impl ExportData {
    /// Create from a session
    pub fn from_session(session: &Session, context: &ContextExtractor) -> Self {
        let reviews: Vec<ExportReview> = session
            .comments
            .all()
            .iter()
            .map(|c| ExportReview::from_comment(c, &session.diff_data, context))
            .collect();

        let stats = ExportStats::from_session(session);

        Self {
            v: ProtocolVersion::V1_0.to_string(),
            sid: session.id.to_string(),
            ts: session.created_at.timestamp(),
            repo: session
                .metadata
                .repository
                .as_ref()
                .map(|p| p.to_string_lossy().to_string()),
            stats,
            reviews,
        }
    }
}

/// Export statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportStats {
    /// File count
    pub f: usize,
    /// Comment count
    pub c: usize,
    /// Severity breakdown
    pub sev: SeverityStats,
}

impl ExportStats {
    /// Create from a session
    pub fn from_session(session: &Session) -> Self {
        let counts = session.comments.count_by_severity();

        Self {
            f: session.file_count(),
            c: session.comment_count(),
            sev: SeverityStats {
                c: *counts.get(&Severity::Critical).unwrap_or(&0),
                w: *counts.get(&Severity::Warning).unwrap_or(&0),
                i: *counts.get(&Severity::Info).unwrap_or(&0),
            },
        }
    }
}

/// Severity statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityStats {
    /// Critical count
    pub c: usize,
    /// Warning count
    pub w: usize,
    /// Info count
    pub i: usize,
}

/// Exported review (comment)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportReview {
    /// Comment ID
    pub id: String,
    /// File path
    pub file: String,
    /// Location
    pub loc: ExportLocation,
    /// Severity (c/w/i)
    pub sev: String,
    /// Message content
    pub msg: String,
    /// Tags
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Code context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ctx: Option<String>,
    /// State
    pub state: String,
    /// Timestamp
    pub ts: i64,
}

impl ExportReview {
    /// Create from a comment
    pub fn from_comment(
        comment: &crate::comment::model::Comment,
        diff: &crate::diff::DiffData,
        context: &ContextExtractor,
    ) -> Self {
        let file_path = comment
            .metadata
            .file_path
            .clone()
            .unwrap_or_else(|| comment.file_id().to_string());

        let line_num = comment.metadata.line_number;

        // Convert CodeContext to simple string for JSON
        let ctx = context.extract(comment, diff).map(|c| {
            c.lines.iter()
                .map(|l| {
                    let line_num = l.line_num.map(|n| format!("{:>4}", n)).unwrap_or_else(|| "    ".to_string());
                    format!("{} {}{}", line_num, l.prefix, l.content)
                })
                .collect::<Vec<_>>()
                .join("\n")
        });

        Self {
            id: comment.id.to_string(),
            file: file_path,
            loc: ExportLocation::from_comment(comment, line_num),
            sev: comment.severity.to_short_string().to_string(),
            msg: comment.content.clone(),
            tags: comment.tags.clone(),
            ctx,
            state: format!("{:?}", comment.state).to_lowercase(),
            ts: comment.created_at.timestamp(),
        }
    }
}

/// Export location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportLocation {
    /// Line number (single or range)
    pub ln: LineNumber,
    /// Side (old/new)
    pub side: String,
}

impl ExportLocation {
    /// Create from a comment
    pub fn from_comment(
        comment: &crate::comment::model::Comment,
        line_num: Option<usize>,
    ) -> Self {
        use crate::comment::model::LineReference;

        let (ln, side) = match &comment.line_ref {
            LineReference::SingleLine { side, .. } => {
                let num = line_num.unwrap_or(0);
                (LineNumber::Single(num), side.to_short_string().to_string())
            }
            LineReference::Range { side, .. } => {
                // For ranges, we'd need to track both line numbers
                let num = line_num.unwrap_or(0);
                (LineNumber::Single(num), side.to_short_string().to_string())
            }
        };

        Self { ln, side }
    }
}

/// Line number (single or range)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LineNumber {
    /// Single line
    Single(usize),
    /// Range [start, end]
    Range(usize, usize),
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
        .content("Critical issue found")
        .critical()
        .tag("security")
        .line_number(10)
        .file_path("src/main.rs")
        .build()
        .unwrap();

        let comment2 = CommentBuilder::new(
            FileId::from_string("file2"),
            LineId::from_string("line2"),
            DiffSide::New,
        )
        .content("Consider refactoring")
        .warning()
        .line_number(25)
        .file_path("src/lib.rs")
        .build()
        .unwrap();

        session.comments.add(comment1).unwrap();
        session.comments.add(comment2).unwrap();

        session
    }

    #[test]
    fn test_json_exporter_creation() {
        let exporter = JsonExporter::new(false);
        assert_eq!(exporter.format_name(), "json");
        assert_eq!(exporter.file_extension(), "json");
    }

    #[test]
    fn test_json_compact_exporter() {
        let exporter = JsonExporter::compact();
        assert_eq!(exporter.format_name(), "json-compact");
    }

    #[test]
    fn test_export_empty_session() {
        let exporter = JsonExporter::new(false);
        let session = create_test_session();

        let result = exporter.export(&session);
        assert!(result.is_ok());

        let json = result.unwrap();
        assert!(json.contains("\"v\":"));
        assert!(json.contains("\"sid\":"));
        assert!(json.contains("\"reviews\":"));
    }

    #[test]
    fn test_export_session_with_comments() {
        let exporter = JsonExporter::new(false);
        let session = create_session_with_comments();

        let result = exporter.export(&session);
        assert!(result.is_ok());

        let json = result.unwrap();
        assert!(json.contains("Critical issue found"));
        assert!(json.contains("security"));
        // Pretty-print uses spaces around colon
        assert!(json.contains("\"sev\": \"c\"") || json.contains("\"sev\":\"c\""));
    }

    #[test]
    fn test_compact_vs_pretty() {
        let session = create_session_with_comments();

        let pretty = JsonExporter::pretty().export(&session).unwrap();
        let compact = JsonExporter::compact().export(&session).unwrap();

        // Compact should be smaller
        assert!(compact.len() < pretty.len());
    }

    #[test]
    fn test_export_data_serialization() {
        let session = create_test_session();
        let context = ContextExtractor::new(2);
        let data = ExportData::from_session(&session, &context);

        let json = serde_json::to_string(&data).unwrap();
        let parsed: ExportData = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.v, data.v);
        assert_eq!(parsed.sid, data.sid);
    }

    #[test]
    fn test_export_stats() {
        let session = create_session_with_comments();
        let stats = ExportStats::from_session(&session);

        assert_eq!(stats.c, 2); // 2 comments
        assert_eq!(stats.sev.c, 1); // 1 critical
        assert_eq!(stats.sev.w, 1); // 1 warning
        assert_eq!(stats.sev.i, 0); // 0 info
    }

    #[test]
    fn test_line_number_serialization() {
        let single = LineNumber::Single(42);
        let json = serde_json::to_string(&single).unwrap();
        assert_eq!(json, "42");

        let range = LineNumber::Range(10, 20);
        let json = serde_json::to_string(&range).unwrap();
        assert_eq!(json, "[10,20]");
    }
}
