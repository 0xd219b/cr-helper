//! Git diff parser

use crate::diff::model::*;
use crate::error::{CrHelperError, Result};
use crate::types::{FileId, HunkId, LineId};
use std::path::PathBuf;
use std::process::Command;

/// Configuration for the diff parser
#[derive(Debug, Clone)]
pub struct ParserConfig {
    /// Include binary files (as placeholders)
    pub include_binary: bool,
    /// Maximum file size to parse (in bytes)
    pub max_file_size: Option<usize>,
}

impl Default for ParserConfig {
    fn default() -> Self {
        Self {
            include_binary: true,
            max_file_size: Some(10 * 1024 * 1024), // 10MB
        }
    }
}

/// Git diff parser
pub struct DiffParser {
    config: ParserConfig,
}

impl DiffParser {
    /// Create a new parser with default config
    pub fn new() -> Self {
        Self {
            config: ParserConfig::default(),
        }
    }

    /// Create a new parser with custom config
    pub fn with_config(config: ParserConfig) -> Self {
        Self { config }
    }

    /// Parse a diff string
    pub fn parse(&self, input: &str) -> Result<DiffData> {
        let mut files = Vec::new();
        let mut current_file: Option<FileDiffBuilder> = None;
        let mut current_hunk: Option<HunkBuilder> = None;

        for line in input.lines() {
            // New file header
            if line.starts_with("diff --git ") {
                // Save current hunk and file
                if let Some(hunk) = current_hunk.take() {
                    if let Some(ref mut file) = current_file {
                        file.hunks.push(hunk.build());
                    }
                }
                if let Some(file) = current_file.take() {
                    files.push(file.build());
                }

                // Parse file paths
                let (old_path, new_path) = self.parse_diff_header(line)?;
                current_file = Some(FileDiffBuilder::new(old_path, new_path));
            }
            // Binary file
            else if line.starts_with("Binary files ") {
                if let Some(ref mut file) = current_file {
                    file.mode = FileMode::Binary;
                }
            }
            // File mode indicators
            else if line.starts_with("new file mode") {
                if let Some(ref mut file) = current_file {
                    file.mode = FileMode::Added;
                }
            } else if line.starts_with("deleted file mode") {
                if let Some(ref mut file) = current_file {
                    file.mode = FileMode::Deleted;
                }
            } else if line.starts_with("rename from ") || line.starts_with("rename to ") {
                if let Some(ref mut file) = current_file {
                    file.mode = FileMode::Renamed;
                }
            } else if line.starts_with("copy from ") || line.starts_with("copy to ") {
                if let Some(ref mut file) = current_file {
                    file.mode = FileMode::Copied;
                }
            }
            // Hunk header
            else if line.starts_with("@@ ") {
                // Save current hunk
                if let Some(hunk) = current_hunk.take() {
                    if let Some(ref mut file) = current_file {
                        file.hunks.push(hunk.build());
                    }
                }

                let (old_range, new_range) = self.parse_hunk_header(line)?;
                let hunk_id = if let Some(ref file) = current_file {
                    HunkId::new(&file.id, file.hunks.len())
                } else {
                    HunkId::new(&FileId::from_string("unknown"), 0)
                };

                current_hunk = Some(HunkBuilder::new(hunk_id, line.to_string(), old_range, new_range));
            }
            // Diff lines
            else if let Some(ref mut hunk) = current_hunk {
                if let Some(line_data) = self.parse_line(line, &current_file, hunk)? {
                    hunk.lines.push(line_data);
                }
            }
        }

        // Save final hunk and file
        if let Some(hunk) = current_hunk.take() {
            if let Some(ref mut file) = current_file {
                file.hunks.push(hunk.build());
            }
        }
        if let Some(file) = current_file.take() {
            files.push(file.build());
        }

        let mut diff_data = DiffData {
            files,
            metadata: DiffMetadata::default(),
            stats: DiffStats::default(),
        };

        // Calculate stats
        diff_data.stats = DiffStats::from_diff(&diff_data);

        Ok(diff_data)
    }

