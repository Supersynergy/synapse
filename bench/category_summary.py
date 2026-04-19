#!/usr/bin/env python3
"""Per-category + per-usecase summary of the 50-usecase JSONL."""
from __future__ import annotations

import json
import statistics
import sys
from collections import defaultdict


def main() -> None:
    path = sys.argv[1] if len(sys.argv) > 1 else "/tmp/synapse_bench_v1.jsonl"
    rows: list[dict] = []
    with open(path) as fh:
        for line in fh:
            line = line.strip()
            if line:
                rows.append(json.loads(line))

    by_cat: dict[str, list[dict]] = defaultdict(list)
    for r in rows:
        by_cat[r["category"]].append(r)

    print(f"# Synapse v1.0 — 50-usecase bench summary ({len(rows)} rows)\n")
    print("## Category medians\n")
    print("| category | n usecases | median latency ms | median throughput |")
    print("|----------|-----------:|------------------:|------------------:|")
    for cat in sorted(by_cat):
        rs = by_cat[cat]
        lats = [r["latency_ms"] for r in rs]
        thrs = [r["throughput"] for r in rs if r["throughput"] > 0]
        ucs = len({r["usecase"] for r in rs})
        print(f"| {cat} | {ucs} | {statistics.median(lats):.3f} | {statistics.median(thrs):.1f} |")
    print()

    print("## Per-usecase minimum latency\n")
    print("| usecase | category | min ms | best knobs | throughput |")
    print("|---------|----------|-------:|------------|-----------:|")
    by_uc = defaultdict(list)
    for r in rows:
        by_uc[r["usecase"]].append(r)
    for uc in sorted(by_uc):
        best = min(by_uc[uc], key=lambda r: r["latency_ms"])
        knobs = f"zstd={best['zstd_level']} ef={best['hnsw_ef']} n={best['n']}"
        print(
            f"| {uc} | {best['category']} | {best['latency_ms']:.3f} | {knobs} | {best['throughput']:.1f} |"
        )


if __name__ == "__main__":
    main()
