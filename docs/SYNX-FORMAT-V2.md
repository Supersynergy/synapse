# `.synx` File Format v2 — RFC

**Status**: Draft (2026-04-20) · **Author**: Maxim Supersynergy · **License**: MIT

Next-generation single-file memory format. Successor to Synapse v1 (SQLite-based) and `.brainpack` v1 (zstd-packed SQLite dump). Purpose-built for AI agent memory at the 1k → 100M document scale.

## Design Goals

1. **Single file**, memory-mappable, crash-safe, portable across OS/arch.
2. **Columnar + row hybrid** (HTAP) — row-store for metadata, column-store for vectors + large text.
3. **First-class vector search** without extensions — HNSW + product-quantization built in.
4. **First-class lexical search** via Tantivy segments (BM25 + typo-tolerance + stemming).
5. **Content-addressed chunks** — BLAKE3-hashed, git-pack-style dedup, IPFS-ready.
6. **Zero-copy reads** — mmap + `rkyv`/Arrow-IPC for direct struct access.
7. **CoW + MVCC** — no in-place mutation; journal + compaction; free time-travel queries.
8. **Signed** — Ed25519 footer, verifiable provenance. Enables paid `.synx` subscriptions.
9. **CRDT-native replication** — Automerge ops-log chunk for multi-writer sync, no server.
10. **Open specification** — any language can implement; reference Rust impl is MIT.

## File Layout

```
┌────────────────────────────────────────────────────────────────┐
│ 0x00   Header (64 bytes)                                        │
│   ├─ Magic         "SYNX" (4B)                                  │
│   ├─ Version       u16 = 2                                      │
│   ├─ Flags         u16 (compression, signed, crdt)              │
│   ├─ Endian        u8  = 0 (little)                             │
│   ├─ Reserved      u8 × 7                                       │
│   ├─ Manifest off  u64                                          │
│   ├─ Footer off    u64                                          │
│   ├─ Created unix  u64                                          │
│   └─ Creator UUID  [u8; 16]                                     │
├────────────────────────────────────────────────────────────────┤
│ Data region — append-only chunks                                │
│                                                                 │
│  Chunk = ┌─ len u32 ─ kind u16 ─ codec u8 ─ flags u8 ─┐        │
│          │ blake3 hash [u8;32]                         │        │
│          │ uncompressed_len u64                        │        │
│          │ payload (zstd or raw)                       │        │
│          └───────────────────────────────────────────┘        │
│                                                                 │
│  Chunk kinds (u16):                                              │
│    0x01  RowBatch      Arrow IPC row-batch (id,uri,title,meta)  │
│    0x02  TextBlob      UTF-8, zstd-dict, BLAKE3-dedup           │
│    0x03  FtsSegment    Tantivy 0.23 segment directory           │
│    0x04  VecIndex      HNSW graph (bincode) + PQ codebook       │
│    0x05  VecPayload    Quantized vectors (int8 or BP128)        │
│    0x06  CRDTOpsLog    Automerge change frames                  │
│    0x07  SchemaDef     Arrow schema + migration history         │
│    0x08  MerkleNode    BLAKE3 tree for content verification     │
│    0xFF  Tombstone     logical delete marker                    │
├────────────────────────────────────────────────────────────────┤
│ Manifest (rkyv archived, zero-copy)                             │
│   ├─ chunks: [(kind, offset, hash, len)]                        │
│   ├─ schema_history: [SchemaVersion]                            │
│   ├─ active_snapshot: u64 (manifest generation)                 │
│   ├─ stats: (n_docs, n_vectors, n_segments)                     │
│   └─ roots: merkle_root [u8;32]                                 │
├────────────────────────────────────────────────────────────────┤
│ Journal (optional) — CoW write-ahead log                        │
│   ├─ pending ops (not yet in manifest)                          │
│   └─ recovered on open if interrupted                           │
├────────────────────────────────────────────────────────────────┤
│ Footer (256 bytes) — fixed position from EOF                    │
│   ├─ Magic         "XNYS"                                       │
│   ├─ Manifest hash [u8; 32]                                     │
│   ├─ Ed25519 sig   [u8; 64]   (optional, flag-gated)            │
│   ├─ Pubkey        [u8; 32]                                     │
│   └─ Version tail  u16                                          │
└────────────────────────────────────────────────────────────────┘
```

## Why Each Choice

| Choice | Alternative | Why |
|--------|-------------|-----|
| Append-only + manifest | In-place B-tree | Crash-safety without WAL; instant snapshot by freezing manifest |
| BLAKE3 content-hash | SHA-256 | 10× faster, tree-hash native, no FIPS needed |
| Arrow IPC row-batches | Custom bin | Polars/DuckDB interop free; Arrow-flight RPC |
| Tantivy segments | FTS5 | Rust-native, typo-tolerance, 3× faster build |
| HNSW + PQ | sqlite-vec flat | 100× faster at 1M scale; 32× smaller via int8 |
| rkyv manifest | serde JSON | zero-copy, mmap-read, no parse |
| zstd-dict blobs | raw gzip | trained dict = 3-5× better on short text |
| Automerge CRDT | None | multi-writer sync without server; offline-first |
| Ed25519 sig | None | sell signed `.synx` subscriptions |
| CoW journal | WAL | survives concurrent readers; simpler recovery |

## Open Questions

- Tantivy vs. custom Rust FTS (smaller footprint)?
- Scalar vs. product quantization default (accuracy/speed tradeoff)?
- CRDT granularity — per-doc or per-field?
- Compaction trigger — on write threshold or background tick?

## Migration Path v1 → v2

1. `synapse export --format synx brain.db → brain.synx` (one-shot)
2. Both formats supported via `Store::open(path)` auto-detect on magic bytes
3. `.brainpack` v2 = `.synx` file (no re-wrap)
4. Client SDKs unchanged; transparent at API level

## Reference Implementation

`crates/synapse-core/src/synx/` behind `--features synx-v2`. See [implementation plan](./SYNX-IMPLEMENTATION.md).

## License

Format specification: **CC0 / public domain**. Reference code: **MIT**. Anyone may implement compatible readers/writers. No patents. No trademarks.