    /// Parse diff from git command
    pub fn parse_from_git(&self, source: &DiffSource) -> Result<DiffData> {
        self.parse_from_git_with_options(source, false)
    }

    /// Parse diff from git command with options
    pub fn parse_from_git_with_options(
        &self,
        source: &DiffSource,
        include_untracked: bool,
    ) -> Result<DiffData> {
        let args = source.to_git_args();
        let mut cmd = Command::new("git");
        cmd.arg("diff").args(&args);

        let output = cmd.output().map_err(|e| {
            CrHelperError::Command {
                command: "git diff".to_string(),
                message: e.to_string(),
            }
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(CrHelperError::Git(stderr.to_string()));
        }

        let diff_str = String::from_utf8_lossy(&output.stdout);
        let mut diff_data = self.parse(&diff_str)?;
        diff_data.metadata.source = source.clone();

        // Include untracked files if requested (only for WorkingTree or Staged)
        if include_untracked
            && matches!(source, DiffSource::WorkingTree | DiffSource::Staged)
        {
            // Get untracked file list (lazy - don't read content yet)
            let untracked_files = self.get_untracked_file_list()?;
            for path in untracked_files {
                diff_data.files.push(FileDiff::lazy_new(PathBuf::from(path)));
            }
            // Update file count in stats
            diff_data.stats.files_changed = diff_data.files.len();
        }

        Ok(diff_data)
    }

    /// Get list of untracked files (without loading content)
    /// Uses .gitignore for exclusions via --exclude-standard
    fn get_untracked_file_list(&self) -> Result<Vec<String>> {
        let output = Command::new("git")
            .args([
                "ls-files",
                "--others",
                "--exclude-standard", // Reads .gitignore, .git/info/exclude, etc.
            ])
            .output()
            .map_err(|e| CrHelperError::Command {
                command: "git ls-files".to_string(),
                message: e.to_string(),
            })?;

        if !output.status.success() {
            return Ok(Vec::new());
        }

        let files_str = String::from_utf8_lossy(&output.stdout);
        let files: Vec<String> = files_str
            .lines()
            .filter(|l| !l.is_empty())
            .map(|s| s.to_string())
            .collect();

        // Warn if too many untracked files
        const WARN_THRESHOLD: usize = 100;
        if files.len() > WARN_THRESHOLD {
            eprintln!(
                "\x1b[33mWarning:\x1b[0m Found {} untracked files. This may take a while.",
                files.len()
            );
            eprintln!(
                "         Consider adding unwanted directories to .gitignore (e.g., target/, node_modules/)"
            );
        }

        Ok(files)
    }

    /// Load content for a lazy file (call this when user navigates to the file)
    pub fn load_lazy_file(&self, file: &mut FileDiff) -> Result<()> {
        use std::fs;

        if !file.needs_loading() {
            return Ok(());
        }

        let path = file.display_path();

        // Check if file exists
        if !path.is_file() {
            file.lazy = false;
            return Ok(());
        }

        // Check file size
        let metadata = fs::metadata(path).map_err(|e| CrHelperError::Io(e))?;
        let max_size = self.config.max_file_size.unwrap_or(10 * 1024 * 1024);
        if metadata.len() as usize > max_size {
            // Too large, mark as binary
            file.mode = FileMode::Binary;
            file.lazy = false;
            return Ok(());
        }

        // Read file content
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => {
                // Binary file
                file.mode = FileMode::Binary;
                file.lazy = false;
                return Ok(());
            }
        };

        if content.is_empty() {
            file.lazy = false;
            return Ok(());
        }

        // Generate hunks and lines
        let lines: Vec<&str> = content.lines().collect();
        let line_count = lines.len();

        let mut hunk_lines = Vec::with_capacity(line_count);
        for (i, line_content) in lines.iter().enumerate() {
            hunk_lines.push(Line {
                id: LineId::from_content(path, line_content, i + 1),
                line_type: LineType::Added,
                old_line_num: None,
                new_line_num: Some(i + 1),
                content: line_content.to_string(),
            });
        }

        let hunk = Hunk {
            id: HunkId::new(&file.id, 0),
            header: format!("@@ -0,0 +1,{} @@", line_count),
            old_range: Range { start: 0, count: 0 },
            new_range: Range { start: 1, count: line_count },
            lines: hunk_lines,
        };

        file.hunks = vec![hunk];
        file.lazy = false;

        Ok(())
    }

