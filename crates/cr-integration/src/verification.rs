//! Installation verification
//!
//! Utilities for verifying cr-helper installation.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Verification check result
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Check name
    pub name: String,
    /// Whether the check passed
    pub passed: bool,
    /// Message describing the result
    pub message: String,
    /// Optional suggestion for fixing
    pub suggestion: Option<String>,
}

impl VerificationResult {
    /// Create a passing result
    pub fn pass(name: &str, message: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: true,
            message: message.to_string(),
            suggestion: None,
        }
    }

    /// Create a failing result
    pub fn fail(name: &str, message: &str, suggestion: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            passed: false,
            message: message.to_string(),
            suggestion: suggestion.map(|s| s.to_string()),
        }
    }

    /// Create a warning result (passed but with suggestion)
    pub fn warn(name: &str, message: &str, suggestion: Option<&str>) -> Self {
        Self {
            name: name.to_string(),
            passed: true,
            message: format!("⚠ {}", message),
            suggestion: suggestion.map(|s| s.to_string()),
        }
    }
}

/// Verify cr-helper CLI is available
pub fn verify_cli() -> VerificationResult {
    match Command::new("cr-helper").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            let version = version.trim();
            VerificationResult::pass("cr-helper CLI", version)
        }
        _ => VerificationResult::fail(
            "cr-helper CLI",
            "not found in PATH",
            Some("Install cr-helper or add it to your PATH"),
        ),
    }
}

/// Verify Git is available
pub fn verify_git() -> VerificationResult {
    match Command::new("git").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            let version = version.trim().replace("git version ", "");
            VerificationResult::pass("Git", &version)
        }
        _ => VerificationResult::fail(
            "Git",
            "not found",
            Some("Install git: https://git-scm.com/"),
        ),
    }
}

/// Verify delta is available (optional)
pub fn verify_delta() -> VerificationResult {
    match Command::new("delta").arg("--version").output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            let version = version
                .trim()
                .split_whitespace()
                .nth(1)
                .unwrap_or("unknown");
            VerificationResult::pass("delta", version)
        }
        _ => VerificationResult::warn(
            "delta",
            "not installed (optional)",
            Some("Install delta for better diffs: https://github.com/dandavison/delta"),
        ),
    }
}

/// Verify Claude Code Skill installation
pub fn verify_skill(claude_dir: &Path) -> VerificationResult {
    let skill_dir = claude_dir.join("skills/cr-helper");
    let skill_md = skill_dir.join("SKILL.md");

    if !skill_dir.exists() {
        return VerificationResult::fail(
            "Skill",
            "not installed",
            Some("Run 'cr-helper install --claude-code'"),
        );
    }

    if !skill_md.exists() {
        return VerificationResult::fail(
            "Skill",
            "SKILL.md missing",
            Some("Run 'cr-helper install --claude-code'"),
        );
    }

    VerificationResult::pass("Skill", &format!("installed at {}", skill_dir.display()))
}

/// Verify Claude Code Hooks installation
pub fn verify_hooks(claude_dir: &Path) -> VerificationResult {
    let hooks_dir = claude_dir.join("hooks");
    let stop_hook = hooks_dir.join("cr-helper-stop.sh");

    if !stop_hook.exists() {
        return VerificationResult::warn(
            "Hooks",
            "not installed",
            Some("Run 'cr-helper install --claude-code'"),
        );
    }

    // Check executable permission on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match std::fs::metadata(&stop_hook) {
            Ok(meta) => {
                let mode = meta.permissions().mode();
                if mode & 0o111 == 0 {
                    return VerificationResult::fail(
                        "Hooks",
                        "not executable",
                        Some("Run: chmod +x .claude/hooks/cr-helper-*.sh"),
                    );
                }
            }
            Err(_) => {
                return VerificationResult::fail("Hooks", "cannot check permissions", None);
            }
        }
    }

    VerificationResult::pass("Hooks", "installed and executable")
}

/// Verify settings.json configuration
pub fn verify_settings(settings_path: &Path) -> VerificationResult {
    if !settings_path.exists() {
        return VerificationResult::warn(
            "settings.json",
            "not found",
            Some("Run 'cr-helper install --claude-code'"),
        );
    }

    match std::fs::read_to_string(settings_path) {
        Ok(content) => match serde_json::from_str::<serde_json::Value>(&content) {
            Ok(settings) => {
                if settings.get("cr-helper").is_some() {
                    VerificationResult::pass("settings.json", "cr-helper configured")
                } else {
                    VerificationResult::warn(
                        "settings.json",
                        "cr-helper not configured",
                        Some("Run 'cr-helper install --claude-code'"),
                    )
                }
            }
            Err(e) => VerificationResult::fail(
                "settings.json",
                &format!("invalid JSON: {}", e),
                Some("Fix syntax errors in settings.json"),
            ),
        },
        Err(_) => VerificationResult::fail("settings.json", "cannot read", None),
    }
}

/// Run all verification checks
pub fn run_all_checks() -> Vec<VerificationResult> {
    let mut results = Vec::new();

    // System checks
    results.push(verify_cli());
    results.push(verify_git());
    results.push(verify_delta());

    // Claude Code checks
    let project_claude = PathBuf::from(".claude");
    if project_claude.exists() {
        results.push(verify_skill(&project_claude));
        results.push(verify_hooks(&project_claude));
        results.push(verify_settings(&project_claude.join("settings.json")));
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verification_result_pass() {
        let result = VerificationResult::pass("test", "ok");
        assert!(result.passed);
        assert!(result.suggestion.is_none());
    }

    #[test]
    fn test_verification_result_fail() {
        let result = VerificationResult::fail("test", "error", Some("fix it"));
        assert!(!result.passed);
        assert!(result.suggestion.is_some());
    }

    #[test]
    fn test_verification_result_warn() {
        let result = VerificationResult::warn("test", "warning", None);
        assert!(result.passed);
        assert!(result.message.contains("⚠"));
    }

    #[test]
    fn test_verify_git() {
        // Git should be installed in most development environments
        let result = verify_git();
        // We don't assert on passed because git might not be installed
        assert!(!result.name.is_empty());
    }
}
