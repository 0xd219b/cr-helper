//! cr-helper - Code Review Helper CLI
//!
//! A terminal-based code review tool that integrates with Claude Code.
//!
//! ## Quick Start
//!
//! ```bash
//! # Initialize in your project
//! cr-helper init
//!
//! # Install Claude Code integration
//! cr-helper install --claude-code
//!
//! # Start a review
//! cr-helper review
//!
//! # Export results
//! cr-helper export --latest
//! ```

mod commands;

fn main() {
    if let Err(err) = commands::run() {
        eprintln!("Error: {:#}", err);
        std::process::exit(1);
    }
}