    /// Parse diff --git header to extract paths
    fn parse_diff_header(&self, line: &str) -> Result<(Option<PathBuf>, Option<PathBuf>)> {
        // Format: "diff --git a/path b/path"
        let parts: Vec<&str> = line.split(' ').collect();
        if parts.len() < 4 {
            return Err(CrHelperError::InvalidDiff(format!(
                "Invalid diff header: {}",
                line
            )));
        }

        let old_path = parts[2].strip_prefix("a/").map(PathBuf::from);
        let new_path = parts[3].strip_prefix("b/").map(PathBuf::from);

        Ok((old_path, new_path))
    }

    /// Parse hunk header to extract ranges
    fn parse_hunk_header(&self, line: &str) -> Result<(Range, Range)> {
        // Format: "@@ -10,5 +10,7 @@" or "@@ -10 +10 @@"
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(CrHelperError::InvalidDiff(format!(
                "Invalid hunk header: {}",
                line
            )));
        }

        let old_range = self.parse_range(parts[1].trim_start_matches('-'))?;
        let new_range = self.parse_range(parts[2].trim_start_matches('+'))?;

        Ok((old_range, new_range))
    }

    /// Parse a range string like "10,5" or "10"
    fn parse_range(&self, s: &str) -> Result<Range> {
        let parts: Vec<&str> = s.split(',').collect();
        let start = parts[0]
            .parse::<usize>()
            .map_err(|_| CrHelperError::InvalidDiff(format!("Invalid range: {}", s)))?;
        let count = if parts.len() > 1 {
            parts[1]
                .parse::<usize>()
                .map_err(|_| CrHelperError::InvalidDiff(format!("Invalid range: {}", s)))?
        } else {
            1
        };

        Ok(Range::new(start, count))
    }

    /// Parse a diff line
    fn parse_line(
        &self,
        line: &str,
        current_file: &Option<FileDiffBuilder>,
        hunk: &HunkBuilder,
    ) -> Result<Option<Line>> {
        if line.is_empty() {
            return Ok(None);
        }

        let first_char = line.chars().next().unwrap_or(' ');
        let (line_type, content) = match first_char {
            '+' => (LineType::Added, &line[1..]),
            '-' => (LineType::Deleted, &line[1..]),
            ' ' => (LineType::Context, &line[1..]),
            '\\' => (LineType::NoNewline, line),
            _ => return Ok(None), // Skip unknown lines
        };

        // Calculate line numbers
        let (old_line_num, new_line_num) = self.calculate_line_nums(line_type, hunk);

        // Generate line ID
        let file_path = current_file
            .as_ref()
            .and_then(|f| f.new_path.as_ref().or(f.old_path.as_ref()))
            .cloned()
            .unwrap_or_else(|| PathBuf::from("unknown"));

        let line_num = new_line_num.or(old_line_num).unwrap_or(0);
        let line_id = LineId::from_content(&file_path, content, line_num);

        Ok(Some(Line {
            id: line_id,
            line_type,
            content: content.to_string(),
            old_line_num,
            new_line_num,
        }))
    }

    /// Calculate line numbers for a line
    fn calculate_line_nums(&self, line_type: LineType, hunk: &HunkBuilder) -> (Option<usize>, Option<usize>) {
        let old_offset = hunk.lines.iter()
            .filter(|l| matches!(l.line_type, LineType::Deleted | LineType::Context))
            .count();
        let new_offset = hunk.lines.iter()
            .filter(|l| matches!(l.line_type, LineType::Added | LineType::Context))
            .count();

        match line_type {
            LineType::Added => (None, Some(hunk.new_range.start + new_offset)),
            LineType::Deleted => (Some(hunk.old_range.start + old_offset), None),
            LineType::Context => (
                Some(hunk.old_range.start + old_offset),
                Some(hunk.new_range.start + new_offset),
            ),
            LineType::NoNewline => (None, None),
        }
    }
}

