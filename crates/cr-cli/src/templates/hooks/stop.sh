#!/bin/bash
# cr-helper stop hook
# Called when Claude Code session is about to stop

set -e

# Get configuration from settings
CR_HELPER_DIR="${CR_HELPER_DIR:-.cr-helper}"
SETTINGS_FILE=".claude/settings.json"

# Read settings
AUTO_REVIEW=true
MIN_CHANGES=3
BLOCK_ON_CRITICAL=true

if [ -f "$SETTINGS_FILE" ]; then
    AUTO_REVIEW=$(jq -r '.["cr-helper"].auto_review_on_stop // true' "$SETTINGS_FILE" 2>/dev/null || echo "true")
    MIN_CHANGES=$(jq -r '.["cr-helper"].min_changes_for_review // 3' "$SETTINGS_FILE" 2>/dev/null || echo "3")
    BLOCK_ON_CRITICAL=$(jq -r '.["cr-helper"].block_on_critical // true' "$SETTINGS_FILE" 2>/dev/null || echo "true")
fi

# Skip if auto-review disabled
if [ "$AUTO_REVIEW" != "true" ]; then
    exit 0
fi

# Check if there are changes to review
CHANGED_FILES=$(git diff --name-only 2>/dev/null | wc -l | tr -d ' ')
STAGED_FILES=$(git diff --cached --name-only 2>/dev/null | wc -l | tr -d ' ')
TOTAL_CHANGES=$((CHANGED_FILES + STAGED_FILES))

if [ "$TOTAL_CHANGES" -lt "$MIN_CHANGES" ]; then
    if [ "${CR_HELPER_VERBOSE:-0}" = "1" ]; then
        echo "[cr-helper] Skipping review: only $TOTAL_CHANGES files changed (min: $MIN_CHANGES)"
    fi
    exit 0
fi

# Run review
echo "[cr-helper] Reviewing $TOTAL_CHANGES changed files..."

# Export review results
OUTPUT_DIR="${CR_HELPER_DIR}/reviews"
mkdir -p "$OUTPUT_DIR"
REVIEW_FILE="${OUTPUT_DIR}/review-$(date +%Y%m%d-%H%M%S).json"

if command -v cr-helper &> /dev/null; then
    cr-helper review --no-tui 2>/dev/null && \
    cr-helper export --latest --format json-compact --output "$REVIEW_FILE" 2>/dev/null

    if [ -f "$REVIEW_FILE" ]; then
        # Check for critical issues
        CRITICAL_COUNT=$(jq -r '.stats.critical // 0' "$REVIEW_FILE" 2>/dev/null || echo "0")

        if [ "$CRITICAL_COUNT" -gt 0 ] && [ "$BLOCK_ON_CRITICAL" = "true" ]; then
            echo "[cr-helper] Found $CRITICAL_COUNT critical issues!"
            echo "[cr-helper] Review the issues before proceeding."
            # Return non-zero to potentially block (depending on hook configuration)
            exit 1
        fi

        echo "[cr-helper] Review complete. Results saved to: $REVIEW_FILE"
    fi
else
    echo "[cr-helper] Warning: cr-helper not found in PATH"
fi

exit 0
