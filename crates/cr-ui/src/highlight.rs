//! Syntax highlighting using syntect

use ratatui::style::{Color, Style};
use ratatui::text::Span;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Style as SyntectStyle, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

/// Syntax highlighter for code
pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    theme_name: String,
}

impl Highlighter {
    /// Create a new highlighter with default theme
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            theme_name: "base16-ocean.dark".to_string(),
        }
    }

    /// Create with a specific theme
    pub fn with_theme(theme_name: &str) -> Self {
        let mut h = Self::new();
        h.theme_name = theme_name.to_string();
        h
    }

    /// Get available theme names
    pub fn available_themes(&self) -> Vec<&str> {
        self.theme_set.themes.keys().map(|s| s.as_str()).collect()
    }

    /// Highlight a single line of code
    pub fn highlight_line<'a>(&self, line: &'a str, file_path: &str) -> Vec<Span<'a>> {
        // Try to get syntax for the file extension
        let syntax = self
            .syntax_set
            .find_syntax_for_file(file_path)
            .ok()
            .flatten()
            .or_else(|| {
                // Fallback: try to detect from extension
                let ext = Path::new(file_path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                self.syntax_set.find_syntax_by_extension(ext)
            })
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = self
            .theme_set
            .themes
            .get(&self.theme_name)
            .unwrap_or_else(|| {
                self.theme_set
                    .themes
                    .values()
                    .next()
                    .expect("No themes available")
            });

        let mut highlighter = HighlightLines::new(syntax, theme);

        // Highlight the line
        match highlighter.highlight_line(line, &self.syntax_set) {
            Ok(ranges) => ranges
                .into_iter()
                .map(|(style, text)| {
                    Span::styled(text.to_string(), syntect_to_ratatui_style(style))
                })
                .collect(),
            Err(_) => {
                // Fallback to plain text on error
                vec![Span::raw(line.to_string())]
            }
        }
    }

    /// Highlight multiple lines and return styled spans for each
    pub fn highlight_lines<'a>(
        &self,
        content: &'a str,
        file_path: &str,
    ) -> Vec<Vec<Span<'static>>> {
        let syntax = self
            .syntax_set
            .find_syntax_for_file(file_path)
            .ok()
            .flatten()
            .or_else(|| {
                let ext = Path::new(file_path)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                self.syntax_set.find_syntax_by_extension(ext)
            })
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = self
            .theme_set
            .themes
            .get(&self.theme_name)
            .unwrap_or_else(|| {
                self.theme_set
                    .themes
                    .values()
                    .next()
                    .expect("No themes available")
            });

        let mut highlighter = HighlightLines::new(syntax, theme);
        let mut result = Vec::new();

        for line in LinesWithEndings::from(content) {
            match highlighter.highlight_line(line, &self.syntax_set) {
                Ok(ranges) => {
                    let spans: Vec<Span<'static>> = ranges
                        .into_iter()
                        .map(|(style, text)| {
                            Span::styled(text.to_string(), syntect_to_ratatui_style(style))
                        })
                        .collect();
                    result.push(spans);
                }
                Err(_) => {
                    result.push(vec![Span::raw(line.to_string())]);
                }
            }
        }

        result
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert syntect style to ratatui style
fn syntect_to_ratatui_style(style: SyntectStyle) -> Style {
    let fg = Color::Rgb(
        style.foreground.r,
        style.foreground.g,
        style.foreground.b,
    );

    Style::default().fg(fg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlighter_creation() {
        let h = Highlighter::new();
        assert!(!h.available_themes().is_empty());
    }

    #[test]
    fn test_highlight_rust() {
        let h = Highlighter::new();
        let spans = h.highlight_line("fn main() {}", "test.rs");
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_highlight_unknown_extension() {
        let h = Highlighter::new();
        let spans = h.highlight_line("some text", "file.xyz");
        assert!(!spans.is_empty());
    }

    #[test]
    fn test_highlight_multiple_lines() {
        let h = Highlighter::new();
        let content = "fn main() {\n    println!(\"Hello\");\n}";
        let lines = h.highlight_lines(content, "test.rs");
        assert_eq!(lines.len(), 3);
    }
}
