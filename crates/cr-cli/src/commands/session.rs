//! Session command
//!
//! Manage review sessions.

use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::PathBuf;

use cr_core::session::SessionManager;
use cr_core::types::SessionId;
use cr_storage::FileSystemStorage;

/// Session subcommands
#[derive(Debug, Subcommand)]
pub enum SessionCommand {
    /// List all sessions
    List {
        /// Show detailed information
        #[arg(long)]
        detailed: bool,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Limit number of sessions
        #[arg(long, short, default_value = "10")]
        limit: usize,
    },

    /// Show session details
    Show {
        /// Session ID
        id: String,

        /// Output as JSON
        #[arg(long)]
        json: bool,
    },

    /// Delete a session
    Delete {
        /// Session ID
        id: String,

        /// Skip confirmation
        #[arg(long, short)]
        yes: bool,
    },

    /// Clean old sessions
    Clean {
        /// Delete sessions older than this many days
        #[arg(long, default_value = "30")]
        older_than: u64,

        /// Skip confirmation
        #[arg(long, short)]
        yes: bool,
    },
}

/// Execute the session command
pub fn execute(cmd: SessionCommand) -> Result<()> {
    let storage_path = PathBuf::from(".cr-helper/sessions");

    if !storage_path.exists() {
        use colored::Colorize;
        eprintln!(
            "{} No sessions directory found. Run '{}' first.",
            "⚠".yellow(),
            "cr-helper init".cyan()
        );
        return Ok(());
    }

    let storage = FileSystemStorage::new(&storage_path)?;
    let manager = SessionManager::new(storage);

    match cmd {
        SessionCommand::List {
            detailed,
            json,
            limit,
        } => list_sessions(manager, detailed, json, limit),
        SessionCommand::Show { id, json } => show_session(manager, &id, json),
        SessionCommand::Delete { id, yes } => delete_session(manager, &id, yes),
        SessionCommand::Clean { older_than, yes } => clean_sessions(manager, older_than, yes),
    }
}

fn list_sessions(
    manager: SessionManager,
    detailed: bool,
    as_json: bool,
    limit: usize,
) -> Result<()> {
    use colored::Colorize;

    let sessions = manager.list()?;

    if sessions.is_empty() {
        println!("No sessions found.");
        return Ok(());
    }

    let sessions: Vec<_> = sessions.into_iter().take(limit).collect();

    if as_json {
        let json = serde_json::to_string_pretty(&sessions)?;
        println!("{}", json);
        return Ok(());
    }

    println!("{}", "Sessions:".bold().underline());
    println!();

    for info in &sessions {
        if detailed {
            println!("  {}", info.id.to_string().green());
            println!("    Files: {}", info.file_count);
            println!("    Comments: {}", info.comment_count);
            if let Some(name) = &info.metadata.name {
                println!("    Name: {}", name);
            }
            if !info.metadata.tags.is_empty() {
                println!("    Tags: {}", info.metadata.tags.join(", "));
            }
            println!(
                "    Created: {}",
                info.created_at.format("%Y-%m-%d %H:%M:%S")
            );
            println!(
                "    Updated: {}",
                info.updated_at.format("%Y-%m-%d %H:%M:%S")
            );
            println!();
        } else {
            let age = chrono::Utc::now()
                .signed_duration_since(info.updated_at)
                .num_hours();
            let age_str = if age < 1 {
                "just now".to_string()
            } else if age < 24 {
                format!("{}h ago", age)
            } else {
                format!("{}d ago", age / 24)
            };

            println!(
                "  {} {} files, {} comments ({})",
                info.id.to_string().green(),
                info.file_count.to_string().cyan(),
                info.comment_count.to_string().yellow(),
                age_str.dimmed()
            );
        }
    }

    let total = manager.count()?;
    if total > limit {
        println!(
            "\n  {} Showing {} of {} sessions. Use --limit to show more.",
            "ℹ".blue(),
            limit,
            total
        );
    }

    Ok(())
}

