//! Diff data models

use crate::types::{Extensions, FileId, HunkId, LineId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Complete diff data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffData {
    /// List of changed files
    pub files: Vec<FileDiff>,
    /// Diff metadata
    pub metadata: DiffMetadata,
    /// Diff statistics
    pub stats: DiffStats,
}

impl DiffData {
    /// Create empty diff data
    pub fn empty() -> Self {
        Self {
            files: Vec::new(),
            metadata: DiffMetadata::default(),
            stats: DiffStats::default(),
        }
    }

    /// Get total line count across all files
    pub fn total_lines(&self) -> usize {
        self.files.iter().map(|f| f.total_lines()).sum()
    }

    /// Get file by ID
    pub fn get_file(&self, id: &FileId) -> Option<&FileDiff> {
        self.files.iter().find(|f| &f.id == id)
    }

    /// Get file by path
    pub fn get_file_by_path(&self, path: &PathBuf) -> Option<&FileDiff> {
        self.files.iter().find(|f| {
            f.new_path.as_ref() == Some(path) || f.old_path.as_ref() == Some(path)
        })
    }
}

/// Single file diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    /// Unique file identifier
    pub id: FileId,
    /// Original file path (None if new file)
    pub old_path: Option<PathBuf>,
    /// New file path (None if deleted)
    pub new_path: Option<PathBuf>,
    /// File change mode
    pub mode: FileMode,
    /// List of hunks
    pub hunks: Vec<Hunk>,
    /// Whether this file's content needs lazy loading
    #[serde(default)]
    pub lazy: bool,
}

impl FileDiff {
    /// Get the display path (prefer new_path)
    pub fn display_path(&self) -> &PathBuf {
        self.new_path.as_ref().or(self.old_path.as_ref()).unwrap()
    }

    /// Get total line count
    pub fn total_lines(&self) -> usize {
        self.hunks.iter().map(|h| h.lines.len()).sum()
    }

    /// Check if this is a binary file
    pub fn is_binary(&self) -> bool {
        matches!(self.mode, FileMode::Binary)
    }

    /// Check if this file needs content to be loaded
    pub fn needs_loading(&self) -> bool {
        self.lazy && self.hunks.is_empty()
    }

    /// Create a lazy file entry (content loaded on demand)
    pub fn lazy_new(path: PathBuf) -> Self {
        Self {
            id: FileId::from_path(&path),
            old_path: None,
            new_path: Some(path),
            mode: FileMode::Added,
            hunks: Vec::new(),
            lazy: true,
        }
    }
}

/// File change mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileMode {
    /// New file added
    Added,
    /// File deleted
    Deleted,
    /// File modified
    Modified,
    /// File renamed
    Renamed,
    /// File copied
    Copied,
    /// Binary file (cannot be diffed)
    Binary,
}

impl FileMode {
    /// Get display character
    pub fn as_char(&self) -> char {
        match self {
            FileMode::Added => '+',
            FileMode::Deleted => '-',
            FileMode::Modified => '~',
            FileMode::Renamed => '→',
            FileMode::Copied => '⊕',
            FileMode::Binary => 'B',
        }
    }
}

/// A hunk in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hunk {
    /// Unique hunk identifier
    pub id: HunkId,
    /// Hunk header line
    pub header: String,
    /// Old file line range
    pub old_range: Range,
    /// New file line range
    pub new_range: Range,
    /// Lines in this hunk
    pub lines: Vec<Line>,
}

/// Line range in a hunk
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Range {
    /// Starting line number
    pub start: usize,
    /// Number of lines
    pub count: usize,
}

impl Range {
    /// Create a new range
    pub fn new(start: usize, count: usize) -> Self {
        Self { start, count }
    }

    /// Get end line number (exclusive)
    pub fn end(&self) -> usize {
        self.start + self.count
    }
}

/// A single line in a diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Line {
    /// Unique line identifier
    pub id: LineId,
    /// Line type
    pub line_type: LineType,
    /// Line content (without prefix)
    pub content: String,
    /// Line number in old file
    pub old_line_num: Option<usize>,
    /// Line number in new file
    pub new_line_num: Option<usize>,
}

