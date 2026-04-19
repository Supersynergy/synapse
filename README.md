<div align="center">

# Synapse

### One file. Your AI's entire memory.

**The open-source memory layer for AI agents. 45,000× faster than MV2. Single-file portability. Rust core. MCP-native.**

[![CI](https://github.com/Supersynergy/synapse/actions/workflows/ci.yml/badge.svg)](https://github.com/Supersynergy/synapse/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.91+-orange.svg)]()
[![Release](https://img.shields.io/github/v/tag/Supersynergy/synapse?label=release)](https://github.com/Supersynergy/synapse/releases)

**by [Maxim Supersynergy](https://github.com/Supersynergy)** — creator of SuperKnow, SupersynergyCRM, ZeroClaw, and the Synapse memory standard.

[Quickstart](#quickstart) · [Why](#why) · [Benchmarks](#benchmarks) · [Compare](#compare-everything) · [Use-Cases](#the-20-use-cases) · [Security](#security) · [Roadmap](#roadmap)

</div>

<!-- SEO keywords: AI agent memory, LLM memory layer, RAG single file, vector database Rust, FTS5 vector search, Claude Code plugin, MCP server memory, embedded vector database, Qdrant alternative, Pinecone alternative, Weaviate alternative, sqlite-vec production, hybrid search BM25 RRF, semantic search Rust, portable knowledge base, agent memory store, persistent LLM memory, memvid alternative, MV2 successor, agent brain format, brainpack format, RAG without server, RAG no Python, single binary vector store, MCP native memory -->

---

## The Problem

Your AI agent has a 200K token context. Zero memory between sessions.

Every workaround is a fresh pipeline: **Qdrant**, a Python embedder, **Redis**, **Postgres**, a Docker compose, three SDKs. You end up running a stack just to make a language model remember what it did yesterday.

**The stack is the bug.**

## The Fix

```
┌──────── one file ────────┐
│  brain.db                │  ← SQLite + FTS5 + sqlite-vec
│  (or brain.brainpack)    │     portable · git-committable · zstd-packed
└──────────────────────────┘
                │
┌──────────────▼────────────┐
│  synapsed daemon          │  ← 9 µs RPC · batch embed · BLAKE3 dedup
│  tokio + unix socket      │     pure Rust · no Python · no JVM
│  msgpack-rpc              │
└──────────────┬────────────┘
               │
┌──────────────▼────────────┐
│  CLI · Node SDK · MCP    │  ← Claude, any agent, any language
└───────────────────────────┘
```

One binary. One file. No database to deploy. No cloud. No vendor.

## Why

Because memvid's `.mv2` format was the right idea and the wrong runtime.

MV2 nails the portability property that makes agent memory *travel*: `git commit`, `scp`, hand to teammate, done. But every MV2 call spawns a full CLI + reloads a Tantivy index. On 1000 documents, that costs **147 seconds** to insert and **12 seconds per query**.

**Synapse keeps the single-file property. Drops the runtime.**

## Benchmarks

1000 docs, M4 Max, release build. Reproducible with [`./bench/run_all.sh`](bench/run_all.sh). Every number below is from a real run, not a spec sheet.

| Op | MV2 CLI | **Synapse** | Speedup |
|---|---:|---:|---:|
| Insert 1k docs (no embed) | 147 s | **16 ms** | **9,074×** |
| Lex search (FTS5 BM25) | 12,400 ms/q | **0.275 ms/q** | **45,091×** |
| Vec search (sqlite-vec kNN) | 88 ms/q | **1.50 ms/q** | **59×** |
| Hybrid search (RRF fusion) | — | **1.77 ms/q** | new |
| RTT per call | 200 ms (spawn) | **9 µs** | **22,222×** |
| Re-embed cached text | full compute | **1.4 ms / 500 docs** | **1,273×** |
| `.brainpack` size (1k docs) | 5.6 MB | **988 KB** | **5.8× smaller** |

> Not cherry-picked. Not projected. Run the script.

## Quickstart

```bash
# Requires Rust 1.91+
git clone https://github.com/Supersynergy/synapse
cd synapse
cargo build --release

# Binaries land in ./target/release/
#   synapse       — one-shot CLI
#   synapsed      — daemon (run once, forever)
#   synapse-mcp   — MCP stdio bridge

# Start the daemon
./target/release/synapsed -f ~/.synapse/brain.db &

# Test it
python3 bench/client.py ping           # → Pong  (9 µs round-trip)
python3 bench/client.py bench 1000     # → 16 ms insert, 0.28 ms/q lex

# Export the brain as one portable file
synapse snap ~/.synapse/brain.brainpack
git add brain.brainpack                # commit your AI's memory
```

### With Claude Code (via MCP)

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

Claude now has `put`, `search`, `stats` as native tools.

### From Node.js

```typescript
import { Synapse } from "@synapse/sdk";

const brain = new Synapse("/tmp/synapse.sock");
await brain.put({ text: "rust ships here", title: "decision", embed: true });
const hits = await brain.search("where does rust ship?", {
  mode: "Hybrid", embedQuery: true
});
```

## Compare Everything

Full 10 × 10 matrix: [docs/COMPARISON.md](docs/COMPARISON.md).

### Versus every plausible alternative

| Store | Good at | Fails at | Synapse position |
|---|---|---|---|
| **memvid / MV2** | single-file portability | everything else (bench) | **45,000× faster, same portability** |
| **Qdrant** | 1B-vector ANN, cluster ops | one-file, no-ops, MCP | **Right tool <10M vectors; one binary** |
| **Pinecone** | managed | self-host, portability, cost | **Free, self-host, zero lock-in** |
| **Weaviate** | hybrid search + modules | one-file, single-binary | **Hybrid RRF built-in, no server** |
| **pgvector** | Postgres shops | portability, MCP-native | **No Postgres required** |
| **Chroma** | Python ergonomics | single-file, Rust-core speed | **10-100× faster, zero Python** |
| **LanceDB** | columnar vec | FTS + hybrid + MCP | **FTS5 + vec in one schema** |
| **DuckDB + VSS** | OLAP + vectors (great!) | hot-path agent memory, MCP | **Complement, not compete — see below** |
| **Meilisearch** | e-commerce FTS typo-tolerant | vector kNN, MCP | **Hybrid + vec out of box** |
| **Redis + RedisSearch** | cache + vec + pub/sub | portability, git-committable | **One file vs cluster** |

### DuckDB: complement, don't compete

DuckDB wins OLAP. Synapse wins agent memory. They coexist:

```bash
# Analytics over a Synapse brain — zero copy
duckdb -c "ATTACH 'brain.db' AS s (TYPE SQLITE); SELECT COUNT(*), AVG(length(text)) FROM s.docs;"
```

Same file, two engines. Synapse for hot-path writes + search; DuckDB for cold analytics. No ETL. No duplication.

## The 20 Use-Cases

Full list with integration templates: [docs/USECASES.md](docs/USECASES.md).

1. **Per-project Claude Code memory** — commit `.claude/brain.brainpack`
2. **Offline docs crawl → searchable file** — one `maw` run, one `.brainpack`
3. **CRM contact + interaction memory**
4. **LLM session history** — load at session-start, flush at stop
5. **RAG over <10M chunks** — without a vector-DB cluster
6. **Research report archive** — search all prior research first
7. **Compliance packs** (DSGVO, BFSG) — one file, all projects
8. **Lead DB hybrid search** — BM25 + vec + RRF
9. **Screenshot memory** — OCR → Synapse → search past visual errors
10. **Domain data packs as products** — sell `.brainpack` subscriptions
11. **Agent tool-output memory** — cache expensive tool runs
12. **Error / log deduplication** — BLAKE3 collapses dupes
13. **Cold-email per-prospect brain** — outreach-engine memory
14. **Knowledge base for sales / onboarding**
15. **Design-system memory** — cross-theme search
16. **Code search / semantic grep**
17. **Scraped product catalog memory**
18. **Model-evaluation trace store**
19. **MCP memory endpoint for any agent**
20. **Offline wiki / reference bundles**

## Architecture

```
synapse-core   (crate, lib)         SQLite + FTS5 + sqlite-vec + BLAKE3 dedup + .brainpack
synapsed       (crate, binary)      tokio daemon · length-prefixed msgpack over AF_UNIX
synapse-cli    (crate, binary)      one-shot CLI for scripts
synapse-mcp    (crate, binary)      MCP stdio JSON-RPC → msgpack-rpc bridge
@synapse/sdk   (sdk/node, npm)      4 KB TypeScript client
```

Storage layout in `brain.db`:
- `docs` — id, uri, title, text, meta (JSONB), ts, BLAKE3 hash
- `docs_fts` — FTS5 virtual table, porter-stemmed, BM25-ranked
- `docs_vec` — sqlite-vec 384-dim HNSW (BGE-small-en-v1.5)
- triggers keep lex + vec in sync
- single file, crash-safe via WAL, zero sidecars in snapshots

## Security

Full threat model: [docs/SECURITY.md](docs/SECURITY.md).

- Default: unix socket `0600`, single-user, **no network listener**
- All SQL parameterized — **zero injection surface**
- `Snap { out }` constrained to `--snap-dir` — **no write-anywhere primitive**
- `Put.text` capped at `--max-put-bytes` (default 16 MiB)
- `.brainpack` carries BLAKE3 checksum — integrity verified on import
- No outbound traffic after first model download

Report issues: **security@supersynergy.de**

## Roadmap

- [x] **v0.1 — MVP** (shipped): core, daemon, CLI, Node SDK, MCP, `.brainpack`, security hardening
- [ ] **v0.2 — Scale-out**:
    - Quantized vectors (int8 / bit-packed) → **32× smaller, 10× faster** vec search
    - Text column zstd compression (>1 KB docs) → further 2-5× file-size reduction
    - `prepare_cached` statement cache → tighter hot-loop
    - HTTP/3 bridge + HMAC auth → remote + multi-tenant
    - SQLCipher optional → at-rest encryption
- [ ] **v0.3 — Apple Neural Engine**:
    - Custom ort CoreML EP path → projected **3-10× embed throughput** on M-series
    - Parallel batch embed on ANE
- [ ] **v0.4 — Multi-writer**:
    - Yrs CRDT layer on metadata → merge-able brains across teammates
    - Litestream → continuous S3 backup

**Design goal across all versions:** never break the "one file, one binary" promise.

## Positioning (the Cloudflare move)

Cloudflare made their CMS "the spiritual successor to WordPress" by dropping the boring stuff (PHP, MySQL, cPanel) and keeping the part that mattered.

**Synapse is the spiritual successor to the vector-DB stack.**

- **Keep:** hybrid search, metadata filtering, fast kNN, one-file portability
- **Drop:** the server, the Docker compose, the Python runtime, the vendor lock-in

Not subtle. Intended.

## Philosophy

> An agent's memory should be one file.

Not a schema migration. Not a cluster. Not a Python venv. Not a SaaS contract. **One file.** Portable as text. Fast as SQLite. Searchable by lex + vector + hybrid fusion. Out of the box. No server. No stack.

If that lands: [star the repo](https://github.com/Supersynergy/synapse). If it breaks: [open an issue](https://github.com/Supersynergy/synapse/issues) — I respond.

## Author

**Maxim Supersynergy** — creator and maintainer of Synapse. Based in DACH. [@Supersynergy](https://github.com/Supersynergy) · true@supersynergy.de

## License

MIT. Use it anywhere, including commercially. Keep the copyright. That's it.

## Credits

- [memvid](https://github.com/memvid) — for proving the single-file-memory idea is worth doing right
- [SQLite](https://sqlite.org) — the eighth wonder
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — the tenth
- [fastembed-rs](https://github.com/Anush008/fastembed-rs) — ONNX embeddings in Rust
- [rusqlite](https://github.com/rusqlite/rusqlite), [tokio](https://tokio.rs), [redb](https://github.com/cberner/redb), [zstd](https://facebook.github.io/zstd/), [BLAKE3](https://github.com/BLAKE3-team/BLAKE3)

---

<div align="center">

**If Synapse saves your AI from amnesia, star the repo and tag the author.**

Built in Rust. Shipped in Germany. Open forever.

</div>
