//! cr-ui - TUI library for cr-helper
//!
//! This crate provides the Terminal User Interface for code review.
//!
//! # Overview
//!
//! The TUI provides:
//! - Diff view with syntax highlighting (via syntect)
//! - Inline comments with vim-style navigation
//! - File navigation
//! - Status bar with session info
//!
//! # Example
//!
//! ```ignore
//! use cr_ui::App;
//! use cr_core::session::Session;
//!
//! let app = App::new(session)?;
//! app.run()?;
//! ```

pub mod app;
pub mod components;
pub mod events;
pub mod highlight;
pub mod input;
pub mod layout;
pub mod theme;

pub use app::{App, AppMode, AppState};
pub use highlight::Highlighter;
