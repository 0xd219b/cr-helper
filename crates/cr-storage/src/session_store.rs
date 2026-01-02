//! File system storage for sessions

use cr_core::error::{CrHelperError, Result};
use cr_core::session::{
    Session, SessionFile, SessionInfo, SessionMigrator, SessionStorage, CURRENT_SCHEMA_VERSION,
};
use cr_core::types::SessionId;
use std::fs;
use std::io::{BufReader, BufWriter, Write};
use std::path::PathBuf;
use tracing::{debug, info, warn};

/// File system based session storage
pub struct FileSystemStorage {
    /// Base directory for session storage
    base_dir: PathBuf,
    /// Sessions subdirectory
    sessions_dir: PathBuf,
}

impl FileSystemStorage {
    /// Create a new file system storage
    pub fn new(base_dir: impl Into<PathBuf>) -> Result<Self> {
        let base_dir = base_dir.into();
        let sessions_dir = base_dir.join("sessions");

        let storage = Self {
            base_dir,
            sessions_dir,
        };

        storage.ensure_dirs()?;
        Ok(storage)
    }

    /// Create storage with default directory (~/.cr-helper)
    pub fn default_location() -> Result<Self> {
        let base_dir = directories::ProjectDirs::from("com", "cr-helper", "cr-helper")
            .map(|dirs| dirs.data_dir().to_path_buf())
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".cr-helper")
            });

        Self::new(base_dir)
    }

    /// Ensure required directories exist
    fn ensure_dirs(&self) -> Result<()> {
        if !self.sessions_dir.exists() {
            fs::create_dir_all(&self.sessions_dir).map_err(|e| {
                CrHelperError::Io(std::io::Error::new(
                    e.kind(),
                    format!("Failed to create sessions directory: {}", e),
                ))
            })?;
            debug!("Created sessions directory: {:?}", self.sessions_dir);
        }
        Ok(())
    }

    /// Get the path for a session file
    fn session_path(&self, id: &SessionId) -> PathBuf {
        self.sessions_dir.join(format!("{}.json", id))
    }

    /// Get a temporary path for atomic writes
    fn temp_path(&self, id: &SessionId) -> PathBuf {
        self.sessions_dir.join(format!(".{}.json.tmp", id))
    }

    /// Write session atomically (write to temp, then rename)
    fn atomic_write(&self, id: &SessionId, session: &Session) -> Result<()> {
        let temp_path = self.temp_path(id);
        let final_path = self.session_path(id);

        // Create session file with schema version
        let file = SessionFile::new(session.clone());

        // Write to temp file
        let temp_file = fs::File::create(&temp_path).map_err(|e| {
            CrHelperError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to create temp file: {}", e),
            ))
        })?;
        let mut writer = BufWriter::new(temp_file);
        serde_json::to_writer_pretty(&mut writer, &file)?;
        writer.flush()?;

        // Rename to final path (atomic on most filesystems)
        fs::rename(&temp_path, &final_path).map_err(|e| {
            // Clean up temp file on failure
            let _ = fs::remove_file(&temp_path);
            CrHelperError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to rename temp file: {}", e),
            ))
        })?;

        debug!("Saved session {} to {:?}", id, final_path);
        Ok(())
    }

    /// Read and parse a session file
    fn read_session(&self, path: &PathBuf) -> Result<Session> {
        let file = fs::File::open(path).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                // Extract session ID from filename
                let id = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("unknown");
                CrHelperError::SessionNotFound(id.to_string())
            } else {
                CrHelperError::Io(e)
            }
        })?;

        let reader = BufReader::new(file);
        let session_file: SessionFile = serde_json::from_reader(reader)?;

        // Migrate if needed
        let migrated = if SessionMigrator::needs_migration(&session_file) {
            info!(
                "Migrating session from version {} to {}",
                session_file.schema_version, CURRENT_SCHEMA_VERSION
            );
            SessionMigrator::migrate(session_file)?
        } else {
            session_file
        };

        Ok(migrated.into_session())
    }

    /// Read session info from a file (without loading full diff)
    fn read_session_info(&self, path: &PathBuf) -> Result<SessionInfo> {
        let session = self.read_session(path)?;
        Ok(session.info())
    }

    /// Get base directory
    pub fn base_dir(&self) -> &PathBuf {
        &self.base_dir
    }

    /// Get sessions directory
    pub fn sessions_dir(&self) -> &PathBuf {
        &self.sessions_dir
    }
}

impl SessionStorage for FileSystemStorage {
    fn save(&self, session: &Session) -> Result<()> {
        self.atomic_write(&session.id, session)
    }

    fn load(&self, id: &SessionId) -> Result<Session> {
        let path = self.session_path(id);
        self.read_session(&path)
    }

