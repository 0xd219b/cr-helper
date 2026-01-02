//! Install command
//!
//! Install cr-helper to Agent CLI tools (Claude Code, etc.)

use anyhow::Result;
use clap::{Args, ValueEnum};
use std::fs;
use std::path::{Path, PathBuf};

/// Installation scope
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
pub enum InstallScope {
    /// Project-level configuration (.claude/settings.json)
    Project,
    /// Local configuration (.claude/settings.local.json)
    Local,
    /// Global configuration (~/.claude/settings.json)
    Global,
}

/// Components to install
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq)]
pub enum Component {
    /// Skill definition
    Skill,
    /// Hook scripts
    Hooks,
    /// MCP Server (v2.0)
    Mcp,
    /// All components
    All,
}

/// Arguments for the install command
#[derive(Debug, Args)]
pub struct InstallArgs {
    /// Install to Claude Code
    #[arg(long)]
    pub claude_code: bool,

    /// Installation scope
    #[arg(long, value_enum, default_value = "project")]
    pub scope: InstallScope,

    /// Components to install
    #[arg(long, value_enum, value_delimiter = ',', default_value = "all")]
    pub components: Vec<Component>,

    /// Skip confirmation prompts
    #[arg(short, long)]
    pub yes: bool,

    /// Dry run (don't actually install)
    #[arg(long)]
    pub dry_run: bool,

    /// Force overwrite existing configuration
    #[arg(long)]
    pub force: bool,

    /// Don't backup existing configuration
    #[arg(long)]
    pub no_backup: bool,

    /// Enable auto-review on stop
    #[arg(long)]
    pub auto_review: Option<bool>,

    /// Minimum changes for review
    #[arg(long)]
    pub min_changes: Option<usize>,
}

/// Execute the install command
pub fn execute(args: InstallArgs) -> Result<()> {
    use colored::Colorize;

    if !args.claude_code {
        println!("{}", "Please specify an agent to install to:".yellow());
        println!("  --claude-code    Install to Claude Code");
        return Ok(());
    }

    println!("{} Installing cr-helper to Claude Code...", "🚀".to_string());

    // Detect environment
    let project_claude_dir = PathBuf::from(".claude");
    let home_claude_dir = dirs::home_dir()
        .map(|h| h.join(".claude"))
        .unwrap_or_else(|| PathBuf::from("~/.claude"));

    let has_project = project_claude_dir.exists();
    let has_global = home_claude_dir.exists();

    if !has_project && !has_global {
        eprintln!(
            "{} Claude Code not detected. Create .claude/ directory first.",
            "✗".red()
        );
        eprintln!("  Run: mkdir -p .claude");
        return Ok(());
    }

    // Determine installation path
    let (settings_path, scope_name) = match args.scope {
        InstallScope::Project => (project_claude_dir.join("settings.json"), "project"),
        InstallScope::Local => (project_claude_dir.join("settings.local.json"), "local"),
        InstallScope::Global => (home_claude_dir.join("settings.json"), "global"),
    };

    println!("{} Installing to {} scope", "✓".green(), scope_name.cyan());

    // Determine components
    let install_skill = args.components.contains(&Component::All)
        || args.components.contains(&Component::Skill);
    let install_hooks = args.components.contains(&Component::All)
        || args.components.contains(&Component::Hooks);
    let install_mcp = args.components.contains(&Component::Mcp);

    // Confirm if not --yes
    if !args.yes && !args.dry_run {
        use dialoguer::Confirm;

        let confirmed = Confirm::new()
            .with_prompt("Proceed with installation?")
            .default(true)
            .interact()?;

        if !confirmed {
            println!("Installation cancelled.");
            return Ok(());
        }
    }

    if args.dry_run {
        println!("\n{} Dry run - no changes will be made", "📋".to_string());
        println!("Would install:");
        if install_skill {
            println!("  - Skill to .claude/skills/cr-helper/");
        }
        if install_hooks {
            println!("  - Hooks to .claude/hooks/");
        }
        if install_mcp {
            println!("  - MCP Server configuration");
        }
        println!("  - Settings to {}", settings_path.display());
        return Ok(());
    }

    // Backup existing settings
    if settings_path.exists() && !args.no_backup {
        let backup_path = format!(
            "{}.backup-{}",
            settings_path.display(),
            chrono::Local::now().format("%Y%m%d-%H%M%S")
        );
        fs::copy(&settings_path, &backup_path)?;
        println!(
            "{} Backed up existing settings to {}",
            "✓".green(),
            backup_path
        );
    }

    // Install components
    let base_dir = match args.scope {
        InstallScope::Global => &home_claude_dir,
        _ => &project_claude_dir,
    };

    if install_skill {
        install_skill_component(base_dir)?;
        println!("{} Installed Skill to .claude/skills/cr-helper/", "✓".green());
    }

    if install_hooks {
        install_hooks_component(base_dir)?;
        println!("{} Installed Hooks to .claude/hooks/", "✓".green());
    }

    // Merge settings
    merge_settings(
        &settings_path,
        install_skill,
        install_hooks,
        install_mcp,
        args.auto_review.unwrap_or(true),
        args.min_changes.unwrap_or(3),
    )?;
    println!("{} Updated {}", "✓".green(), settings_path.display());

    // Print summary
    println!("\n{} Installation complete!", "✅".to_string());
    println!("\n{}", "Next steps:".bold());
    println!("  1. Test the integration:");
    println!("     ");
    println!("     {}", "claude".cyan());
    println!("     ");
    println!("  2. Run a manual review:");
    println!("     ");
    println!("     {}", "cr-helper review".cyan());
    println!(
        "\n{} Tip: Run '{}' to verify the installation",
        "💡".to_string(),
        "cr-helper doctor --claude-code".cyan()
    );

    Ok(())
}

