//! Diff navigation logic

use crate::diff::model::{DiffData, FileDiff, Hunk, Line};

/// Position in a diff
#[derive(Debug, Clone, Copy, Default)]
pub struct Position {
    /// Current file index
    pub file_idx: usize,
    /// Current hunk index within file
    pub hunk_idx: usize,
    /// Current line index within hunk
    pub line_idx: usize,
}

impl Position {
    /// Create a new position at the start
    pub fn new() -> Self {
        Self::default()
    }
}

/// Navigator for traversing diff data
pub struct DiffNavigator {
    diff: DiffData,
    position: Position,
}

impl DiffNavigator {
    /// Create a new navigator
    pub fn new(diff: DiffData) -> Self {
        Self {
            diff,
            position: Position::new(),
        }
    }

    /// Get current position
    pub fn current_position(&self) -> Position {
        self.position
    }

    /// Get reference to the diff data
    pub fn diff(&self) -> &DiffData {
        &self.diff
    }

    /// Move to next line
    pub fn next_line(&mut self) -> bool {
        if let Some(hunk) = self.current_hunk() {
            if self.position.line_idx + 1 < hunk.lines.len() {
                self.position.line_idx += 1;
                return true;
            }
        }
        // Try next hunk
        if self.next_hunk() {
            self.position.line_idx = 0;
            return true;
        }
        false
    }

    /// Move to previous line
    pub fn prev_line(&mut self) -> bool {
        if self.position.line_idx > 0 {
            self.position.line_idx -= 1;
            return true;
        }
        // Try previous hunk
        let old_hunk = self.position.hunk_idx;
        if self.prev_hunk() {
            // Go to last line of previous hunk
            if let Some(hunk) = self.current_hunk() {
                self.position.line_idx = hunk.lines.len().saturating_sub(1);
            }
            return true;
        }
        self.position.hunk_idx = old_hunk;
        false
    }

    /// Move to next hunk
    pub fn next_hunk(&mut self) -> bool {
        if let Some(file) = self.current_file() {
            if self.position.hunk_idx + 1 < file.hunks.len() {
                self.position.hunk_idx += 1;
                self.position.line_idx = 0;
                return true;
            }
        }
        // Try next file
        if self.next_file() {
            self.position.hunk_idx = 0;
            self.position.line_idx = 0;
            return true;
        }
        false
    }

    /// Move to previous hunk
    pub fn prev_hunk(&mut self) -> bool {
        if self.position.hunk_idx > 0 {
            self.position.hunk_idx -= 1;
            self.position.line_idx = 0;
            return true;
        }
        // Try previous file
        let old_file = self.position.file_idx;
        if self.prev_file() {
            // Go to last hunk of previous file
            if let Some(file) = self.current_file() {
                self.position.hunk_idx = file.hunks.len().saturating_sub(1);
            }
            self.position.line_idx = 0;
            return true;
        }
        self.position.file_idx = old_file;
        false
    }

    /// Move to next file
    pub fn next_file(&mut self) -> bool {
        if self.position.file_idx + 1 < self.diff.files.len() {
            self.position.file_idx += 1;
            self.position.hunk_idx = 0;
            self.position.line_idx = 0;
            return true;
        }
        false
    }

    /// Move to previous file
    pub fn prev_file(&mut self) -> bool {
        if self.position.file_idx > 0 {
            self.position.file_idx -= 1;
            self.position.hunk_idx = 0;
            self.position.line_idx = 0;
            return true;
        }
        false
    }

    /// Go to a specific file
    pub fn goto_file(&mut self, file_idx: usize) -> bool {
        if file_idx < self.diff.files.len() {
            self.position.file_idx = file_idx;
            self.position.hunk_idx = 0;
            self.position.line_idx = 0;
            return true;
        }
        false
    }

    /// Go to a specific line within a file
    pub fn goto_line(&mut self, file_idx: usize, global_line_idx: usize) -> bool {
        if !self.goto_file(file_idx) {
            return false;
        }

        let file = &self.diff.files[file_idx];
        let mut current_line = 0;

        for (hunk_idx, hunk) in file.hunks.iter().enumerate() {
            if current_line + hunk.lines.len() > global_line_idx {
                self.position.hunk_idx = hunk_idx;
                self.position.line_idx = global_line_idx - current_line;
                return true;
            }
            current_line += hunk.lines.len();
        }

        false
    }

