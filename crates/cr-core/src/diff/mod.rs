//! Diff engine module
//!
//! Handles parsing, modeling, and navigation of git diffs.

pub mod model;
pub mod parser;
pub mod navigator;
pub mod delta;

pub use model::*;
pub use parser::{DiffParser, ParserConfig};
pub use navigator::{DiffNavigator, Position};
pub use delta::{DeltaRenderer, DeltaConfig};
