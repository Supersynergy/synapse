#!/usr/bin/env python3
"""Codex usage stream → synapse-ultra event ingest.

Reads Codex JSONL usage files (one per session) and emits synapse-ultra
events + token_cost rows to ~/.synapse/ingest/codex.jsonl and
~/.synapse/ingest/codex_cost.jsonl.

Reuses agent-token-saver's full_context_ledger.py if available (optional).
Falls back to a minimal parser that handles the standard Codex usage shape:
  {"timestamp": "...", "model": "...", "input_tokens": N, "output_tokens": N, ...}

Usage:
  python3 codex-usage.py /path/to/usage.jsonl
  python3 codex-usage.py /path/to/usage.jsonl --session my-session
"""
from __future__ import annotations
import argparse
import json
import os
import sys
import time
from pathlib import Path

INGEST_DIR = Path(os.environ.get("SYNAPSE_INGEST_DIR", Path.home() / ".synapse" / "ingest"))
AGENT = "codex"

def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser()
    p.add_argument("usage_file", type=Path)
    p.add_argument("--session", default=None)
    p.add_argument("--agent", default=AGENT)
    return p.parse_args()

def main() -> int:
    args = parse_args()
    if not args.usage_file.exists():
        sys.stderr.write(f"codex-usage: file not found: {args.usage_file}\n")
        return 1
    INGEST_DIR.mkdir(parents=True, exist_ok=True)
    events_path = INGEST_DIR / "codex.jsonl"
    cost_path = INGEST_DIR / "codex_cost.jsonl"
    n_events = 0
    n_cost = 0
    with args.usage_file.open("r", encoding="utf-8") as f, \
         events_path.open("a", encoding="utf-8") as ev, \
         cost_path.open("a", encoding="utf-8") as co:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            ts = int(row.get("timestamp") or row.get("ts") or time.time())
            model = row.get("model") or "unknown"
            in_tok = int(row.get("input_tokens") or row.get("input") or 0)
            out_tok = int(row.get("output_tokens") or row.get("output") or 0)
            cache_read = int(row.get("cache_read_tokens") or 0)
            cache_write = int(row.get("cache_write_tokens") or 0)
            cost = float(row.get("cost_usd") or 0.0)
            session_id = args.session or row.get("session_id")
            # event row
            event = {
                "ts": ts,
                "session_id": session_id,
                "agent": args.agent,
                "kind": "tool_call",
                "uri": f"model:{model}",
                "content": row.get("prompt") or row.get("content"),
                "meta": {"model": model},
            }
            ev.write(json.dumps(event, separators=(",", ":")) + "\n")
            n_events += 1
            # cost row (separate file for clarity; loaded by a small extension)
            cost_row = {
                "ts": ts,
                "session_id": session_id,
                "agent": args.agent,
                "model": model,
                "input_tokens": in_tok,
                "output_tokens": out_tok,
                "cache_read": cache_read,
                "cache_write": cache_write,
                "cost_usd": cost,
            }
            co.write(json.dumps(cost_row, separators=(",", ":")) + "\n")
            n_cost += 1
    sys.stderr.write(f"codex-usage: wrote {n_events} events, {n_cost} cost rows\n")
    return 0

if __name__ == "__main__":
    sys.exit(main())