fn show_session(manager: SessionManager, id: &str, as_json: bool) -> Result<()> {
    use colored::Colorize;

    let session_id = SessionId::from_string(id)
        .context(format!("Invalid session ID: {}", id))?;
    let session = manager
        .load(&session_id)
        .context(format!("Session '{}' not found", id))?;

    if as_json {
        let json = serde_json::to_string_pretty(&session)?;
        println!("{}", json);
        return Ok(());
    }

    println!("{}", "Session Details".bold().underline());
    println!();
    println!("  ID: {}", session.id.to_string().green());
    if let Some(name) = &session.metadata.name {
        println!("  Name: {}", name);
    }
    if let Some(desc) = &session.metadata.description {
        println!("  Description: {}", desc);
    }
    if !session.metadata.tags.is_empty() {
        println!("  Tags: {}", session.metadata.tags.join(", ").cyan());
    }
    println!(
        "  Created: {}",
        session.created_at.format("%Y-%m-%d %H:%M:%S")
    );
    println!(
        "  Updated: {}",
        session.updated_at.format("%Y-%m-%d %H:%M:%S")
    );

    println!();
    println!("{}", "Diff Statistics".bold());
    println!(
        "  Files: {}",
        session.diff_data.files.len().to_string().cyan()
    );
    println!(
        "  Additions: {}",
        session.diff_data.stats.insertions.to_string().green()
    );
    println!(
        "  Deletions: {}",
        session.diff_data.stats.deletions.to_string().red()
    );

    println!();
    println!("{}", "Comments".bold());
    println!(
        "  Total: {}",
        session.comments.count().to_string().yellow()
    );

    // Count by severity
    let mut critical = 0;
    let mut warning = 0;
    let mut info = 0;
    for comment in session.comments.all_sorted() {
        match comment.severity {
            cr_core::comment::Severity::Critical => critical += 1,
            cr_core::comment::Severity::Warning => warning += 1,
            cr_core::comment::Severity::Info => info += 1,
        }
    }
    if critical > 0 {
        println!("  Critical: {}", critical.to_string().red());
    }
    if warning > 0 {
        println!("  Warning: {}", warning.to_string().yellow());
    }
    if info > 0 {
        println!("  Info: {}", info.to_string().blue());
    }

    if !session.diff_data.files.is_empty() {
        println!();
        println!("{}", "Files".bold());
        for file in &session.diff_data.files {
            let mode_char = match file.mode {
                cr_core::diff::FileMode::Added => "+".green(),
                cr_core::diff::FileMode::Deleted => "-".red(),
                cr_core::diff::FileMode::Modified => "~".yellow(),
                cr_core::diff::FileMode::Renamed => ">".cyan(),
                cr_core::diff::FileMode::Copied => "C".blue(),
                cr_core::diff::FileMode::Binary => "B".magenta(),
            };
            let path = file.display_path().to_string_lossy();
            println!("  {} {}", mode_char, path);
        }
    }

    Ok(())
}

fn delete_session(manager: SessionManager, id: &str, yes: bool) -> Result<()> {
    use colored::Colorize;

    // Check if exists
    let session_id = SessionId::from_string(id)
        .context(format!("Invalid session ID: {}", id))?;
    let session = manager
        .load(&session_id)
        .context(format!("Session '{}' not found", id))?;

    if !yes {
        use dialoguer::Confirm;

        println!("Session: {}", id.green());
        println!(
            "  {} files, {} comments",
            session.diff_data.files.len(),
            session.comments.count()
        );

        let confirmed = Confirm::new()
            .with_prompt("Delete this session?")
            .default(false)
            .interact()?;

        if !confirmed {
            println!("Deletion cancelled.");
            return Ok(());
        }
    }

    manager.delete(&session_id)?;
    println!("{} Session '{}' deleted.", "✓".green(), id);

    Ok(())
}

fn clean_sessions(manager: SessionManager, older_than_days: u64, yes: bool) -> Result<()> {
    use colored::Colorize;

    let cutoff = chrono::Utc::now() - chrono::Duration::days(older_than_days as i64);

    let sessions = manager.list()?;
    let old_sessions: Vec<_> = sessions
        .iter()
        .filter(|s| s.updated_at < cutoff)
        .collect();

    if old_sessions.is_empty() {
        println!(
            "No sessions older than {} days found.",
            older_than_days
        );
        return Ok(());
    }

    println!(
        "Found {} sessions older than {} days:",
        old_sessions.len().to_string().yellow(),
        older_than_days
    );
    for session in &old_sessions {
        let age = chrono::Utc::now()
            .signed_duration_since(session.updated_at)
            .num_days();
        println!(
            "  {} ({} days old)",
            session.id.to_string().dimmed(),
            age
        );
    }

    if !yes {
        use dialoguer::Confirm;

        let confirmed = Confirm::new()
            .with_prompt(format!("Delete {} sessions?", old_sessions.len()))
            .default(false)
            .interact()?;

        if !confirmed {
            println!("Cleanup cancelled.");
            return Ok(());
        }
    }

    let mut deleted = 0;
    for session in old_sessions {
        if manager.delete(&session.id).is_ok() {
            deleted += 1;
        }
    }

    println!(
        "{} Deleted {} sessions.",
        "✓".green(),
        deleted
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_command_list() {
        // Just verify the enum can be constructed
        let _cmd = SessionCommand::List {
            detailed: false,
            json: false,
            limit: 10,
        };
    }

    #[test]
    fn test_session_command_show() {
        let _cmd = SessionCommand::Show {
            id: "test".to_string(),
            json: false,
        };
    }
}
