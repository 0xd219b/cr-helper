//! Init command
//!
//! Initialize cr-helper configuration in a project.

use anyhow::{Context, Result};
use clap::{Args, ValueEnum};
use std::fs;
use std::path::{Path, PathBuf};

/// Project template options
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum ProjectTemplate {
    /// Rust project
    Rust,
    /// TypeScript/JavaScript project
    Typescript,
    /// Python project
    Python,
    /// Go project
    Go,
    /// Generic template
    #[default]
    Generic,
}

/// Arguments for the init command
#[derive(Debug, Args)]
pub struct InitArgs {
    /// Use default settings (non-interactive)
    #[arg(long)]
    pub defaults: bool,

    /// Project template to use
    #[arg(long, short, value_enum)]
    pub template: Option<ProjectTemplate>,

    /// Force overwrite existing configuration
    #[arg(long)]
    pub force: bool,

    /// Directory to initialize (default: current directory)
    #[arg(long)]
    pub path: Option<PathBuf>,
}

/// Execute the init command
pub fn execute(args: InitArgs) -> Result<()> {
    use colored::Colorize;

    let project_dir = args
        .path
        .clone()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    println!(
        "{} Initializing cr-helper in {}...",
        "🚀".to_string(),
        project_dir.display()
    );

    // Check if already initialized
    let cr_helper_dir = project_dir.join(".cr-helper");
    if cr_helper_dir.exists() && !args.force {
        eprintln!(
            "{} cr-helper already initialized. Use --force to reinitialize.",
            "⚠".yellow()
        );
        return Ok(());
    }

    // Detect project type
    let template = args.template.unwrap_or_else(|| detect_project_type(&project_dir));
    println!(
        "{} Detected project type: {:?}",
        "✓".green(),
        template
    );

    // Check if git repository
    let is_git_repo = project_dir.join(".git").exists();
    if !is_git_repo {
        eprintln!(
            "{} Not a git repository. Some features may not work.",
            "⚠".yellow()
        );
    }

    // Create directory structure
    create_directory_structure(&cr_helper_dir)?;
    println!("{} Created .cr-helper/ directory", "✓".green());

    // Generate configuration
    let config = generate_config(template);
    let config_path = cr_helper_dir.join("config.toml");
    fs::write(&config_path, config).context("Failed to write config.toml")?;
    println!("{} Generated config.toml with {:?} template", "✓".green(), template);

    // Create guidelines template
    let guidelines_path = cr_helper_dir.join("guidelines.md");
    if !guidelines_path.exists() {
        fs::write(&guidelines_path, generate_guidelines(template))
            .context("Failed to write guidelines.md")?;
        println!("{} Created review guidelines template", "✓".green());
    }

    // Update .gitignore
    if is_git_repo {
        update_gitignore(&project_dir)?;
        println!("{} Updated .gitignore", "✓".green());
    }

    // Print summary
    println!("\n{} Configuration saved to .cr-helper/config.toml", "📝".to_string());
    println!("\n{}", "Next steps:".bold());
    println!("  1. Review and customize .cr-helper/config.toml");
    println!("  2. Edit .cr-helper/guidelines.md to define your review standards");
    println!("  3. Install Claude Code integration:");
    println!("     ");
    println!("     {}", "cr-helper install --claude-code".cyan());
    println!("     ");
    println!("  4. Start your first review:");
    println!("     ");
    println!("     {}", "cr-helper review".cyan());
    println!("\n{} Tip: Run '{}' to verify your setup", "💡".to_string(), "cr-helper doctor".cyan());

    Ok(())
}

fn detect_project_type(path: &Path) -> ProjectTemplate {
    if path.join("Cargo.toml").exists() {
        ProjectTemplate::Rust
    } else if path.join("package.json").exists() {
        ProjectTemplate::Typescript
    } else if path.join("pyproject.toml").exists() || path.join("setup.py").exists() {
        ProjectTemplate::Python
    } else if path.join("go.mod").exists() {
        ProjectTemplate::Go
    } else {
        ProjectTemplate::Generic
    }
}

