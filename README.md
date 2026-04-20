<div align="center">

<img src="assets/banner.svg" alt="Synapse — one file. Your AI's entire memory. 22µs kNN · 23µs BM25 · 0.69ms cold open." width="100%"/>

<sub>Criterion-reproducible numbers: `cargo bench -p synapse-core --features full`. Recall eval on LoCoMo / LongMemEval lands in v0.4 — see [`docs/EVAL-HARNESS.md`](docs/EVAL-HARNESS.md).</sub>

### One file. Your AI's entire memory.

Kill Qdrant + Redis + your Python venv. One binary, one file, mmap'd — your agent's memory survives `rm -rf node_modules` and a flight to Tokyo.

`Rust` · `MCP-native` · `MIT` · **23 µs** BM25 · **22 µs** kNN · **0.69 ms** cold open

[![CI](https://github.com/Supersynergy/synapse/actions/workflows/rust-ci.yml/badge.svg)](https://github.com/Supersynergy/synapse/actions)
[![Release](https://img.shields.io/github/v/tag/Supersynergy/synapse?label=release)](https://github.com/Supersynergy/synapse/releases)
[![License](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Stars](https://img.shields.io/github/stars/Supersynergy/synapse?style=flat&color=ffcc00)](https://github.com/Supersynergy/synapse/stargazers)

</div>

---

```text
put → [ BM25 ∥ HNSW+PQ ∥ KG ] → fused rank → Ed25519-signed CRDT log → .synx
```

```bash
# install — pinned to a release tag (crates.io publish queued for v1.0.1)
cargo install --locked --git https://github.com/Supersynergy/synapse --tag v1.0.0 synapse-cli synapsed synapse-mcp

# run
synapsed -f ~/brain.db &

# remember
synapse put "we chose Rust because single-binary shipping matters"
synapse search "why Rust?"            # 23 µs
synapse snap ~/brain.brainpack        # signed, content-addressed, offline-verifiable
```

Wire into Claude Code — memory across every session:

```json
{ "mcpServers": { "synapse": {
    "command": "synapse-mcp",
    "args": ["--sock", "/tmp/synapse.sock"]
  } } }
```

---

## Real-competitor bench (agent memory, 1 k docs, 200 q, 384-d)

Same corpus, same embeddings, run locally. Full script: [`bench/real_competitors.py`](bench/real_competitors.py) · narrative: [`bench/RESULTS-REAL-COMPETITORS.md`](bench/RESULTS-REAL-COMPETITORS.md).

| engine | insert ms | ms / query | size KB | carries |
|--------|----------:|-----------:|--------:|---------|
| FAISS flat (floor) | 0.20 | 0.010 | 1 500 | vector only |
| SQLite FTS5 (floor) | 2.03 | 0.012 | 140 | keyword only |
| **Synapse v1.0** | **67.0** | **0.023** | **1 290** | **BM25 + HNSW + KG + CRDT + sign + MCP** |
| Chroma | 96.3 | 0.303 | 4 434 | vector only |
| LanceDB | 41.7 | 1.440 | 1 574 | vector only |

Synapse sits **2.3× off the theoretical FAISS floor** while carrying **eight capabilities** the floor doesn't.

## Head to head, numbers you can re-run

Every row below reproduces with one command: `bash bench/bench_20_usecases.sh`.
Full 50-usecase matrix + CatBoost-picked defaults: [`bench/RESULTS-V1.md`](bench/RESULTS-V1.md).

| op (10 k docs, M4 Max, p50) | Synapse | runner-up | gap |
|-----------------------------|--------:|----------:|----:|
| cold open | **0.69 ms** mmap | SQLite WAL ~ 7 ms | **10×** |
| BM25 query | **23 µs** | Meilisearch ~ 1.0 ms | **43×** |
| vector kNN k = 10 | **22 µs** | LanceDB ~ 50 µs | **2×** |
| CRDT merge, 200 ops | **0.59 ms** | Automerge baseline ~ 1.0 ms | **1.7×** |
| sign + verify manifest | **25 µs each** | `ed25519-dalek` stock | parity |
| pack → ship → verify → mmap | **12 ms end-to-end** | tar + cosign + sqlite ≈ 340 ms | **28×** |

## What Synapse makes obsolete

Every row below is a real stack a production team is paying for today. Synapse replaces the whole row with **one file and one binary**:

| Today | Synapse v1.0 |
|-------|-----|
| Qdrant / Pinecone / Chroma + Postgres + Redis + a Python embedder + Docker compose | one `synapsed` binary + `brain.synx` |
| memvid (MV2) — 200 ms per CLI call, 12 400 ms lex query | 5.7 µs RPC, 23 µs BM25 query |
| Meilisearch server + vector plugin + nightly reindex | Tantivy + HNSW built into the same file |
| mem0 + Zep + Letta orchestration loops | put / search / snap — three verbs, same file |
| LangChain session-store hacks around SQLite | first-class memory scopes (user / session / project / global) |
| Handcrafted diff-based memory versioning | Supersedes / References / Contradicts / Summarises edges |
| `rsync` + GPG + separate checksum step | `synapse snap` → Ed25519-signed `.brainpack` |
| cluster-mode Automerge server for multi-writer docs | single-file CRDT sync, no server |
| Bloomberg-style paid data API | content-addressed `.brainpack` subscriptions, you own the format |

No RAG framework in production has all of the above in one process. That's the gap.

## Why one file?

- **No daemons to supervise.** `synapsed` is optional; the CLI speaks to the file directly.
- **`cp brain.synx` = backup.** `git diff brain.brainpack` = audit. `scp` = deploy.
- **Offline-first by default.** Sign with Ed25519, verify on any peer, no trust server.
- **Portable forever.** `.synx` is a CC0 spec — any language, any runtime, no asking.
- **Indistinguishable from in-process state.** 5.7 µs RPC is below every perceptual threshold.

## Top features

1. **Hybrid retrieval in one call** — BM25 (Tantivy) ∥ HNSW+PQ vectors ∥ KG edges, fused with reciprocal-rank.
2. **Temporal knowledge graph built in** — `Supersedes`, `References`, `Contradicts`, `Summarises`, with valid-at windows.
3. **Memory scopes as a primitive** — Global / User / Session / Project, indexed natively, not a namespace hack.
4. **CRDT multi-writer sync** — Automerge under the hood, deterministic + commutative merges, no server.
5. **Ed25519-signed `.brainpack`** — sign with key rotation, verify on any peer, ship memory as an artefact.
6. **Zero-copy `mmap` reader** — 0.69 ms cold open on 10 k docs, raw slice in 2 µs.
7. **BLAKE3 content-addressed chunks** — dedup for free, every blob hash-verifiable.
8. **MCP-native** — Claude Code, Cursor, Cline, Continue, Aider all accept one JSON block.
9. **Rust core, MIT code, CC0 format** — no vendor, no lock-in, no BSL.
10. **One binary, one file, no servers.** Indistinguishable from an in-process library until you want replication — then the CRDT is already there.

## Compared to the field

**9 capabilities. 20 incumbents. Only Synapse ships them all in one file.**

<div align="center">

<a href="assets/matrix-full.svg"><img src="assets/matrix-full.svg" alt="Synapse vs the field — 20-tool capability matrix" width="100%"/></a>

<sub>Click for the full-size 1600×1200 version · ASCII-friendly markdown table below.</sub>

</div>

| Tool | BM25 | Vector | KG | Scopes | CRDT | Sign | OneFile | µs-IPC | MCP | OSS |
|------|:----:|:------:|:--:|:------:|:----:|:----:|:-------:|:------:|:---:|:---:|
| **→ Synapse** | **✅** | **✅** | **✅** | **✅** | **✅** | **✅** | **✅** | **✅** | **✅** | **MIT + CC0** |
| SQLite | ✅ | ext | — | — | — | — | ✅ | — | wrap | ✅ |
| DuckDB | ext | ext | — | — | — | — | ✅ | — | wrap | ✅ |
| SurrealDB | ✅ | ✅ | ✅ | — | — | — | — | — | wrap | ❌ BSL |
| PocketBase | ✅ | ext | — | — | — | — | ✅ | — | wrap | ✅ |
| Qdrant | — | ✅ | — | ns | — | — | — | — | wrap | ✅ |
| Meilisearch | ✅ | part | — | — | — | — | — | — | wrap | ✅ |
| LanceDB | ✅ | ✅ | — | — | — | — | part | — | wrap | ✅ |
| Chroma | — | ✅ | — | — | — | — | — | — | wrap | ✅ |
| Weaviate | ✅ | ✅ | part | ns | — | — | — | — | wrap | ✅ |
| Pinecone | — | ✅ | — | ns | — | — | — | — | wrap | ❌ closed |
| memvid | ✅ | — | — | — | — | — | ✅ | — | — | ✅ |
| mem0 | — | ext | ext | ✅ | — | — | — | — | wrap | ✅ |
| Graphiti | — | ext | ✅ | ✅ | — | — | — | — | wrap | ✅ |
| cognee | — | ext | ✅ | ✅ | — | — | — | — | wrap | ✅ |
| Memori | — | — | — | ✅ | — | — | — | — | wrap | ✅ |
| Zep | — | ✅ | part | ✅ | — | — | — | — | wrap | ✅ |
| Letta | — | ✅ | — | ✅ | — | — | — | — | wrap | ✅ |
| Automerge | — | — | — | — | ✅ | — | — | — | — | ✅ |
| RocksDB | — | — | — | — | — | — | ✅ | — | — | ✅ |
| Parquet | — | — | — | — | — | — | ✅ | — | — | ✅ |

<sup>Legend: `ext` via extension · `ns` via namespace filter, no scope type · `part` partial · `wrap` wrapper bridge · `BSL / closed` not permissively OSS.</sup>

Full one-by-one breakdown: [`docs/COMPARISON-V1.md`](docs/COMPARISON-V1.md).

## 60-second tour of the idea

Your agent has a 200 K-token context and zero memory between sessions. The usual fix is a pipeline — Qdrant, Redis, a Python venv, a Docker compose, three SDKs — so a language model can remember what it did yesterday.

**The stack is the bug.** Synapse is one file, one binary, one process. Every capability above lives inside a `.synx` container you can `git commit`, `scp`, sign and hand to a teammate. That's it.

## Deeper dives

- [`docs/CLAUDE-CODE-MEMORY.md`](docs/CLAUDE-CODE-MEMORY.md) — five real agentic workflows
- [`docs/SYNX-FORMAT-V2.md`](docs/SYNX-FORMAT-V2.md) — binary format spec (CC0)
- [`docs/BRAINPACK-V2.md`](docs/BRAINPACK-V2.md) — distribution wrapper + signing
- [`docs/COMPARISON-V1.md`](docs/COMPARISON-V1.md) — 20-tool head-to-head
- [`bench/RESULTS-V1.md`](bench/RESULTS-V1.md) — 50-usecase benchmark, 5 categories
- [`docs/LICENSE-STRATEGY.md`](docs/LICENSE-STRATEGY.md) — MIT + CC0 split + monetisation paths
- [`docs/USECASES.md`](docs/USECASES.md) — 20 deployment recipes
- [`docs/STRATEGY.md`](docs/STRATEGY.md) — how Synapse tops each competitor on their home turf
- [`CHANGELOG.md`](CHANGELOG.md) — v0.2.0 → v1.0.0 history

## Ecosystem

- **Claude Code** — paste the MCP block above, restart, done
- **Any MCP agent** — Cursor, Cline, Continue, Aider share the same config
- **Node.js** — [`sdk/node`](sdk/node)
- **Python** — [`sdk/python/synapse_reader.py`](sdk/python/synapse_reader.py), stdlib + `zstandard` + `blake3`
- **DuckDB analytics** — `ATTACH 'brain.db' AS s (TYPE sqlite, READ_ONLY);`

## License

MIT for the code. CC0 for the format specification. The format outlives the repo — any language, any product, no asking.

---

<div align="center">

Built in Rust. Shipped in Germany. Open forever.

*by [Maxim Supersynergy](https://github.com/Supersynergy)*

</div>
