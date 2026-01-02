//! Comment data models

use crate::types::{CommentId, Extensions, FileId, LineId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A review comment attached to a line in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    /// Unique comment identifier
    pub id: CommentId,
    /// Reference to the line this comment is attached to
    pub line_ref: LineReference,
    /// Comment content
    pub content: String,
    /// Severity level
    pub severity: Severity,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// When the comment was created
    pub created_at: DateTime<Utc>,
    /// When the comment was last updated
    pub updated_at: DateTime<Utc>,
    /// Comment state (lifecycle)
    #[serde(default)]
    pub state: CommentState,
    /// Additional metadata
    #[serde(default)]
    pub metadata: CommentMetadata,
    /// Extensions for future compatibility
    #[serde(default, skip_serializing_if = "Extensions::is_empty")]
    pub extensions: Extensions,
}

impl Comment {
    /// Update the content and refresh updated_at
    pub fn update_content(&mut self, content: impl Into<String>) {
        self.content = content.into();
        self.updated_at = Utc::now();
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        let tag = tag.into();
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a tag
    pub fn remove_tag(&mut self, tag: &str) -> bool {
        if let Some(pos) = self.tags.iter().position(|t| t == tag) {
            self.tags.remove(pos);
            self.updated_at = Utc::now();
            true
        } else {
            false
        }
    }

    /// Change severity
    pub fn set_severity(&mut self, severity: Severity) {
        self.severity = severity;
        self.updated_at = Utc::now();
    }

    /// Change state
    pub fn set_state(&mut self, state: CommentState) {
        self.state = state;
        self.updated_at = Utc::now();
    }

    /// Get the file ID from the line reference
    pub fn file_id(&self) -> &FileId {
        match &self.line_ref {
            LineReference::SingleLine { file_id, .. } => file_id,
            LineReference::Range { file_id, .. } => file_id,
        }
    }

    /// Get the line ID(s) from the line reference
    pub fn line_ids(&self) -> Vec<&LineId> {
        match &self.line_ref {
            LineReference::SingleLine { line_id, .. } => vec![line_id],
            LineReference::Range { start_line_id, end_line_id, .. } => {
                vec![start_line_id, end_line_id]
            }
        }
    }
}

/// Reference to a line or range of lines in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LineReference {
    /// Single line reference
    SingleLine {
        /// File ID
        file_id: FileId,
        /// Line ID
        line_id: LineId,
        /// Which side of the diff
        side: DiffSide,
    },
    /// Range of lines (v2.0)
    Range {
        /// File ID
        file_id: FileId,
        /// Start line ID
        start_line_id: LineId,
        /// End line ID
        end_line_id: LineId,
        /// Which side of the diff
        side: DiffSide,
    },
}

impl LineReference {
    /// Create a single line reference
    pub fn single(file_id: FileId, line_id: LineId, side: DiffSide) -> Self {
        LineReference::SingleLine {
            file_id,
            line_id,
            side,
        }
    }

    /// Create a range reference
    pub fn range(
        file_id: FileId,
        start_line_id: LineId,
        end_line_id: LineId,
        side: DiffSide,
    ) -> Self {
        LineReference::Range {
            file_id,
            start_line_id,
            end_line_id,
            side,
        }
    }

    /// Check if this is a single line reference
    pub fn is_single(&self) -> bool {
        matches!(self, LineReference::SingleLine { .. })
    }

    /// Check if this is a range reference
    pub fn is_range(&self) -> bool {
        matches!(self, LineReference::Range { .. })
    }
}

/// Side of the diff (old/left or new/right)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffSide {
    /// Old/left side (deleted code)
    Old,
    /// New/right side (added code)
    New,
}

impl DiffSide {
    /// Convert to short string for export
    pub fn to_short_string(&self) -> &'static str {
        match self {
            DiffSide::Old => "old",
            DiffSide::New => "new",
        }
    }
}

/// Comment severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Severity {
    /// Informational - nice to have improvements
    Info,
    /// Warning - should be addressed
    Warning,
    /// Critical - must be fixed before merge
    Critical,
}

