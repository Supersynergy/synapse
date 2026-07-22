#!/usr/bin/env python3
"""Git post-commit hook → synapse-ultra decision ingest.

Install as .git/hooks/post-commit (symlink or copy). Emits one decision
event per commit to ~/.synapse/ingest/git.jsonl.

The commit's files become source_uri (first file) and target_uri (the
repo path). The commit message is the rationale.

Usage in .git/hooks/post-commit:
  python3 /path/to/git-post-commit.py
"""
from __future__ import annotations
import json
import os
import subprocess
import sys
import time
from pathlib import Path

INGEST_DIR = Path(os.environ.get("SYNAPSE_INGEST_DIR", Path.home() / ".synapse" / "ingest"))
INGEST_FILE = INGEST_DIR / "git.jsonl"
AGENT = "git"

def git(*args: str) -> str:
    try:
        out = subprocess.run(["git", *args], capture_output=True, text=True, check=False)
        return out.stdout.strip()
    except Exception:
        return ""

def main() -> int:
    repo_root = git("rev-parse", "--show-toplevel") or os.getcwd()
    commit = git("rev-parse", "HEAD")
    if not commit:
        return 0
    subject = git("log", "-1", "--pretty=%s")
    body = git("log", "-1", "--pretty=%b")
    files = git("show", "--pretty=format:", "--name-only", commit).splitlines()
    files = [f for f in files if f.strip()]
    first_file = files[0] if files else None
    rationale = subject + ("\n" + body if body else "")
    event = {
        "ts": int(time.time()),
        "session_id": commit[:12],
        "agent": AGENT,
        "kind": "decision",
        "uri": f"git:{commit}",
        "content": rationale,
        "meta": {
            "commit": commit,
            "repo": repo_root,
            "files": files[:50],
            "author": git("log", "-1", "--pretty=%an"),
        },
    }
    # also emit a decision row with source/target for graph auto-population
    decision = {
        "ts": event["ts"],
        "session_id": event["session_id"],
        "agent": AGENT,
        "uri": f"git:{commit}",
        "rationale": rationale,
        "source_uri": first_file,
        "target_uri": repo_root,
        "meta": event["meta"],
    }
    INGEST_DIR.mkdir(parents=True, exist_ok=True)
    with INGEST_FILE.open("a", encoding="utf-8") as f:
        f.write(json.dumps(event, separators=(",", ":")) + "\n")
        f.write(json.dumps({"__decision__": True, **decision}, separators=(",", ":")) + "\n")
    return 0

if __name__ == "__main__":
    sys.exit(main())