impl Default for DiffParser {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for FileDiff
struct FileDiffBuilder {
    id: FileId,
    old_path: Option<PathBuf>,
    new_path: Option<PathBuf>,
    mode: FileMode,
    hunks: Vec<Hunk>,
}

impl FileDiffBuilder {
    fn new(old_path: Option<PathBuf>, new_path: Option<PathBuf>) -> Self {
        let id = FileId::from_path(new_path.as_ref().or(old_path.as_ref()).unwrap());
        Self {
            id,
            old_path,
            new_path,
            mode: FileMode::Modified,
            hunks: Vec::new(),
        }
    }

    fn build(self) -> FileDiff {
        FileDiff {
            id: self.id,
            old_path: self.old_path,
            new_path: self.new_path,
            mode: self.mode,
            hunks: self.hunks,
            lazy: false,
        }
    }
}

/// Builder for Hunk
struct HunkBuilder {
    id: HunkId,
    header: String,
    old_range: Range,
    new_range: Range,
    lines: Vec<Line>,
}

impl HunkBuilder {
    fn new(id: HunkId, header: String, old_range: Range, new_range: Range) -> Self {
        Self {
            id,
            header,
            old_range,
            new_range,
            lines: Vec::new(),
        }
    }

    fn build(self) -> Hunk {
        Hunk {
            id: self.id,
            header: self.header,
            old_range: self.old_range,
            new_range: self.new_range,
            lines: self.lines,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DIFF: &str = r#"diff --git a/src/main.rs b/src/main.rs
index 1234567..abcdefg 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,6 @@
 fn main() {
-    println!("Hello, world!");
+    println!("Hello, Rust!");
+    println!("Welcome!");
 }
"#;

    #[test]
    fn test_parse_simple_diff() {
        let parser = DiffParser::new();
        let diff = parser.parse(SAMPLE_DIFF).unwrap();

        assert_eq!(diff.files.len(), 1);
        assert_eq!(diff.files[0].hunks.len(), 1);

        let hunk = &diff.files[0].hunks[0];
        assert!(hunk.lines.len() >= 4);
    }

    #[test]
    fn test_parse_hunk_header() {
        let parser = DiffParser::new();

        let (old, new) = parser.parse_hunk_header("@@ -10,5 +10,7 @@").unwrap();
        assert_eq!(old.start, 10);
        assert_eq!(old.count, 5);
        assert_eq!(new.start, 10);
        assert_eq!(new.count, 7);

        let (old, new) = parser.parse_hunk_header("@@ -1 +1 @@").unwrap();
        assert_eq!(old.count, 1);
        assert_eq!(new.count, 1);
    }

    #[test]
    fn test_parse_diff_header() {
        let parser = DiffParser::new();

        let (old, new) = parser.parse_diff_header("diff --git a/foo.rs b/foo.rs").unwrap();
        assert_eq!(old, Some(PathBuf::from("foo.rs")));
        assert_eq!(new, Some(PathBuf::from("foo.rs")));
    }

    #[test]
    fn test_line_types() {
        let parser = DiffParser::new();
        let diff = parser.parse(SAMPLE_DIFF).unwrap();

        let hunk = &diff.files[0].hunks[0];
        let types: Vec<_> = hunk.lines.iter().map(|l| l.line_type).collect();

        assert!(types.contains(&LineType::Added));
        assert!(types.contains(&LineType::Deleted));
        assert!(types.contains(&LineType::Context));
    }

    #[test]
    fn test_stats_calculation() {
        let parser = DiffParser::new();
        let diff = parser.parse(SAMPLE_DIFF).unwrap();

        assert_eq!(diff.stats.files_changed, 1);
        assert!(diff.stats.insertions >= 2);
        assert!(diff.stats.deletions >= 1);
    }
}
