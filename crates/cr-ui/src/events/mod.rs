//! Event handling

use crossterm::event::KeyEvent;

/// Application events
#[derive(Debug, Clone)]
pub enum Event {
    /// Keyboard input
    Input(KeyEvent),
    /// Terminal resize
    Resize(u16, u16),
    /// Periodic tick
    Tick,
    /// Session was updated
    SessionUpdated,
    /// Error occurred
    Error(String),
}
