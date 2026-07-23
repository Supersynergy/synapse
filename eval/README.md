# Synapse Eval — LoCoMo & LongMemEval Harness

Automated benchmark harness for long-context memory retrieval. Measures Synapse's
hybrid search (FTS5 + ANN + RRF + rerank) against two published academic benchmarks:

- **LoCoMo** — Long Context Memory benchmark (Ding et al., 2024)
  - 5 conversation categories: short-dialog, long-dialog, open-domain, single-doc, multi-doc
  - 1,500+ QA pairs across 50 conversations
  - Tests: temporal reasoning, multi-hop, retrieval from long dialogues
- **LongMemEval** — Long-term Memory Evaluation (Wu et al., 2024)
  - 500 questions over 118 long conversations (600+ turns avg)
  - 5 user-centric tasks: info reorganization, detail following, multi-session reasoning
  - Tests: cross-session recall, temporal ordering, user preference tracking

## Layout

```
eval/
├── README.md              — this file
├── harness.py             — main runner
├── datasets/
│   ├── locomo/            — downloaded LoCoMo JSON (gitignored)
│   └── longmemeval/       — downloaded LongMemEval JSON (gitignored)
├── results/
│   └── *.json             — per-run results (gitignored)
└── reports/
    └── latest.md          — generated summary
```

## Quick start

```bash
# 1. Download datasets (one-time, requires internet)
python3 eval/harness.py download

# 2. Ingest into a fresh Synapse brain
python3 eval/harness.py ingest --db /tmp/eval-brain.db

# 3. Run benchmark
python3 eval/harness.py run --db /tmp/eval-brain.db --k 5

# 4. Print report
python3 eval/harness.py report
```

## Metrics

- **Recall@k** — fraction of questions where the gold passage is in top-k retrieved
- **MRR** — Mean Reciprocal Rank of the first correct passage
- **Latency p50/p95** — retrieval latency in milliseconds
- **Context precision** — signal-to-noise in the packed context window

## Reproducibility

Each run writes a JSON results file with:
- git SHA of the synapse-ultra crate
- dataset version + SHA
- exact CLI commands used
- per-question retrieval results
- aggregate metrics

## Notes

- Datasets are NOT bundled — download via `harness.py download`
- Benchmark requires ~2 GB RAM for LongMemEval
- Full run takes ~15 min on M2 Pro
