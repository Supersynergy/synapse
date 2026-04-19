#!/bin/bash
# Large-corpus bench: 1M docs, no-embed default (embed is separately benched).
# Runs run_all.sh with N=1000000 EMBED_N=0. Takes ~1–2 min on M-series.

set -e
cd "$(dirname "$0")/.."

export N=${N:-1000000}
export EMBED_N=${EMBED_N:-0}

echo "=== bench_1m.sh · N=$N EMBED_N=$EMBED_N ==="
./bench/run_all.sh
echo "Results → bench/results.json"
