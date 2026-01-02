//! cr-integration - Integration library for cr-helper
//!
//! This crate provides integrations with Claude Code and other Agent CLIs.
//!
//! ## Features
//!
//! - Agent adapter trait for extensibility
//! - Claude Code adapter implementation
//! - Installation and verification utilities
//!
//! ## Usage
//!
//! ```rust,ignore
//! use cr_integration::{AgentAdapter, ClaudeCodeAdapter};
//!
//! let adapter = ClaudeCodeAdapter::new();
//! if let Some(info) = adapter.detect()? {
//!     println!("Found Claude Code: {:?}", info);
//! }
//! ```

pub mod adapter;
pub mod detection;
pub mod verification;

pub use adapter::{AgentAdapter, AgentInfo, AgentType};
pub use adapter::claude_code::ClaudeCodeAdapter;
pub use detection::detect_agents;
pub use verification::VerificationResult;
