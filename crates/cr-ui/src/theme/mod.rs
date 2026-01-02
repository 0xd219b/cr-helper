//! Theme system

use ratatui::prelude::*;

/// Application theme
#[derive(Debug, Clone)]
pub struct Theme {
    /// Border color for focused elements
    pub focus_border: Color,
    /// Border color for unfocused elements
    pub unfocus_border: Color,
    /// Added line color
    pub added: Color,
    /// Deleted line color
    pub deleted: Color,
    /// Context line color
    pub context: Color,
    /// Critical severity color
    pub critical: Color,
    /// Warning severity color
    pub warning: Color,
    /// Info severity color
    pub info: Color,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            focus_border: Color::Cyan,
            unfocus_border: Color::DarkGray,
            added: Color::Green,
            deleted: Color::Red,
            context: Color::Gray,
            critical: Color::Red,
            warning: Color::Yellow,
            info: Color::Blue,
        }
    }
}