impl Severity {
    /// Convert to short string ("c"/"w"/"i")
    pub fn to_short_string(&self) -> &'static str {
        match self {
            Severity::Info => "i",
            Severity::Warning => "w",
            Severity::Critical => "c",
        }
    }

    /// Parse from short string
    pub fn from_short_string(s: &str) -> Option<Self> {
        match s {
            "i" | "info" => Some(Severity::Info),
            "w" | "warning" => Some(Severity::Warning),
            "c" | "critical" => Some(Severity::Critical),
            _ => None,
        }
    }

    /// Get display emoji
    pub fn emoji(&self) -> &'static str {
        match self {
            Severity::Info => "ℹ️",
            Severity::Warning => "⚠️",
            Severity::Critical => "🔴",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "Info"),
            Severity::Warning => write!(f, "Warning"),
            Severity::Critical => write!(f, "Critical"),
        }
    }
}

impl Default for Severity {
    fn default() -> Self {
        Severity::Info
    }
}

/// Comment lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CommentState {
    /// Newly created, needs attention
    Open,
    /// Agent has acknowledged (v2.0)
    Acknowledged,
    /// Issue has been resolved
    Resolved,
    /// Comment was dismissed (with reason)
    Dismissed,
    /// Code has changed, comment may be outdated (v2.0)
    Outdated,
}

impl Default for CommentState {
    fn default() -> Self {
        CommentState::Open
    }
}

impl CommentState {
    /// Check if this state means the comment is active
    pub fn is_active(&self) -> bool {
        matches!(self, CommentState::Open | CommentState::Acknowledged)
    }

    /// Check if this state means the comment is closed
    pub fn is_closed(&self) -> bool {
        matches!(self, CommentState::Resolved | CommentState::Dismissed)
    }
}

/// Additional comment metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommentMetadata {
    /// Author of the comment
    pub author: Option<String>,
    /// Source of the comment (manual/auto)
    pub source: Option<String>,
    /// Line number for display
    pub line_number: Option<usize>,
    /// File path for display
    pub file_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_comment() -> Comment {
        Comment {
            id: CommentId::new(),
            line_ref: LineReference::single(
                FileId::from_string("test-file"),
                LineId::from_string("test-line"),
                DiffSide::New,
            ),
            content: "Test comment".to_string(),
            severity: Severity::Warning,
            tags: vec!["test".to_string()],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            state: CommentState::Open,
            metadata: CommentMetadata::default(),
            extensions: Extensions::new(),
        }
    }

    #[test]
    fn test_comment_creation() {
        let comment = create_test_comment();
        assert_eq!(comment.content, "Test comment");
        assert_eq!(comment.severity, Severity::Warning);
        assert!(comment.state.is_active());
    }

    #[test]
    fn test_comment_update() {
        let mut comment = create_test_comment();
        let old_updated = comment.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        comment.update_content("New content");
        assert_eq!(comment.content, "New content");
        assert!(comment.updated_at > old_updated);
    }

    #[test]
    fn test_tags() {
        let mut comment = create_test_comment();
        comment.add_tag("security");
        assert!(comment.tags.contains(&"security".to_string()));

        comment.add_tag("security"); // Duplicate
        assert_eq!(comment.tags.iter().filter(|t| *t == "security").count(), 1);

        assert!(comment.remove_tag("security"));
        assert!(!comment.tags.contains(&"security".to_string()));
    }

    #[test]
    fn test_severity_short_string() {
        assert_eq!(Severity::Critical.to_short_string(), "c");
        assert_eq!(Severity::Warning.to_short_string(), "w");
        assert_eq!(Severity::Info.to_short_string(), "i");

        assert_eq!(Severity::from_short_string("c"), Some(Severity::Critical));
        assert_eq!(Severity::from_short_string("critical"), Some(Severity::Critical));
    }

    #[test]
    fn test_comment_state() {
        assert!(CommentState::Open.is_active());
        assert!(CommentState::Acknowledged.is_active());
        assert!(CommentState::Resolved.is_closed());
        assert!(CommentState::Dismissed.is_closed());
        assert!(!CommentState::Outdated.is_active());
    }

    #[test]
    fn test_line_reference() {
        let single = LineReference::single(
            FileId::from_string("f1"),
            LineId::from_string("l1"),
            DiffSide::New,
        );
        assert!(single.is_single());
        assert!(!single.is_range());

        let range = LineReference::range(
            FileId::from_string("f1"),
            LineId::from_string("l1"),
            LineId::from_string("l2"),
            DiffSide::New,
        );
        assert!(range.is_range());
        assert!(!range.is_single());
    }

    #[test]
    fn test_comment_serialization() {
        let comment = create_test_comment();
        let json = serde_json::to_string(&comment).unwrap();
        let comment2: Comment = serde_json::from_str(&json).unwrap();
        assert_eq!(comment.content, comment2.content);
        assert_eq!(comment.severity, comment2.severity);
    }
}
