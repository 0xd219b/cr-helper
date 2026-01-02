//! CLI commands module
//!
//! This module contains all CLI command implementations.

pub mod config;
pub mod doctor;
pub mod export;
pub mod init;
pub mod install;
pub mod review;
pub mod session;
pub mod uninstall;

use clap::{Parser, Subcommand};

/// cr-helper - Code Review Helper for Claude Code
#[derive(Debug, Parser)]
#[command(name = "cr-helper")]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Configuration file path
    #[arg(short, long, global = true)]
    pub config: Option<std::path::PathBuf>,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Start a code review session
    Review(review::ReviewArgs),

    /// Export review session
    Export(export::ExportArgs),

    /// Initialize cr-helper in current project
    Init(init::InitArgs),

    /// Install cr-helper to Agent CLI (Claude Code, etc.)
    Install(install::InstallArgs),

    /// Uninstall cr-helper from Agent CLI
    Uninstall(uninstall::UninstallArgs),

    /// Diagnose installation and configuration
    Doctor(doctor::DoctorArgs),

    /// Manage configuration
    #[command(subcommand)]
    Config(config::ConfigCommand),

    /// Manage review sessions
    #[command(subcommand)]
    Session(session::SessionCommand),
}

/// Run the CLI application
pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Set up logging based on verbosity
    setup_logging(cli.verbose);

    // Handle color output
    if cli.no_color {
        colored::control::set_override(false);
    }

    // Dispatch to command handler
    match cli.command {
        Commands::Review(args) => review::execute(args),
        Commands::Export(args) => export::execute(args),
        Commands::Init(args) => init::execute(args),
        Commands::Install(args) => install::execute(args),
        Commands::Uninstall(args) => uninstall::execute(args),
        Commands::Doctor(args) => doctor::execute(args),
        Commands::Config(cmd) => config::execute(cmd),
        Commands::Session(cmd) => session::execute(cmd),
    }
}

fn setup_logging(verbosity: u8) {
    use tracing_subscriber::EnvFilter;

    let filter = match verbosity {
        0 => EnvFilter::new("warn"),
        1 => EnvFilter::new("info"),
        2 => EnvFilter::new("debug"),
        _ => EnvFilter::new("trace"),
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn test_cli_parse() {
        Cli::command().debug_assert();
    }

    #[test]
    fn test_help_text() {
        let cmd = Cli::command();
        assert!(cmd.get_about().is_some());
    }
}
