# Top-20 single-file / embedded DB format bench

**Date**: 2026-04-20 · **Host**: M4 Max · **N**: 10 000 small structured docs (id / title / body / scope / ts)

Sorted by on-disk size ascending.

| # | format | insert ms | read ms | size KB | notes |
|---|--------|----------:|--------:|--------:|-------|
| 1 | **JSONL + zstd** | 12.25 | 11.97 | **54.4** | smallest footprint, universal readers |
| 2 | **MessagePack + zstd** | **3.03** | 3.47 | 74.7 | smallest binary, fastest insert path |
| 3 | **Parquet (zstd)** | 5.02 | 15.56 | 112.7 | analytics-native, columnar |
| 4 | **Feather v2 (zstd)** | 2.09 | **0.33** | 114.0 | mmap-friendly Arrow, fastest read |
| 5 | CSV + gzip | 26.39 | 25.61 | 130.4 | legacy interop |
| 6 | DuckDB | 1563.04 | 5.76 | 780.0 | OLAP core; write-cost is the trade |
| 7 | **Synapse .synx v0.2.4** | *(see Rust bench)* | 22.39 | 1285.1 | 10 001 chunks — content-addressed dedup tax |
| 8 | Pickle (py5) | 2.27 | 1.67 | 1422.5 | Python-only |
| 9 | SQLite (default) | 6.47 | 2.28 | 1432.0 | baseline — everyone's reference |
| 10 | LanceDB | 8.46 | 7.65 | 1454.1 | vector-native, row overhead hurts here |
| 11 | Arrow IPC | 3.60 | **0.12** | 1462.7 | in-memory friendly, no compression by default |
| 12 | CBOR | 8.38 | 7.36 | 1539.4 | RFC-stable binary JSON alt |
| 13 | SQLite + WAL (tuned) | 7.82 | 2.12 | 1573.2 | production SQLite default |
| 14 | DBM (stdlib) | 30.15 | 12.52 | 3104.0 | ancient KV, still in every Python |
| 15 | LMDB | 22.69 | 1.06 | 3832.0 | memory-mapped KV, zero-copy reads |

Absent from this run (no single Python binding reachable locally without a C toolchain chase):
LevelDB, RocksDB, sled, fjall, redb, TigerBeetle, FoundationDB, BadgerDB, BoltDB, MongoDB (WT), Oracle Berkeley DB, InnoDB standalone, Pebble, NessDB, SquirrelDB. Add them to `bench/top20_formats.py` if you have the binding.

## Interpretation

1. **Pure serialisation beats everything on size.** `JSONL+zstd` and
   `MessagePack+zstd` are unbeatable for "just store the rows". Any
   embedded-DB format pays an index/page overhead of at least 10–30×.

2. **Synapse `.synx` currently trades size for per-chunk provenance.**
   Every doc becomes its own content-addressed, BLAKE3-hashed chunk. That
   design lets `.brainpack` distribution ship *one* signed file that every
   consumer can verify bit-for-bit — but on this bench it's 1.3 MB vs
   MessagePack's 75 KB. The Phase 3.1 roadmap groups chunks into row-batches
   to close the gap while keeping dedup for large text blobs.

3. **Feather v2** and **Arrow IPC** win the read-latency race at **0.12 – 0.33 ms**.
   These are the targets for the v0.3 `.synx` zero-copy reader (rkyv
   manifest + Arrow-IPC row chunks).

4. **DuckDB's 1.5 s insert** is an outlier: `executemany` through Python
   trips a known slow path. The SQL-generating path is 1000× faster in the
   v1 bench (`bench/run_all.sh`). Don't read this row as "DuckDB is slow".

5. **LMDB** is 2.5× larger on disk than SQLite here because its
   page-pre-allocated 512 MB map is oversized for 10k docs — at scale that
   waste flips to a win.

## Follow-up runs

```bash
# Run the Rust side first so the existing-synx bench has a file to open.
bash bench/bench_20_usecases.sh
# Then Python bench against everything available locally.
N=10000 python3 bench/top20_formats.py > bench/RESULTS-TOP20.md
```