    /// Go to the top of the diff
    pub fn goto_top(&mut self) {
        self.position = Position::new();
    }

    /// Go to the bottom of the diff
    pub fn goto_bottom(&mut self) {
        if self.diff.files.is_empty() {
            return;
        }

        self.position.file_idx = self.diff.files.len() - 1;
        if let Some(file) = self.current_file() {
            if !file.hunks.is_empty() {
                self.position.hunk_idx = file.hunks.len() - 1;
                if let Some(hunk) = self.current_hunk() {
                    self.position.line_idx = hunk.lines.len().saturating_sub(1);
                }
            }
        }
    }

    /// Get current file
    pub fn current_file(&self) -> Option<&FileDiff> {
        self.diff.files.get(self.position.file_idx)
    }

    /// Get current hunk
    pub fn current_hunk(&self) -> Option<&Hunk> {
        self.current_file()
            .and_then(|f| f.hunks.get(self.position.hunk_idx))
    }

    /// Get current line
    pub fn current_line(&self) -> Option<&Line> {
        self.current_hunk()
            .and_then(|h| h.lines.get(self.position.line_idx))
    }

    /// Get total line count
    pub fn line_count(&self) -> usize {
        self.diff.total_lines()
    }

    /// Get file count
    pub fn file_count(&self) -> usize {
        self.diff.files.len()
    }

    /// Move down by N lines
    pub fn move_down(&mut self, n: usize) {
        for _ in 0..n {
            if !self.next_line() {
                break;
            }
        }
    }

    /// Move up by N lines
    pub fn move_up(&mut self, n: usize) {
        for _ in 0..n {
            if !self.prev_line() {
                break;
            }
        }
    }

    /// Get global line index (across all files)
    pub fn global_line_index(&self) -> usize {
        let mut index = 0;

        for (file_idx, file) in self.diff.files.iter().enumerate() {
            if file_idx < self.position.file_idx {
                index += file.total_lines();
            } else if file_idx == self.position.file_idx {
                for (hunk_idx, hunk) in file.hunks.iter().enumerate() {
                    if hunk_idx < self.position.hunk_idx {
                        index += hunk.lines.len();
                    } else if hunk_idx == self.position.hunk_idx {
                        index += self.position.line_idx;
                        break;
                    }
                }
                break;
            }
        }

        index
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::parser::DiffParser;

    fn create_test_navigator() -> DiffNavigator {
        let diff_str = r#"diff --git a/file1.rs b/file1.rs
@@ -1,3 +1,4 @@
 line1
-line2
+line2_modified
+line3
 line4
diff --git a/file2.rs b/file2.rs
@@ -1,2 +1,2 @@
-old
+new
"#;
        let parser = DiffParser::new();
        let diff = parser.parse(diff_str).unwrap();
        DiffNavigator::new(diff)
    }

    #[test]
    fn test_initial_position() {
        let nav = create_test_navigator();
        let pos = nav.current_position();
        assert_eq!(pos.file_idx, 0);
        assert_eq!(pos.hunk_idx, 0);
        assert_eq!(pos.line_idx, 0);
    }

    #[test]
    fn test_next_line() {
        let mut nav = create_test_navigator();
        assert!(nav.next_line());
        assert_eq!(nav.current_position().line_idx, 1);
    }

    #[test]
    fn test_prev_line() {
        let mut nav = create_test_navigator();
        nav.next_line();
        nav.next_line();
        assert!(nav.prev_line());
        assert_eq!(nav.current_position().line_idx, 1);
    }

    #[test]
    fn test_next_file() {
        let mut nav = create_test_navigator();
        assert!(nav.next_file());
        assert_eq!(nav.current_position().file_idx, 1);
    }

    #[test]
    fn test_goto_top_and_bottom() {
        let mut nav = create_test_navigator();
        nav.goto_bottom();
        assert_eq!(nav.current_position().file_idx, 1);
        nav.goto_top();
        assert_eq!(nav.current_position().file_idx, 0);
        assert_eq!(nav.current_position().line_idx, 0);
    }

    #[test]
    fn test_current_accessors() {
        let nav = create_test_navigator();
        assert!(nav.current_file().is_some());
        assert!(nav.current_hunk().is_some());
        assert!(nav.current_line().is_some());
    }

    #[test]
    fn test_file_count() {
        let nav = create_test_navigator();
        assert_eq!(nav.file_count(), 2);
    }
}