impl Line {
    /// Get the display line number
    pub fn display_line_num(&self) -> Option<usize> {
        self.new_line_num.or(self.old_line_num)
    }
}

/// Type of line change
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LineType {
    /// Line was added
    Added,
    /// Line was deleted
    Deleted,
    /// Context line (unchanged)
    Context,
    /// No newline at end of file marker
    NoNewline,
}

impl LineType {
    /// Get the diff prefix character
    pub fn prefix(&self) -> char {
        match self {
            LineType::Added => '+',
            LineType::Deleted => '-',
            LineType::Context => ' ',
            LineType::NoNewline => '\\',
        }
    }
}

/// Diff metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffMetadata {
    /// Source of the diff
    pub source: DiffSource,
    /// When the diff was generated
    pub timestamp: DateTime<Utc>,
    /// Repository path
    pub repository: Option<PathBuf>,
    /// Extensions for future compatibility
    #[serde(default, skip_serializing_if = "Extensions::is_empty")]
    pub extensions: Extensions,
}

impl Default for DiffMetadata {
    fn default() -> Self {
        Self {
            source: DiffSource::WorkingTree,
            timestamp: Utc::now(),
            repository: None,
            extensions: Extensions::new(),
        }
    }
}

/// Source of the diff
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DiffSource {
    /// Working tree changes
    WorkingTree,
    /// Staged changes
    Staged,
    /// Specific commit
    Commit { commit: String },
    /// Range of commits
    CommitRange { from: String, to: String },
    /// Branch comparison
    Branch { branch: String },
    /// Custom git diff arguments
    Custom { args: Vec<String> },
}

impl DiffSource {
    /// Convert to git diff arguments
    pub fn to_git_args(&self) -> Vec<String> {
        match self {
            DiffSource::WorkingTree => vec![],
            DiffSource::Staged => vec!["--staged".to_string()],
            DiffSource::Commit { commit } => vec![format!("{}^..{}", commit, commit)],
            DiffSource::CommitRange { from, to } => vec![format!("{}..{}", from, to)],
            DiffSource::Branch { branch } => vec![branch.clone()],
            DiffSource::Custom { args } => args.clone(),
        }
    }
}

/// Diff statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiffStats {
    /// Number of files changed
    pub files_changed: usize,
    /// Number of insertions
    pub insertions: usize,
    /// Number of deletions
    pub deletions: usize,
}

impl DiffStats {
    /// Calculate stats from diff data
    pub fn from_diff(diff: &DiffData) -> Self {
        let mut insertions = 0;
        let mut deletions = 0;

        for file in &diff.files {
            for hunk in &file.hunks {
                for line in &hunk.lines {
                    match line.line_type {
                        LineType::Added => insertions += 1,
                        LineType::Deleted => deletions += 1,
                        _ => {}
                    }
                }
            }
        }

        Self {
            files_changed: diff.files.len(),
            insertions,
            deletions,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_data_empty() {
        let diff = DiffData::empty();
        assert!(diff.files.is_empty());
        assert_eq!(diff.total_lines(), 0);
    }

    #[test]
    fn test_file_mode_char() {
        assert_eq!(FileMode::Added.as_char(), '+');
        assert_eq!(FileMode::Deleted.as_char(), '-');
        assert_eq!(FileMode::Modified.as_char(), '~');
    }

    #[test]
    fn test_line_type_prefix() {
        assert_eq!(LineType::Added.prefix(), '+');
        assert_eq!(LineType::Deleted.prefix(), '-');
        assert_eq!(LineType::Context.prefix(), ' ');
    }

    #[test]
    fn test_diff_source_to_git_args() {
        assert!(DiffSource::WorkingTree.to_git_args().is_empty());
        assert_eq!(
            DiffSource::Staged.to_git_args(),
            vec!["--staged".to_string()]
        );
        assert_eq!(
            DiffSource::Commit {
                commit: "abc123".to_string()
            }
            .to_git_args(),
            vec!["abc123^..abc123".to_string()]
        );
    }

    #[test]
    fn test_range() {
        let range = Range::new(10, 5);
        assert_eq!(range.start, 10);
        assert_eq!(range.count, 5);
        assert_eq!(range.end(), 15);
    }
}
