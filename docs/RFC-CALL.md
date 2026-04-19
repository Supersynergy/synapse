# RFC: `.synx` Open File Format for Agent Memory

**Status**: Draft — open for peer review · **Maintainer**: Maxim Supersynergy

## Call for Review

We are proposing `.synx` as the open single-file format for AI agent memory. Goals: beat SQLite for this specific workload, remain vendor-neutral, enable a subscription-pack market. Full spec: [SYNX-FORMAT-V2.md](./SYNX-FORMAT-V2.md).

## Who We Want Feedback From

| Project | Area | Why |
|---------|------|-----|
| **LanceDB** (lancedb/lance) | Columnar vector file format | Direct prior art; best-in-class Lance format |
| **Tantivy** (quickwit-oss/tantivy) | Rust FTS | Reference for segment format |
| **fastembed-rs** (Anush008/fastembed-rs) | Rust ONNX embeddings | Already used in v1 |
| **Automerge** (automerge/automerge) | CRDT | Proposed ops-log chunk design |
| **rkyv** (rkyv/rkyv) | Zero-copy serialization | Manifest format |
| **Apache Arrow** (apache/arrow-rs) | Columnar IPC | Row-batch chunks |
| **DuckDB** (duckdb/duckdb) | Embedded OLAP file format | Compression and chunking |
| **BLAKE3** (BLAKE3-team/BLAKE3) | Hashing | Merkle tree usage |
| **memvid** (memvid) | Prior-art single-file memory | Lineage ack |

## How to Contribute

1. Read [SYNX-FORMAT-V2.md](./SYNX-FORMAT-V2.md) (~3 pages)
2. Open an issue on `Supersynergy/synapse` tagged `rfc-synx-v2`
3. Propose concrete changes to chunk layout, hashing, CRDT granularity, or compaction triggers
4. Suggest compatibility tests (a byte-exact conformance suite will live at `spec/conformance/`)

## Timeline

- **2026-04-20**: Draft published (this doc)
- **2026-05-20**: Review window closes
- **2026-06-01**: v2 spec frozen, reference impl begins in `crates/synapse-core/src/synx/`
- **2026-09-01**: Synapse v0.2 ships with read/write support behind feature flag
- **2027-Q1**: Default format switch, multi-language impls invited

## Out of Scope

- Networking protocols (use MCP / msgpack as today)
- Authentication (orthogonal; handled by server layer)
- Query language (Synapse API unchanged)
- Specific embedding models (container format, not content format)

## Contact

Open an issue. No private email review. All decisions in public.
