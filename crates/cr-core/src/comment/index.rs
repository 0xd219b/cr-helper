//! Comment indexing for fast lookup

use super::model::{Comment, LineReference, Severity};
use crate::types::{CommentId, FileId, LineId};
use std::collections::HashMap;

/// Multi-dimensional index for comments
#[derive(Debug, Clone, Default)]
pub struct CommentIndex {
    /// Index by line ID
    by_line: HashMap<LineId, Vec<CommentId>>,
    /// Index by file ID
    by_file: HashMap<FileId, Vec<CommentId>>,
    /// Index by severity
    by_severity: HashMap<Severity, Vec<CommentId>>,
}

impl CommentIndex {
    /// Create a new empty index
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a comment to the index
    pub fn add(&mut self, comment: &Comment) {
        // Index by line
        for line_id in comment.line_ids() {
            self.by_line
                .entry(line_id.clone())
                .or_default()
                .push(comment.id.clone());
        }

        // Index by file
        self.by_file
            .entry(comment.file_id().clone())
            .or_default()
            .push(comment.id.clone());

        // Index by severity
        self.by_severity
            .entry(comment.severity)
            .or_default()
            .push(comment.id.clone());
    }

    /// Remove a comment from the index
    pub fn remove(&mut self, comment: &Comment) {
        // Remove from line index
        for line_id in comment.line_ids() {
            if let Some(ids) = self.by_line.get_mut(line_id) {
                ids.retain(|id| id != &comment.id);
                if ids.is_empty() {
                    self.by_line.remove(line_id);
                }
            }
        }

        // Remove from file index
        if let Some(ids) = self.by_file.get_mut(comment.file_id()) {
            ids.retain(|id| id != &comment.id);
            if ids.is_empty() {
                self.by_file.remove(comment.file_id());
            }
        }

        // Remove from severity index
        if let Some(ids) = self.by_severity.get_mut(&comment.severity) {
            ids.retain(|id| id != &comment.id);
            if ids.is_empty() {
                self.by_severity.remove(&comment.severity);
            }
        }
    }

    /// Get comments by line ID
    pub fn get_by_line(&self, line_id: &LineId) -> Vec<CommentId> {
        self.by_line.get(line_id).cloned().unwrap_or_default()
    }

    /// Get comments by file ID
    pub fn get_by_file(&self, file_id: &FileId) -> Vec<CommentId> {
        self.by_file.get(file_id).cloned().unwrap_or_default()
    }

    /// Get comments by severity
    pub fn get_by_severity(&self, severity: Severity) -> Vec<CommentId> {
        self.by_severity.get(&severity).cloned().unwrap_or_default()
    }

    /// Check if a line has any comments
    pub fn has_comments_on_line(&self, line_id: &LineId) -> bool {
        self.by_line
            .get(line_id)
            .map(|ids| !ids.is_empty())
            .unwrap_or(false)
    }

    /// Get comment count for a file
    pub fn file_comment_count(&self, file_id: &FileId) -> usize {
        self.by_file.get(file_id).map(|ids| ids.len()).unwrap_or(0)
    }

    /// Get all file IDs that have comments
    pub fn files_with_comments(&self) -> Vec<&FileId> {
        self.by_file.keys().collect()
    }

    /// Clear the entire index
    pub fn clear(&mut self) {
        self.by_line.clear();
        self.by_file.clear();
        self.by_severity.clear();
    }

    /// Rebuild index from a collection of comments
    pub fn rebuild(&mut self, comments: impl IntoIterator<Item = impl std::borrow::Borrow<Comment>>) {
        self.clear();
        for comment in comments {
            self.add(comment.borrow());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comment::model::{CommentState, DiffSide};
    use chrono::Utc;

    fn create_test_comment(file: &str, line: &str, severity: Severity) -> Comment {
        Comment {
            id: CommentId::new(),
            line_ref: LineReference::SingleLine {
                file_id: FileId::from_string(file),
                line_id: LineId::from_string(line),
                side: DiffSide::New,
            },
            content: "Test".to_string(),
            severity,
            tags: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
            state: CommentState::Open,
            metadata: Default::default(),
            extensions: Default::default(),
        }
    }

    #[test]
    fn test_add_and_get_by_line() {
        let mut index = CommentIndex::new();
        let comment = create_test_comment("file1", "line1", Severity::Warning);
        let line_id = LineId::from_string("line1");

        index.add(&comment);

        let results = index.get_by_line(&line_id);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], comment.id);
    }

    #[test]
    fn test_add_and_get_by_file() {
        let mut index = CommentIndex::new();
        let comment1 = create_test_comment("file1", "line1", Severity::Warning);
        let comment2 = create_test_comment("file1", "line2", Severity::Info);
        let file_id = FileId::from_string("file1");

        index.add(&comment1);
        index.add(&comment2);

        let results = index.get_by_file(&file_id);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_add_and_get_by_severity() {
        let mut index = CommentIndex::new();
        let comment1 = create_test_comment("file1", "line1", Severity::Critical);
        let comment2 = create_test_comment("file1", "line2", Severity::Critical);
        let comment3 = create_test_comment("file1", "line3", Severity::Warning);

        index.add(&comment1);
        index.add(&comment2);
        index.add(&comment3);

        assert_eq!(index.get_by_severity(Severity::Critical).len(), 2);
        assert_eq!(index.get_by_severity(Severity::Warning).len(), 1);
        assert_eq!(index.get_by_severity(Severity::Info).len(), 0);
    }

    #[test]
    fn test_remove() {
        let mut index = CommentIndex::new();
        let comment = create_test_comment("file1", "line1", Severity::Warning);
        let line_id = LineId::from_string("line1");

        index.add(&comment);
        assert_eq!(index.get_by_line(&line_id).len(), 1);

        index.remove(&comment);
        assert_eq!(index.get_by_line(&line_id).len(), 0);
    }

    #[test]
    fn test_has_comments_on_line() {
        let mut index = CommentIndex::new();
        let line_id = LineId::from_string("line1");

        assert!(!index.has_comments_on_line(&line_id));

        let comment = create_test_comment("file1", "line1", Severity::Warning);
        index.add(&comment);

        assert!(index.has_comments_on_line(&line_id));
    }

    #[test]
    fn test_file_comment_count() {
        let mut index = CommentIndex::new();
        let file_id = FileId::from_string("file1");

        assert_eq!(index.file_comment_count(&file_id), 0);

        index.add(&create_test_comment("file1", "line1", Severity::Warning));
        index.add(&create_test_comment("file1", "line2", Severity::Info));

        assert_eq!(index.file_comment_count(&file_id), 2);
    }

    #[test]
    fn test_clear() {
        let mut index = CommentIndex::new();
        index.add(&create_test_comment("file1", "line1", Severity::Warning));

        assert!(!index.by_line.is_empty());

        index.clear();

        assert!(index.by_line.is_empty());
        assert!(index.by_file.is_empty());
        assert!(index.by_severity.is_empty());
    }
}
