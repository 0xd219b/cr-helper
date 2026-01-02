//! Session file format and schema migration

use super::model::Session;
use crate::error::{CrHelperError, Result};
use crate::types::ProtocolVersion;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Current schema version
pub const CURRENT_SCHEMA_VERSION: &str = "1.0";

/// Session file format with schema version
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFile {
    /// Schema version for migration
    pub schema_version: String,
    /// The session data
    pub session: Session,
    /// Extra fields for forward compatibility
    #[serde(flatten, default)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl SessionFile {
    /// Create a new session file with current schema version
    pub fn new(session: Session) -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION.to_string(),
            session,
            extra: HashMap::new(),
        }
    }

    /// Get the session, consuming the file
    pub fn into_session(self) -> Session {
        self.session
    }

    /// Parse schema version
    pub fn parse_version(&self) -> Option<ProtocolVersion> {
        let parts: Vec<&str> = self.schema_version.split('.').collect();
        if parts.len() != 2 {
            return None;
        }
        let major = parts[0].parse().ok()?;
        let minor = parts[1].parse().ok()?;
        Some(ProtocolVersion { major, minor })
    }
}

/// Session schema migrator
pub struct SessionMigrator;

impl SessionMigrator {
    /// Migrate a session file to the current schema version
    pub fn migrate(file: SessionFile) -> Result<SessionFile> {
        let version = file
            .parse_version()
            .ok_or_else(|| CrHelperError::Validation("Invalid schema version format".to_string()))?;

        // Check if migration is needed
        let current = ProtocolVersion::V1_0;
        if !version.is_compatible(&current) {
            return Err(CrHelperError::Validation(format!(
                "Incompatible schema version: {} (expected {}.x)",
                file.schema_version, current.major
            )));
        }

        // For now, no migrations needed within v1.x
        // Future migrations would be handled here:
        // if version.minor < 1 {
        //     file = migrate_1_0_to_1_1(file)?;
        // }

        Ok(file)
    }

    /// Check if a file needs migration
    pub fn needs_migration(file: &SessionFile) -> bool {
        file.schema_version != CURRENT_SCHEMA_VERSION
    }

    /// Get the current schema version
    pub fn current_version() -> &'static str {
        CURRENT_SCHEMA_VERSION
    }
}

// Future migration functions would be added here:
// fn migrate_1_0_to_1_1(mut file: SessionFile) -> Result<SessionFile> {
//     // Apply migration logic
//     file.schema_version = "1.1".to_string();
//     Ok(file)
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::DiffData;
    use crate::session::model::DiffSource;

    fn create_test_session() -> Session {
        Session::new(DiffSource::WorkingTree, DiffData::empty())
    }

    #[test]
    fn test_session_file_creation() {
        let session = create_test_session();
        let file = SessionFile::new(session.clone());

        assert_eq!(file.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(file.session.id, session.id);
    }

    #[test]
    fn test_session_file_parse_version() {
        let session = create_test_session();
        let file = SessionFile::new(session);

        let version = file.parse_version().unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 0);
    }

    #[test]
    fn test_session_file_into_session() {
        let session = create_test_session();
        let id = session.id.clone();
        let file = SessionFile::new(session);

        let restored = file.into_session();
        assert_eq!(restored.id, id);
    }

    #[test]
    fn test_migrator_no_migration_needed() {
        let session = create_test_session();
        let file = SessionFile::new(session);

        assert!(!SessionMigrator::needs_migration(&file));
    }

    #[test]
    fn test_migrator_migrate_current_version() {
        let session = create_test_session();
        let file = SessionFile::new(session);

        let migrated = SessionMigrator::migrate(file).unwrap();
        assert_eq!(migrated.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_migrator_incompatible_version() {
        let session = create_test_session();
        let mut file = SessionFile::new(session);
        file.schema_version = "2.0".to_string();

        let result = SessionMigrator::migrate(file);
        assert!(result.is_err());
    }

    #[test]
    fn test_session_file_serialization() {
        let session = create_test_session();
        let file = SessionFile::new(session);

        let json = serde_json::to_string(&file).unwrap();
        assert!(json.contains("schema_version"));
        assert!(json.contains(CURRENT_SCHEMA_VERSION));

        let file2: SessionFile = serde_json::from_str(&json).unwrap();
        assert_eq!(file2.schema_version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn test_session_file_forward_compatibility() {
        // Simulate a file with extra fields from a future version
        let json = r#"{
            "schema_version": "1.0",
            "session": {
                "id": "20241231120000-abcd1234",
                "created_at": "2024-12-31T12:00:00Z",
                "updated_at": "2024-12-31T12:00:00Z",
                "diff_source": "WorkingTree",
                "diff_data": {
                    "files": [],
                    "metadata": {
                        "source": "WorkingTree",
                        "timestamp": "2024-12-31T12:00:00Z"
                    },
                    "stats": { "files_changed": 0, "insertions": 0, "deletions": 0 }
                },
                "comments": { "comments": {} },
                "metadata": {}
            },
            "future_field": "some value"
        }"#;

        let file: SessionFile = serde_json::from_str(json).unwrap();
        assert_eq!(file.schema_version, "1.0");
        assert!(file.extra.contains_key("future_field"));
    }
}
