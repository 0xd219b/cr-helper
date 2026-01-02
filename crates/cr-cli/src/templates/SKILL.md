# Code Review Helper

A skill for automated code review with Claude Code.

## Usage

Use `/review` to start a code review session.

### Commands

- `/review` - Review current changes
- `/review --staged` - Review staged changes
- `/review --commit <hash>` - Review specific commit

### Review Flow

1. Claude detects code changes
2. Runs `cr-helper review` to analyze
3. Presents findings with severity levels
4. Allows interactive resolution

## Severity Levels

- **Critical** (🔴): Security vulnerabilities, data loss risks
- **Warning** (🟡): Performance issues, potential bugs
- **Info** (🔵): Style suggestions, improvements

## Configuration

Edit `.cr-helper/config.toml` to customize:

```toml
[review]
checks = ["security", "error-handling", "performance"]

[review.severity_thresholds]
critical = ["security"]
warning = ["error-handling"]
info = ["style"]
```

## Integration

This skill integrates with cr-helper CLI tool.
Run `cr-helper doctor` to verify installation.
