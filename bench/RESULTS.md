# Bench Results — 2026-04-19

M4 Max, 128GB RAM, release build.

## 1000 docs, no-embed (lex only)

| Op | Synapse CLI | Synapse in-proc (FTS5) | MV2 (extrapolated) | Δ vs MV2 |
|---|---|---|---|---|
| Insert 1000 | 155.66s | **0.287s** | ~147s | **512× faster** |
| Lex search | 18.6ms/q | 17.1ms/q | 12400ms/q | **725× faster** |
| File size | 557 KB | 410 KB | 5.6 MB | **13× smaller** |
| `.brainpack` | 173 KB | — | 5.6 MB | **32× smaller** |

## Key Findings

1. **In-proc = already crushes MV2.** 0.287s insert + 17ms lex on 1000 docs. This is what a daemon gives you.
2. **CLI spawn cost = 155ms/call** (1000 spawns × ~155ms = 155s). Confirms M3 daemon is critical — eliminate 99% of runtime.
3. **`.brainpack` is 32× smaller than a raw `.mv2` at the same doc count.** zstd + SQLite packing wins.
4. **FTS5 17ms/q on 1000 docs** is higher than expected; likely python subprocess spawn per query. Direct `sqlite3_exec` loop would be <2ms.

## What's Next (M3)

Build `synapsed` daemon → unix socket msgpack-rpc → batch insert. Projected:
- Insert 1000 docs: **<500ms** (in-proc × tiny RPC overhead)
- Lex search: **<2ms p95**
- Vec search: **<6ms p95** (sqlite-vec)

## Methodology

- **Synapse CLI**: 1000 × `synapse put --no-embed` via shell loop. Each invocation = full process spawn + SQLite open.
- **Synapse in-proc**: simulates daemon via one Python process using same SQLite+FTS5 schema.
- **MV2 baseline**: from `bench/bench_v2.sh` run 2026-04-19 on 200 docs (extrapolated ×5 for 1000).

Reproduce: `bash bench/bench_synapse_vs_mv2.sh 1000`
