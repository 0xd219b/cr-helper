//! Layout management

use ratatui::prelude::*;

/// Layout manager
pub struct LayoutManager;

impl LayoutManager {
    /// Create a new layout manager
    pub fn new() -> Self {
        Self
    }
}

impl Default for LayoutManager {
    fn default() -> Self {
        Self::new()
    }
}
