//! Doctor command
//!
//! Diagnose installation and configuration.

use anyhow::Result;
use clap::Args;
use std::path::PathBuf;
use std::process::Command;

/// Arguments for the doctor command
#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Check Claude Code integration
    #[arg(long)]
    pub claude_code: bool,

    /// Check project configuration
    #[arg(long)]
    pub project: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

/// Check result
struct CheckResult {
    name: String,
    passed: bool,
    message: String,
    suggestion: Option<String>,
}

impl CheckResult {
    fn ok(name: &str, message: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: true,
            message: message.to_string(),
            suggestion: None,
        }
    }

    fn fail(name: &str, message: &str, suggestion: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            passed: false,
            message: message.to_string(),
            suggestion: suggestion.map(|s| s.to_string()),
        }
    }

    fn warn(name: &str, message: &str, suggestion: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            passed: true,
            message: format!("⚠ {}", message),
            suggestion: suggestion.map(|s| s.to_string()),
        }
    }
}

/// Execute the doctor command
pub fn execute(args: DoctorArgs) -> Result<()> {
    use colored::Colorize;

    let mut results = Vec::new();
    let mut warnings = Vec::new();
    let mut errors = Vec::new();

    // System checks
    println!("\n{}", "1. System Environment".bold().underline());
    results.extend(check_system_environment());

    // Project checks
    if args.project || !args.claude_code {
        println!("\n{}", "2. Project Configuration".bold().underline());
        results.extend(check_project_configuration());
    }

    // Claude Code checks
    if args.claude_code || !args.project {
        println!("\n{}", "3. Claude Code Integration".bold().underline());
        results.extend(check_claude_code_integration());
    }

    // Print results
    for result in &results {
        let status = if result.passed {
            if result.message.starts_with('⚠') {
                "⚠".yellow()
            } else {
                "✓".green()
            }
        } else {
            "✗".red()
        };

        println!("   {} {}: {}", status, result.name, result.message);

        if args.verbose {
            if let Some(suggestion) = &result.suggestion {
                println!("     {}", suggestion.dimmed());
            }
        }

        if !result.passed {
            errors.push(result);
        } else if result.message.starts_with('⚠') {
            warnings.push(result);
        }
    }

    // Summary
    println!(
        "\n{}: {} warnings, {} errors",
        "Summary".bold(),
        warnings.len().to_string().yellow(),
        errors.len().to_string().red()
    );

    if !warnings.is_empty() {
        println!("\n{}", "⚠ Warnings:".yellow());
        for result in &warnings {
            println!("  - {}", result.name);
            if let Some(suggestion) = &result.suggestion {
                println!("    {}", suggestion.dimmed());
            }
        }
    }

    if !errors.is_empty() {
        println!("\n{}", "✗ Errors:".red());
        for result in &errors {
            println!("  - {}: {}", result.name, result.message);
            if let Some(suggestion) = &result.suggestion {
                println!("    Fix: {}", suggestion);
            }
        }
    }

    if errors.is_empty() && warnings.is_empty() {
        println!("\n{} All checks passed!", "✓".green());
    }

    Ok(())
}

fn check_system_environment() -> Vec<CheckResult> {
    let mut results = Vec::new();

    // cr-helper version
    results.push(CheckResult::ok(
        "cr-helper version",
        env!("CARGO_PKG_VERSION"),
    ));

    // Git version
    match Command::new("git").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            let version = version.trim().replace("git version ", "");
            results.push(CheckResult::ok("Git version", &version));
        }
        _ => {
            results.push(CheckResult::fail(
                "Git",
                "not found",
                Some("Install git: https://git-scm.com/"),
            ));
        }
    }

    // Delta (optional)
    match Command::new("delta").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            let version = version.trim().split_whitespace().nth(1).unwrap_or("unknown");
            results.push(CheckResult::ok("Delta", version));
        }
        _ => {
            results.push(CheckResult::warn(
                "Delta",
                "not installed (optional)",
                Some("Install delta for better diffs: https://github.com/dandavison/delta"),
            ));
        }
    }

    results
}

