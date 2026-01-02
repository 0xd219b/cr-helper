//! Config command
//!
//! Manage cr-helper configuration.

use anyhow::{Context, Result};
use clap::Subcommand;
use std::fs;
use std::path::PathBuf;

/// Config subcommands
#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Show current configuration
    Show {
        /// Show as JSON
        #[arg(long)]
        json: bool,
    },

    /// Edit configuration in editor
    Edit,

    /// Reset to default configuration
    Reset {
        /// Force reset without confirmation
        #[arg(long)]
        force: bool,
    },

    /// Validate configuration
    Validate,

    /// List available templates
    Templates,
}

/// Execute the config command
pub fn execute(cmd: ConfigCommand) -> Result<()> {
    match cmd {
        ConfigCommand::Show { json } => show_config(json),
        ConfigCommand::Edit => edit_config(),
        ConfigCommand::Reset { force } => reset_config(force),
        ConfigCommand::Validate => validate_config(),
        ConfigCommand::Templates => list_templates(),
    }
}

fn get_config_path() -> PathBuf {
    PathBuf::from(".cr-helper/config.toml")
}

fn show_config(as_json: bool) -> Result<()> {
    use colored::Colorize;

    let config_path = get_config_path();

    if !config_path.exists() {
        eprintln!(
            "{} Configuration not found. Run '{}' to create.",
            "⚠".yellow(),
            "cr-helper init".cyan()
        );
        return Ok(());
    }

    let content = fs::read_to_string(&config_path)?;

    if as_json {
        let config: toml::Value = toml::from_str(&content)?;
        let json = serde_json::to_string_pretty(&config)?;
        println!("{}", json);
    } else {
        println!("{}", "Configuration:".bold().underline());
        println!("{}", config_path.display().to_string().dimmed());
        println!();
        println!("{}", content);
    }

    Ok(())
}

fn edit_config() -> Result<()> {
    use colored::Colorize;

    let config_path = get_config_path();

    if !config_path.exists() {
        eprintln!(
            "{} Configuration not found. Run '{}' to create.",
            "⚠".yellow(),
            "cr-helper init".cyan()
        );
        return Ok(());
    }

    // Get editor from environment
    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| {
            if cfg!(windows) {
                "notepad".to_string()
            } else {
                "vi".to_string()
            }
        });

    println!("Opening {} in {}...", config_path.display(), editor.cyan());

    let status = std::process::Command::new(&editor)
        .arg(&config_path)
        .status()
        .context(format!("Failed to open editor: {}", editor))?;

    if status.success() {
        // Validate after edit
        let content = fs::read_to_string(&config_path)?;
        match toml::from_str::<toml::Value>(&content) {
            Ok(_) => println!("{} Configuration saved and validated.", "✓".green()),
            Err(e) => eprintln!("{} Configuration has errors: {}", "✗".red(), e),
        }
    }

    Ok(())
}

fn reset_config(force: bool) -> Result<()> {
    use colored::Colorize;

    let config_path = get_config_path();

    if !force {
        use dialoguer::Confirm;

        let confirmed = Confirm::new()
            .with_prompt("Reset configuration to defaults?")
            .default(false)
            .interact()?;

        if !confirmed {
            println!("Reset cancelled.");
            return Ok(());
        }
    }

    // Backup existing
    if config_path.exists() {
        let backup_path = format!(
            "{}.backup-{}",
            config_path.display(),
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        );
        fs::copy(&config_path, &backup_path)?;
        println!("{} Backed up to {}", "✓".green(), backup_path);
    }

    // Write default config
    let default_config = r#"# cr-helper configuration

[review]
checks = ["security", "error-handling", "performance"]

[review.severity_thresholds]
critical = ["security"]
warning = ["error-handling"]
info = ["style"]

[export]
default_format = "markdown-enhanced"
include_code_context = true
context_lines = 3
include_suggestions = true

[diff]
include_patterns = ["*"]
exclude_patterns = [".git/", "node_modules/", "target/", "__pycache__/"]
"#;

    fs::create_dir_all(config_path.parent().unwrap())?;
    fs::write(&config_path, default_config)?;

    println!(
        "{} Configuration reset to defaults.",
        "✓".green()
    );

    Ok(())
}

fn validate_config() -> Result<()> {
    use colored::Colorize;

    let config_path = get_config_path();

    if !config_path.exists() {
        eprintln!(
            "{} Configuration not found at {}",
            "✗".red(),
            config_path.display()
        );
        return Ok(());
    }

    let content = fs::read_to_string(&config_path)?;

    match toml::from_str::<toml::Value>(&content) {
        Ok(config) => {
            println!("{} Configuration is valid TOML", "✓".green());

            // Check for expected sections
            let mut warnings = Vec::new();

            if config.get("review").is_none() {
                warnings.push("[review] section not found");
            }
            if config.get("export").is_none() {
                warnings.push("[export] section not found");
            }
            if config.get("diff").is_none() {
                warnings.push("[diff] section not found");
            }

            if warnings.is_empty() {
                println!("{} All expected sections present", "✓".green());
            } else {
                for warning in warnings {
                    println!("{} {}", "⚠".yellow(), warning);
                }
            }
        }
        Err(e) => {
            eprintln!("{} Invalid TOML: {}", "✗".red(), e);
        }
    }

    Ok(())
}

fn list_templates() -> Result<()> {
    use colored::Colorize;

    println!("{}", "Available templates:".bold().underline());
    println!();
    println!(
        "  {} - Rust project with unsafe-code and ownership checks",
        "rust".cyan()
    );
    println!(
        "  {} - TypeScript/JavaScript with type-safety checks",
        "typescript".cyan()
    );
    println!(
        "  {} - Python with type-hints and testing checks",
        "python".cyan()
    );
    println!(
        "  {} - Go with concurrency and error-handling checks",
        "go".cyan()
    );
    println!(
        "  {} - Generic template for any project",
        "generic".cyan()
    );
    println!();
    println!(
        "Use '{}' to apply a template",
        "cr-helper init --template <name>".cyan()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_config_path() {
        let path = get_config_path();
        assert!(path.ends_with("config.toml"));
    }
}