fn install_skill_component(base_dir: &Path) -> Result<()> {
    let skill_dir = base_dir.join("skills/cr-helper");
    fs::create_dir_all(&skill_dir)?;

    // SKILL.md
    let skill_md = include_str!("../templates/SKILL.md");
    fs::write(skill_dir.join("SKILL.md"), skill_md)?;

    // scripts/
    let scripts_dir = skill_dir.join("scripts");
    fs::create_dir_all(&scripts_dir)?;

    let parse_script = include_str!("../templates/parse-review.py");
    fs::write(scripts_dir.join("parse-review.py"), parse_script)?;

    Ok(())
}

fn install_hooks_component(base_dir: &Path) -> Result<()> {
    let hooks_dir = base_dir.join("hooks");
    fs::create_dir_all(&hooks_dir)?;

    // Hook scripts
    let hooks = [
        ("cr-helper-session-start.sh", include_str!("../templates/hooks/session-start.sh")),
        ("cr-helper-stop.sh", include_str!("../templates/hooks/stop.sh")),
    ];

    for (name, content) in hooks {
        let path = hooks_dir.join(name);
        fs::write(&path, content)?;

        // Set executable permission on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&path, perms)?;
        }
    }

    Ok(())
}

fn merge_settings(
    settings_path: &Path,
    _skill: bool,
    hooks: bool,
    _mcp: bool,
    auto_review: bool,
    min_changes: usize,
) -> Result<()> {
    // Load existing settings
    let mut settings: serde_json::Value = if settings_path.exists() {
        let content = fs::read_to_string(settings_path)?;
        serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // Add cr-helper configuration
    settings["cr-helper"] = serde_json::json!({
        "auto_review_on_stop": auto_review,
        "min_changes_for_review": min_changes,
        "block_on_critical": true,
        "output_dir": ".claude/cr-helper"
    });

    // Add hooks configuration
    if hooks {
        if settings["hooks"].is_null() {
            settings["hooks"] = serde_json::json!({});
        }

        // Stop hook
        if settings["hooks"]["Stop"].is_null() {
            settings["hooks"]["Stop"] = serde_json::json!([]);
        }
        let stop_hooks = settings["hooks"]["Stop"].as_array_mut().unwrap();
        let cr_helper_hook = serde_json::json!({
            "matcher": "",
            "hooks": [
                {
                    "type": "command",
                    "command": ".claude/hooks/cr-helper-stop.sh"
                }
            ]
        });
        if !stop_hooks.iter().any(|h| {
            h.get("hooks")
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
        }) {
            stop_hooks.push(cr_helper_hook);
        }
    }

    // Write settings
    let parent = settings_path.parent().unwrap();
    fs::create_dir_all(parent)?;
    let content = serde_json::to_string_pretty(&settings)?;
    fs::write(settings_path, content)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_scope_values() {
        assert!(InstallScope::from_str("project", true).is_ok());
        assert!(InstallScope::from_str("local", true).is_ok());
        assert!(InstallScope::from_str("global", true).is_ok());
    }

    #[test]
    fn test_component_values() {
        assert!(Component::from_str("skill", true).is_ok());
        assert!(Component::from_str("hooks", true).is_ok());
        assert!(Component::from_str("mcp", true).is_ok());
        assert!(Component::from_str("all", true).is_ok());
    }
}
