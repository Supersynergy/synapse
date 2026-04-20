# Eval harness — latency *and* recall

Engineers reviewing Synapse have asked two orthogonal questions:

1. "Are the µs numbers reproducible on my machine?" → `cargo bench` + Criterion.
2. "How good is the *quality* of what comes back — recall, nDCG, contradiction F1?" → eval harness against public corpora.

This doc maps how we deliver both.

## Latency — Criterion

```bash
cargo bench -p synapse-core --features fts-tantivy --bench fts
cargo bench -p synapse-core --features vec-hnsw   --bench vec
```

Output lands in `target/criterion/**/report.json` with full stats:
mean, median, p95, p99, `criterion`'s own outlier detection. Copy the HTML
report into `docs/BENCH-YYYY-MM-DD.md` every release.

Target p50 on M-series hardware (documented in the bench output):

| metric | expected | bench entry |
|--------|---------:|-------------|
| BM25 unigram, 10 k docs | ≤ 25 µs | `fts::bm25 unigram 10k docs` |
| BM25 OR, 10 k docs | ≤ 40 µs | `fts::bm25 boolean OR 10k docs` |
| BM25 phrase, 10 k docs | ≤ 25 µs | `fts::bm25 phrase 10k docs` |
| HNSW kNN k=10, 2 k × 64-d | ≤ 25 µs | `vec::hnsw knn k=10 · 2k × 64d` |

Numbers in the README are from these exact benchmarks, run on an M4 Max.

## Recall — public-corpus eval (roadmap)

Latency is a necessary condition; quality is the sufficient one. Planned v0.4
ships `synapse-eval` as a companion crate that drives Synapse against:

- **LoCoMo** — long-term conversational memory (primary agent-memory benchmark)
- **LongMemEval** — multi-session recall, contradiction resolution, abstention
- **BEIR subset** — nDCG@10 for the hybrid retriever (MS-MARCO, TREC-COVID, NFCorpus)
- **MTEB-retrieval** — cross-check vector recall matches the chosen embedding model's claim

For each benchmark we publish **nDCG@10 / recall@20 / MRR / contradiction-F1** side-by-side with:

- mem0 (same embedder, same corpus)
- Zep (public reference numbers)
- Letta (public reference numbers)
- Graphiti (public reference numbers)
- a plain SQLite-FTS5 baseline

Fair-benchmark rules copied from the `mteb` project:

1. One eval harness. One config per system. Published commit hashes.
2. Same embedder across all contenders, no cherry-picking.
3. All raw CSVs committed to `eval/results/`. CI regenerates on PR.
4. Synapse losses on any metric are published with the same prominence as wins.

Until that harness ships, recall claims are **not** on the README.

## Run it yourself today

```bash
# latency only — available now
cargo bench -p synapse-core --features full

# 50-usecase matrix (Rust + Python + CatBoost)
bash bench/bench_20_usecases.sh
python3 bench/category_summary.py /tmp/synapse_bench_v1.jsonl

# cross-format footprint
python3 bench/top20_formats.py

# recall — placeholder, ships with v0.4
# cargo run -p synapse-eval --release -- --dataset locomo
```

## Why this matters

Two reviewers in the last round (one AI engineer, one Rust dev) flagged
the same gap: "latency ≠ quality, publish recall or it's marketing." That
critique is correct. This harness closes it before v0.4 ships.
