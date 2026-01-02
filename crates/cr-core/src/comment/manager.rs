//! Comment manager for CRUD operations

use super::index::CommentIndex;
use super::model::{Comment, CommentState, Severity};
use crate::error::{CrHelperError, Result};
use crate::types::{CommentId, FileId, LineId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Manager for comments with indexing support
#[derive(Debug, Clone, Serialize)]
pub struct CommentManager {
    /// All comments by ID
    comments: HashMap<CommentId, Comment>,
    /// Multi-dimensional index
    #[serde(skip)]
    index: CommentIndex,
}

impl CommentManager {
    /// Create a new empty comment manager
    pub fn new() -> Self {
        Self {
            comments: HashMap::new(),
            index: CommentIndex::new(),
        }
    }

    /// Add a comment
    pub fn add(&mut self, comment: Comment) -> Result<CommentId> {
        let id = comment.id.clone();

        if self.comments.contains_key(&id) {
            return Err(CrHelperError::Validation(format!(
                "Comment with ID {} already exists",
                id
            )));
        }

        self.index.add(&comment);
        self.comments.insert(id.clone(), comment);
        Ok(id)
    }

    /// Get a comment by ID
    pub fn get(&self, id: &CommentId) -> Option<&Comment> {
        self.comments.get(id)
    }

    /// Get a mutable comment by ID
    pub fn get_mut(&mut self, id: &CommentId) -> Option<&mut Comment> {
        self.comments.get_mut(id)
    }

    /// Update comment content
    pub fn update(&mut self, id: &CommentId, content: String) -> Result<()> {
        let comment = self.comments.get_mut(id).ok_or_else(|| {
            CrHelperError::CommentNotFound(id.to_string())
        })?;

        comment.update_content(content);
        Ok(())
    }

    /// Update comment state
    pub fn update_state(&mut self, id: &CommentId, state: CommentState) -> Result<()> {
        let comment = self.comments.get_mut(id).ok_or_else(|| {
            CrHelperError::CommentNotFound(id.to_string())
        })?;

        comment.set_state(state);
        Ok(())
    }

    /// Delete a comment
    pub fn delete(&mut self, id: &CommentId) -> Result<Comment> {
        let comment = self.comments.remove(id).ok_or_else(|| {
            CrHelperError::CommentNotFound(id.to_string())
        })?;

        self.index.remove(&comment);
        Ok(comment)
    }

    /// Delete all comments for a file
    pub fn delete_by_file(&mut self, file_id: &FileId) -> usize {
        let ids: Vec<CommentId> = self.index.get_by_file(file_id);
        let count = ids.len();

        for id in ids {
            if let Some(comment) = self.comments.remove(&id) {
                self.index.remove(&comment);
            }
        }

        count
    }

    /// Get all comments
    pub fn all(&self) -> Vec<&Comment> {
        self.comments.values().collect()
    }

    /// Get all comments sorted by creation time
    pub fn all_sorted(&self) -> Vec<&Comment> {
        let mut comments: Vec<_> = self.comments.values().collect();
        comments.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        comments
    }

    /// Get comments by line ID
    pub fn get_by_line(&self, line_id: &LineId) -> Vec<&Comment> {
        self.index
            .get_by_line(line_id)
            .iter()
            .filter_map(|id| self.comments.get(id))
            .collect()
    }

    /// Get comments by file ID
    pub fn get_by_file(&self, file_id: &FileId) -> Vec<&Comment> {
        self.index
            .get_by_file(file_id)
            .iter()
            .filter_map(|id| self.comments.get(id))
            .collect()
    }

    /// Get comments by severity
    pub fn get_by_severity(&self, severity: Severity) -> Vec<&Comment> {
        self.index
            .get_by_severity(severity)
            .iter()
            .filter_map(|id| self.comments.get(id))
            .collect()
    }

    /// Search comments by content
    pub fn search(&self, query: &str) -> Vec<&Comment> {
        let query_lower = query.to_lowercase();
        self.comments
            .values()
            .filter(|c| {
                c.content.to_lowercase().contains(&query_lower)
                    || c.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect()
    }

    /// Get total comment count
    pub fn count(&self) -> usize {
        self.comments.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.comments.is_empty()
    }

    /// Get count by severity
    pub fn count_by_severity(&self) -> HashMap<Severity, usize> {
        let mut counts = HashMap::new();
        for comment in self.comments.values() {
            *counts.entry(comment.severity).or_insert(0) += 1;
        }
        counts
    }

    /// Get active (open/acknowledged) comments
    pub fn get_active(&self) -> Vec<&Comment> {
        self.comments
            .values()
            .filter(|c| c.state.is_active())
            .collect()
    }

    /// Rebuild index (after deserialization)
    pub fn rebuild_index(&mut self) {
        self.index = CommentIndex::new();
        for comment in self.comments.values() {
            self.index.add(comment);
        }
    }
}

impl Default for CommentManager {
    fn default() -> Self {
        Self::new()
    }
}

// Custom deserialization to rebuild index
impl<'de> serde::de::Deserialize<'de> for CommentManager {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct CommentManagerHelper {
            comments: HashMap<CommentId, Comment>,
        }

        let helper = CommentManagerHelper::deserialize(deserializer)?;
        let mut manager = Self {
            comments: helper.comments,
            index: CommentIndex::new(),
        };
        manager.rebuild_index();
        Ok(manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::comment::model::{DiffSide, LineReference};

    fn create_test_comment(content: &str, severity: Severity) -> Comment {
        Comment {
            id: CommentId::new(),
            line_ref: LineReference::single(
                FileId::from_string("test-file"),
                LineId::from_string("test-line"),
                DiffSide::New,
            ),
            content: content.to_string(),
            severity,
            tags: vec![],
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
            state: CommentState::default(),
            metadata: Default::default(),
            extensions: Default::default(),
        }
    }

    #[test]
    fn test_add_and_get() {
        let mut manager = CommentManager::new();
        let comment = create_test_comment("Test", Severity::Warning);
        let id = comment.id.clone();

        manager.add(comment).unwrap();

        assert!(manager.get(&id).is_some());
        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn test_duplicate_add_fails() {
        let mut manager = CommentManager::new();
        let comment = create_test_comment("Test", Severity::Warning);

        manager.add(comment.clone()).unwrap();
        assert!(manager.add(comment).is_err());
    }

    #[test]
    fn test_update() {
        let mut manager = CommentManager::new();
        let comment = create_test_comment("Original", Severity::Warning);
        let id = comment.id.clone();

        manager.add(comment).unwrap();
        manager.update(&id, "Updated".to_string()).unwrap();

        assert_eq!(manager.get(&id).unwrap().content, "Updated");
    }

    #[test]
    fn test_delete() {
        let mut manager = CommentManager::new();
        let comment = create_test_comment("Test", Severity::Warning);
        let id = comment.id.clone();

        manager.add(comment).unwrap();
        assert_eq!(manager.count(), 1);

        manager.delete(&id).unwrap();
        assert_eq!(manager.count(), 0);
        assert!(manager.get(&id).is_none());
    }

    #[test]
    fn test_get_by_severity() {
        let mut manager = CommentManager::new();

        manager.add(create_test_comment("Info 1", Severity::Info)).unwrap();
        manager.add(create_test_comment("Warning 1", Severity::Warning)).unwrap();
        manager.add(create_test_comment("Critical 1", Severity::Critical)).unwrap();
        manager.add(create_test_comment("Warning 2", Severity::Warning)).unwrap();

        assert_eq!(manager.get_by_severity(Severity::Warning).len(), 2);
        assert_eq!(manager.get_by_severity(Severity::Critical).len(), 1);
        assert_eq!(manager.get_by_severity(Severity::Info).len(), 1);
    }

    #[test]
    fn test_search() {
        let mut manager = CommentManager::new();

        manager.add(create_test_comment("Fix this bug", Severity::Critical)).unwrap();
        manager.add(create_test_comment("Improve performance", Severity::Warning)).unwrap();
        manager.add(create_test_comment("Another bug fix", Severity::Info)).unwrap();

        assert_eq!(manager.search("bug").len(), 2);
        assert_eq!(manager.search("performance").len(), 1);
        assert_eq!(manager.search("nonexistent").len(), 0);
    }

    #[test]
    fn test_count_by_severity() {
        let mut manager = CommentManager::new();

        manager.add(create_test_comment("1", Severity::Info)).unwrap();
        manager.add(create_test_comment("2", Severity::Warning)).unwrap();
        manager.add(create_test_comment("3", Severity::Warning)).unwrap();

        let counts = manager.count_by_severity();
        assert_eq!(counts.get(&Severity::Info), Some(&1));
        assert_eq!(counts.get(&Severity::Warning), Some(&2));
        assert_eq!(counts.get(&Severity::Critical), None);
    }

    #[test]
    fn test_serialization() {
        let mut manager = CommentManager::new();
        manager.add(create_test_comment("Test", Severity::Warning)).unwrap();

        let json = serde_json::to_string(&manager).unwrap();
        let manager2: CommentManager = serde_json::from_str(&json).unwrap();

        assert_eq!(manager.count(), manager2.count());
    }
}
