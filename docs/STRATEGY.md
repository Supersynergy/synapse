# How Synapse Tops Each Competitor in Its Home Category

A category-by-category playbook for beating the incumbent at its own game. Some are already shipped, some are roadmap. Each entry lists **what the incumbent wins today**, **why Synapse can beat them**, and **the concrete engineering path**.

---

## 1. vs. memvid (MV2) — "single file portable memory"

**Incumbent wins on:** portability (already matched).
**Synapse wins on:** everything measurable (9,074× insert, 45,091× lex, 5.8× smaller file, MCP, hybrid RRF, cache).
**Status:** ✅ won, measured.

## 2. vs. bare SQLite + FTS5 — "fastest embedded FTS"

**Incumbent wins on:** in-proc lex latency (0.026 ms vs Synapse 0.28 ms — RPC cost).
**Why we can close it:** offer an in-proc `synapse-core` mode that skips the socket.
**Path (v0.2):**
- Add `use synapse_core::Store;` public API — already present.
- Publish `@synapse/sdk-embedded` — Node N-API binding to `synapse-core`. Zero RPC.
- Projected parity with bare SQLite + keep Synapse's hybrid + cache + snapshot.

## 3. vs. DuckDB — "embedded analytics on the same file"

**Incumbent wins on:** grouped aggregations, window functions, TB scans, Parquet.
**Why we don't beat DuckDB at its own game:** we shouldn't. DuckDB can `ATTACH` a Synapse file zero-copy.
**Path:** ship `synapse analytics 'SELECT ...'` subcommand that shells out to DuckDB with `ATTACH brain.db AS s (TYPE SQLITE); <query>`. Two engines, one file, zero ETL. **Best of both.**

## 4. vs. Qdrant — "billion-scale ANN"

**Incumbent wins on:** 1B+ vectors, distributed sharding, gRPC.
**Why we don't compete head-on:** single-node SQLite is wrong for 1B.
**Path to catch for 10M-100M (the realistic SMB range):**
- **Quantized vectors** (v0.2): sqlite-vec supports `int8` / bit-packed. 32× smaller, 4-8× faster kNN.
- **IVF + HNSW hybrid** (v0.2): partition vectors, search top-k clusters.
- **Shard-pool daemon** (v0.3): one `synapsed` per shard file (`brain-00.db` ... `brain-NN.db`). Hash-route on `doc.id`. Linear horizontal scale on one box.
- **Synapse Cloud** (future): optional hosted version for true distributed; keep OSS single-node.

## 5. vs. Weaviate — "hybrid search quality"

**Incumbent wins on:** learned sparse + dense + reranker pipeline, modular retrievers.
**Path:**
- **Learnable RRF weights** (v0.2): expose `alpha` / `beta` params per query (`mode=HybridWeighted { lex_weight, vec_weight }`).
- **Pluggable reranker** (v0.3): optional cross-encoder call after retrieval. Ship `bge-reranker-v2-m3` ONNX as a named feature flag.
- **Metadata filter pushdown** (already has `meta` JSON column; need query DSL).

Target: match Weaviate's hybrid quality on BEIR-style benchmarks, stay single-binary.

## 6. vs. Meilisearch / Typesense — "typo-tolerant FTS"

**Incumbent wins on:** out-of-box typo tolerance, faceted search, synonym maps.
**Path (v0.2):**
- **Trigram tokenizer** for FTS5 (SQLite extension exists).
- **Edit-distance rerank pass** on top-K lex results: cheap Levenshtein on the candidate set.
- **Synonym table** — native SQL, join at query time.
- Match Meilisearch on accuracy; keep Synapse's hybrid + one-file + MCP.

## 7. vs. Elasticsearch — "log aggregation"

