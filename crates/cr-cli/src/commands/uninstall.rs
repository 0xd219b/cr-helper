//! Uninstall command
//!
//! Remove cr-helper from Agent CLI tools.

use anyhow::{Context, Result};
use clap::Args;
use std::fs;
use std::path::PathBuf;

use super::install::{Component, InstallScope};

/// Arguments for the uninstall command
#[derive(Debug, Args)]
pub struct UninstallArgs {
    /// Uninstall from Claude Code
    #[arg(long)]
    pub claude_code: bool,

    /// Uninstallation scope
    #[arg(long, value_enum, default_value = "project")]
    pub scope: InstallScope,

    /// Components to uninstall
    #[arg(long, value_enum, value_delimiter = ',', default_value = "all")]
    pub components: Vec<Component>,

    /// Skip confirmation prompts
    #[arg(short, long)]
    pub yes: bool,

    /// Keep backup files
    #[arg(long)]
    pub keep_backup: bool,
}

/// Execute the uninstall command
pub fn execute(args: UninstallArgs) -> Result<()> {
    use colored::Colorize;

    if !args.claude_code {
        println!("{}", "Please specify an agent to uninstall from:".yellow());
        println!("  --claude-code    Uninstall from Claude Code");
        return Ok(());
    }

    println!(
        "{} Detecting cr-helper installations...",
        "🔍".to_string()
    );

    // Detect environment
    let project_claude_dir = PathBuf::from(".claude");
    let home_claude_dir = dirs::home_dir()
        .map(|h| h.join(".claude"))
        .unwrap_or_else(|| PathBuf::from("~/.claude"));

    // Determine installation path
    let base_dir = match args.scope {
        InstallScope::Project | InstallScope::Local => &project_claude_dir,
        InstallScope::Global => &home_claude_dir,
    };

    let settings_path = match args.scope {
        InstallScope::Project => base_dir.join("settings.json"),
        InstallScope::Local => base_dir.join("settings.local.json"),
        InstallScope::Global => base_dir.join("settings.json"),
    };

    // Check what's installed
    let skill_dir = base_dir.join("skills/cr-helper");
    let hooks_dir = base_dir.join("hooks");

    let has_skill = skill_dir.exists();
    let has_hooks = hooks_dir.join("cr-helper-stop.sh").exists()
        || hooks_dir.join("cr-helper-session-start.sh").exists();

    if !has_skill && !has_hooks {
        println!("{} No cr-helper installation found.", "ℹ".blue());
        return Ok(());
    }

    println!("{} Found installations:", "✓".green());
    if has_skill {
        println!("  - Skill: {}", skill_dir.display());
    }
    if has_hooks {
        println!("  - Hooks: {}", hooks_dir.display());
    }

    // Confirm
    if !args.yes {
        use dialoguer::Confirm;

        println!("\n{} This will remove:", "⚠".yellow());
        if has_skill {
            println!("  - {}", skill_dir.display());
        }
        if has_hooks {
            println!("  - cr-helper-*.sh from {}", hooks_dir.display());
        }
        println!("  - cr-helper configuration from {}", settings_path.display());

        let confirmed = Confirm::new()
            .with_prompt("Proceed?")
            .default(false)
            .interact()?;

        if !confirmed {
            println!("Uninstallation cancelled.");
            return Ok(());
        }
    }

    // Backup settings
    if settings_path.exists() && !args.keep_backup {
        let backup_path = format!(
            "{}.backup-{}",
            settings_path.display(),
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        );
        fs::copy(&settings_path, &backup_path)?;
        println!(
            "{} Backed up {} to {}",
            "✓".green(),
            settings_path.display(),
            backup_path
        );
    }

    // Remove components
    let remove_skill = args.components.contains(&Component::All)
        || args.components.contains(&Component::Skill);
    let remove_hooks = args.components.contains(&Component::All)
        || args.components.contains(&Component::Hooks);

    if remove_skill && has_skill {
        fs::remove_dir_all(&skill_dir)?;
        println!("{} Removed {}", "✓".green(), skill_dir.display());
    }

    if remove_hooks && has_hooks {
        // Remove hook scripts
        for name in &["cr-helper-session-start.sh", "cr-helper-stop.sh"] {
            let path = hooks_dir.join(name);
            if path.exists() {
                fs::remove_file(&path)?;
            }
        }
        println!(
            "{} Removed cr-helper hooks from {}",
            "✓".green(),
            hooks_dir.display()
        );
    }

    // Clean settings
    if settings_path.exists() {
        clean_settings(&settings_path)?;
        println!(
            "{} Removed cr-helper configuration from {}",
            "✓".green(),
            settings_path.display()
        );
    }

    println!("\n{} Uninstallation complete!", "✅".to_string());

    if !args.keep_backup {
        println!(
            "\n{} Tip: Backup files are kept in {}",
            "💡".to_string(),
            base_dir.display()
        );
    }

    Ok(())
}

fn clean_settings(settings_path: &PathBuf) -> Result<()> {
    let content = fs::read_to_string(settings_path)?;
    let mut settings: serde_json::Value =
        serde_json::from_str(&content).context("Invalid JSON in settings file")?;

    // Remove cr-helper configuration
    if let Some(obj) = settings.as_object_mut() {
        obj.remove("cr-helper");
    }

    // Remove cr-helper hooks
    if let Some(hooks) = settings.get_mut("hooks") {
        if let Some(hooks_obj) = hooks.as_object_mut() {
            for (_event, hooks_array) in hooks_obj.iter_mut() {
                if let Some(arr) = hooks_array.as_array_mut() {
                    arr.retain(|hook| {
                        !hook
                            .get("hooks")
                            .and_then(|h| h.as_array())
                            .map(|a| {
                                a.iter().any(|i| {
                                    i.get("command")
                                        .and_then(|c| c.as_str())
                                        .map(|s| s.contains("cr-helper"))
                                        .unwrap_or(false)
                                })
                            })
                            .unwrap_or(false)
                    });
                }
            }
        }
    }

    // Write cleaned settings
    let content = serde_json::to_string_pretty(&settings)?;
    fs::write(settings_path, content)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_settings() {
        let temp = tempfile::tempdir().unwrap();
        let settings_path = temp.path().join("settings.json");

        let initial = serde_json::json!({
            "cr-helper": {
                "auto_review_on_stop": true
            },
            "other": "value",
            "hooks": {
                "Stop": [
                    {
                        "matcher": "",
                        "hooks": [
                            { "type": "command", "command": ".claude/hooks/cr-helper-stop.sh" }
                        ]
                    },
                    {
                        "matcher": "",
                        "hooks": [
                            { "type": "command", "command": "other-hook.sh" }
                        ]
                    }
                ]
            }
        });

        fs::write(&settings_path, serde_json::to_string(&initial).unwrap()).unwrap();
        clean_settings(&settings_path.to_path_buf()).unwrap();

        let content = fs::read_to_string(&settings_path).unwrap();
        let result: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert!(result.get("cr-helper").is_none());
        assert!(result.get("other").is_some());

        let stop_hooks = result["hooks"]["Stop"].as_array().unwrap();
        assert_eq!(stop_hooks.len(), 1);
    }
}