fn check_project_configuration() -> Vec<CheckResult> {
    let mut results = Vec::new();

    // Git repository
    let is_git_repo = PathBuf::from(".git").exists();
    if is_git_repo {
        results.push(CheckResult::ok("Git repository", "detected"));
    } else {
        results.push(CheckResult::warn(
            "Git repository",
            "not detected",
            Some("Run 'git init' to initialize"),
        ));
    }

    // .cr-helper directory
    let cr_helper_dir = PathBuf::from(".cr-helper");
    if cr_helper_dir.exists() {
        results.push(CheckResult::ok(".cr-helper/", "exists"));

        // config.toml
        let config_path = cr_helper_dir.join("config.toml");
        if config_path.exists() {
            match std::fs::read_to_string(&config_path) {
                Ok(content) => {
                    match toml::from_str::<toml::Value>(&content) {
                        Ok(_) => results.push(CheckResult::ok("config.toml", "valid")),
                        Err(e) => results.push(CheckResult::fail(
                            "config.toml",
                            &format!("invalid TOML: {}", e),
                            Some("Fix syntax errors in .cr-helper/config.toml"),
                        )),
                    }
                }
                Err(_) => results.push(CheckResult::fail(
                    "config.toml",
                    "cannot read",
                    None,
                )),
            }
        } else {
            results.push(CheckResult::warn(
                "config.toml",
                "not found",
                Some("Run 'cr-helper init' to create"),
            ));
        }

        // sessions directory
        let sessions_dir = cr_helper_dir.join("sessions");
        if sessions_dir.exists() {
            results.push(CheckResult::ok("sessions/", "exists"));
        } else {
            results.push(CheckResult::warn(
                "sessions/",
                "not found",
                Some("Will be created on first review"),
            ));
        }
    } else {
        results.push(CheckResult::warn(
            ".cr-helper/",
            "not found",
            Some("Run 'cr-helper init' to initialize"),
        ));
    }

    results
}

fn check_claude_code_integration() -> Vec<CheckResult> {
    let mut results = Vec::new();

    let project_claude = PathBuf::from(".claude");
    let home_claude = dirs::home_dir()
        .map(|h| h.join(".claude"))
        .unwrap_or_else(|| PathBuf::from("~/.claude"));

    // Check project-level
    if project_claude.exists() {
        results.push(CheckResult::ok("Claude Code (project)", ".claude/ exists"));

        // Check skill
        let skill_dir = project_claude.join("skills/cr-helper");
        if skill_dir.exists() {
            let skill_md = skill_dir.join("SKILL.md");
            if skill_md.exists() {
                results.push(CheckResult::ok("Skill", "installed"));
            } else {
                results.push(CheckResult::fail(
                    "Skill",
                    "SKILL.md missing",
                    Some("Run 'cr-helper install --claude-code'"),
                ));
            }
        } else {
            results.push(CheckResult::warn(
                "Skill",
                "not installed",
                Some("Run 'cr-helper install --claude-code'"),
            ));
        }

        // Check hooks
        let hooks_dir = project_claude.join("hooks");
        let stop_hook = hooks_dir.join("cr-helper-stop.sh");
        if stop_hook.exists() {
            // Check executable
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                match std::fs::metadata(&stop_hook) {
                    Ok(meta) => {
                        let mode = meta.permissions().mode();
                        if mode & 0o111 != 0 {
                            results.push(CheckResult::ok("Hooks", "installed (executable)"));
                        } else {
                            results.push(CheckResult::fail(
                                "Hooks",
                                "not executable",
                                Some("Run: chmod +x .claude/hooks/cr-helper-*.sh"),
                            ));
                        }
                    }
                    Err(_) => {
                        results.push(CheckResult::warn("Hooks", "cannot check permissions", None));
                    }
                }
            }
            #[cfg(not(unix))]
            {
                results.push(CheckResult::ok("Hooks", "installed"));
            }
        } else {
            results.push(CheckResult::warn(
                "Hooks",
                "not installed",
                Some("Run 'cr-helper install --claude-code'"),
            ));
        }

        // Check settings.json
        let settings_path = project_claude.join("settings.json");
        if settings_path.exists() {
            match std::fs::read_to_string(&settings_path) {
                Ok(content) => {
                    match serde_json::from_str::<serde_json::Value>(&content) {
                        Ok(settings) => {
                            if settings.get("cr-helper").is_some() {
                                results.push(CheckResult::ok("settings.json", "cr-helper configured"));
                            } else {
                                results.push(CheckResult::warn(
                                    "settings.json",
                                    "cr-helper not configured",
                                    Some("Run 'cr-helper install --claude-code'"),
                                ));
                            }
                        }
                        Err(e) => {
                            results.push(CheckResult::fail(
                                "settings.json",
                                &format!("invalid JSON: {}", e),
                                Some("Fix syntax errors in .claude/settings.json"),
                            ));
                        }
                    }
                }
                Err(_) => {
                    results.push(CheckResult::fail("settings.json", "cannot read", None));
                }
            }
        } else {
            results.push(CheckResult::warn(
                "settings.json",
                "not found",
                Some("Run 'cr-helper install --claude-code'"),
            ));
        }
    } else {
        results.push(CheckResult::warn(
            "Claude Code (project)",
            ".claude/ not found",
            Some("Create .claude/ directory or run in a Claude Code project"),
        ));
    }

    // Check global
    if home_claude.exists() {
        results.push(CheckResult::ok(
            "Claude Code (global)",
            &format!("{} exists", home_claude.display()),
        ));
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_result_ok() {
        let result = CheckResult::ok("test", "message");
        assert!(result.passed);
        assert!(result.suggestion.is_none());
    }

    #[test]
    fn test_check_result_fail() {
        let result = CheckResult::fail("test", "error", Some("fix it"));
        assert!(!result.passed);
        assert!(result.suggestion.is_some());
    }
}
