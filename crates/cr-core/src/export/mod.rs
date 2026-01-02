//! Export functionality for sessions
//!
//! This module provides exporters for converting review sessions
//! to various formats including JSON, Markdown, and enhanced Markdown.
//!
//! # Overview
//!
//! Export functionality supports:
//! - JSON format (compact and pretty-printed)
//! - Markdown format (human-readable reports)
//! - Enhanced Markdown (with YAML frontmatter and anchors)
//!
//! # Example
//!
//! ```ignore
//! use cr_core::export::ExportManager;
//!
//! let manager = ExportManager::new();
//! let json = manager.export(&session, "json")?;
//! let md = manager.export(&session, "markdown")?;
//! ```

mod context;
mod exporter;
mod json;
mod markdown;

pub use context::ContextExtractor;
pub use exporter::{ExportManager, Exporter};
pub use json::{ExportData, ExportLocation, ExportReview, ExportStats, JsonExporter, SeverityStats};
pub use markdown::{MarkdownEnhancedExporter, MarkdownExporter};
