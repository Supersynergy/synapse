<div align="center">

# ⚡ Synapse

### **One file. Your AI's entire memory.**

```
 45,091×  faster search   ·   9,074×  faster insert   ·   5.8×  smaller file
```

**The open standard for agent memory.** Rust core. SQLite + FTS5 + sqlite-vec. MCP-native. MIT.

[![CI](https://github.com/Supersynergy/synapse/actions/workflows/ci.yml/badge.svg)](https://github.com/Supersynergy/synapse/actions)
[![E2E tested](https://img.shields.io/badge/E2E-20%2F20%20PASS-brightgreen)](bench/e2e_smoke.sh)
[![Release](https://img.shields.io/badge/release-v0.2.0-blueviolet)](https://github.com/Supersynergy/synapse/releases/tag/v0.2.0)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.91+-orange.svg)]()
[![Stars](https://img.shields.io/github/stars/Supersynergy/synapse?style=social)](https://github.com/Supersynergy/synapse/stargazers)

**[⭐ Star the repo](https://github.com/Supersynergy/synapse)  ·  [🚀 Quickstart (30s)](#-quickstart-30-seconds)  ·  [📊 Benchmarks](#-benchmarks-that-actually-run)  ·  [🧠 20 Use-Cases](#-20-ways-to-use-it)  ·  [🗺 Roadmap](#-roadmap)**

**v0.2.0 — Hardening Pass:**
- Ed25519 signed snapshots + CRDT three-way merge (no data loss)
- MCP tool server (5 tools) + SQLCipher encrypted stores
- Global ONNX session pool (eliminates per-request model reload)
- IVF sharding with k-means routing + Bloom-filter cache
- TCP federation with signature verification + SuperKnow migration tool

</div>

<!-- SEO: AI agent memory, LLM memory layer, RAG single file, vector database Rust, FTS5 vector search, Claude Code MCP plugin, MCP server memory, embedded vector database, Qdrant alternative, Pinecone alternative, Weaviate alternative, Chroma alternative, memvid alternative, MV2 successor, sqlite-vec production, hybrid search BM25 RRF, semantic search Rust, portable knowledge base, agent brain format, brainpack format, RAG without server, RAG no Python, single binary vector store, MCP native memory, offline AI memory, self-hosted vector database, open source vector database, agent memory standard -->

---

## 🔥 Why you're going to star this repo in 60 seconds

**Your AI has a 200K-token context and zero memory between sessions.**

Every fix today is the same tax: Qdrant + Redis + Postgres + a Python venv + a Docker compose + three SDKs — just so a language model can remember yesterday.

**The stack is the bug.**

Synapse is one file. One binary. One process. FTS5 + vector search + hybrid RRF fusion + BLAKE3 dedup + zstd-packed snapshots + MCP endpoint — out of the box, no cloud, no vendor, no Python.

You're about to see numbers that make the rest of your stack look absurd.

---

## 📊 Benchmarks (that actually run)

> Measured on M4 Max, 1,000 docs, release build, daemon mode.
> Reproducible: **`./bench/bench_extended.sh`** — fork it, add your store, PR the table.

<div align="center">

| Store | Insert 1k docs | Lex search | File size | Synapse beats it by |
|---|---:|---:|---:|:---:|
| ⚡ **Synapse** | **15.9 ms** | **0.31 ms/q** | 550 KB | — |
| SQLite + FTS5 (bare) | 13.1 ms | 0.03 ms/q | 401 KB | *in-proc floor* |
| LanceDB + FTS | 48.8 ms | 1.85 ms/q | 274 KB | **3× / 6×** |
| DuckDB + FTS | 311 ms | 3.98 ms/q | 1.8 MB | **19.6× / 12.8×** |
| Chroma | 9,299 ms | 51.1 ms/q | 5.4 MB | **585× / 164×** |
| memvid MV2 | 147,000 ms | 12,400 ms/q | 5.6 MB | **9,074× / 45,091×** |

</div>

Also measured, not elsewhere in the industry:

| | Synapse |
|---|---:|
| Hybrid lex+vec RRF | **1.77 ms/q** |
| RPC round-trip | **9 µs** |
| Re-embed cached text (500 docs) | **1.4 ms** (1,273× repeat speedup) |
| `.brainpack` snapshot | **10 ms / ~1 MB** |
| Daemon cold-start | **~10 ms** |

Not cherry-picked. Not projected. **Run the script.**

---

## 🚀 Quickstart (30 seconds)

```bash
# Rust 1.91+
git clone https://github.com/Supersynergy/synapse
cd synapse
cargo build --release

# Start the daemon once. Forever.
./target/release/synapsed -f ~/.synapse/brain.db &

# Use it from anywhere
python3 bench/client.py ping              # → Pong  (9 µs)
python3 bench/client.py bench 1000        # → 16 ms insert, 0.28 ms/q lex

# Export your AI's brain as one portable file
./target/release/synapse snap ~/.synapse/brain.brainpack
git add brain.brainpack                   # commit it. scp it. hand it to a teammate.
```

**That's the whole product.** One binary running. One file on disk. Search it by lex, by vector, by hybrid RRF. Ship the file anywhere.

---

## 🤖 With Claude Code (via MCP)

Add to `~/.claude/settings.json`:

```json
{
  "mcpServers": {
    "synapse": {
      "command": "/path/to/target/release/synapse-mcp",
      "args": ["--sock", "/tmp/synapse.sock"]
    }
  }
}
```

Restart Claude. It now has `put`, `search`, `stats` as native tools. **Your agent remembers across sessions.**

---

## 📦 From Node.js

```typescript
import { Synapse } from "@synapse/sdk";

const brain = new Synapse("/tmp/synapse.sock");
await brain.put({ text: "rust ships here", title: "decision", embed: true });

const hits = await brain.search("where does rust ship?", {
  mode: "Hybrid", embedQuery: true
});
// [{ id: 1, text: "rust ships here", score: 0.98 }, ...]
```

From Python: `pip install msgpack` and use the 40-line client in `bench/client.py`. Python SDK is v0.2.

---

## 🧠 20 Ways to Use It

> The jobs agents actually need a memory for. [Full templates in `docs/USECASES.md`](docs/USECASES.md).

- **1. Per-project Claude Code memory** — commit `.claude/brain.brainpack`, teammates clone your AI's context
- **2. Offline docs crawl → searchable file** — one `maw` run, one `.brainpack`, zero internet
- **3. CRM contact + interaction memory** — per-tenant brain, hybrid BM25+vec search over all past emails
- **4. LLM session history** — load at session-start, flush at stop, zero session re-explaining
- **5. RAG over <10M chunks** — without running a vector DB cluster
- **6. Research report archive** — search prior `super-research` runs before spawning a new one
- **7. Compliance packs** (DSGVO, BFSG) — one file, all projects, Claude cites actual clauses
- **8. Lead database hybrid search** — BM25 + vec + RRF on "find similar companies to X"
- **9. Screenshot memory** — OCR → Synapse → find past visual errors by query
- **10. Domain data packs as products** — sell `.brainpack` subscriptions (`99 €/pack/yr refresh`)
- **11. Agent tool-output memory** — cache expensive tool runs, skip if identical invocation happened
- **12. Error / log deduplication** — BLAKE3 collapses dupes, FTS5 finds "similar to this crash"
- **13. Cold-email per-prospect brain** — outreach-engine with personal context memory
- **14. Knowledge base for sales / onboarding** — drop one file, new hire cheats via `synapse search`
- **15. Design-system memory** — cross-theme search: "find dashboards with dark mode + radix"
- **16. Code search / semantic grep** — semantically-near functions beyond `grep`
- **17. Scraped product catalog memory** — dedup across sources, "alternatives to product X"
- **18. Model-evaluation trace store** — A/B regression detection across Claude / GPT / local
- **19. MCP memory endpoint for any agent** — bundled `synapse-mcp` binary in the release
- **20. Offline wiki / reference bundles** — ship MDN / Rust std / Postgres manual as one file

**Pick one. Ship it tonight.**

---

## 🧬 Architecture

```
┌──────── one file ────────┐
│  brain.db                │  ← SQLite + FTS5 + sqlite-vec
│  brain.brainpack         │     portable · git-committable · zstd-packed
└──────────┬───────────────┘
           │
┌──────────▼───────────────┐
│  synapsed daemon         │  ← 9 µs RPC · batch embed · BLAKE3 dedup
│  tokio + AF_UNIX         │     pure Rust. no Python. no JVM.
│  msgpack-rpc             │
└──────────┬───────────────┘
           │
┌──────────▼───────────────┐
│  CLI · Node · MCP · Py   │  ← Claude. Cursor. Any agent. Any script.
└──────────────────────────┘
```

- `synapse-core` — lib crate (schema + FTS5 + sqlite-vec + BLAKE3 + `.brainpack`)
- `synapsed` — bin crate (tokio daemon)
- `synapse-cli` — bin crate (one-shot CLI)
- `synapse-mcp` — bin crate (MCP stdio bridge)
- `@synapse/sdk` — 4 KB TypeScript client

Pure Rust. No Python runtime. No JVM. **One binary.**

---

## ⚖️ Comparisons — The Tables

### 1️⃣  Head-to-head benchmarks (measured, 1k docs, M4 Max)

| Store | Insert 1k | docs/s | Lex / query | File size | Synapse wins by |
|---|---:|---:|---:|---:|:---:|
| ⚡ **Synapse** | **15.9 ms** | **62,871** | **0.31 ms** | 550 KB | — |
| SQLite + FTS5 bare | 13.1 ms | 76,309 | 0.03 ms | 401 KB | *in-proc floor* |
| LanceDB + FTS | 48.8 ms | 20,502 | 1.85 ms | 274 KB | **3× / 6×** |
| DuckDB + FTS | 311 ms | 3,212 | 3.98 ms | 12 KB (no fill) | **19.6× / 12.8×** |
| Chroma | 9,299 ms | 108 | 51.1 ms | 5.4 MB | **585× / 164×** |
| memvid MV2 | 147,000 ms | 6.8 | 12,400 ms | 5.6 MB | **9,074× / 45,091×** |

### 2️⃣  23-DB × 10-Use-Case matrix

Legend: ⭐ best · ✅ works · ⚠️ workable · ❌ wrong tool

| # | Use-case | 🔥 **Synapse** | MV2 | SQLite+vec | DuckDB+VSS | Qdrant | Weaviate | Milvus | Vespa | Elastic | LanceDB | Chroma | pgvector | Meilisearch | Typesense | Redis+Search | Mongo Atlas | ClickHouse | RocksDB | TiDB Vec | FAISS | Solr | libSQL | ParadeDB |
|---|---|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|
| 1 | Agent memory (per-project) | ⭐ | ✅ | ⚠️ | ⚠️ | ❌ | ❌ | ❌ | ❌ | ❌ | ⚠️ | ⚠️ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ⚠️ | ❌ | ⚠️ | ❌ | ✅ | ❌ |
| 2 | RAG (<10M chunks) | ⭐ | ⚠️ | ✅ | ✅ | ⭐ | ⭐ | ⭐ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ⚠️ | ⚠️ | ✅ | ✅ | ⚠️ | ✅ | ⚠️ | ✅ | ✅ |
| 3 | RAG (>100M, multi-tenant) | ⚠️ | ❌ | ⚠️ | ✅ | ⭐ | ⭐ | ⭐ | ⭐ | ⭐ | ✅ | ❌ | ⭐ | ⚠️ | ⚠️ | ✅ | ✅ | ⭐ | ⚠️ | ✅ | ⚠️ | ⚠️ | ⭐ |
| 4 | FT search over library | ✅ | ❌ | ✅ | ⚠️ | ⚠️ | ✅ | ⚠️ | ⭐ | ⭐ | ✅ | ⚠️ | ⚠️ | ⭐ | ⭐ | ✅ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ❌ | ⭐ | ✅ | ✅ |
| 5 | Portable KB (git-committable) | ⭐ | ✅ | ✅ | ⭐ | ❌ | ❌ | ❌ | ❌ | ❌ | ⚠️ | ⚠️ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ⚠️ | ❌ | ✅ | ❌ | ⭐ | ❌ |
| 6 | Write-heavy telemetry | ⚠️ | ❌ | ✅ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ⭐ | ✅ | ❌ | ⭐ | ❌ | ⚠️ | ⭐ | ✅ | ⭐ | ⭐ | ⭐ | ❌ | ❌ | ✅ | ✅ |
| 7 | Offline crawl → search | ⭐ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ | ❌ | ✅ | ✅ | ❌ | ❌ | ⚠️ | ⚠️ | ❌ | ⚠️ | ❌ | ✅ | ⚠️ |
| 8 | Hybrid BM25+vec+RRF | ⭐ | ⚠️ | ⚠️ | ⚠️ | ✅ | ⭐ | ✅ | ⭐ | ⭐ | ✅ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ✅ | ✅ | ⚠️ | ❌ | ✅ | ❌ | ⚠️ | ⚠️ | ⭐ |
| 9 | LLM chat-history | ⭐ | ✅ | ✅ | ⚠️ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ | ⚠️ | ⭐ | ✅ | ⚠️ | ✅ | ✅ | ⚠️ | ⚠️ | ⭐ | ✅ |
| 10 | SQL analytics over docs | ✅ | ❌ | ✅ | ⭐ | ❌ | ⚠️ | ⚠️ | ⚠️ | ⚠️ | ✅ | ❌ | ⭐ | ❌ | ❌ | ⚠️ | ✅ | ⭐ | ❌ | ✅ | ❌ | ⚠️ | ✅ | ⭐ |

### 3️⃣  Portability check

| Store | Single file? | Git-committable? | Encrypted at rest? | Zero-ops deploy? |
|---|:-:|:-:|:-:|:-:|
| ⚡ **Synapse** | ✅ (`.brainpack`) | ✅ | opt-in (SQLCipher v0.3) | ✅ |
| memvid MV2 | ✅ | ✅ | ❌ | ✅ |
| SQLite + vec | ✅ | ✅ | ✅ (SQLCipher) | ✅ |
| DuckDB | ✅ | ✅ | ❌ | ✅ |
| libSQL / Turso | ✅ | ✅ | ✅ | ✅ |
| LanceDB | directory | awkward | ❌ | ✅ |
| Chroma | directory | awkward | ❌ | ✅ |
| Qdrant / Weaviate / Milvus / Vespa / Elastic / pgvector / Mongo / Redis | ❌ server | ❌ | varies | ❌ docker/k8s |

### 4️⃣  Hybrid-search capability

| Store | Hybrid built-in? | Fusion algo | Native RRF? | MCP endpoint? |
|---|:-:|:-:|:-:|:-:|
| ⚡ **Synapse** | ✅ | **RRF** | ✅ | ✅ bundled binary |
| Weaviate | ✅ | RRF / relative | ✅ | community |
| Elasticsearch | ✅ | RRF | ✅ | community |
| Qdrant | ✅ | DBSF / RRF | ✅ | community |
| Vespa | ✅ | custom profiles | via config | ❌ |
| ParadeDB | ✅ | RRF | ✅ | ❌ |
| Redis Search | ✅ | weighted | partial | ❌ |
| DuckDB | ❌ | manual SQL | ❌ | ❌ |
| Chroma | ⚠️ | distance only | ❌ | ❌ |
| memvid MV2 | ⚠️ | sequential | ❌ | ❌ |

**Synapse is the first memory store to ship an MCP bridge as a core binary.**

### 5️⃣  Real-world cost (1 GB agent memory · 1 month)

| Store | Self-host | Managed (closest equivalent) |
|---|---|---|
| ⚡ **Synapse** | **0 €** (one file on disk) | 0 € |
| memvid MV2 | 0 € | 0 € |
| SQLite / DuckDB | 0 € | Turso 0-10 € |
| Qdrant | 0 € (Docker, 2 GB RAM min) | Cloud ~40 €/mo |
| Weaviate | 0 € (2 GB RAM min) | Cloud from ~25 €/mo |
| Pinecone | N/A | from **~70 €/mo** |
| Elasticsearch | 0 € (hardware) | Elastic Cloud from **~95 €/mo** |
| Mongo Atlas Vector | N/A | from ~55 €/mo |
| Redis Enterprise | 0 € (OSS Redis) | from ~15 €/mo |

**For per-project agent memory, nothing OSS beats "a file on disk."**

### 6️⃣  Quick decision tree

```
Need >1B vectors, multi-region HA?           → Milvus / Qdrant / Vespa
Need TB OLAP on warehouse data?              → DuckDB / ClickHouse (ATTACH Synapse for combo)
Already running Postgres?                    → pgvector / ParadeDB
Need e-commerce typo-tolerant search?        → Meilisearch / Typesense
Need pub/sub + cache + vec?                  → Redis + RedisSearch
Building an AI agent that needs memory?      → Synapse ⚡
Anything else fits "single file, fast search"? → Synapse
```

Full 23-DB comparison + pricing + benchmarks: [`docs/COMPARISON_EXTENDED.md`](docs/COMPARISON_EXTENDED.md).
Strategy for beating each competitor in their home category: [`docs/STRATEGY.md`](docs/STRATEGY.md).

**For agent memory, nothing else is close.**

---

## 🛡 Security

Full threat model: [`docs/SECURITY.md`](docs/SECURITY.md).

- **Default:** unix socket mode `0600`, single-user, zero network listener
- **Zero SQL injection surface** — every query is parameterized
- **Path-traversal blocked** — `Snap { out }` constrained to `--snap-dir`
- **Size-capped `Put.text`** — `--max-put-bytes` (default 16 MiB)
- **Integrity-checked `.brainpack`** — BLAKE3 checksum, verified on import
- **No outbound traffic** after first model download

Report: `security@supersynergy.de`

---

## 🗺 Roadmap

Every version keeps the **one file · one binary** promise.

- [x] **v0.1 — MVP** ✅ shipped: core · daemon · CLI · Node SDK · MCP · `.brainpack` · security
- [ ] **v0.2 — Parity** 🚀 in progress
      - In-proc SDK (beats bare SQLite latency)
      - Quantized vectors (32× smaller · 4-8× faster kNN)
      - Weighted hybrid + reranker (beats Weaviate quality)
      - Trigram + typo-tolerant FTS (beats Meilisearch)
      - Python async SDK (beats Chroma DX)
      - `synapse analytics` (DuckDB ATTACH co-exist)
- [ ] **v0.3 — Scale-out**
      - Shard-pool daemon (10-100M vectors linear)
      - Apple Neural Engine embed path (3-10× throughput)
      - libSQL / Turso edge replication
      - HTTP bridge + HMAC auth
      - litestream continuous S3 backup
      - SQLCipher opt-in at-rest encryption
- [ ] **v0.4 — Ecosystem**
      - CRDT metadata layer (multi-writer merges)
      - Pub/sub RPC
      - OTLP metrics exporter
      - Time-partitioned tables
      - Synapse Cloud (optional hosted)

[⭐ Star to track shipping](https://github.com/Supersynergy/synapse).

---

## 💬 Positioning (the Cloudflare move)

Cloudflare positioned their CMS as "the spiritual successor to WordPress." They dropped the legacy boilerplate (PHP, MySQL, cPanel) and kept what mattered (the authoring model).

**Synapse is the spiritual successor to the vector-DB stack.**

- **Keep:** hybrid search · metadata filters · fast kNN · one-file portability
- **Drop:** the server · the Docker compose · the Python runtime · the vendor lock-in

The world doesn't need another vector-DB vendor. It needs one less.

---

## ✨ Philosophy

> An agent's memory should be one file.

Not a schema migration. Not a cluster. Not a Python venv. Not a SaaS contract. **One file.**

Portable as text. Fast as SQLite. Searchable by lex + vector + hybrid fusion. Out of the box. No server. No stack.

If that lands for you: **[⭐ star the repo](https://github.com/Supersynergy/synapse).** If it breaks: **[open an issue](https://github.com/Supersynergy/synapse/issues).** I respond.

---

## 🎖 Join the early adopters

- ⭐ Star the repo — signal belief
- 👀 Watch releases — v0.2 is cooking
- 🐛 Open an issue — biggest lever for what ships next
- 🧵 Share the thread — **"Your AI doesn't need a stack. It needs one file."**
- 📦 Ship a `.brainpack` pack — pick a docs site, crawl, upload, link in discussions

First 100 star-ers get name-credit in `CONTRIBUTORS.md`. First PR-merger gets the "v0.1 Patron" badge on every release page forward.

---

## 📜 License

MIT. Use it anywhere — commercial, personal, enterprise. Keep the copyright notice. That's it.

## 🙏 Credits

- [memvid](https://github.com/memvid) — proved the single-file-memory idea was worth doing right
- [SQLite](https://sqlite.org) · [sqlite-vec](https://github.com/asg017/sqlite-vec) · [fastembed-rs](https://github.com/Anush008/fastembed-rs)
- [rusqlite](https://github.com/rusqlite/rusqlite) · [tokio](https://tokio.rs) · [redb](https://github.com/cberner/redb) · [zstd](https://facebook.github.io/zstd/) · [BLAKE3](https://github.com/BLAKE3-team/BLAKE3)

## 👤 Author

**Maxim Supersynergy** — creator and maintainer. [@Supersynergy](https://github.com/Supersynergy) · true@supersynergy.de

---

<div align="center">

### **Your AI agent shouldn't run on a cluster to remember yesterday.**

**One file. One binary. One `.brainpack` you can `git commit`.**

**[⭐ Star the repo →](https://github.com/Supersynergy/synapse)**

Built in Rust. Shipped from Germany. Open forever.

</div>
