//! Core type definitions for cr-helper

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::Path;
use uuid::Uuid;

/// Unique identifier for a file in a diff
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FileId(pub String);

impl FileId {
    /// Create a FileId from a file path
    pub fn from_path(path: &Path) -> Self {
        let hash = blake3::hash(path.to_string_lossy().as_bytes());
        FileId(format!("f_{}", &hash.to_hex()[..12]))
    }

    /// Create a FileId from a string
    pub fn from_string(s: impl Into<String>) -> Self {
        FileId(s.into())
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a hunk in a diff
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HunkId(pub String);

impl HunkId {
    /// Generate a new HunkId
    pub fn new(file_id: &FileId, hunk_index: usize) -> Self {
        HunkId(format!("{}:h{}", file_id.0, hunk_index))
    }
}

impl fmt::Display for HunkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a line in a diff
/// Based on file path + content hash for stability across diff regeneration
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LineId(pub String);

impl LineId {
    /// Create a LineId from file path and line content
    pub fn from_content(file_path: &Path, content: &str, line_num: usize) -> Self {
        let hash = blake3::hash(format!("{}:{}:{}", file_path.display(), line_num, content).as_bytes());
        LineId(format!("l_{}", &hash.to_hex()[..16]))
    }

    /// Create a LineId from a string
    pub fn from_string(s: impl Into<String>) -> Self {
        LineId(s.into())
    }
}

impl fmt::Display for LineId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a comment
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommentId(pub Uuid);

impl CommentId {
    /// Generate a new CommentId
    pub fn new() -> Self {
        CommentId(Uuid::new_v4())
    }

    /// Create from UUID string
    pub fn from_string(s: &str) -> Result<Self, uuid::Error> {
        Ok(CommentId(Uuid::parse_str(s)?))
    }
}

impl Default for CommentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for CommentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Unique identifier for a session
/// Format: YYYYMMDDHHMMSS-<short_uuid>
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionId(pub String);

impl SessionId {
    /// Generate a new SessionId
    pub fn generate() -> Self {
        let now = chrono::Utc::now();
        let uuid = Uuid::new_v4();
        let short_uuid = &uuid.to_string()[..8];
        SessionId(format!("{}-{}", now.format("%Y%m%d%H%M%S"), short_uuid))
    }

    /// Create from a string with validation
    pub fn from_string(s: impl Into<String>) -> crate::Result<Self> {
        let s = s.into();
        if Self::validate(&s) {
            Ok(SessionId(s))
        } else {
            Err(crate::CrHelperError::Validation(format!(
                "Invalid session ID format: {}",
                s
            )))
        }
    }

    /// Validate session ID format
    fn validate(s: &str) -> bool {
        // Format: YYYYMMDDHHMMSS-xxxxxxxx
        if s.len() < 23 {
            return false;
        }
        let parts: Vec<&str> = s.splitn(2, '-').collect();
        if parts.len() != 2 {
            return false;
        }
        // Check timestamp part (14 digits)
        parts[0].len() == 14 && parts[0].chars().all(|c| c.is_ascii_digit())
    }

    /// Get the string value
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Protocol version for compatibility
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u32,
    pub minor: u32,
}

impl ProtocolVersion {
    pub const V1_0: Self = Self { major: 1, minor: 0 };

    /// Check if this version is compatible with another version
    pub fn is_compatible(&self, other: &Self) -> bool {
        self.major == other.major
    }
}

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl Default for ProtocolVersion {
    fn default() -> Self {
        Self::V1_0
    }
}

/// Extensions field for future compatibility
/// Stores arbitrary JSON values for forward compatibility
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Extensions {
    #[serde(flatten)]
    pub data: HashMap<String, serde_json::Value>,
}

impl Extensions {
    /// Create empty extensions
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if extensions is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get a value by key
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.data.get(key)
    }

    /// Get a typed value by key
    pub fn get_as<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.data
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Set a value by key
    pub fn set<T: Serialize>(&mut self, key: impl Into<String>, value: T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.data.insert(key.into(), v);
        }
    }

    /// Remove a value by key
    pub fn remove(&mut self, key: &str) -> Option<serde_json::Value> {
        self.data.remove(key)
    }

    // v1.1 convenience methods

    /// Get suggested fix (v1.1 extension)
    pub fn suggested_fix(&self) -> Option<&str> {
        self.data.get("suggested_fix")?.as_str()
    }

    /// Set suggested fix (v1.1 extension)
    pub fn set_suggested_fix(&mut self, fix: impl Into<String>) {
        self.data
            .insert("suggested_fix".to_string(), serde_json::json!(fix.into()));
    }

    /// Get related reviews (v1.1 extension)
    pub fn related_reviews(&self) -> Option<Vec<String>> {
        self.get_as("related_reviews")
    }

    /// Set related reviews (v1.1 extension)
    pub fn set_related_reviews(&mut self, reviews: Vec<String>) {
        self.set("related_reviews", reviews);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_file_id_from_path() {
        let path = PathBuf::from("src/main.rs");
        let id = FileId::from_path(&path);
        assert!(id.0.starts_with("f_"));
        assert_eq!(id.0.len(), 14); // "f_" + 12 chars
    }

    #[test]
    fn test_line_id_stability() {
        let path = PathBuf::from("src/main.rs");
        let content = "fn main() {}";
        let id1 = LineId::from_content(&path, content, 1);
        let id2 = LineId::from_content(&path, content, 1);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_comment_id_uniqueness() {
        let id1 = CommentId::new();
        let id2 = CommentId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_session_id_generation() {
        let id = SessionId::generate();
        assert!(id.0.len() >= 23);
        assert!(id.0.contains('-'));
    }

    #[test]
    fn test_session_id_validation() {
        assert!(SessionId::from_string("20241231120000-abcd1234").is_ok());
        assert!(SessionId::from_string("invalid").is_err());
        assert!(SessionId::from_string("2024-abcd1234").is_err());
    }

    #[test]
    fn test_protocol_version_compatibility() {
        let v1_0 = ProtocolVersion::V1_0;
        let v1_1 = ProtocolVersion { major: 1, minor: 1 };
        let v2_0 = ProtocolVersion { major: 2, minor: 0 };

        assert!(v1_0.is_compatible(&v1_1));
        assert!(!v1_0.is_compatible(&v2_0));
    }

    #[test]
    fn test_extensions() {
        let mut ext = Extensions::new();
        assert!(ext.is_empty());

        ext.set("key1", "value1");
        assert!(!ext.is_empty());
        assert_eq!(ext.get_as::<String>("key1"), Some("value1".to_string()));

        ext.set_suggested_fix("use Result<T>");
        assert_eq!(ext.suggested_fix(), Some("use Result<T>"));

        ext.set_related_reviews(vec!["r1".to_string(), "r2".to_string()]);
        assert_eq!(
            ext.related_reviews(),
            Some(vec!["r1".to_string(), "r2".to_string()])
        );
    }

    #[test]
    fn test_extensions_serialization() {
        let mut ext = Extensions::new();
        ext.set("test", "value");

        let json = serde_json::to_string(&ext).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("value"));

        let ext2: Extensions = serde_json::from_str(&json).unwrap();
        assert_eq!(ext2.get_as::<String>("test"), Some("value".to_string()));
    }
}
