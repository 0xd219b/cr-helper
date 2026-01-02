//! Review command
//!
//! Start a code review session with TUI interface.

use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;

use cr_core::diff::DiffParser;
use cr_core::session::{DiffSource, SessionManager};
use cr_core::types::SessionId;
use cr_storage::FileSystemStorage;

/// Arguments for the review command
#[derive(Debug, Args)]
pub struct ReviewArgs {
    /// Git diff arguments (passed directly to git diff)
    #[arg(trailing_var_arg = true)]
    pub git_args: Vec<String>,

    /// Review staged changes
    #[arg(long)]
    pub staged: bool,

    /// Review specific commit
    #[arg(long)]
    pub commit: Option<String>,

    /// Include untracked (new) files in the review
    #[arg(long, short = 'u')]
    pub untracked: bool,

    /// Resume an existing session
    #[arg(long, short)]
    pub session: Option<String>,

    /// Output directory for session data
    #[arg(long, short)]
    pub output: Option<PathBuf>,

    /// Don't start TUI, just create session
    #[arg(long)]
    pub no_tui: bool,
}

/// Execute the review command
pub fn execute(args: ReviewArgs) -> Result<()> {
    use colored::Colorize;

    println!("{}", "Starting code review...".cyan());

    // Determine diff source
    let diff_source = determine_diff_source(&args)?;
    tracing::info!("Diff source: {:?}", diff_source);

    // Set up storage
    let storage_path = args
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from(".cr-helper/sessions"));
    let storage = FileSystemStorage::new(&storage_path)?;
    let mut manager = SessionManager::new(storage);

    // Create or resume session
    let session = if let Some(session_id) = args.session {
        println!("Resuming session: {}", session_id.yellow());
        let id = SessionId::from_string(&session_id)
            .context(format!("Invalid session ID: {}", session_id))?;
        manager
            .load(&id)
            .context(format!("Session '{}' not found", session_id))?
    } else {
        println!("Creating new session...");
        if args.untracked {
            println!("{}", "Including untracked files...".dimmed());
        }
        create_new_session(&diff_source, &mut manager, args.untracked)?
    };

    let session_id = session.id.clone();
    println!("Session ID: {}", session_id.to_string().green());

    // Start TUI or just print info
    if args.no_tui {
        print_session_info(&session);
        Ok(())
    } else {
        // Run TUI
        run_tui(session, manager)
    }
}

fn determine_diff_source(args: &ReviewArgs) -> Result<DiffSource> {
    if args.staged {
        Ok(DiffSource::Staged)
    } else if let Some(commit) = &args.commit {
        Ok(DiffSource::Commit {
            commit: commit.clone(),
        })
    } else if !args.git_args.is_empty() {
        Ok(DiffSource::Custom {
            args: args.git_args.clone(),
        })
    } else {
        Ok(DiffSource::WorkingTree)
    }
}

fn create_new_session(
    source: &DiffSource,
    manager: &mut SessionManager,
    include_untracked: bool,
) -> Result<cr_core::session::Session> {
    use colored::Colorize;
    use cr_core::diff::DiffSource as ParserDiffSource;

    // Convert session DiffSource to parser DiffSource
    let parser_source = match source {
        DiffSource::WorkingTree => ParserDiffSource::WorkingTree,
        DiffSource::Staged => ParserDiffSource::Staged,
        DiffSource::Commit { commit } => ParserDiffSource::Commit {
            commit: commit.clone(),
        },
        DiffSource::CommitRange { from, to } => ParserDiffSource::CommitRange {
            from: from.clone(),
            to: to.clone(),
        },
        DiffSource::Branch { branch } => ParserDiffSource::Branch {
            branch: branch.clone(),
        },
        DiffSource::PullRequest { base, .. } => ParserDiffSource::CommitRange {
            from: base.clone(),
            to: "HEAD".to_string(),
        },
        DiffSource::Custom { args } => ParserDiffSource::Custom { args: args.clone() },
    };

    // Parse diff using DiffParser with untracked option
    let parser = DiffParser::new();
    let diff_data = parser.parse_from_git_with_options(&parser_source, include_untracked)?;

    if diff_data.files.is_empty() {
        println!("{}", "No changes detected.".yellow());
        anyhow::bail!("No changes to review");
    }

    println!(
        "Found {} files with {} additions and {} deletions",
        diff_data.stats.files_changed.to_string().cyan(),
        diff_data.stats.insertions.to_string().green(),
        diff_data.stats.deletions.to_string().red()
    );

    // Create session
    let session = manager.create(source.clone(), diff_data)?;

    Ok(session)
}

fn print_session_info(session: &cr_core::session::Session) {
    use colored::Colorize;

    println!("\n{}", "Session Information".bold().underline());
    println!("  ID: {}", session.id.to_string().green());
    println!(
        "  Files: {}",
        session.diff_data.files.len().to_string().cyan()
    );
    println!(
        "  Comments: {}",
        session.comments.count().to_string().yellow()
    );
    println!("  Created: {}", session.created_at);
    println!("  Updated: {}", session.updated_at);

    if !session.diff_data.files.is_empty() {
        println!("\n{}", "Files:".bold());
        for file in &session.diff_data.files {
            let status = match file.mode {
                cr_core::diff::FileMode::Added => "+".green(),
                cr_core::diff::FileMode::Deleted => "-".red(),
                cr_core::diff::FileMode::Modified => "~".yellow(),
                cr_core::diff::FileMode::Renamed => ">".cyan(),
                cr_core::diff::FileMode::Copied => "C".blue(),
                cr_core::diff::FileMode::Binary => "B".magenta(),
            };
            let path = file.display_path().to_string_lossy();
            println!("  {} {}", status, path);
        }
    }
}

fn run_tui(session: cr_core::session::Session, mut manager: SessionManager) -> Result<()> {
    use cr_ui::App;

    let mut app = App::new(session)?;
    app.run()?;

    // Save session after TUI exits
    let mut session = app.get_session();
    manager.save(&mut session)?;

    println!("Session saved: {}", session.id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_determine_diff_source_staged() {
        let args = ReviewArgs {
            git_args: vec![],
            staged: true,
            commit: None,
            untracked: false,
            session: None,
            output: None,
            no_tui: false,
        };
        let source = determine_diff_source(&args).unwrap();
        assert!(matches!(source, DiffSource::Staged));
    }

    #[test]
    fn test_determine_diff_source_commit() {
        let args = ReviewArgs {
            git_args: vec![],
            staged: false,
            commit: Some("abc123".to_string()),
            untracked: false,
            session: None,
            output: None,
            no_tui: false,
        };
        let source = determine_diff_source(&args).unwrap();
        assert!(matches!(source, DiffSource::Commit { .. }));
    }

    #[test]
    fn test_determine_diff_source_working_tree() {
        let args = ReviewArgs {
            git_args: vec![],
            staged: false,
            commit: None,
            untracked: false,
            session: None,
            output: None,
            no_tui: false,
        };
        let source = determine_diff_source(&args).unwrap();
        assert!(matches!(source, DiffSource::WorkingTree));
    }
}
