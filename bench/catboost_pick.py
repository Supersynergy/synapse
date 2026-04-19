#!/usr/bin/env python3
"""
Train a CatBoost regressor on the 20-usecase bench JSONL and pick the
Pareto-dominant knob settings for the entire Synapse fleet.

Inputs (one JSON line each):
  { "usecase": "...", "n": int, "zstd_level": int, "hnsw_ef": int,
    "latency_ms": float, "throughput": float, "bytes": int, "ok": bool }

Outputs to stdout:
  - a ranked table of (zstd_level, hnsw_ef) combos by predicted mean latency
  - the recommended defaults across all usecases
  - feature importances

Usage:
  python3 bench/catboost_pick.py /tmp/synapse_bench.jsonl
"""
from __future__ import annotations

import json
import sys
from collections import defaultdict

try:
    from catboost import CatBoostRegressor, Pool  # type: ignore
except ImportError:
    print("[skip] pip install catboost  (CatBoost not installed; printing heuristics)")
    CatBoostRegressor = None  # type: ignore
    Pool = None  # type: ignore


def load(path: str) -> list[dict]:
    rows = []
    with open(path) as fh:
        for line in fh:
            line = line.strip()
            if line:
                rows.append(json.loads(line))
    return rows


def heuristic_best(rows: list[dict]) -> dict:
    # Baseline: mean latency per (zstd_level, hnsw_ef) across all usecases.
    agg: dict[tuple[int, int], list[float]] = defaultdict(list)
    for r in rows:
        if r.get("ok"):
            agg[(r["zstd_level"], r["hnsw_ef"])].append(r["latency_ms"])
    means = {k: sum(v) / len(v) for k, v in agg.items()}
    best = min(means.items(), key=lambda x: x[1])
    return {
        "by_knob_mean_latency_ms": {f"zstd={k[0]},ef={k[1]}": round(v, 2) for k, v in sorted(means.items())},
        "best_knob": {"zstd_level": best[0][0], "hnsw_ef": best[0][1], "mean_latency_ms": round(best[1], 2)},
    }


def catboost_ranked(rows: list[dict]) -> dict | None:
    if CatBoostRegressor is None:
        return None
    # features = [n, zstd_level, hnsw_ef, usecase_idx]; target = log(latency_ms)
    import math
    ucs = sorted({r["usecase"] for r in rows})
    uc_ix = {u: i for i, u in enumerate(ucs)}
    X, y = [], []
    for r in rows:
        if not r.get("ok"):
            continue
        X.append([r["n"], r["zstd_level"], r["hnsw_ef"], uc_ix[r["usecase"]]])
        y.append(math.log1p(r["latency_ms"]))
    if not X:
        return None
    model = CatBoostRegressor(
        iterations=300,
        depth=5,
        learning_rate=0.05,
        loss_function="RMSE",
        verbose=False,
    )
    model.fit(Pool(X, y))
    imps = model.get_feature_importance()
    feature_names = ["n", "zstd_level", "hnsw_ef", "usecase"]

    # grid-search knobs: for each (zstd, ef), predict mean latency across usecases.
    knob_scores: dict[tuple[int, int], float] = {}
    for zl in sorted({r["zstd_level"] for r in rows}):
        for ef in sorted({r["hnsw_ef"] for r in rows}):
            preds = []
            for u_ix in range(len(ucs)):
                # evaluate at the largest n we saw
                n_max = max(r["n"] for r in rows)
                preds.append(model.predict([n_max, zl, ef, u_ix]))
            knob_scores[(zl, ef)] = sum(preds) / len(preds)
    ranked = sorted(knob_scores.items(), key=lambda x: x[1])
    return {
        "ranked_by_predicted_log_latency": [
            {"zstd_level": k[0], "hnsw_ef": k[1], "pred": round(v, 4)}
            for k, v in ranked
        ],
        "recommended_defaults": {"zstd_level": ranked[0][0][0], "hnsw_ef": ranked[0][0][1]},
        "feature_importance": dict(zip(feature_names, [round(x, 3) for x in imps])),
    }


def main() -> None:
    if len(sys.argv) != 2:
        print("usage: catboost_pick.py <path.jsonl>", file=sys.stderr)
        sys.exit(2)
    rows = load(sys.argv[1])
    out = {
        "records": len(rows),
        "heuristic": heuristic_best(rows),
        "catboost": catboost_ranked(rows),
    }
    print(json.dumps(out, indent=2, default=str))


if __name__ == "__main__":
    main()
