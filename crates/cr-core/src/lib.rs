//! cr-core - Core library for cr-helper
//!
//! This crate provides the core business logic for the Code Review Helper tool,
//! including diff parsing, comment management, session handling, and export functionality.

pub mod error;
pub mod types;
pub mod config;
pub mod diff;
pub mod comment;
pub mod session;
pub mod export;

pub use error::{CrHelperError, Result};
pub use types::*;
