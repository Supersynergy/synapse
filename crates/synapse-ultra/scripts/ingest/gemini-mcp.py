#!/usr/bin/env python3
"""Gemini MCP events → synapse-ultra ingest.

Reads JSON from stdin (one event per line, or a single JSON object) and
appends to ~/.synapse/ingest/gemini.jsonl in synapse-ultra Event format.

Usage:
  gemini-mcp-events.py < events.jsonl
"""
from __future__ import annotations
import json
import os
import sys
import time
from pathlib import Path

INGEST_DIR = Path(os.environ.get("SYNAPSE_INGEST_DIR", Path.home() / ".synapse" / "ingest"))
INGEST_FILE = INGEST_DIR / "gemini.jsonl"
AGENT = "gemini"

def normalize(row: dict) -> dict:
    ts = int(row.get("ts") or row.get("timestamp") or time.time())
    kind = row.get("kind") or row.get("type") or "message"
    return {
        "ts": ts,
        "session_id": row.get("session_id"),
        "agent": row.get("agent") or AGENT,
        "kind": kind,
        "uri": row.get("uri") or row.get("tool"),
        "content": row.get("content") or row.get("text"),
        "meta": row.get("meta"),
    }

def main() -> int:
    INGEST_DIR.mkdir(parents=True, exist_ok=True)
    n = 0
    with INGEST_FILE.open("a", encoding="utf-8") as f:
        for line in sys.stdin:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            if isinstance(row, list):
                for r in row:
                    f.write(json.dumps(normalize(r), separators=(",", ":")) + "\n")
                    n += 1
            else:
                f.write(json.dumps(normalize(row), separators=(",", ":")) + "\n")
                n += 1
    sys.stderr.write(f"gemini-mcp-events: wrote {n} events\n")
    return 0

if __name__ == "__main__":
    sys.exit(main())
