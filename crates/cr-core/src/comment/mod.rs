//! Comment system module
//!
//! Handles comment management, indexing, and validation.

pub mod model;
pub mod manager;
pub mod index;
pub mod validator;
pub mod builder;

pub use model::*;
pub use manager::CommentManager;
pub use index::CommentIndex;
pub use validator::CommentValidator;
pub use builder::CommentBuilder;
