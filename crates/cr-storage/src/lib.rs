//! cr-storage - Storage library for cr-helper
//!
//! This crate provides storage implementations for sessions and other data.

mod session_store;

pub use session_store::FileSystemStorage;
