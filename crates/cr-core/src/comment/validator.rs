//! Comment validation

use super::model::{Comment, LineReference};
use crate::diff::DiffData;
use crate::error::{CrHelperError, Result};

/// Maximum comment length (default)
pub const MAX_COMMENT_LENGTH: usize = 10000;

/// Minimum comment length
pub const MIN_COMMENT_LENGTH: usize = 1;

/// Validator for comments
pub struct CommentValidator {
    max_length: usize,
    min_length: usize,
}

impl CommentValidator {
    /// Create a new validator with default settings
    pub fn new() -> Self {
        Self {
            max_length: MAX_COMMENT_LENGTH,
            min_length: MIN_COMMENT_LENGTH,
        }
    }

    /// Create a new validator with custom max length
    pub fn with_max_length(max_length: usize) -> Self {
        Self {
            max_length,
            min_length: MIN_COMMENT_LENGTH,
        }
    }

    /// Validate comment content
    pub fn validate_content(&self, content: &str) -> Result<()> {
        let trimmed = content.trim();

        if trimmed.len() < self.min_length {
            return Err(CrHelperError::Validation(
                "Comment content cannot be empty".to_string(),
            ));
        }

        if trimmed.len() > self.max_length {
            return Err(CrHelperError::Validation(format!(
                "Comment content exceeds maximum length of {} characters",
                self.max_length
            )));
        }

        Ok(())
    }

    /// Validate line reference against diff data
    pub fn validate_line_ref(&self, line_ref: &LineReference, diff: &DiffData) -> Result<()> {
        match line_ref {
            LineReference::SingleLine { file_id, line_id, .. } => {
                // Check file exists
                let file = diff.get_file(file_id).ok_or_else(|| {
                    CrHelperError::Validation(format!("File {} not found in diff", file_id))
                })?;

                // Check line exists in file
                let line_exists = file.hunks.iter().any(|h| {
                    h.lines.iter().any(|l| &l.id == line_id)
                });

                if !line_exists {
                    return Err(CrHelperError::Validation(format!(
                        "Line {} not found in file {}",
                        line_id, file_id
                    )));
                }
            }
            LineReference::Range {
                file_id,
                start_line_id,
                end_line_id,
                ..
            } => {
                // Check file exists
                let file = diff.get_file(file_id).ok_or_else(|| {
                    CrHelperError::Validation(format!("File {} not found in diff", file_id))
                })?;

                // Check both lines exist
                let start_exists = file.hunks.iter().any(|h| {
                    h.lines.iter().any(|l| &l.id == start_line_id)
                });
                let end_exists = file.hunks.iter().any(|h| {
                    h.lines.iter().any(|l| &l.id == end_line_id)
                });

                if !start_exists {
                    return Err(CrHelperError::Validation(format!(
                        "Start line {} not found in file {}",
                        start_line_id, file_id
                    )));
                }
                if !end_exists {
                    return Err(CrHelperError::Validation(format!(
                        "End line {} not found in file {}",
                        end_line_id, file_id
                    )));
                }
            }
        }

        Ok(())
    }

    /// Validate a complete comment
    pub fn validate(&self, comment: &Comment, diff: Option<&DiffData>) -> Result<()> {
        // Validate content
        self.validate_content(&comment.content)?;

        // Validate line reference if diff is provided
        if let Some(diff) = diff {
            self.validate_line_ref(&comment.line_ref, diff)?;
        }

        // Validate tags (no empty tags)
        for tag in &comment.tags {
            if tag.trim().is_empty() {
                return Err(CrHelperError::Validation(
                    "Tags cannot be empty".to_string(),
                ));
            }
        }

        Ok(())
    }
}

impl Default for CommentValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_content_valid() {
        let validator = CommentValidator::new();
        assert!(validator.validate_content("Valid comment").is_ok());
    }

    #[test]
    fn test_validate_content_empty() {
        let validator = CommentValidator::new();
        assert!(validator.validate_content("").is_err());
        assert!(validator.validate_content("   ").is_err());
    }

    #[test]
    fn test_validate_content_too_long() {
        let validator = CommentValidator::with_max_length(10);
        assert!(validator.validate_content("Short").is_ok());
        assert!(validator.validate_content("This is too long").is_err());
    }

    #[test]
    fn test_validate_content_trims_whitespace() {
        let validator = CommentValidator::new();
        assert!(validator.validate_content("  Valid  ").is_ok());
    }
}
