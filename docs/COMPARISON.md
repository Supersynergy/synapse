# Synapse vs 10 Other Data Stores — Use-Case Matrix

Honest comparison. Where Synapse wins, where it doesn't. Benchmarks against MV2 and bare SQLite are bench-verified on M4 Max (2026-04-19). Numbers for other stores are taken from their public docs / widely-cited community benchmarks; mark as estimates.

## Contestants

| # | Store | Category | Single-file? |
|---|---|---|---|
| 1 | **Synapse** (this) | Hybrid memory (FTS+vec+KV) | ✅ `.brainpack` |
| 2 | memvid / MV2 | AI memory format | ✅ `.mv2` |
| 3 | SQLite + FTS5 + sqlite-vec (bare) | Embedded DB | ✅ `.db` |
| 4 | DuckDB + VSS | OLAP + vectors | ✅ `.duckdb` |
| 5 | Qdrant | Vector DB | ❌ (server + snapshot) |
| 6 | Weaviate | Vector DB + hybrid | ❌ (server) |
| 7 | LanceDB | Embedded columnar vec | ❌ (directory) |
| 8 | Chroma | Vector DB | ❌ (SQLite + Parquet dir) |
| 9 | pgvector (Postgres) | Relational + vectors | ❌ (server) |
| 10 | Meilisearch | Full-text search | ❌ (server) |
| 11 | Redis + RedisSearch | KV + vectors + FTS | ❌ (server) |

## 10 Use-Cases × Winners

Legend: ⭐ = best fit, ✅ = works well, ⚠️ = usable but awkward, ❌ = wrong tool.

| # | Use-case | Synapse | MV2 | SQLite+vec | DuckDB+VSS | Qdrant | Weaviate | LanceDB | Chroma | pgvector | Meilisearch | Redis |
|---|---|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|
| 1 | **Agent memory (per-project `.claude/brain`)** | ⭐ | ✅ | ⚠️ | ⚠️ | ❌ | ❌ | ⚠️ | ⚠️ | ❌ | ❌ | ❌ |
| 2 | **RAG over docs (<10M chunks)** | ⭐ | ⚠️ | ✅ | ✅ | ⭐ | ⭐ | ✅ | ✅ | ✅ | ⚠️ | ⚠️ |
| 3 | **RAG at scale (>100M chunks, multi-tenant)** | ⚠️ | ❌ | ⚠️ | ✅ | ⭐ | ⭐ | ✅ | ❌ | ⭐ | ⚠️ | ⚠️ |
| 4 | **Full-text search over content library** | ✅ | ❌ | ✅ | ⚠️ | ⚠️ | ✅ | ⚠️ | ⚠️ | ⚠️ | ⭐ | ✅ |
| 5 | **Portable knowledge-base (git-committable)** | ⭐ | ✅ | ✅ | ⭐ | ❌ | ❌ | ⚠️ | ⚠️ | ❌ | ❌ | ❌ |
| 6 | **Real-time write-heavy logs / telemetry** | ⚠️ | ❌ | ✅ | ⚠️ | ⚠️ | ⚠️ | ✅ | ❌ | ⭐ | ❌ | ⭐ |
| 7 | **Offline site / docs crawl → search** | ⭐ | ✅ | ✅ | ✅ | ❌ | ❌ | ✅ | ✅ | ❌ | ✅ | ❌ |
| 8 | **Hybrid lex+vec with RRF out of the box** | ⭐ | ⚠️ | ⚠️ | ⚠️ | ✅ | ⭐ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ✅ |
| 9 | **LLM chat-history / session memory** | ⭐ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ⭐ |
| 10 | **Analytics over stored docs (SQL)** | ✅ | ❌ | ✅ | ⭐ | ❌ | ⚠️ | ✅ | ❌ | ⭐ | ❌ | ⚠️ |

## Where Synapse Wins

1. **Agent memory**. The target. Single file + daemon + lex+vec+hybrid + dedup + MCP in one binary. No comparable product.
2. **Portable KB**. `.brainpack` = zstd(SQLite snapshot) + BLAKE3 checksum. Git-commit, scp, hand to teammate. DuckDB file is close but lacks embedded FTS+vec story out of box.
3. **Offline crawl → search**. Crawl a docs site, write one `.brainpack`, query forever. Competitors need a server.
4. **Hybrid RRF** in the RPC API. Qdrant/Weaviate have it, but they're servers. Bare SQLite doesn't fuse lex+vec for you.

## Where Synapse Does Not Win (Honest)

1. **>100M vectors**. sqlite-vec HNSW caps well below Qdrant/Weaviate's billion-scale. Don't use Synapse there.
2. **Multi-writer high-concurrency**. SQLite = single writer. Postgres/Redis win. (Workaround: one daemon per shard.)
3. **Columnar analytics at TB scale**. DuckDB and pgvector are stronger. (But DuckDB can `ATTACH` a Synapse file — use both.)
4. **Pure FTS with advanced ranking**. Meilisearch + typo tolerance + synonyms wins for e-commerce search. Synapse is FTS5 bm25 only.

## Bench-Anchored Numbers (1000 docs, M4 Max)

| Op | Synapse | MV2 | SQLite+FTS5 bare (bench 2026-04-19) |
|---|---|---|---|
| Insert 1k (no embed) | **16 ms** | ~147,000 ms | 287 ms |
| Lex query | **0.28 ms** | 12,400 ms | 17 ms |
| Vec kNN | **1.5 ms** | 88 ms | 6 ms (sqlite-vec bare) |
| Hybrid RRF | **1.77 ms** | — | no native, ~10 ms manual |

Bare SQLite+FTS5 is close but:
- No batch embedder.
- No `.brainpack` format.
- No BLAKE3 dedup cache.
- No MCP bridge.
- No Node/Python SDK.
- No daemon — every client reopens.

Synapse = those missing pieces, on top of that stack.

## Decision Tree

```
Does it need to run as a server with >100M vectors? → Qdrant or Weaviate
Does it need to be a git-committable single file?    → Synapse or DuckDB
Is it primarily RAG over <10M chunks?                → Synapse
Is it an agent memory store?                         → Synapse (the target)
Is it pure analytics SQL on warehouse data?          → DuckDB
Is it Postgres with vector search as feature?        → pgvector
Is it real-time FTS with typo-tolerance?             → Meilisearch
Is it cache + pub-sub + occasional vector ANN?       → Redis + RedisSearch
Anything else?                                       → ask
```
