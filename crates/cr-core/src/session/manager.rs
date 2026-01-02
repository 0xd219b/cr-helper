//! Session manager for CRUD operations

use super::model::{DiffSource, Session, SessionFilter, SessionInfo, SessionMetadata};
use super::persistence::SessionStorage;
use crate::diff::DiffData;
use crate::error::{CrHelperError, Result};
use crate::types::SessionId;
use chrono::{DateTime, Utc};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Manager for session lifecycle
pub struct SessionManager {
    /// Storage backend
    storage: Arc<dyn SessionStorage>,
    /// Auto-save throttle duration
    auto_save_interval: Duration,
    /// Last auto-save time
    last_auto_save: Option<Instant>,
}

impl SessionManager {
    /// Create a new session manager with the given storage
    pub fn new(storage: impl SessionStorage + 'static) -> Self {
        Self {
            storage: Arc::new(storage),
            auto_save_interval: Duration::from_secs(30),
            last_auto_save: None,
        }
    }

    /// Create a new session manager with shared storage
    pub fn with_storage(storage: Arc<dyn SessionStorage>) -> Self {
        Self {
            storage,
            auto_save_interval: Duration::from_secs(30),
            last_auto_save: None,
        }
    }

    /// Set the auto-save interval
    pub fn set_auto_save_interval(&mut self, interval: Duration) {
        self.auto_save_interval = interval;
    }

    /// Create a new session from diff source
    pub fn create(&self, diff_source: DiffSource, diff_data: DiffData) -> Result<Session> {
        let session = Session::new(diff_source, diff_data);
        self.storage.save(&session)?;
        Ok(session)
    }

    /// Create a new session with a specific ID
    pub fn create_with_id(
        &self,
        id: SessionId,
        diff_source: DiffSource,
        diff_data: DiffData,
    ) -> Result<Session> {
        if self.storage.exists(&id) {
            return Err(CrHelperError::Validation(format!(
                "Session with ID {} already exists",
                id
            )));
        }
        let session = Session::with_id(id, diff_source, diff_data);
        self.storage.save(&session)?;
        Ok(session)
    }

    /// Create a new session with metadata
    pub fn create_with_metadata(
        &self,
        diff_source: DiffSource,
        diff_data: DiffData,
        metadata: SessionMetadata,
    ) -> Result<Session> {
        let mut session = Session::new(diff_source, diff_data);
        session.metadata = metadata;
        self.storage.save(&session)?;
        Ok(session)
    }

    /// Load a session by ID
    pub fn load(&self, id: &SessionId) -> Result<Session> {
        self.storage.load(id)
    }

    /// Load the most recently updated session
    pub fn load_latest(&self) -> Result<Option<Session>> {
        self.storage.latest()
    }

    /// Save a session
    pub fn save(&self, session: &mut Session) -> Result<()> {
        session.touch();
        self.storage.save(session)
    }

    /// Auto-save with throttling
    pub fn auto_save(&mut self, session: &mut Session) -> Result<bool> {
        let now = Instant::now();

        if let Some(last) = self.last_auto_save {
            if now.duration_since(last) < self.auto_save_interval {
                return Ok(false);
            }
        }

        self.save(session)?;
        self.last_auto_save = Some(now);
        Ok(true)
    }

    /// Force auto-save reset (for testing)
    pub fn reset_auto_save(&mut self) {
        self.last_auto_save = None;
    }

    /// List all sessions
    pub fn list(&self) -> Result<Vec<SessionInfo>> {
        let mut sessions = self.storage.list()?;
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(sessions)
    }

    /// Search sessions with filter
    pub fn search(&self, filter: SessionFilter) -> Result<Vec<SessionInfo>> {
        let sessions = self.list()?;
        Ok(sessions.into_iter().filter(|s| filter.matches(s)).collect())
    }

    /// Delete a session
    pub fn delete(&self, id: &SessionId) -> Result<()> {
        self.storage.delete(id)
    }

    /// Clean up sessions older than the given date
    pub fn clean(&self, before: DateTime<Utc>) -> Result<usize> {
        let sessions = self.storage.list()?;
        let mut deleted = 0;

        for info in sessions {
            if info.updated_at < before {
                if self.storage.delete(&info.id).is_ok() {
                    deleted += 1;
                }
            }
        }

        Ok(deleted)
    }

    /// Check if a session exists
    pub fn exists(&self, id: &SessionId) -> bool {
        self.storage.exists(id)
    }

    /// Get session count
    pub fn count(&self) -> Result<usize> {
        Ok(self.storage.list()?.len())
    }

