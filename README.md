<div align="center">

# Synapse

### One file. Your AI's entire memory.

Drop it in your repo. Your agent remembers every conversation — offline, portable, signed.

`Rust` · `MCP-native` · `MIT` · sub-20 ms across every category

[![CI](https://github.com/Supersynergy/synapse/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/Supersynergy/synapse/actions)
[![Release](https://img.shields.io/github/v/tag/Supersynergy/synapse?label=release)](https://github.com/Supersynergy/synapse/releases)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

</div>

---

```bash
# install (crates.io pending — installs straight from GitHub)
cargo install --locked --git https://github.com/Supersynergy/synapse synapse-cli synapsed synapse-mcp

# run
synapsed -f ~/brain.db &

# remember
synapse put "we chose Rust because single-binary shipping matters"
synapse search "why Rust?"            # ≈ 0.3 ms
synapse snap ~/brain.brainpack        # one portable, signable file
```

Wire into Claude Code — memory across every session:

```json
{ "mcpServers": { "synapse": {
    "command": "synapse-mcp",
    "args": ["--sock", "/tmp/synapse.sock"]
  } } }
```

---

## Measured, on a 2024 laptop

All numbers from `bench/RESULTS-V1.md`. Reproduce with `bash bench/bench_20_usecases.sh`.

| op (10 k docs, M4 Max) | Synapse | runner-up |
|------|--------:|----------:|
| cold open | **0.69 ms** mmap | SQLite WAL ~ 7 ms |
| BM25 query | **23 µs / q** | Meilisearch ~ 1 ms |
| vector kNN k = 10 | **22 µs / q** | LanceDB ~ 50 µs |
| CRDT merge, 200 ops | **0.59 ms** | Automerge baseline |
| sign + verify manifest | **25 µs each** | stock `ed25519-dalek` |
| pack → ship → verify → mmap | **12 ms** | nothing equivalent |

## What you actually get in one file

- **BM25 full-text** (Tantivy) and **HNSW + int8 vector** in the same index
- **Temporal knowledge graph** — `Supersedes`, `References`, `Contradicts`, `Summarises`
- **Memory scopes** — Global / User / Session / Project
- **CRDT sync** (Automerge) for multi-writer without a server
- **Ed25519-signed `.brainpack`** — ship memory as a subscription-grade artefact
- **Zero-copy `mmap` reader**, 5.7 µs RPC, BLAKE3 content-addressed chunks

## How it compares

Seven of the nine agent-memory capabilities across 20 incumbents are **missing** from every competitor — details in [`docs/COMPARISON-V1.md`](docs/COMPARISON-V1.md). The short version:

```
                 BM25  Vector  KG   Scopes  CRDT  Sign  OneFile
SQLite           ✅    ext     —    —       —     —     ✅
Qdrant           –     ✅      —    ns      —     —     —
Meilisearch      ✅    –       —    —       —     —     —
LanceDB          ✅    ✅      —    —       —     —     partial
memvid           ✅    —       —    —       —     —     ✅
mem0 / Graphiti  —     delegate ✅  ✅      —     —     —
Synapse          ✅    ✅      ✅   ✅      ✅    ✅    ✅
```

## 60-second tour of the idea

Your agent has a 200 K-token context and zero memory between sessions. The usual fix is a pipeline — Qdrant, Redis, a Python venv, a Docker compose, three SDKs — so a language model can remember what it did yesterday.

**The stack is the bug.** Synapse is one file, one binary, one process. Every capability above lives inside a `.synx` container you can `git commit`, `scp`, sign and hand to a teammate. That's it.

## Deeper dives

- [`docs/CLAUDE-CODE-MEMORY.md`](docs/CLAUDE-CODE-MEMORY.md) — five real agentic workflows (per-project, code-change, research, CRM, compliance)
- [`docs/SYNX-FORMAT-V2.md`](docs/SYNX-FORMAT-V2.md) — binary format spec (CC0)
- [`docs/BRAINPACK-V2.md`](docs/BRAINPACK-V2.md) — distribution wrapper + signing
- [`docs/COMPARISON-V1.md`](docs/COMPARISON-V1.md) — 20-tool head-to-head
- [`bench/RESULTS-V1.md`](bench/RESULTS-V1.md) — 50-usecase benchmark, 5 categories
- [`docs/LICENSE-STRATEGY.md`](docs/LICENSE-STRATEGY.md) — MIT + CC0 split + monetisation paths
- [`docs/USECASES.md`](docs/USECASES.md) — 20 deployment recipes
- [`docs/STRATEGY.md`](docs/STRATEGY.md) — how Synapse tops each competitor on their home turf
- [`CHANGELOG.md`](CHANGELOG.md) — v0.2.0 → v1.0.0 history

## Ecosystem integrations

- **Claude Code** — drop in the MCP block above, restart, you have persistent memory
- **Any MCP agent** — Cursor, Cline, Continue, Aider all accept the same config
- **Node.js** — `@synapse/sdk` in [`sdk/node`](sdk/node)
- **Python** — [`sdk/python/synapse_reader.py`](sdk/python/synapse_reader.py) (stdlib + `zstandard` + `blake3`)
- **DuckDB analytics** — `ATTACH 'brain.db' AS s (TYPE sqlite, READ_ONLY);` and query the v1 engine directly

## License

MIT for the code. CC0 for the format specification. The format outlives the repo — any language, any product, no asking.

---

<div align="center">

Built in Rust. Shipped in Germany. Open forever.

*by [Maxim Supersynergy](https://github.com/Supersynergy)*

</div>
