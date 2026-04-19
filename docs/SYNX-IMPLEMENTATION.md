# `.synx` Reference Implementation Plan

## Module Layout

```
crates/synapse-core/src/synx/
  mod.rs           — public API, feature-gated
  header.rs        — SynxHeader, SynxFooter, magic bytes
  chunk.rs         — Chunk reader/writer, codec (zstd/raw)
  manifest.rs      — rkyv-archived manifest (zero-copy)
  journal.rs       — CoW write-ahead log, recovery
  reader.rs        — mmap reader, chunk-index, range queries
  writer.rs        — append-only writer, compaction trigger
  migrate.rs       — v1 (SQLite) → v2 (.synx) export
```

## Crates Behind Feature Flag `synx-v2`

| Crate | Purpose |
|-------|---------|
| `memmap2` | mmap reader |
| `rkyv` | zero-copy manifest |
| `arrow-ipc` | row-batch chunks |
| `tantivy` | FTS segment chunks |
| `hnsw_rs` (or `instant-distance`) | ANN graph |
| `zstd` | chunk compression (already present) |
| `blake3` | content hash (already present) |
| `automerge` | CRDT ops-log chunk |
| `ed25519-dalek` | footer signing |

Feature flag kept opt-in until v0.2. Default build continues to use SQLite core.

## Phases

### Phase 1 — Format Skeleton (this sprint)
- [x] RFC spec published
- [ ] `synx/header.rs` — magic, footer, offsets
- [ ] `synx/chunk.rs` — read/write single chunk
- [ ] `synx/manifest.rs` — rkyv-archived manifest
- [ ] Unit tests: round-trip header + 1 chunk
- [ ] `synapse export --format synx` CLI one-shot

### Phase 2 — Read Path (Q3 2026)
- [ ] `synx/reader.rs` — mmap open, chunk index
- [ ] Arrow-IPC row-batch decode
- [ ] Tantivy segment mounted read-only
- [ ] HNSW graph loaded from chunk
- [ ] Hybrid search via reader (lex + vec + RRF)
- [ ] Bench: compare v1 (.db) vs. v2 (.synx) same corpus

### Phase 3 — Write Path + CRDT (Q4 2026)
- [ ] `synx/writer.rs` — append chunk, update manifest, flush footer
- [ ] CoW journal with recovery
- [ ] Automerge ops-log chunk; `synapse sync peer@url`
- [ ] Compaction background task
- [ ] Ed25519 signing optional flag

### Phase 4 — Standard + Community (2027)
- [ ] CC0 spec frozen at `spec/synx-v2.md`
- [ ] Byte-exact conformance suite `spec/conformance/`
- [ ] Python reference reader (~200 LOC) in `sdk/python`
- [ ] Go reader in `sdk/go`
- [ ] Invited talk submission: LocalLLaMA summit, Rust conf

## Testing Matrix

| Test | Phase 1 | Phase 2 | Phase 3 |
|------|---------|---------|---------|
| Round-trip 1k docs | ✓ | ✓ | ✓ |
| Open 1M-doc file in <10ms | — | ✓ | ✓ |
| Concurrent reader + writer | — | — | ✓ |
| Crash mid-write recovery | — | — | ✓ |
| CRDT merge 3-way divergence | — | — | ✓ |
| Signed pack verification | — | — | ✓ |
| Conformance: Rust ⇄ Python read | — | — | ✓ |

## Non-Goals for Ref Impl

- Distributed consensus (Synapse is single-node; use libSQL or Postgres for distributed)
- SQL-compat query engine (Synapse API is put/search/get; no JOINs)
- In-place updates without CoW (violates crash safety)
