//! Session storage trait and abstractions

use super::model::{Session, SessionInfo};
use crate::error::Result;
use crate::types::SessionId;

/// Trait for session storage implementations
pub trait SessionStorage: Send + Sync {
    /// Save a session
    fn save(&self, session: &Session) -> Result<()>;

    /// Load a session by ID
    fn load(&self, id: &SessionId) -> Result<Session>;

    /// List all sessions (as info)
    fn list(&self) -> Result<Vec<SessionInfo>>;

    /// Delete a session
    fn delete(&self, id: &SessionId) -> Result<()>;

    /// Check if a session exists
    fn exists(&self, id: &SessionId) -> bool;

    /// Get the latest session (by updated_at)
    fn latest(&self) -> Result<Option<Session>> {
        let sessions = self.list()?;
        if sessions.is_empty() {
            return Ok(None);
        }

        let latest_info = sessions
            .into_iter()
            .max_by_key(|s| s.updated_at)
            .expect("Non-empty list should have max");

        self.load(&latest_info.id).map(Some)
    }
}

/// In-memory storage for testing
#[cfg(test)]
pub mod memory {
    use super::*;
    use std::collections::HashMap;
    use std::sync::RwLock;

    /// In-memory session storage for testing
    pub struct MemoryStorage {
        sessions: RwLock<HashMap<SessionId, Session>>,
    }

    impl MemoryStorage {
        /// Create a new in-memory storage
        pub fn new() -> Self {
            Self {
                sessions: RwLock::new(HashMap::new()),
            }
        }
    }

    impl Default for MemoryStorage {
        fn default() -> Self {
            Self::new()
        }
    }

    impl SessionStorage for MemoryStorage {
        fn save(&self, session: &Session) -> Result<()> {
            let mut sessions = self.sessions.write().unwrap();
            sessions.insert(session.id.clone(), session.clone());
            Ok(())
        }

        fn load(&self, id: &SessionId) -> Result<Session> {
            let sessions = self.sessions.read().unwrap();
            sessions
                .get(id)
                .cloned()
                .ok_or_else(|| crate::CrHelperError::SessionNotFound(id.to_string()))
        }

        fn list(&self) -> Result<Vec<SessionInfo>> {
            let sessions = self.sessions.read().unwrap();
            Ok(sessions.values().map(|s| s.info()).collect())
        }

        fn delete(&self, id: &SessionId) -> Result<()> {
            let mut sessions = self.sessions.write().unwrap();
            sessions
                .remove(id)
                .ok_or_else(|| crate::CrHelperError::SessionNotFound(id.to_string()))?;
            Ok(())
        }

        fn exists(&self, id: &SessionId) -> bool {
            let sessions = self.sessions.read().unwrap();
            sessions.contains_key(id)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::diff::DiffData;
        use crate::session::model::DiffSource;

        fn create_test_session() -> Session {
            Session::new(DiffSource::WorkingTree, DiffData::empty())
        }

        #[test]
        fn test_memory_storage_save_load() {
            let storage = MemoryStorage::new();
            let session = create_test_session();
            let id = session.id.clone();

            storage.save(&session).unwrap();
            let loaded = storage.load(&id).unwrap();

            assert_eq!(loaded.id, session.id);
        }

        #[test]
        fn test_memory_storage_list() {
            let storage = MemoryStorage::new();

            let session1 = create_test_session();
            let session2 = create_test_session();

            storage.save(&session1).unwrap();
            storage.save(&session2).unwrap();

            let list = storage.list().unwrap();
            assert_eq!(list.len(), 2);
        }

        #[test]
        fn test_memory_storage_delete() {
            let storage = MemoryStorage::new();
            let session = create_test_session();
            let id = session.id.clone();

            storage.save(&session).unwrap();
            assert!(storage.exists(&id));

            storage.delete(&id).unwrap();
            assert!(!storage.exists(&id));
        }

        #[test]
        fn test_memory_storage_latest() {
            let storage = MemoryStorage::new();

            let session1 = create_test_session();
            std::thread::sleep(std::time::Duration::from_millis(10));
            let session2 = create_test_session();
            let expected_id = session2.id.clone();

            storage.save(&session1).unwrap();
            storage.save(&session2).unwrap();

            let latest = storage.latest().unwrap().unwrap();
            assert_eq!(latest.id, expected_id);
        }

        #[test]
        fn test_memory_storage_load_nonexistent() {
            let storage = MemoryStorage::new();
            let id = SessionId::generate();

            let result = storage.load(&id);
            assert!(result.is_err());
        }
    }
}
