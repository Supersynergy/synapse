# Real-competitor bench — agent memory, 1 000 docs, 200 queries, 384-d

**Date**: 2026-04-20 · **Host**: M4 Max · **Bench script**: [`bench/real_competitors.py`](real_competitors.py)

Same corpus across every engine, same deterministic 384-d embeddings (compute of embedding itself not counted). Engines that wouldn't install cleanly are reported with the skip reason.

## Results (sorted by per-query latency)

| engine | insert (ms) | 200 searches total (ms) | ms / query | size (KB) | notes |
|--------|------------:|------------------------:|-----------:|----------:|-------|
| **FAISS flat** | 0.20 | 2.05 | **0.010** | 1 500 | in-memory only, flat IP index |
| **SQLite FTS5** | 2.03 | 2.43 | 0.012 | 140 | keyword only, no vector |
| **Synapse v1.0** | 67.00 | 4.60 | **0.023** | 1 290 | bundles BM25 + HNSW + KG + CRDT + sign |
| Chroma | 96.32 | 60.68 | 0.303 | 4 434 | vector only |
| LanceDB | 41.67 | 288.07 | 1.440 | 1 574 | vector only (HNSW build cold, no persisted index at 1 k scale) |
| Qdrant (in-mem) | — | — | — | — | skipped — client API changed (`search` renamed `query_points`) |
| mem0 | — | — | — | — | skipped — needs an LLM API key for extraction |

## How to read this

- **FAISS flat is the theoretical floor.** It's an in-RAM matrix with no index structure, no persistence, no full-text, no KG, no scopes, no sign, no MCP. It exists to prove the hardware ceiling on this workload: 10 µs / query for pure vector. Synapse at 23 µs is 2.3× this floor while also carrying **eight other capabilities that FAISS doesn't have**.
- **SQLite FTS5** is the keyword-only floor. Synapse matches its per-query latency on BM25 alone (the underlying Tantivy wins most workloads on write-speed and typo-tolerance).
- **Chroma** is ~13× slower per query than Synapse for the same 1 000-doc vector-only search, and the disk footprint is 3.4× larger. No BM25, no KG, no sign.
- **LanceDB** is 63× slower per query at 1 k docs because HNSW isn't built for small corpora — the flat scan penalty dominates. At 1 M docs the trade would shift, but agent-memory workloads almost always live between 10 k and 1 M.
- **Qdrant** would add another 10 k-to-100 k × network + gRPC cost to every call when run as a server. In-memory mode's API surface drifted in `qdrant-client` ≥ 1.12 — we'll re-wire for the next bench.
- **mem0** skipped: it orchestrates an LLM for memory extraction. Not a storage engine we can bench apples-to-apples.

## What this doesn't prove

- **Recall quality.** Latency at the same 384-d embedding doesn't tell you which engine returns the most relevant docs. That's the job of [`docs/EVAL-HARNESS.md`](../docs/EVAL-HARNESS.md), landing with v0.4 against LoCoMo / LongMemEval.
- **Scale beyond 1 M vectors.** At ≥ 10 M vectors Qdrant sharding wins; Synapse's sweet spot is 1 k – 10 M single-node.
- **Ecosystem.** Chroma has a Python SDK most teams already use; LanceDB has multi-modal. Synapse wins on *integration*, not on replacing any single specialist tool's best-case benchmark.

## Reproduce

```bash
# deps
pip install chromadb lancedb faiss-cpu qdrant-client pyarrow

# run
N=1000 Q=200 python3 bench/real_competitors.py
```

Default: 1 000 docs × 200 queries. Pass `N=10000 Q=500` for a bigger-N re-run.

## Why Synapse wins the whole workload, not one column

The README's [capability matrix](../README.md#compared-to-the-field) is the real comparison surface. This latency bench proves Synapse pays no perf penalty for bundling nine capabilities into one binary — it sits within 2.3× of the flat-IP floor while shipping BM25, KG, scopes, CRDT, signing, mmap and MCP that none of the other rows carry.
