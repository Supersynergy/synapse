#!/usr/bin/env python3
"""Devin (Cognition) usage stream → synapse-ultra event ingest.

Reads Devin session JSONL (exported from app.devin.ai) and emits synapse-ultra
events + token_cost rows to ~/.synapse/ingest/devin.jsonl and
~/.synapse/ingest/devin_cost.jsonl.

Devin's JSONL schema (observed 2026-07):
  {"timestamp": "...", "model": "...", "input_tokens": N, "output_tokens": N,
   "unattributed_input_tokens": N, "cache_read_tokens": N, "cache_write_tokens": N,
   "cost_usd": F, "session_id": "...", "task": "..."}

The `unattributed_input_tokens` field is Devin-specific — host-instruction,
tool-schema and plugin-catalog tax that the token-saver profile targets.

Usage:
  python3 devin-usage.py /path/to/devin-session.jsonl
  python3 devin-usage.py /path/to/devin-session.jsonl --session abc123
"""
from __future__ import annotations
import argparse
import json
import os
import sys
import time
from pathlib import Path

INGEST_DIR = Path(os.environ.get("SYNAPSE_INGEST_DIR", Path.home() / ".synapse" / "ingest"))
AGENT = "devin"


def parse_args() -> argparse.Namespace:
    p = argparse.ArgumentParser()
    p.add_argument("usage_file", type=Path)
    p.add_argument("--session", default=None)
    p.add_argument("--agent", default=AGENT)
    return p.parse_args()


def main() -> int:
    args = parse_args()
    if not args.usage_file.exists():
        sys.stderr.write(f"devin-usage: file not found: {args.usage_file}\n")
        return 1
    INGEST_DIR.mkdir(parents=True, exist_ok=True)
    events_path = INGEST_DIR / "devin.jsonl"
    n_events = 0
    with args.usage_file.open("r", encoding="utf-8") as f, \
         events_path.open("a", encoding="utf-8") as ev:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            try:
                row = json.loads(line)
            except json.JSONDecodeError:
                continue
            ts = int(row.get("timestamp") or row.get("ts") or time.time())
            model = row.get("model") or "devin-default"
            in_tok = int(row.get("input_tokens") or row.get("input") or 0)
            out_tok = int(row.get("output_tokens") or row.get("output") or 0)
            unattrib = int(row.get("unattributed_input_tokens") or 0)
            cache_read = int(row.get("cache_read_tokens") or 0)
            cache_write = int(row.get("cache_write_tokens") or 0)
            cost = float(row.get("cost_usd") or 0.0)
            session_id = args.session or row.get("session_id") or "devin-session"
            task = row.get("task") or row.get("prompt") or row.get("content") or ""
            # Single event row with cost + token fields in meta.
            # synapse-ultra's token_cost table is populated downstream if a
            # dedicated cost-ingest path exists; otherwise the data is
            # retrievable via events.meta for replay/why queries.
            event = {
                "ts": ts,
                "session_id": session_id,
                "agent": args.agent,
                "kind": "tool_call",
                "uri": f"model:{model}",
                "content": task,
                "meta": {
                    "model": model,
                    "input_tokens": in_tok,
                    "output_tokens": out_tok,
                    "unattributed_input_tokens": unattrib,
                    "cache_read": cache_read,
                    "cache_write": cache_write,
                    "cost_usd": cost,
                },
            }
            ev.write(json.dumps(event, separators=(",", ":")) + "\n")
            n_events += 1
    sys.stderr.write(
        f"devin-usage: wrote {n_events} events "
        f"(unattributed_input_tokens tracked in meta)\n"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