fn create_directory_structure(cr_helper_dir: &Path) -> Result<()> {
    fs::create_dir_all(cr_helper_dir)?;
    fs::create_dir_all(cr_helper_dir.join("sessions"))?;
    Ok(())
}

fn generate_config(template: ProjectTemplate) -> String {
    match template {
        ProjectTemplate::Rust => {
            r#"# cr-helper configuration for Rust project

[review]
# Rust-specific review checks
checks = [
    "security",
    "unsafe-code",
    "error-handling",
    "ownership",
    "performance"
]

[review.severity_thresholds]
critical = ["security", "unsafe-code"]
warning = ["error-handling", "performance"]
info = ["style"]

[export]
default_format = "markdown-enhanced"
include_code_context = true
context_lines = 2
include_suggestions = true

[diff]
# Rust file patterns
include_patterns = ["*.rs", "Cargo.toml", "Cargo.lock"]
exclude_patterns = ["target/"]
"#
            .to_string()
        }
        ProjectTemplate::Typescript => {
            r#"# cr-helper configuration for TypeScript project

[review]
# TypeScript-specific review checks
checks = [
    "security",
    "type-safety",
    "error-handling",
    "performance",
    "accessibility"
]

[review.severity_thresholds]
critical = ["security", "type-safety"]
warning = ["error-handling", "performance"]
info = ["style", "accessibility"]

[export]
default_format = "markdown-enhanced"
include_code_context = true
context_lines = 3
include_suggestions = true

[diff]
# TypeScript/JavaScript file patterns
include_patterns = ["*.ts", "*.tsx", "*.js", "*.jsx", "*.json"]
exclude_patterns = ["node_modules/", "dist/", "build/", "*.min.js"]
"#
            .to_string()
        }
        ProjectTemplate::Python => {
            r#"# cr-helper configuration for Python project

[review]
# Python-specific review checks
checks = [
    "security",
    "type-hints",
    "error-handling",
    "performance",
    "testing"
]

[review.severity_thresholds]
critical = ["security"]
warning = ["type-hints", "error-handling"]
info = ["style", "testing"]

[export]
default_format = "markdown-enhanced"
include_code_context = true
context_lines = 3
include_suggestions = true

[diff]
# Python file patterns
include_patterns = ["*.py", "pyproject.toml", "setup.py", "requirements*.txt"]
exclude_patterns = ["__pycache__/", "*.pyc", ".venv/", "venv/", ".eggs/"]
"#
            .to_string()
        }
        ProjectTemplate::Go => {
            r#"# cr-helper configuration for Go project

[review]
# Go-specific review checks
checks = [
    "security",
    "error-handling",
    "concurrency",
    "performance",
    "testing"
]

[review.severity_thresholds]
critical = ["security", "concurrency"]
warning = ["error-handling", "performance"]
info = ["style", "testing"]

[export]
default_format = "markdown-enhanced"
include_code_context = true
context_lines = 2
include_suggestions = true

[diff]
# Go file patterns
include_patterns = ["*.go", "go.mod", "go.sum"]
exclude_patterns = ["vendor/"]
"#
            .to_string()
        }
        ProjectTemplate::Generic => {
            r#"# cr-helper configuration

[review]
# General review checks
checks = [
    "security",
    "error-handling",
    "performance"
]

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
# File patterns (customize for your project)
include_patterns = ["*"]
exclude_patterns = [".git/", "node_modules/", "target/", "__pycache__/"]
"#
            .to_string()
        }
    }
}

