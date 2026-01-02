//! Session data models

use crate::comment::CommentManager;
use crate::diff::DiffData;
use crate::types::{Extensions, SessionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A code review session containing diff data and comments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session identifier
    pub id: SessionId,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last updated
    pub updated_at: DateTime<Utc>,
    /// Source of the diff
    pub diff_source: DiffSource,
    /// Parsed diff data
    pub diff_data: DiffData,
    /// Comments on the diff
    pub comments: CommentManager,
    /// Session metadata
    #[serde(default)]
    pub metadata: SessionMetadata,
    /// Extensions for future compatibility
    #[serde(default, skip_serializing_if = "Extensions::is_empty")]
    pub extensions: Extensions,
}

impl Session {
    /// Create a new session
    pub fn new(diff_source: DiffSource, diff_data: DiffData) -> Self {
        let now = Utc::now();
        Self {
            id: SessionId::generate(),
            created_at: now,
            updated_at: now,
            diff_source,
            diff_data,
            comments: CommentManager::new(),
            metadata: SessionMetadata::default(),
            extensions: Extensions::new(),
        }
    }

    /// Create a new session with a specific ID
    pub fn with_id(id: SessionId, diff_source: DiffSource, diff_data: DiffData) -> Self {
        let now = Utc::now();
        Self {
            id,
            created_at: now,
            updated_at: now,
            diff_source,
            diff_data,
            comments: CommentManager::new(),
            metadata: SessionMetadata::default(),
            extensions: Extensions::new(),
        }
    }

    /// Mark session as updated
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }

    /// Get the number of comments in the session
    pub fn comment_count(&self) -> usize {
        self.comments.count()
    }

    /// Get the number of files in the diff
    pub fn file_count(&self) -> usize {
        self.diff_data.files.len()
    }

    /// Get session info summary
    pub fn info(&self) -> SessionInfo {
        SessionInfo::from(self)
    }
}

/// Source of the diff data
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffSource {
    /// Working tree changes (unstaged)
    WorkingTree,
    /// Staged changes
    Staged,
    /// Specific commit
    Commit {
        /// Commit hash
        commit: String,
    },
    /// Range of commits
    CommitRange {
        /// Start commit
        from: String,
        /// End commit
        to: String,
    },
    /// Branch comparison
    Branch {
        /// Branch name to compare against current
        branch: String,
    },
    /// Pull request
    PullRequest {
        /// PR number
        number: u64,
        /// Base branch
        base: String,
    },
    /// Custom git diff arguments
    Custom {
        /// Raw git diff arguments
        args: Vec<String>,
    },
}

impl DiffSource {
    /// Convert to git diff arguments
    pub fn to_git_args(&self) -> Vec<String> {
        match self {
            DiffSource::WorkingTree => vec![],
            DiffSource::Staged => vec!["--cached".to_string()],
            DiffSource::Commit { commit } => vec![format!("{}^..{}", commit, commit)],
            DiffSource::CommitRange { from, to } => vec![format!("{}..{}", from, to)],
            DiffSource::Branch { branch } => vec![branch.clone()],
            DiffSource::PullRequest { base, .. } => vec![format!("{}..HEAD", base)],
            DiffSource::Custom { args } => args.clone(),
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> String {
        match self {
            DiffSource::WorkingTree => "Working tree changes".to_string(),
            DiffSource::Staged => "Staged changes".to_string(),
            DiffSource::Commit { commit } => format!("Commit {}", &commit[..7.min(commit.len())]),
            DiffSource::CommitRange { from, to } => {
                format!(
                    "{}..{}",
                    &from[..7.min(from.len())],
                    &to[..7.min(to.len())]
                )
            }
            DiffSource::Branch { branch } => format!("Branch: {}", branch),
            DiffSource::PullRequest { number, .. } => format!("PR #{}", number),
            DiffSource::Custom { args } => format!("Custom: {}", args.join(" ")),
        }
    }
}

impl Default for DiffSource {
    fn default() -> Self {
        DiffSource::WorkingTree
    }
}

/// Session metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Optional session name
    pub name: Option<String>,
    /// Optional description
    pub description: Option<String>,
    /// Repository path
    pub repository: Option<PathBuf>,
    /// Tags for categorization
    #[serde(default)]
    pub tags: Vec<String>,
    /// Reviewer name
    pub reviewer: Option<String>,
}

impl SessionMetadata {
    /// Create new metadata with a name
    pub fn with_name(name: impl Into<String>) -> Self {
        Self {
            name: Some(name.into()),
            ..Default::default()
        }
    }

