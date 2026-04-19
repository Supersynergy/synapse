# Synapse

> Single-file memory for AI agents. SQLite speed. Daemon mode. Rust core.

**Status:** MVP shipped (M0–M6). All benchmark targets met or exceeded.

## What

Drop-in replacement for memvid's `.mv2` format. Keeps single-file portability. Kills spawn overhead. Crushes MV2 on every axis:

| Op (1000 docs, M4 Max) | MV2 CLI | Synapse daemon | Δ |
|---|---|---|---|
| Insert batch (no embed) | ~147 s | **16 ms** | **9,074×** |
| Lex search | 12,400 ms/q | **0.275 ms/q** | **45,091×** |
| Vec search | 88 ms/q | **1.50 ms/q** | **59×** |
| Hybrid RRF | — | **1.77 ms/q** | new |
| RTT per call | ~200 ms (spawn) | **9 µs** | **22,222×** |
| Re-embed cached text | full compute | **1.4 ms / 500 docs** | **1,273×** (M2 cache) |
| `.brainpack` size (1000 docs) | 5.6 MB | **988 KB** | **5.8× smaller** |

See [bench/RESULTS.md](./bench/RESULTS.md) for methodology.

## Architecture

```
clients ──msgpack/unix-socket──▶ synapsed ──▶ SQLite(FTS5 + sqlite-vec + redb-cache)
                                    │
                                    └── fastembed-rs (BGE-small-en-v1.5 ONNX, 384-dim)
```

- **synapse-core**: library — schema, FTS5, sqlite-vec, BLAKE3 dedup, `.brainpack` export/import
- **synapsed**: daemon — tokio + length-prefixed msgpack over `AF_UNIX`, 8 RPC methods
- **synapse-cli**: one-shot CLI (init/put/find/vec/hybrid/stats/snap/restore)
- **synapse-mcp**: MCP stdio bridge — exposes synapsed as MCP server for Claude/agent tool-use
- **@synapse/sdk** (sdk/node): Node.js client, 4 KB

## Install & Run

```bash
git clone https://github.com/Supersynergy/synapse
cd synapse
cargo build --release

# start daemon
./target/release/synapsed -f ~/.synapse/brain.db &

# use from Python
python3 bench/client.py ping
python3 bench/client.py bench 1000

# or from Node
cd sdk/node && npm install && npm run build
node --eval "
  import { Synapse } from './dist/index.js';
  const s = new Synapse();
  console.log(await s.ping());
  const id = await s.put({ text: 'rust sqlite memory', embed: true });
  console.log(await s.search('rust', { mode: 'Hybrid', embedQuery: true }));
" --experimental-vm-modules
```

## Milestones

- [x] **M0** — masterplan + architecture
- [x] **M1** — `synapse-core` crate, SQLite+FTS5+sqlite-vec, 5/5 unit tests pass
- [x] **M2** — embedding pipeline + **BLAKE3 redb cache** (1,273× speedup on repeat text)
- [x] **M3** — `synapsed` daemon (tokio + unix socket + msgpack-rpc)
- [x] **M4** — CLI + Node SDK + MCP stdio bridge
- [x] **M5** — `.brainpack` export/import (zstd + BLAKE3 checksum)
- [x] **M6** — benchmark harness + GitHub Actions CI
- [ ] **M7** — ANE CoreML EP for embeddings (requires fastembed fork; projected +3-10× embed throughput)

## License

MIT