fn generate_guidelines(template: ProjectTemplate) -> String {
    let lang_specific = match template {
        ProjectTemplate::Rust => {
            r#"
## Rust-Specific Guidelines

### Memory Safety
- Review `unsafe` blocks carefully
- Check for proper lifetime annotations
- Verify ownership transfers

### Error Handling
- Prefer `Result` over `panic!`
- Use `?` operator consistently
- Provide meaningful error messages

### Performance
- Check for unnecessary allocations
- Review clone usage
- Consider zero-copy alternatives
"#
        }
        ProjectTemplate::Typescript => {
            r#"
## TypeScript-Specific Guidelines

### Type Safety
- Avoid `any` type
- Use strict null checks
- Prefer interfaces over type aliases for objects

### Error Handling
- Use proper try-catch blocks
- Handle Promise rejections
- Validate external data

### React (if applicable)
- Check for missing keys in lists
- Review hook dependencies
- Avoid unnecessary re-renders
"#
        }
        ProjectTemplate::Python => {
            r#"
## Python-Specific Guidelines

### Type Hints
- Add type hints to public functions
- Use `Optional` for nullable types
- Consider using `TypedDict` for dicts

### Error Handling
- Use specific exception types
- Document exceptions in docstrings
- Avoid bare `except` clauses

### Testing
- Maintain test coverage
- Use proper mocking
- Test edge cases
"#
        }
        ProjectTemplate::Go => {
            r#"
## Go-Specific Guidelines

### Error Handling
- Always check returned errors
- Wrap errors with context
- Use error types appropriately

### Concurrency
- Check for race conditions
- Use proper synchronization
- Consider goroutine leaks

### Interfaces
- Keep interfaces small
- Accept interfaces, return structs
- Document interface contracts
"#
        }
        ProjectTemplate::Generic => "",
    };

    format!(
        r#"# Code Review Guidelines

## General Principles

### Security
- Check for injection vulnerabilities (SQL, XSS, command injection)
- Validate all external input
- Review authentication and authorization logic
- Check for sensitive data exposure

### Code Quality
- Follow existing code style
- Ensure functions have single responsibility
- Check for code duplication
- Verify naming conventions
{}

## Severity Levels

- **Critical**: Security vulnerabilities, data loss risks, breaking changes
- **Warning**: Performance issues, potential bugs, maintainability concerns
- **Info**: Style suggestions, minor improvements, documentation
"#,
        lang_specific
    )
}

fn update_gitignore(project_dir: &Path) -> Result<()> {
    let gitignore_path = project_dir.join(".gitignore");
    let entries = "\n# cr-helper\n.cr-helper/sessions/\n.cr-helper/cache/\n";

    if gitignore_path.exists() {
        let content = fs::read_to_string(&gitignore_path)?;
        if !content.contains(".cr-helper/sessions/") {
            let mut file = fs::OpenOptions::new()
                .append(true)
                .open(&gitignore_path)?;
            use std::io::Write;
            file.write_all(entries.as_bytes())?;
        }
    } else {
        fs::write(&gitignore_path, entries)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_rust_project() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("Cargo.toml"), "").unwrap();
        assert!(matches!(detect_project_type(temp.path()), ProjectTemplate::Rust));
    }

    #[test]
    fn test_detect_typescript_project() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("package.json"), "{}").unwrap();
        assert!(matches!(detect_project_type(temp.path()), ProjectTemplate::Typescript));
    }

    #[test]
    fn test_detect_python_project() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("pyproject.toml"), "").unwrap();
        assert!(matches!(detect_project_type(temp.path()), ProjectTemplate::Python));
    }

    #[test]
    fn test_detect_go_project() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("go.mod"), "").unwrap();
        assert!(matches!(detect_project_type(temp.path()), ProjectTemplate::Go));
    }

    #[test]
    fn test_detect_generic_project() {
        let temp = tempfile::tempdir().unwrap();
        assert!(matches!(detect_project_type(temp.path()), ProjectTemplate::Generic));
    }

    #[test]
    fn test_generate_config() {
        let config = generate_config(ProjectTemplate::Rust);
        assert!(config.contains("Rust"));
        assert!(config.contains("unsafe-code"));
    }
}