**Incumbent wins on:** cluster ingest, aggregations, Kibana, alerting.
**Why we don't compete at TB ingest:** wrong tool.
**Path for GB-scale structured logs (the majority of use-cases):**
- **Shard-pool daemon** (see #4).
- **Time-partitioned tables** (`docs_2026_04`, `docs_2026_05`) via a daemon helper.
- **OTLP exporter** from the daemon → Grafana.
Stays one-binary, covers 80% of "we need an Elastic" asks in startups.

## 8. vs. pgvector — "Postgres-native vectors"

**Incumbent wins on:** Postgres shop integration, multi-writer OLTP.
**Why pgvector wins when you already run Postgres:** it's in the box. We can't beat that.
**Path for teams starting fresh:**
- Synapse ships as one binary, one file. Postgres ships as a fleet.
- We can't beat pgvector in its home. We can make sure greenfield projects never need it.

## 9. vs. Chroma — "Python-friendly RAG store"

**Incumbent wins on:** Python-first ergonomics.
**Measured:** Synapse is 585× faster insert, 164× faster lex. Ergonomics need parity.
**Path (v0.2):**
- **`synapse-py` SDK** — async Python client with the same surface as Chroma's `add()`/`query()`.
- **Drop-in adapter** `from synapse import ChromaCompat; col = ChromaCompat()` — your LangChain code works unchanged.
- Keep Rust core, ship Python parity at the edges.

## 10. vs. LanceDB — "columnar vector store"

**Incumbent wins on:** columnar storage, large-batch vec ops, Arrow interop.
**Measured:** Synapse is 3× faster on 1k-doc FTS + 6× faster on lex query. LanceDB wins on analytics-scale.
**Path:**
- **DuckDB ATTACH** (see #3) → columnar analytics on Synapse files.
- No plan to reimplement LanceDB's columnar engine. Use the right tool.

## 11. vs. Turso / libSQL — "edge SQLite"

**Incumbent wins on:** edge replication, global latency.
**Path:** Synapse already uses SQLite underneath. `TURSO_DATABASE_URL` drop-in via libSQL driver in `synapse-core` (v0.3 feature flag `libsql`). Same API, replicated storage. **One binary, edge-ready.**

## 12. vs. Redis + RedisSearch — "cache + pub/sub + ANN"

**Incumbent wins on:** sub-ms cache, pub/sub, vector sidecar.
**Path for the "I just need a fast local memory" use-case:**
- Synapse's 9 µs RPC is competitive with Redis' 100 µs.
- Missing: pub/sub. **Roadmap v0.4:** add `Subscribe { pattern }` RPC — emit events when docs change.
- Keep it optional; most agent memory stores don't need pub/sub.

## 13. vs. MongoDB Atlas Vector — "document DB with vectors"

**Incumbent wins on:** managed, flexible schema.
**Path:** Synapse's `meta JSONB` column + FTS + vec covers the same shape locally. Not a drop-in migration — **positioning**: "never needed an Atlas instance in the first place."

## 14. vs. ClickHouse — "OLAP at speed"

**Incumbent wins on:** PB-scale OLAP, materialized views.
**Why we don't compete:** wrong tool. ATTACH pattern via DuckDB if you need OLAP on a Synapse file.

## 15. vs. RocksDB / fjall — "embedded KV"

**Incumbent wins on:** raw KV throughput.
**Why we don't compete:** Synapse is not a KV store. If you need KV, use redb / fjall / sled. Synapse already uses redb for the embed cache.

## 16. vs. ParadeDB — "Postgres + BM25 + RRF"

**Incumbent wins on:** Postgres-native FTS + hybrid.
**Path:** covered by #8 — if you have Postgres, use ParadeDB; if you don't, don't start running Postgres for this.

## 17. vs. Pinecone — "managed vector DB"

**Incumbent wins on:** zero-ops managed service.
**Path:**
- **Synapse Cloud** (future, optional): hosted daemon with `.brainpack` sync.
- For self-host: we're already one binary. Ops cost = 0.

## 18. vs. Vespa — "search+rank at scale"

**Incumbent wins on:** custom rank profiles, tensor expressions.
**Path:** not for OSS v0.1. Vespa is a Ferrari; Synapse is the best pocket knife.

## 19. vs. Solr — "full-text search legacy"

**Incumbent wins on:** enterprise Java FTS.
**Path:** same as Meilisearch (#6). FTS5 + tokenizer extensions cover 95% of Solr use-cases for teams under 1M docs.

## 20. vs. FAISS — "pure ANN library"

**Incumbent wins on:** best-in-class ANN C++.
**Why we don't compete:** FAISS is a library, not a store. Synapse uses `sqlite-vec` which rivals FAISS on HNSW. For teams that only want ANN, FAISS still wins. For teams that want memory + storage + search, Synapse wins.

---

## The Unified Strategy: Don't Beat Everyone Everywhere

Synapse's thesis is **not** "be the fastest at everything."
It's **"be the only right answer when the question is 'my agent needs memory.'"**

For every adjacent category, Synapse either:
- **Co-exists** (DuckDB ATTACH, libSQL replication, FAISS as optional backend)
- **Wins on portability** (one file beats a cluster)
- **Concedes honestly** (billion-vector ANN → Milvus/Qdrant)

The goal is that the first thing any new AI project reaches for — across the full category list above — is Synapse. **Agent memory, hybrid search, portable KB, MCP-native.** If one of those phrases matches your problem, Synapse is the answer.

---

## Concrete Roadmap (What Actually Ships)

| Version | Target | Beats |
|---|---|---|
| **v0.1** (shipped) | core + daemon + CLI + MCP + .brainpack + bench | MV2 by 4 orders of magnitude |
| **v0.2** | in-proc SDK · quantized vec · typo-tolerant FTS · weighted RRF · Python SDK · `analytics` cmd | SQLite bare (in-proc), Meilisearch (typo), Chroma (Python), DuckDB (co-exist), Weaviate (weighted hybrid) |
| **v0.3** | shard-pool daemon · reranker · libSQL · ANE embed · HMAC auth · HTTP bridge · litestream | Qdrant/Elastic (10M-100M scale), Turso (edge), Weaviate (quality) |
| **v0.4** | CRDT metadata · pub/sub · time-partitioned tables · OTLP · Synapse Cloud | Redis (pub/sub), MongoDB (multi-writer), Pinecone (managed option) |

Each version keeps the **one file, one binary** promise. If a feature breaks that, it goes into an optional feature-flag, never default.
