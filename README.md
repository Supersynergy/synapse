<div align="center">

# Synapse

**One file. Your AI's entire memory.**

Single-file. Daemon-mode. Rust. **45,000× faster than MV2.**

[![build](https://github.com/Supersynergy/synapse/actions/workflows/ci.yml/badge.svg)](https://github.com/Supersynergy/synapse/actions)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.91+-orange.svg)]()

[Quickstart](#quickstart) · [Why](#why) · [Benchmarks](#benchmarks) · [Architecture](#architecture) · [Compare](#compare) · [Security](#security)

</div>

---

## The Problem

Your AI agent has a 200K token context window. Zero memory between sessions.

Every workaround is a fresh pipeline: Qdrant, a Python embedder, a Redis cache, a Postgres for metadata, a Docker compose file, and three different SDKs to stitch it all together. You end up running a stack just to make a language model remember what it did yesterday.

**The stack is the bug.**

## The Fix

```
┌──────── one file ────────┐
│  brain.db                │  ← SQLite + FTS5 + sqlite-vec
│  (or brain.brainpack)    │     portable, git-committable, zstd-packed
└──────────────────────────┘
                │
┌──────────────▼────────────┐
│  synapsed daemon          │  ← 9 µs RPC, batch embed, BLAKE3 dedup
│  tokio + unix socket      │     Rust. No Python runtime.
│  msgpack-rpc              │
└──────────────┬────────────┘
               │
┌──────────────▼────────────┐
│  CLI  · Node SDK · MCP   │  ← talk to Claude, any agent, any script
└───────────────────────────┘
```

One binary. One file. No database to deploy. No cloud. No vendor.

## Why

Because memvid's `.mv2` was the right idea and the wrong runtime.

MV2 nails the "single file, no sidecars" property that makes agent memory *portable*: `git commit`, `scp`, hand to a teammate, done. But every memvid call spawns a full CLI + reloads the Tantivy index. On 1000 docs, that costs **147 seconds** to insert and **12 seconds per query**.

Synapse keeps the single-file property. Throws away the runtime.

## Benchmarks

1000 docs, M4 Max, release build. [Reproduce: `./bench/run_all.sh`](bench/run_all.sh).

| Op | MV2 CLI | **Synapse** | speedup |
|---|---:|---:|---:|
| Insert batch (no embed) | 147 s | **16 ms** | **9,074×** |
| Lex search | 12,400 ms/q | **0.275 ms/q** | **45,091×** |
| Vec search | 88 ms/q | **1.5 ms/q** | **59×** |
| Hybrid (RRF fusion) | — | **1.77 ms/q** | new capability |
| RTT / call | 200,000 µs (spawn) | **9 µs** | **22,222×** |
| Re-embed cached text | full compute | **1.4 ms / 500 docs** | **1,273× on repeat** |
| `.brainpack` size (1k docs) | 5.6 MB | **988 KB** | **5.8× smaller** |

Not cherry-picked. Not projected. Run the script.

## Quickstart

### Install

```bash
# Requires Rust 1.91+
git clone https://github.com/Supersynergy/synapse
cd synapse
cargo build --release

# Binaries land in ./target/release/
#   synapse        — one-shot CLI
#   synapsed       — daemon
#   synapse-mcp    — MCP stdio bridge
```

Pre-built binaries: coming with first tagged release.

### 30-second tour

```bash
# Start the daemon once
./target/release/synapsed -f ~/.synapse/brain.db &

# From any script, any language
python3 bench/client.py ping           # → Pong
python3 bench/client.py bench 1000     # → 16 ms insert, 0.28 ms/q lex

# Export the brain as one file
synapse snap ~/.synapse/brain.brainpack
git add brain.brainpack       # commit your AI's memory
```

### With Claude Code (MCP)

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
const hits = await brain.search("where does rust ship", { mode: "Hybrid", embedQuery: true });
console.log(hits);
```

## Architecture

```
synapse-core   (crate, lib)         SQLite + FTS5 + sqlite-vec + BLAKE3 dedup + .brainpack
synapsed       (crate, binary)      tokio daemon, length-prefixed msgpack over AF_UNIX
synapse-cli    (crate, binary)      one-shot CLI for scripts
synapse-mcp    (crate, binary)      MCP stdio JSON-RPC → msgpack-rpc bridge
@synapse/sdk   (sdk/node, npm)      4 KB TypeScript client
```

Storage layout in `brain.db`:
- `docs` — id, uri, title, text, meta (JSONB), ts, BLAKE3 hash
- `docs_fts` — FTS5 virtual table, porter-stemmed
- `docs_vec` — sqlite-vec, 384-dim HNSW, BGE-small embeddings
- triggers keep lex + vec in sync
- single-file, crash-safe via WAL

## Compare

See [docs/COMPARISON.md](docs/COMPARISON.md) for the full 10 × 10 matrix.

| Tool | Good for | Bad for |
|---|---|---|
| **Synapse** | agent memory, RAG ≤10M chunks, portable KBs | billion-scale ANN, multi-writer |
| memvid / MV2 | portability | everything else (see bench) |
| Qdrant | billion-scale ANN | "one file", no-ops deployments |
| pgvector | Postgres shops | single-file portability |
| DuckDB + VSS | OLAP + vectors | live agent memory, MCP |
| Meilisearch | e-commerce FTS with typo tolerance | vector kNN |

## The 20 Use-Cases

Full list with project templates: [docs/USECASES.md](docs/USECASES.md).

Highlights:
1. **Per-project Claude Code memory** — commit `.claude/brain.brainpack`
2. **Offline docs crawl → searchable file** — one `maw` run, one `.brainpack`
3. **CRM contact + interaction memory** — per-contact or per-tenant brain
4. **LLM session history** — load at session-start, flush at stop
5. **RAG over <10M chunks** — without running a vector DB
6. **Research report archive** — search all prior super-research runs
7. **Compliance packs** (BFSG, DSGVO) — one file, all projects
8. **Lead DB hybrid search** — BM25 + vec + RRF fusion
9. **Screenshot memory** — OCR → Synapse, query past visual errors
10. **Domain data packs as products** — sell `.brainpack` subscriptions

## Security

See [docs/SECURITY.md](docs/SECURITY.md) for the full threat model.

- Default: unix socket mode `0600`, single-user, no network listener
- All SQL is parameterized — no injection surface
- `Snap { out }` is constrained to `--snap-dir` (no write-anywhere)
- `Put.text` is capped at `--max-put-bytes` (default 16 MiB)
- `.brainpack` includes BLAKE3 checksum — integrity verified on import

Issues: security@supersynergy.de

## Project Status

- [x] **M0** — masterplan
- [x] **M1** — core crate (5/5 tests)
- [x] **M2** — BLAKE3 embed cache (1,273× speedup on repeat)
- [x] **M3** — daemon (tokio, unix socket, msgpack-rpc)
- [x] **M4** — CLI + Node SDK + MCP bridge
- [x] **M5** — `.brainpack` export/import (zstd + BLAKE3)
- [x] **M6** — benchmark harness + CI
- [ ] **M7** — ANE (CoreML EP) embed, HTTP bridge, SQLCipher, HMAC auth

## Philosophy

> An agent's memory should be one file.

Not a schema migration. Not a cluster. Not a Python venv. One file. Portable as text. Fast as SQLite. Searchable by lex and by vector and by hybrid fusion — out of the box, no server, no stack.

If that lands for you, [star the repo](https://github.com/Supersynergy/synapse). If it breaks, [open an issue](https://github.com/Supersynergy/synapse/issues) — we respond.

## License

MIT. Use it anywhere, including commercially. Do not remove the copyright line; otherwise, no conditions.

## Credits

- [memvid](https://github.com/memvid) — for proving the single-file-memory idea is worth doing right
- [SQLite](https://sqlite.org) — the eighth wonder
- [sqlite-vec](https://github.com/asg017/sqlite-vec) — the tenth
- [fastembed-rs](https://github.com/Anush008/fastembed-rs) — ONNX embeddings in Rust
- [rusqlite](https://github.com/rusqlite/rusqlite), [tokio](https://tokio.rs), [redb](https://github.com/cberner/redb), [zstd](https://facebook.github.io/zstd/), [BLAKE3](https://github.com/BLAKE3-team/BLAKE3)
