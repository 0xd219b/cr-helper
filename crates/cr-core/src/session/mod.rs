//! Session management module
//!
//! This module provides session lifecycle management for code reviews,
//! including creation, persistence, and querying of review sessions.
//!
//! # Overview
//!
//! A session represents a single code review context, containing:
//! - The diff being reviewed
//! - Comments made during the review
//! - Metadata about the review
//!
//! # Example
//!
//! ```ignore
//! use cr_core::session::{SessionManager, DiffSource};
//! use cr_core::diff::DiffData;
//!
//! // Create a session manager with file storage
//! let storage = FileSystemStorage::new("/tmp/cr-helper")?;
//! let manager = SessionManager::new(storage);
//!
//! // Create a new review session
//! let session = manager.create(DiffSource::Staged, diff_data)?;
//!
//! // Later, load the session
//! let loaded = manager.load(&session.id)?;
//! ```

mod manager;
pub mod migration;
mod model;
mod persistence;

// Re-export public API
pub use manager::SessionManager;
pub use migration::{SessionFile, SessionMigrator, CURRENT_SCHEMA_VERSION};
pub use model::{DiffSource, Session, SessionFilter, SessionInfo, SessionMetadata};
pub use persistence::SessionStorage;

// Re-export memory storage for testing
#[cfg(test)]
pub use persistence::memory::MemoryStorage;
