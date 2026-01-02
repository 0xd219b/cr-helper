#!/bin/bash
# cr-helper session start hook
# Called when a Claude Code session starts

set -e

# Get cr-helper config directory
CR_HELPER_DIR="${CR_HELPER_DIR:-.cr-helper}"
SESSION_DIR="${CR_HELPER_DIR}/sessions"

# Ensure directories exist
mkdir -p "$SESSION_DIR"

# Log session start (if verbose)
if [ "${CR_HELPER_VERBOSE:-0}" = "1" ]; then
    echo "[cr-helper] Session started at $(date -Iseconds)" >> "${CR_HELPER_DIR}/hook.log"
fi

# Check for existing active session
LATEST_SESSION=$(ls -t "$SESSION_DIR"/*.json 2>/dev/null | head -1)
if [ -n "$LATEST_SESSION" ]; then
    SESSION_ID=$(basename "$LATEST_SESSION" .json)
    if [ "${CR_HELPER_VERBOSE:-0}" = "1" ]; then
        echo "[cr-helper] Found existing session: $SESSION_ID"
    fi
fi

exit 0