    /// Set the repository path
    pub fn with_repository(mut self, path: PathBuf) -> Self {
        self.repository = Some(path);
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Session summary information (for listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    /// Session ID
    pub id: SessionId,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Last update time
    pub updated_at: DateTime<Utc>,
    /// Session metadata
    pub metadata: SessionMetadata,
    /// Number of comments
    pub comment_count: usize,
    /// Number of files
    pub file_count: usize,
    /// Diff source description
    pub source_description: String,
}

impl From<&Session> for SessionInfo {
    fn from(session: &Session) -> Self {
        Self {
            id: session.id.clone(),
            created_at: session.created_at,
            updated_at: session.updated_at,
            metadata: session.metadata.clone(),
            comment_count: session.comment_count(),
            file_count: session.file_count(),
            source_description: session.diff_source.description(),
        }
    }
}

/// Filter criteria for session search
#[derive(Debug, Clone, Default)]
pub struct SessionFilter {
    /// Filter by name (substring match)
    pub name: Option<String>,
    /// Filter by tags (any match)
    pub tags: Vec<String>,
    /// Created after this time
    pub created_after: Option<DateTime<Utc>>,
    /// Created before this time
    pub created_before: Option<DateTime<Utc>>,
    /// Has comments
    pub has_comments: Option<bool>,
}

impl SessionFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Filter by tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Filter by creation time range
    pub fn created_between(mut self, after: DateTime<Utc>, before: DateTime<Utc>) -> Self {
        self.created_after = Some(after);
        self.created_before = Some(before);
        self
    }

    /// Filter sessions with comments
    pub fn with_comments(mut self) -> Self {
        self.has_comments = Some(true);
        self
    }

    /// Check if a session matches this filter
    pub fn matches(&self, info: &SessionInfo) -> bool {
        // Name filter
        if let Some(ref name) = self.name {
            let name_lower = name.to_lowercase();
            let matches_name = info
                .metadata
                .name
                .as_ref()
                .map(|n| n.to_lowercase().contains(&name_lower))
                .unwrap_or(false);
            if !matches_name {
                return false;
            }
        }

        // Tags filter
        if !self.tags.is_empty() {
            let has_tag = self.tags.iter().any(|t| info.metadata.tags.contains(t));
            if !has_tag {
                return false;
            }
        }

        // Created after filter
        if let Some(after) = self.created_after {
            if info.created_at < after {
                return false;
            }
        }

        // Created before filter
        if let Some(before) = self.created_before {
            if info.created_at > before {
                return false;
            }
        }

        // Has comments filter
        if let Some(has_comments) = self.has_comments {
            if has_comments && info.comment_count == 0 {
                return false;
            }
            if !has_comments && info.comment_count > 0 {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_session() -> Session {
        Session::new(DiffSource::WorkingTree, DiffData::empty())
    }

    #[test]
    fn test_session_creation() {
        let session = create_test_session();
        assert_eq!(session.comment_count(), 0);
        assert_eq!(session.file_count(), 0);
        assert!(session.created_at <= session.updated_at);
    }

    #[test]
    fn test_session_with_id() {
        let id = SessionId::generate();
        let session = Session::with_id(id.clone(), DiffSource::Staged, DiffData::empty());
        assert_eq!(session.id, id);
    }

    #[test]
    fn test_session_touch() {
        let mut session = create_test_session();
        let old_updated = session.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        session.touch();
        assert!(session.updated_at > old_updated);
    }

    #[test]
    fn test_diff_source_git_args() {
        assert_eq!(DiffSource::WorkingTree.to_git_args(), Vec::<String>::new());
        assert_eq!(DiffSource::Staged.to_git_args(), vec!["--cached"]);
        assert_eq!(
            DiffSource::Commit {
                commit: "abc123".to_string()
            }
            .to_git_args(),
            vec!["abc123^..abc123"]
        );
        assert_eq!(
            DiffSource::CommitRange {
                from: "abc".to_string(),
                to: "def".to_string()
            }
            .to_git_args(),
            vec!["abc..def"]
        );
        assert_eq!(
            DiffSource::Branch {
                branch: "main".to_string()
            }
            .to_git_args(),
            vec!["main"]
        );
    }

    #[test]
    fn test_diff_source_description() {
        assert_eq!(DiffSource::WorkingTree.description(), "Working tree changes");
        assert_eq!(DiffSource::Staged.description(), "Staged changes");
        assert_eq!(
            DiffSource::Commit {
                commit: "abc1234567890".to_string()
            }
            .description(),
            "Commit abc1234"
        );
    }

    #[test]
    fn test_session_metadata() {
        let metadata = SessionMetadata::with_name("Test Review")
            .with_repository(PathBuf::from("/tmp/repo"))
            .with_tag("security");

        assert_eq!(metadata.name, Some("Test Review".to_string()));
        assert_eq!(metadata.repository, Some(PathBuf::from("/tmp/repo")));
        assert!(metadata.tags.contains(&"security".to_string()));
    }

    #[test]
    fn test_session_info_from_session() {
        let mut session = create_test_session();
        session.metadata.name = Some("Test".to_string());

        let info = session.info();
        assert_eq!(info.id, session.id);
        assert_eq!(info.metadata.name, Some("Test".to_string()));
        assert_eq!(info.comment_count, 0);
    }

    #[test]
    fn test_session_filter_name() {
        let filter = SessionFilter::new().with_name("test");

        let mut info = SessionInfo {
            id: SessionId::generate(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: SessionMetadata::default(),
            comment_count: 0,
            file_count: 0,
            source_description: "test".to_string(),
        };

        // No name - doesn't match
        assert!(!filter.matches(&info));

        // Name contains "test" - matches
        info.metadata.name = Some("My test session".to_string());
        assert!(filter.matches(&info));

        // Name doesn't contain "test" - doesn't match
        info.metadata.name = Some("Other session".to_string());
        assert!(!filter.matches(&info));
    }

    #[test]
    fn test_session_filter_tags() {
        let filter = SessionFilter::new().with_tag("security");

        let mut info = SessionInfo {
            id: SessionId::generate(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: SessionMetadata::default(),
            comment_count: 0,
            file_count: 0,
            source_description: "test".to_string(),
        };

        // No tags - doesn't match
        assert!(!filter.matches(&info));

        // Has matching tag - matches
        info.metadata.tags = vec!["security".to_string()];
        assert!(filter.matches(&info));
    }

    #[test]
    fn test_session_filter_comments() {
        let filter = SessionFilter::new().with_comments();

        let mut info = SessionInfo {
            id: SessionId::generate(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: SessionMetadata::default(),
            comment_count: 0,
            file_count: 0,
            source_description: "test".to_string(),
        };

        // No comments - doesn't match
        assert!(!filter.matches(&info));

        // Has comments - matches
        info.comment_count = 5;
        assert!(filter.matches(&info));
    }

    #[test]
    fn test_session_serialization() {
        let session = create_test_session();
        let json = serde_json::to_string(&session).unwrap();
        let session2: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(session.id, session2.id);
        assert_eq!(session.diff_source, session2.diff_source);
    }
}
