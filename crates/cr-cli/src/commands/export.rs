//! Export command
//!
//! Export review session to various formats.

use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use std::io::Write;
use std::path::PathBuf;

use cr_core::export::ExportManager;
use cr_core::session::SessionManager;
use cr_core::types::SessionId;
use cr_storage::FileSystemStorage;

/// Export format options
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// Compact JSON (optimized for Claude Code)
    JsonCompact,
    /// Markdown format
    Markdown,
    /// Enhanced Markdown with anchors and frontmatter
    MarkdownEnhanced,
}

/// Arguments for the export command
#[derive(Debug, Args)]
pub struct ExportArgs {
    /// Session ID to export
    #[arg(long, short)]
    pub session: Option<String>,

    /// Export the latest session
    #[arg(long)]
    pub latest: bool,

    /// Export format
    #[arg(long, short, value_enum, default_value = "markdown")]
    pub format: ExportFormat,

    /// Output file path (stdout if not specified)
    #[arg(long, short)]
    pub output: Option<PathBuf>,

    /// Use compact format (for JSON)
    #[arg(long)]
    pub compact: bool,

    /// Session storage directory
    #[arg(long)]
    pub sessions_dir: Option<PathBuf>,
}

/// Execute the export command
pub fn execute(args: ExportArgs) -> Result<()> {
    use colored::Colorize;

    // Set up storage
    let storage_path = args
        .sessions_dir
        .clone()
        .unwrap_or_else(|| PathBuf::from(".cr-helper/sessions"));
    let storage = FileSystemStorage::new(&storage_path)?;
    let manager = SessionManager::new(storage);

    // Load session
    let session = if args.latest {
        manager
            .load_latest()?
            .context("No sessions found")?
    } else if let Some(session_id) = &args.session {
        let id = SessionId::from_string(session_id)
            .context(format!("Invalid session ID: {}", session_id))?;
        manager
            .load(&id)
            .context(format!("Session '{}' not found", session_id))?
    } else {
        // Try to load latest
        manager
            .load_latest()?
            .context("No session specified. Use --session <ID> or --latest")?
    };

    eprintln!(
        "Exporting session {} ({} comments)...",
        session.id.to_string().cyan(),
        session.comments.count().to_string().yellow()
    );

    // Set up exporter - ExportManager::new() already registers default exporters
    let export_manager = ExportManager::new();

    // Get format name
    let format_name = match args.format {
        ExportFormat::Json => "json",
        ExportFormat::JsonCompact => "json-compact",
        ExportFormat::Markdown => "markdown",
        ExportFormat::MarkdownEnhanced => "markdown-enhanced",
    };

    // Export
    let output = export_manager.export(&session, format_name)?;

    // Write output
    if let Some(output_path) = args.output {
        std::fs::write(&output_path, &output)
            .context(format!("Failed to write to {}", output_path.display()))?;
        eprintln!("{} Exported to {}", "✓".green(), output_path.display());
    } else {
        // Write to stdout
        std::io::stdout()
            .write_all(output.as_bytes())
            .context("Failed to write to stdout")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_format_values() {
        // Test that all enum values can be parsed
        assert!(ExportFormat::from_str("json", true).is_ok());
        assert!(ExportFormat::from_str("json-compact", true).is_ok());
        assert!(ExportFormat::from_str("markdown", true).is_ok());
        assert!(ExportFormat::from_str("markdown-enhanced", true).is_ok());
    }
}
