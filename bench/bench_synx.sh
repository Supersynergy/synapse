#!/bin/bash
# .synx format round-trip micro-bench.
# Writes N raw text blobs + matching row batches, reads them back, times both.

set -e
cd "$(dirname "$0")/.."

N=${N:-10000}
OUT=/tmp/synapse_bench.synx
rm -f "$OUT"

echo "=== Build release with synx-v2 ==="
cargo build --release --quiet -p synapse-core --features synx-v2

echo "=== Run inline synx bench ==="
cat > /tmp/synx_bench.rs <<'RS'
use synapse_core::synx::{SynxWriter, SynxReader, ChunkKind, Codec};
use synapse_core::synx::header::SynxFlags;

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(10000);
    let out = std::env::args().nth(2).unwrap_or("/tmp/synapse_bench.synx".into());
    let _ = std::fs::remove_file(&out);

    let t = std::time::Instant::now();
    let mut w = SynxWriter::create(&out, SynxFlags::COMPRESSED).unwrap();
    for i in 0..n {
        let txt = format!("document {i} quick brown fox jumps over lazy dog");
        w.append(ChunkKind::TextBlob, Codec::Zstd, txt.as_bytes()).unwrap();
    }
    w.finish().unwrap();
    let write_ms = t.elapsed().as_secs_f64() * 1000.0;
    let size = std::fs::metadata(&out).unwrap().len();

    let t = std::time::Instant::now();
    let mut r = SynxReader::open(&out).unwrap();
    let open_ms = t.elapsed().as_secs_f64() * 1000.0;

    let t = std::time::Instant::now();
    let mut bytes = 0u64;
    for i in 0..r.manifest.chunks.len() {
        let c = r.read_chunk_at(i).unwrap();
        bytes += c.decode().unwrap().len() as u64;
    }
    let read_ms = t.elapsed().as_secs_f64() * 1000.0;

    println!("synx bench · N={n}");
    println!("  write:      {write_ms:>8.2} ms  ({:.0} docs/s)", n as f64 / write_ms * 1000.0);
    println!("  open:       {open_ms:>8.2} ms");
    println!("  read-all:   {read_ms:>8.2} ms  ({} bytes)", bytes);
    println!("  file size:  {size:>8} bytes");
}
RS

# build as a one-off binary linked against the local workspace
mkdir -p /tmp/synx_bench_crate/src
cat > /tmp/synx_bench_crate/Cargo.toml <<TOML
[package]
name = "synx_bench"
version = "0.0.1"
edition = "2021"

[dependencies]
synapse-core = { path = "$PWD/crates/synapse-core", features = ["synx-v2"] }
TOML
mv /tmp/synx_bench.rs /tmp/synx_bench_crate/src/main.rs
cargo run --release --manifest-path /tmp/synx_bench_crate/Cargo.toml --quiet -- "$N" "$OUT"
