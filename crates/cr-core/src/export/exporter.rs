//! Exporter trait and manager

use crate::error::{CrHelperError, Result};
use crate::session::Session;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::Path;

/// Trait for session exporters
pub trait Exporter: Send + Sync {
    /// Export a session to string
    fn export(&self, session: &Session) -> Result<String>;

    /// Get the format name
    fn format_name(&self) -> &str;

    /// Get the file extension
    fn file_extension(&self) -> &str;
}

/// Manager for handling multiple export formats
pub struct ExportManager {
    exporters: HashMap<String, Box<dyn Exporter>>,
}

impl ExportManager {
    /// Create a new export manager with default exporters
    pub fn new() -> Self {
        let mut manager = Self {
            exporters: HashMap::new(),
        };

        // Register default exporters
        manager.register(Box::new(super::json::JsonExporter::new(false)));
        manager.register(Box::new(super::json::JsonExporter::compact()));
        manager.register(Box::new(super::markdown::MarkdownExporter::new()));
        manager.register(Box::new(super::markdown::MarkdownEnhancedExporter::new()));

        manager
    }

    /// Register a new exporter
    pub fn register(&mut self, exporter: Box<dyn Exporter>) {
        self.exporters
            .insert(exporter.format_name().to_string(), exporter);
    }

    /// Export a session to the specified format
    pub fn export(&self, session: &Session, format: &str) -> Result<String> {
        let exporter = self.exporters.get(format).ok_or_else(|| {
            CrHelperError::Validation(format!("Unknown export format: {}", format))
        })?;

        exporter.export(session)
    }

    /// Export a session to a file
    pub fn export_to_file(&self, session: &Session, format: &str, path: &Path) -> Result<()> {
        let content = self.export(session, format)?;

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        // Get file extension from exporter
        let exporter = self.exporters.get(format).ok_or_else(|| {
            CrHelperError::Validation(format!("Unknown export format: {}", format))
        })?;

        // Add extension if needed
        let final_path = if path.extension().is_some() {
            path.to_path_buf()
        } else {
            path.with_extension(exporter.file_extension())
        };

        // Atomic write using temp file
        let temp_path = final_path.with_extension("tmp");
        {
            let mut file = fs::File::create(&temp_path)?;
            file.write_all(content.as_bytes())?;
            file.flush()?;
        }

        fs::rename(&temp_path, &final_path)?;
        Ok(())
    }

    /// Export a session and write to stdout
    pub fn export_to_stdout(&self, session: &Session, format: &str) -> Result<()> {
        let content = self.export(session, format)?;
        print!("{}", content);
        Ok(())
    }

    /// Get list of available format names
    pub fn available_formats(&self) -> Vec<String> {
        let mut formats: Vec<_> = self.exporters.keys().cloned().collect();
        formats.sort();
        formats
    }

    /// Check if a format is available
    pub fn has_format(&self, format: &str) -> bool {
        self.exporters.contains_key(format)
    }

    /// Get an exporter by format name
    pub fn get(&self, format: &str) -> Option<&dyn Exporter> {
        self.exporters.get(format).map(|e| e.as_ref())
    }
}

impl Default for ExportManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::DiffData;
    use crate::session::DiffSource;

    fn create_test_session() -> Session {
        Session::new(DiffSource::WorkingTree, DiffData::empty())
    }

    struct TestExporter;

    impl Exporter for TestExporter {
        fn export(&self, _session: &Session) -> Result<String> {
            Ok("test export".to_string())
        }

        fn format_name(&self) -> &str {
            "test"
        }

        fn file_extension(&self) -> &str {
            "txt"
        }
    }

    #[test]
    fn test_export_manager_creation() {
        let manager = ExportManager::new();
        assert!(manager.has_format("json"));
        assert!(manager.has_format("json-compact"));
        assert!(manager.has_format("markdown"));
        assert!(manager.has_format("markdown-enhanced"));
    }

    #[test]
    fn test_register_exporter() {
        let mut manager = ExportManager::new();
        manager.register(Box::new(TestExporter));
        assert!(manager.has_format("test"));
    }

    #[test]
    fn test_export_unknown_format() {
        let manager = ExportManager::new();
        let session = create_test_session();
        let result = manager.export(&session, "unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_available_formats() {
        let manager = ExportManager::new();
        let formats = manager.available_formats();
        assert!(formats.contains(&"json".to_string()));
        assert!(formats.contains(&"markdown".to_string()));
    }

    #[test]
    fn test_export_json() {
        let manager = ExportManager::new();
        let session = create_test_session();
        let result = manager.export(&session, "json");
        assert!(result.is_ok());
        let json = result.unwrap();
        assert!(json.contains("v"));
        assert!(json.contains("sid"));
    }

    #[test]
    fn test_export_markdown() {
        let manager = ExportManager::new();
        let session = create_test_session();
        let result = manager.export(&session, "markdown");
        assert!(result.is_ok());
        let md = result.unwrap();
        assert!(md.contains("# Code Review Report"));
    }
}
