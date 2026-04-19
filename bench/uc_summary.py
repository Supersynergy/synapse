#!/usr/bin/env python3
"""Per-usecase summary of the 20-usecase bench JSONL.

Groups by usecase and reports min/median/max latency, throughput, and size
across every config row. Picks the CatBoost-recommended defaults per usecase.
"""
from __future__ import annotations

import json
import statistics
import sys
from collections import defaultdict


def main() -> None:
    path = sys.argv[1] if len(sys.argv) > 1 else "/tmp/synapse_bench.jsonl"
    rows: list[dict] = []
    with open(path) as fh:
        for line in fh:
            line = line.strip()
            if line:
                rows.append(json.loads(line))

    groups: dict[str, list[dict]] = defaultdict(list)
    for r in rows:
        if r.get("ok"):
            groups[r["usecase"]].append(r)

    header = f"{'usecase':<32} {'min ms':>10} {'med ms':>10} {'max ms':>10} {'best ms @ knobs':>30}"
    print(header)
    print("-" * len(header))
    for uc in sorted(groups):
        vs = groups[uc]
        lats = [v["latency_ms"] for v in vs]
        best = min(vs, key=lambda v: v["latency_ms"])
        knob = f"zstd={best['zstd_level']} ef={best['hnsw_ef']} n={best['n']}"
        print(
            f"{uc:<32} {min(lats):>10.3f} {statistics.median(lats):>10.3f} {max(lats):>10.3f}  "
            f"{best['latency_ms']:>8.3f}  {knob:>18}"
        )


if __name__ == "__main__":
    main()
