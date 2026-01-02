//! Code context extraction for exports

use crate::comment::model::Comment;
use crate::diff::DiffData;
use std::path::Path;

/// A single line in the code context
pub struct ContextLine {
    /// Line number (if available)
    pub line_num: Option<usize>,
    /// Diff prefix (+, -, or space)
    pub prefix: char,
    /// Line content
    pub content: String,
    /// Whether this is the target line (the one with the comment)
    pub is_target: bool,
}

/// Code context around a comment
pub struct CodeContext {
    /// Lines of context
    pub lines: Vec<ContextLine>,
    /// Target line number
    pub target_line_num: Option<usize>,
    /// Target line content
    pub target_content: String,
}

/// Extracts code context around comments
pub struct ContextExtractor {
    /// Number of lines before and after
    context_lines: usize,
}

impl ContextExtractor {
    /// Create a new context extractor
    pub fn new(context_lines: usize) -> Self {
        Self { context_lines }
    }

    /// Extract context for a comment
    pub fn extract(&self, comment: &Comment, diff: &DiffData) -> Option<CodeContext> {
        let file = diff.get_file(comment.file_id())?;

        // Get the line ID from the comment
        let line_ids = comment.line_ids();
        let primary_line_id = line_ids.first()?;

        // Find the line in the diff
        let mut found_hunk_idx = None;
        let mut found_line_idx = None;

        for (hunk_idx, hunk) in file.hunks.iter().enumerate() {
            for (line_idx, line) in hunk.lines.iter().enumerate() {
                if &line.id == *primary_line_id {
                    found_hunk_idx = Some(hunk_idx);
                    found_line_idx = Some(line_idx);
                    break;
                }
            }
            if found_hunk_idx.is_some() {
                break;
            }
        }

        let hunk_idx = found_hunk_idx?;
        let line_idx = found_line_idx?;

        let hunk = &file.hunks[hunk_idx];

        // Calculate context range
        let start = line_idx.saturating_sub(self.context_lines);
        let end = (line_idx + self.context_lines + 1).min(hunk.lines.len());

        // Get the target line info
        let target_line = &hunk.lines[line_idx];
        let target_line_num = target_line.new_line_num.or(target_line.old_line_num);

        // Build context lines with line numbers
        let mut lines = Vec::new();
        for i in start..end {
            let line = &hunk.lines[i];
            let prefix = line.line_type.prefix();
            let line_num = line.new_line_num.or(line.old_line_num);
            let is_target = i == line_idx;
            lines.push(ContextLine {
                line_num,
                prefix,
                content: line.content.clone(),
                is_target,
            });
        }

        Some(CodeContext {
            lines,
            target_line_num,
            target_content: target_line.content.clone(),
        })
    }

    /// Get the programming language from file extension
    pub fn get_language(file_path: &str) -> &'static str {
        let path = Path::new(file_path);
        match path.extension().and_then(|e| e.to_str()) {
            Some("rs") => "rust",
            Some("py") => "python",
            Some("js") => "javascript",
            Some("ts") => "typescript",
            Some("tsx") => "tsx",
            Some("jsx") => "jsx",
            Some("go") => "go",
            Some("java") => "java",
            Some("c") => "c",
            Some("cpp") | Some("cc") | Some("cxx") => "cpp",
            Some("h") | Some("hpp") => "cpp",
            Some("rb") => "ruby",
            Some("php") => "php",
            Some("swift") => "swift",
            Some("kt") | Some("kts") => "kotlin",
            Some("cs") => "csharp",
            Some("sh") | Some("bash") => "bash",
            Some("json") => "json",
            Some("yaml") | Some("yml") => "yaml",
            Some("toml") => "toml",
            Some("xml") => "xml",
            Some("html") | Some("htm") => "html",
            Some("css") => "css",
            Some("scss") | Some("sass") => "scss",
            Some("sql") => "sql",
            Some("md") | Some("markdown") => "markdown",
            _ => "",
        }
    }

    /// Format context as a code block with language
    pub fn format_code_block(context: &CodeContext, file_path: &str) -> String {
        let lang = Self::get_language(file_path);
        let mut output = String::new();

        // Show the target line prominently first
        if let Some(line_num) = context.target_line_num {
            output.push_str(&format!("> **Line {}:** `{}`\n\n", line_num, context.target_content.trim()));
        }

        // Then show the context with the target line highlighted
        output.push_str(&format!("```{}\n", lang));
        for line in &context.lines {
            let line_num_str = line.line_num
                .map(|n| format!("{:>4}", n))
                .unwrap_or_else(|| "    ".to_string());

            let marker = if line.is_target { " ◀◀◀" } else { "" };
            output.push_str(&format!("{} {}{}{}\n", line_num_str, line.prefix, line.content, marker));
        }
        output.push_str("```");

        output
    }
}

impl Default for ContextExtractor {
    fn default() -> Self {
        Self::new(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_extractor_creation() {
        let extractor = ContextExtractor::new(3);
        assert_eq!(extractor.context_lines, 3);
    }

    #[test]
    fn test_get_language() {
        assert_eq!(ContextExtractor::get_language("main.rs"), "rust");
        assert_eq!(ContextExtractor::get_language("app.py"), "python");
        assert_eq!(ContextExtractor::get_language("index.js"), "javascript");
        assert_eq!(ContextExtractor::get_language("main.go"), "go");
        assert_eq!(ContextExtractor::get_language("unknown.xyz"), "");
    }

    #[test]
    fn test_format_code_block() {
        let context = CodeContext {
            lines: vec![
                ContextLine { line_num: Some(1), prefix: ' ', content: "fn main() {".to_string(), is_target: false },
                ContextLine { line_num: Some(2), prefix: '+', content: "    println!(\"Hello\");".to_string(), is_target: true },
                ContextLine { line_num: Some(3), prefix: ' ', content: "}".to_string(), is_target: false },
            ],
            target_line_num: Some(2),
            target_content: "    println!(\"Hello\");".to_string(),
        };
        let block = ContextExtractor::format_code_block(&context, "main.rs");

        assert!(block.contains("```rust"));
        assert!(block.contains("```"));
        assert!(block.contains("fn main()"));
        assert!(block.contains("◀◀◀")); // Target marker
        assert!(block.contains("**Line 2:**")); // Line highlight
    }

    #[test]
    fn test_extract_no_diff() {
        let extractor = ContextExtractor::new(2);
        let diff = DiffData::empty();

        // Create a comment with non-existent file
        use crate::comment::builder::CommentBuilder;
        use crate::comment::model::DiffSide;
        use crate::types::{FileId, LineId};

        let comment = CommentBuilder::new(
            FileId::from_string("nonexistent"),
            LineId::from_string("line1"),
            DiffSide::New,
        )
        .content("Test")
        .build()
        .unwrap();

        let result = extractor.extract(&comment, &diff);
        assert!(result.is_none());
    }
}