    /// Get access to the underlying storage
    pub fn storage(&self) -> &dyn SessionStorage {
        self.storage.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::persistence::memory::MemoryStorage;

    fn create_manager() -> SessionManager {
        SessionManager::new(MemoryStorage::new())
    }

    #[test]
    fn test_create_session() {
        let manager = create_manager();
        let session = manager
            .create(DiffSource::WorkingTree, DiffData::empty())
            .unwrap();

        assert!(manager.exists(&session.id));
    }

    #[test]
    fn test_create_with_id() {
        let manager = create_manager();
        let id = SessionId::generate();

        let session = manager
            .create_with_id(id.clone(), DiffSource::Staged, DiffData::empty())
            .unwrap();

        assert_eq!(session.id, id);

        // Duplicate ID should fail
        let result = manager.create_with_id(id, DiffSource::Staged, DiffData::empty());
        assert!(result.is_err());
    }

    #[test]
    fn test_create_with_metadata() {
        let manager = create_manager();
        let metadata = SessionMetadata::with_name("Test Review").with_tag("security");

        let session = manager
            .create_with_metadata(DiffSource::WorkingTree, DiffData::empty(), metadata)
            .unwrap();

        assert_eq!(session.metadata.name, Some("Test Review".to_string()));
        assert!(session.metadata.tags.contains(&"security".to_string()));
    }

    #[test]
    fn test_load_session() {
        let manager = create_manager();
        let session = manager
            .create(DiffSource::WorkingTree, DiffData::empty())
            .unwrap();
        let id = session.id.clone();

        let loaded = manager.load(&id).unwrap();
        assert_eq!(loaded.id, session.id);
    }

    #[test]
    fn test_load_nonexistent() {
        let manager = create_manager();
        let id = SessionId::generate();

        let result = manager.load(&id);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_latest() {
        let manager = create_manager();

        // No sessions
        assert!(manager.load_latest().unwrap().is_none());

        // Create sessions
        let _s1 = manager
            .create(DiffSource::WorkingTree, DiffData::empty())
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        let s2 = manager
            .create(DiffSource::Staged, DiffData::empty())
            .unwrap();

        let latest = manager.load_latest().unwrap().unwrap();
        assert_eq!(latest.id, s2.id);
    }

    #[test]
    fn test_save_updates_timestamp() {
        let manager = create_manager();
        let mut session = manager
            .create(DiffSource::WorkingTree, DiffData::empty())
            .unwrap();

        let old_updated = session.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));

        manager.save(&mut session).unwrap();
        assert!(session.updated_at > old_updated);
    }

    #[test]
    fn test_auto_save_throttle() {
        let mut manager = create_manager();
        manager.set_auto_save_interval(Duration::from_millis(100));

        let mut session = manager
            .create(DiffSource::WorkingTree, DiffData::empty())
            .unwrap();

        // First auto-save should work
        assert!(manager.auto_save(&mut session).unwrap());

        // Immediate second auto-save should be throttled
        assert!(!manager.auto_save(&mut session).unwrap());

        // After interval, should work again
        std::thread::sleep(Duration::from_millis(110));
        assert!(manager.auto_save(&mut session).unwrap());
    }

    #[test]
    fn test_list_sessions() {
        let manager = create_manager();

        manager
            .create(DiffSource::WorkingTree, DiffData::empty())
            .unwrap();
        manager
            .create(DiffSource::Staged, DiffData::empty())
            .unwrap();

        let list = manager.list().unwrap();
        assert_eq!(list.len(), 2);

        // Should be sorted by updated_at descending
        assert!(list[0].updated_at >= list[1].updated_at);
    }

    #[test]
    fn test_search_sessions() {
        let manager = create_manager();

        let metadata1 = SessionMetadata::with_name("Security Review").with_tag("security");
        let metadata2 = SessionMetadata::with_name("Code Cleanup").with_tag("refactor");

        manager
            .create_with_metadata(DiffSource::WorkingTree, DiffData::empty(), metadata1)
            .unwrap();
        manager
            .create_with_metadata(DiffSource::WorkingTree, DiffData::empty(), metadata2)
            .unwrap();

        // Search by name
        let filter = SessionFilter::new().with_name("security");
        let results = manager.search(filter).unwrap();
        assert_eq!(results.len(), 1);

        // Search by tag
        let filter = SessionFilter::new().with_tag("refactor");
        let results = manager.search(filter).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_delete_session() {
        let manager = create_manager();
        let session = manager
            .create(DiffSource::WorkingTree, DiffData::empty())
            .unwrap();
        let id = session.id.clone();

        assert!(manager.exists(&id));
        manager.delete(&id).unwrap();
        assert!(!manager.exists(&id));
    }

    #[test]
    fn test_clean_old_sessions() {
        let manager = create_manager();

        // Create a session
        manager
            .create(DiffSource::WorkingTree, DiffData::empty())
            .unwrap();

        // Clean with future date should delete all
        let future = Utc::now() + chrono::Duration::hours(1);
        let deleted = manager.clean(future).unwrap();
        assert_eq!(deleted, 1);
        assert_eq!(manager.count().unwrap(), 0);
    }

    #[test]
    fn test_session_count() {
        let manager = create_manager();

        assert_eq!(manager.count().unwrap(), 0);

        manager
            .create(DiffSource::WorkingTree, DiffData::empty())
            .unwrap();
        manager
            .create(DiffSource::Staged, DiffData::empty())
            .unwrap();

        assert_eq!(manager.count().unwrap(), 2);
    }
}