    fn list(&self) -> Result<Vec<SessionInfo>> {
        let mut sessions = Vec::new();

        let entries = fs::read_dir(&self.sessions_dir).map_err(|e| {
            CrHelperError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to read sessions directory: {}", e),
            ))
        })?;

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    warn!("Failed to read directory entry: {}", e);
                    continue;
                }
            };

            let path = entry.path();

            // Skip non-json files and temp files
            if !path.extension().map(|e| e == "json").unwrap_or(false) {
                continue;
            }
            if path
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with('.'))
                .unwrap_or(false)
            {
                continue;
            }

            match self.read_session_info(&path) {
                Ok(info) => sessions.push(info),
                Err(e) => {
                    warn!("Failed to read session file {:?}: {}", path, e);
                }
            }
        }

        Ok(sessions)
    }

    fn delete(&self, id: &SessionId) -> Result<()> {
        let path = self.session_path(id);

        if !path.exists() {
            return Err(CrHelperError::SessionNotFound(id.to_string()));
        }

        fs::remove_file(&path).map_err(|e| {
            CrHelperError::Io(std::io::Error::new(
                e.kind(),
                format!("Failed to delete session file: {}", e),
            ))
        })?;

        debug!("Deleted session {} from {:?}", id, path);
        Ok(())
    }

    fn exists(&self, id: &SessionId) -> bool {
        self.session_path(id).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cr_core::diff::DiffData;
    use cr_core::session::DiffSource;
    use tempfile::TempDir;

    fn create_test_storage() -> (FileSystemStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let storage = FileSystemStorage::new(temp_dir.path()).unwrap();
        (storage, temp_dir)
    }

    fn create_test_session() -> Session {
        Session::new(DiffSource::WorkingTree, DiffData::empty())
    }

    #[test]
    fn test_storage_creation() {
        let (storage, _temp) = create_test_storage();
        assert!(storage.sessions_dir().exists());
    }

    #[test]
    fn test_session_path() {
        let (storage, _temp) = create_test_storage();
        let id = SessionId::generate();

        let path = storage.session_path(&id);
        assert!(path.to_string_lossy().ends_with(".json"));
        assert!(path.to_string_lossy().contains(&id.to_string()));
    }

    #[test]
    fn test_save_and_load() {
        let (storage, _temp) = create_test_storage();
        let session = create_test_session();
        let id = session.id.clone();

        storage.save(&session).unwrap();
        assert!(storage.exists(&id));

        let loaded = storage.load(&id).unwrap();
        assert_eq!(loaded.id, session.id);
        assert_eq!(loaded.diff_source, session.diff_source);
    }

    #[test]
    fn test_load_nonexistent() {
        let (storage, _temp) = create_test_storage();
        let id = SessionId::generate();

        let result = storage.load(&id);
        assert!(result.is_err());
    }

    #[test]
    fn test_list_sessions() {
        let (storage, _temp) = create_test_storage();

        // Empty initially
        assert!(storage.list().unwrap().is_empty());

        // Add sessions
        let session1 = create_test_session();
        let session2 = create_test_session();

        storage.save(&session1).unwrap();
        storage.save(&session2).unwrap();

        let list = storage.list().unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_delete_session() {
        let (storage, _temp) = create_test_storage();
        let session = create_test_session();
        let id = session.id.clone();

        storage.save(&session).unwrap();
        assert!(storage.exists(&id));

        storage.delete(&id).unwrap();
        assert!(!storage.exists(&id));
    }

    #[test]
    fn test_delete_nonexistent() {
        let (storage, _temp) = create_test_storage();
        let id = SessionId::generate();

        let result = storage.delete(&id);
        assert!(result.is_err());
    }

    #[test]
    fn test_atomic_write() {
        let (storage, temp) = create_test_storage();
        let session = create_test_session();
        let id = session.id.clone();

        storage.save(&session).unwrap();

        // Check that temp file doesn't exist
        let temp_path = storage.temp_path(&id);
        assert!(!temp_path.exists());

        // Check that final file exists
        let final_path = storage.session_path(&id);
        assert!(final_path.exists());

        // Check file content
        let content = fs::read_to_string(&final_path).unwrap();
        assert!(content.contains("schema_version"));
        assert!(content.contains(&id.to_string()));
    }

    #[test]
    fn test_latest_session() {
        let (storage, _temp) = create_test_storage();

        // No sessions
        assert!(storage.latest().unwrap().is_none());

        // Add sessions
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
    fn test_ignores_temp_files() {
        let (storage, _temp) = create_test_storage();

        // Create a temp file manually
        let temp_file = storage.sessions_dir().join(".temp.json.tmp");
        fs::write(&temp_file, "{}").unwrap();

        // Should not appear in list
        assert!(storage.list().unwrap().is_empty());
    }

    #[test]
    fn test_ignores_non_json_files() {
        let (storage, _temp) = create_test_storage();

        // Create a non-json file
        let other_file = storage.sessions_dir().join("readme.txt");
        fs::write(&other_file, "test").unwrap();

        // Should not appear in list
        assert!(storage.list().unwrap().is_empty());
    }
}
