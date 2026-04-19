#!/bin/bash
# 20-usecase end-to-end bench + CatBoost-guided parameter search.
#
# Phase A: run a synthetic-but-realistic matrix of engine configs across each
#          usecase (hybrid search, analytics, bulk insert, etc.).
# Phase B: feed the measurements into a CatBoost regressor that picks the
#          knob settings that Pareto-dominate latency × size.
#
# The Rust side writes one JSON line per config-usecase pair; the Python side
# reads the JSONL, trains CatBoost, and emits the recommended knobs.

set -e
cd "$(dirname "$0")/.."

mkdir -p /tmp/synapse_uc_bench/src
cat > /tmp/synapse_uc_bench/Cargo.toml <<TOML
[package]
name = "synapse_uc_bench"
version = "0.0.1"
edition = "2021"

[dependencies]
synapse-core = { path = "$PWD/crates/synapse-core", features = ["full"] }
serde_json = "1"
TOML

cp bench/uc_bench.rs /tmp/synapse_uc_bench/src/main.rs

echo "=== Build release ==="
cargo build --release --manifest-path /tmp/synapse_uc_bench/Cargo.toml --quiet

OUT=/tmp/synapse_bench.jsonl
> "$OUT"
echo "=== Run 20 usecases ==="
/tmp/synapse_uc_bench/target/release/synapse_uc_bench "$OUT"
echo "JSONL → $OUT · lines: $(wc -l < $OUT)"

echo "=== Train CatBoost ==="
python3 bench/catboost_pick.py "$OUT"
