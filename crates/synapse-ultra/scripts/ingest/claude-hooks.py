#!/usr/bin/env python3
"""Claude Code hook → synapse-ultra event ingest.

Install in ~/.claude/hooks/ or wherever Claude Code reads hooks from.
Writes one JSONL line per hook event to ~/.synapse/ingest/claude.jsonl.
Run `synapse-ultra ingest --jsonl ~/.synapse/ingest/claude.jsonl` to load.

Hook events (stdin JSON, one per invocation):
  SessionStart  → kind=session_start
  PreToolUse    → kind=tool_call
  PostToolUse   → kind=tool_result
  Stop          → kind=session_end
  Notification  → kind=message

Usage in claude hooks config:
  command: python3 /path/to/claude-hooks.py
"""
from __future__ import annotations
import json
import os
import sys
import time
from pathlib import Path

INGEST_DIR = Path(os.environ.get("SYNAPSE_INGEST_DIR", Path.home() / ".synapse" / "ingest"))
INGEST_FILE = INGEST_DIR / "claude.jsonl"
AGENT = "claude"

KIND_MAP = {
    "SessionStart": "session_start",
    "SessionEnd": "session_end",
    "PreToolUse": "tool_call",
    "PostToolUse": "tool_result",
    "Stop": "session_end",
    "Notification": "message",
    "UserPromptSubmit": "message",
}

def main() -> int:
    try:
        payload = json.load(sys.stdin)
    except Exception as e:
        sys.stderr.write(f"claude-hooks: stdin not JSON: {e}\n")
        return 0  # never block the agent
    hook_event = payload.get("hook_event_name") or payload.get("type") or "unknown"
    kind = KIND_MAP.get(hook_event, hook_event.lower())
    session_id = payload.get("session_id") or payload.get("transcript_path")
    uri = payload.get("tool_name") or payload.get("cwd")
    content = json.dumps(payload, separators=(",", ":"))[:8192]
    event = {
        "ts": int(time.time()),
        "session_id": session_id,
        "agent": AGENT,
        "kind": kind,
        "uri": uri,
        "content": content,
        "meta": {"hook": hook_event},
    }
    INGEST_DIR.mkdir(parents=True, exist_ok=True)
    with INGEST_FILE.open("a", encoding="utf-8") as f:
        f.write(json.dumps(event, separators=(",", ":")) + "\n")
    return 0

if __name__ == "__main__":
    sys.exit(main())
