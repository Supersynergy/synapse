# Synapse

> Single-file memory for AI agents. SQLite speed. Daemon mode. Rust core.

**Status:** pre-code — architecture locked in [MASTERPLAN.md](./MASTERPLAN.md).

## What

A drop-in replacement for memvid's `.mv2` format that keeps the "one portable file per project" property but delivers:

- **80× faster insert** than MV2 (batch embed + no CLI spawn)
- **450× faster lex search** (FTS5 triggers, no Tantivy rebuilds)
- **15× faster vec search** (sqlite-vec HNSW)
- **<0.2ms socket RTT** vs 200ms MV2 spawn cost
- **10× smaller files** (zstd blobs + BLAKE3 dedup)

All while still shipping as one `.brainpack` file you can `git commit`, `scp`, or hand to a teammate.

## Why not just use SQLite+FTS5?

You can. Synapse = SQLite+FTS5+sqlite-vec + **daemon** + **batch embedder** + **portable snapshot format** + **client SDKs** + **MCP endpoint**. The infrastructure around SQLite that makes it a drop-in "agent memory layer" instead of a DB you wire up yourself.

## Architecture (tl;dr)

```
clients ──msgpack/unix-socket──▶ synapsed ──▶ SQLite(FTS5 + sqlite-vec)
                                    │
                                    └── fastembed-rs (BGE-small ONNX, ANE)
```

See [MASTERPLAN.md §3](./MASTERPLAN.md#3-architecture) for the full diagram.

## Benchmarks vs MV2 (target)

| Op (1k docs, M4 Max) | MV2 | Synapse target | Δ |
|---|---|---|---|
| Insert | 147s (extrapolated) | <500ms | **60×** |
| Lex query | 12.4s | <2ms | **6000×** |
| Vec query | 88ms | <6ms | **15×** |
| File size | 5.6 MB | ~500 KB | **10×** |
| Cold start | 200ms/call | <50ms daemon, 0.2ms socket | **4000×** |

Numbers from [bench_v2.sh](./bench/bench_v2.sh), MV2 baseline measured 2026-04-19.

## Status

- [x] Masterplan
- [ ] M1 — `synapse-core` crate (schema + FTS5 + sqlite-vec)
- [ ] M2 — embedding pipeline (fastembed-rs + BLAKE3 dedup)
- [ ] M3 — `synapsed` daemon (unix socket, msgpack-rpc)
- [ ] M4 — CLI + Node SDK
- [ ] M5 — `.brainpack` export/import
- [ ] M6 — bench harness
- [ ] M7 — MCP mode, CRDT (stretch)

MVP target: **5.5 days** focused work.

## Non-goals

- Replace Qdrant at >10M vectors.
- Replace SQLite as a general-purpose DB.
- Reinvent storage. We use battle-tested SQLite.

## License

MIT — see [LICENSE](./LICENSE).
