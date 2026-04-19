# Why Synapse v2 Goes Beyond SQLite

**Author**: Maxim Supersynergy · **Date**: 2026-04-20

Synapse v1 is SQLite + FTS5 + sqlite-vec + BLAKE3. It wins by a factor of 45,000× against MV2 and holds its own against every incumbent in [STRATEGY.md](./STRATEGY.md). So why move beyond?

## The Ceiling

Bench numbers from `bench/run_all.sh`:

| Op | v1 (SQLite) | v2 target (.synx) | Gain |
|----|-------------|-------------------|------|
| Embed 500 docs | 141 s | ~5 s | **28×** (MLX batch + int8-quant) |
| Vec kNN @ 1M | est. 50 ms | <5 ms | **10×** (HNSW + PQ) |
| File size 1k docs | 10 MB | 2 MB | **5×** (Arrow + zstd-dict) |
| Open a 100M-doc file | slow (parse) | instant (mmap + rkyv) | **∞** |
| Concurrent writers | 1 (WAL) | N (CRDT) | **N×** |
| Time-travel query | impossible | free | **new** |
| Signed + verifiable | impossible | Ed25519 footer | **new** |

SQLite is great; the ceiling is real.

## The Real Problem

AI-agent memory is **not** a TP-app workload. It is:
- **Bulk append + rare rewrite** (append-only fits)
- **High-dimension vector search** (row-store is wrong layout)
- **Content-heavy dedup** (Merkle tree fits; page-cache doesn't)
- **Multi-agent concurrent write** (single-writer WAL is a bottleneck)
- **Sold as a product** (needs signing, needs portable IPFS-friendly format)

SQLite was designed for OLTP apps on a phone in 2000. `.synx` is designed for AI agents in 2026.

## What Survives From v1

- **Single file**, portable, crash-safe
- **BLAKE3** dedup
- **MCP + msgpack IPC** (5.7 µs RTT)
- **Node SDK + CLI UX**
- **`.brainpack`** as distribution unit (now means `.synx` natively)

Synapse v1 interfaces; Synapse v2 engine.

## What Changes

- Storage: SQLite pages → content-addressed chunks
- FTS: FTS5 → Tantivy
- Vector: sqlite-vec flat → HNSW + product-quant
- Manifest: in-DB tables → rkyv-archived manifest (zero-copy)
- Writer model: single-writer WAL → CoW + CRDT merge
- Embed: ONNX CPU → MLX/CoreML batch
- Signing: none → Ed25519 footer
- Replication: libSQL-fork needed → Automerge ops-log chunk

## What Stays Compatible

`Store::open(path)` auto-detects magic bytes. Existing `brain.db` files keep working forever. Export via `synapse export --format synx` one-way. New clients get v2 by default.

## The Bigger Play

**The `.synx` format itself becomes a standard.** Like Parquet for analytics, like SafeTensors for ML weights, like HLS for video — we propose the neutral single-file open-spec for agent memory.

- **CC0 spec**, MIT ref impl
- Anyone can write compatible readers (Python, Go, Zig, C)
- Subscription `.synx` packs become a product category: DSGVO-compliance, medical knowledge, legal precedents, DACH leads, etc.
- No cloud lock-in; no vendor capture.

## Ship Sequence

**Phase 1** (this sprint): RFC spec + skeleton code behind feature flag. v1 untouched.  
**Phase 2** (3 months): Runtime writes `.synx` direct, mmap reads, Tantivy wired.  
**Phase 3** (6 months): Automerge CRDT, `synapse sync` command.  
**Phase 4** (12 months): Pack Standard RFC public, CC0 spec, community impls.

Built in Rust. Shipped in Germany. Open forever.
