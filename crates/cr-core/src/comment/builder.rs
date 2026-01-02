//! Comment builder for fluent API

use super::model::{Comment, CommentMetadata, CommentState, DiffSide, LineReference, Severity};
use crate::error::{CrHelperError, Result};
use crate::types::{CommentId, Extensions, FileId, LineId};
use chrono::Utc;

/// Builder for creating comments with fluent API
pub struct CommentBuilder {
    line_ref: LineReference,
    content: Option<String>,
    severity: Severity,
    tags: Vec<String>,
    state: CommentState,
    metadata: CommentMetadata,
    extensions: Extensions,
}

impl CommentBuilder {
    /// Create a new builder for a single line comment
    pub fn new(file_id: FileId, line_id: LineId, side: DiffSide) -> Self {
        Self {
            line_ref: LineReference::single(file_id, line_id, side),
            content: None,
            severity: Severity::Info,
            tags: Vec::new(),
            state: CommentState::Open,
            metadata: CommentMetadata::default(),
            extensions: Extensions::new(),
        }
    }

    /// Create a new builder for a range comment
    pub fn new_range(
        file_id: FileId,
        start_line_id: LineId,
        end_line_id: LineId,
        side: DiffSide,
    ) -> Self {
        Self {
            line_ref: LineReference::range(file_id, start_line_id, end_line_id, side),
            content: None,
            severity: Severity::Info,
            tags: Vec::new(),
            state: CommentState::Open,
            metadata: CommentMetadata::default(),
            extensions: Extensions::new(),
        }
    }

    /// Create from an existing line reference
    pub fn from_line_ref(line_ref: LineReference) -> Self {
        Self {
            line_ref,
            content: None,
            severity: Severity::Info,
            tags: Vec::new(),
            state: CommentState::Open,
            metadata: CommentMetadata::default(),
            extensions: Extensions::new(),
        }
    }

    /// Set the comment content
    pub fn content(mut self, content: impl Into<String>) -> Self {
        self.content = Some(content.into());
        self
    }

    /// Set the severity level
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = severity;
        self
    }

    /// Set severity to Info
    pub fn info(self) -> Self {
        self.severity(Severity::Info)
    }

    /// Set severity to Warning
    pub fn warning(self) -> Self {
        self.severity(Severity::Warning)
    }

    /// Set severity to Critical
    pub fn critical(self) -> Self {
        self.severity(Severity::Critical)
    }

    /// Add a tag
    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Add multiple tags
    pub fn tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags.extend(tags.into_iter().map(|t| t.into()));
        self
    }

    /// Set the initial state
    pub fn state(mut self, state: CommentState) -> Self {
        self.state = state;
        self
    }

    /// Set author
    pub fn author(mut self, author: impl Into<String>) -> Self {
        self.metadata.author = Some(author.into());
        self
    }

    /// Set source
    pub fn source(mut self, source: impl Into<String>) -> Self {
        self.metadata.source = Some(source.into());
        self
    }

    /// Set line number for display
    pub fn line_number(mut self, line_number: usize) -> Self {
        self.metadata.line_number = Some(line_number);
        self
    }

    /// Set file path for display
    pub fn file_path(mut self, file_path: impl Into<String>) -> Self {
        self.metadata.file_path = Some(file_path.into());
        self
    }

    /// Set suggested fix (extension)
    pub fn suggested_fix(mut self, fix: impl Into<String>) -> Self {
        self.extensions.set_suggested_fix(fix);
        self
    }

    /// Set related reviews (extension)
    pub fn related_reviews(mut self, reviews: Vec<String>) -> Self {
        self.extensions.set_related_reviews(reviews);
        self
    }

    /// Build the comment
    pub fn build(self) -> Result<Comment> {
        let content = self.content.ok_or_else(|| {
            CrHelperError::Validation("Comment content is required".to_string())
        })?;

        if content.trim().is_empty() {
            return Err(CrHelperError::Validation(
                "Comment content cannot be empty".to_string(),
            ));
        }

        let now = Utc::now();

        Ok(Comment {
            id: CommentId::new(),
            line_ref: self.line_ref,
            content,
            severity: self.severity,
            tags: self.tags,
            created_at: now,
            updated_at: now,
            state: self.state,
            metadata: self.metadata,
            extensions: self.extensions,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_builder() {
        let comment = CommentBuilder::new(
            FileId::from_string("file1"),
            LineId::from_string("line1"),
            DiffSide::New,
        )
        .content("Test comment")
        .build()
        .unwrap();

        assert_eq!(comment.content, "Test comment");
        assert_eq!(comment.severity, Severity::Info);
    }

    #[test]
    fn test_builder_with_severity() {
        let comment = CommentBuilder::new(
            FileId::from_string("file1"),
            LineId::from_string("line1"),
            DiffSide::New,
        )
        .content("Critical issue")
        .critical()
        .build()
        .unwrap();

        assert_eq!(comment.severity, Severity::Critical);
    }

    #[test]
    fn test_builder_with_tags() {
        let comment = CommentBuilder::new(
            FileId::from_string("file1"),
            LineId::from_string("line1"),
            DiffSide::New,
        )
        .content("Security issue")
        .tag("security")
        .tag("urgent")
        .build()
        .unwrap();

        assert_eq!(comment.tags.len(), 2);
        assert!(comment.tags.contains(&"security".to_string()));
    }

    #[test]
    fn test_builder_with_metadata() {
        let comment = CommentBuilder::new(
            FileId::from_string("file1"),
            LineId::from_string("line1"),
            DiffSide::New,
        )
        .content("Test")
        .author("john")
        .line_number(42)
        .build()
        .unwrap();

        assert_eq!(comment.metadata.author, Some("john".to_string()));
        assert_eq!(comment.metadata.line_number, Some(42));
    }

    #[test]
    fn test_builder_with_extensions() {
        let comment = CommentBuilder::new(
            FileId::from_string("file1"),
            LineId::from_string("line1"),
            DiffSide::New,
        )
        .content("Test")
        .suggested_fix("Use Result<T>")
        .build()
        .unwrap();

        assert_eq!(comment.extensions.suggested_fix(), Some("Use Result<T>"));
    }

    #[test]
    fn test_builder_without_content_fails() {
        let result = CommentBuilder::new(
            FileId::from_string("file1"),
            LineId::from_string("line1"),
            DiffSide::New,
        )
        .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_builder_with_empty_content_fails() {
        let result = CommentBuilder::new(
            FileId::from_string("file1"),
            LineId::from_string("line1"),
            DiffSide::New,
        )
        .content("   ")
        .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_range_builder() {
        let comment = CommentBuilder::new_range(
            FileId::from_string("file1"),
            LineId::from_string("line1"),
            LineId::from_string("line5"),
            DiffSide::New,
        )
        .content("Range comment")
        .build()
        .unwrap();

        assert!(comment.line_ref.is_range());
    }
}
