#!/bin/bash
# v0.2 feature bench: Tantivy FTS + KG scopes + .synx round-trip.
# Runs in release mode via a throwaway sibling crate so it links the lib.

set -e
cd "$(dirname "$0")/.."

N=${N:-10000}
Q=${Q:-200}

mkdir -p /tmp/synx_v2_bench/src
cat > /tmp/synx_v2_bench/Cargo.toml <<TOML
[package]
name = "synx_v2_bench"
version = "0.0.1"
edition = "2021"

[dependencies]
synapse-core = { path = "$PWD/crates/synapse-core", features = ["fts-tantivy"] }
TOML

cat > /tmp/synx_v2_bench/src/main.rs <<'RS'
use synapse_core::synx::{
    fts::FtsIndex,
    header::SynxFlags,
    kg::{Edge, EdgeKind, EdgeSet, Scope},
    ChunkKind, Codec, SynxReader, SynxWriter,
};
use std::time::Instant;

fn main() {
    let n: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(10_000);
    let q: usize = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(200);

    let words = "rust ships ferris ownership borrow mcp memory vector embed synx tantivy hnsw blake3 zstd cow journal merkle scope session global supersedes references contradicts summarises agent";
    let ws: Vec<&str> = words.split_whitespace().collect();
    let phrase = |i: usize| -> String {
        (0..10).map(|j| ws[(i + j) % ws.len()]).collect::<Vec<_>>().join(" ")
    };

    // 1) .synx write
    let out = "/tmp/synx_v2_bench.synx";
    let _ = std::fs::remove_file(out);
    let t = Instant::now();
    let mut w = SynxWriter::create(out, SynxFlags::COMPRESSED).unwrap();
    for i in 0..n {
        w.append(ChunkKind::TextBlob, Codec::Zstd, phrase(i).as_bytes()).unwrap();
    }
    // also stash a KG edge set as a TextBlob tagged via schema-def later
    let mut edges = EdgeSet::default();
    for i in 0..(n / 1000).max(1) {
        edges.edges.push(
            Edge::new(format!("d{i}"), format!("d{}", i + 1), EdgeKind::Supersedes)
                .with_scope(Scope::Project("supersynergy".into())),
        );
    }
    w.append(ChunkKind::SchemaDef, Codec::Zstd, &edges.to_json()).unwrap();
    w.finish().unwrap();
    let write_ms = t.elapsed().as_secs_f64() * 1000.0;
    let size = std::fs::metadata(out).unwrap().len();

    // 2) .synx open
    let t = Instant::now();
    let mut r = SynxReader::open(out).unwrap();
    let open_ms = t.elapsed().as_secs_f64() * 1000.0;

    // 3) build Tantivy over all text chunks
    let t = Instant::now();
    let fts = FtsIndex::new().unwrap();
    let mut rows = Vec::with_capacity(n);
    for i in 0..n {
        let c = r.read_chunk_at(i).unwrap();
        if !matches!(c.kind, ChunkKind::TextBlob) {
            continue;
        }
        let body = String::from_utf8(c.decode().unwrap()).unwrap();
        rows.push((format!("d{i}"), format!("doc {i}"), body, "global".into()));
    }
    fts.write(&rows).unwrap();
    let build_ms = t.elapsed().as_secs_f64() * 1000.0;

    // 4) Q queries
    let t = Instant::now();
    let mut total_hits = 0usize;
    for i in 0..q {
        let term = ws[i % ws.len()];
        let hits = fts.search(term, 10).unwrap();
        total_hits += hits.len();
    }
    let query_ms = t.elapsed().as_secs_f64() * 1000.0;

    println!("synapse v0.2 feature bench · N={n} Q={q}");
    println!("  synx write:       {write_ms:>9.2} ms  ({:.0} docs/s)", n as f64 / write_ms * 1000.0);
    println!("  synx open:        {open_ms:>9.2} ms");
    println!("  tantivy build:    {build_ms:>9.2} ms  ({:.0} docs/s)", n as f64 / build_ms * 1000.0);
    println!("  tantivy query:    {query_ms:>9.2} ms  ({:.3} ms/q, {total_hits} hits)", query_ms / q as f64);
    println!("  file size:        {size:>9} bytes  ({:.1} B/doc)", size as f64 / n as f64);
}
RS

cargo run --release --manifest-path /tmp/synx_v2_bench/Cargo.toml --quiet -- "$N" "$Q"
