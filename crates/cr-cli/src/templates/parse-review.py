#!/usr/bin/env python3
"""Parse cr-helper review output for Claude Code integration."""

import json
import sys
from typing import Any


def parse_review_output(input_data: str) -> dict[str, Any]:
    """Parse JSON review output from cr-helper."""
    try:
        data = json.loads(input_data)
        return format_for_claude(data)
    except json.JSONDecodeError as e:
        return {"error": f"Invalid JSON: {e}"}


def format_for_claude(data: dict[str, Any]) -> dict[str, Any]:
    """Format review data for Claude Code consumption."""
    result = {
        "session_id": data.get("session_id", "unknown"),
        "summary": {
            "files": data.get("stats", {}).get("files_reviewed", 0),
            "comments": data.get("stats", {}).get("total_comments", 0),
            "critical": data.get("stats", {}).get("critical", 0),
            "warning": data.get("stats", {}).get("warning", 0),
            "info": data.get("stats", {}).get("info", 0),
        },
        "reviews": [],
    }

    for review in data.get("reviews", []):
        formatted = {
            "file": review.get("file", ""),
            "line": review.get("line"),
            "severity": review.get("sev", "i"),
            "content": review.get("content", ""),
        }
        result["reviews"].append(formatted)

    return result


def main():
    """Main entry point."""
    if len(sys.argv) > 1:
        # Read from file
        with open(sys.argv[1]) as f:
            input_data = f.read()
    else:
        # Read from stdin
        input_data = sys.stdin.read()

    output = parse_review_output(input_data)
    print(json.dumps(output, indent=2))


if __name__ == "__main__":
    main()
