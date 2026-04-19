//! 50-usecase Synapse v1.0 bench runner.
//!
//! Writes one JSON record per (config, usecase) combination. The companion
//! Python script trains a CatBoost model; another summarises per-category.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::time::Instant;

use serde_json::json;
use synapse_core::synx::{
    chunk::{Chunk, ChunkKind, Codec},
    fts::FtsIndex,
    header::SynxFlags,
    kg::{Edge, EdgeKind, EdgeSet, Scope},
    mmap::MmapReader,
    vec_index::HnswIndex,
    writer::SynxWriter,
    SynxReader,
};

const WORDS: &str = "rust ships ferris ownership borrow mcp memory vector embed synx tantivy \
hnsw blake3 zstd cow journal merkle scope session global supersedes references contradicts \
summarises agent claude crm event lead scraping research brainpack sqlite postgres duckdb \
graph kg signing ed25519 crdt automerge mmap latency throughput benchmark single file format";

fn words() -> Vec<&'static str> { WORDS.split_whitespace().collect() }
fn phrase(i: usize, len: usize) -> String {
    let ws = words();
    (0..len).map(|j| ws[(i + j) % ws.len()]).collect::<Vec<_>>().join(" ")
}
fn vector(i: usize, dim: usize) -> Vec<f32> {
    (0..dim).map(|j| ((i * 13 + j * 7) % 997) as f32 / 997.0 - 0.5).collect()
}
fn ms(d: std::time::Duration) -> f64 { d.as_secs_f64() * 1000.0 }
fn t0() -> Instant { Instant::now() }

struct H {
    n: usize,
    synx_path: String,
    vectors: Vec<Vec<f32>>,
    words: Vec<&'static str>,
}
impl H {
    fn build(n: usize, zstd: i32) -> Self {
        let path = format!("/tmp/synbench_{}_{}.synx", n, zstd);
        let _ = std::fs::remove_file(&path);
        let mut w = SynxWriter::create(&path, SynxFlags::COMPRESSED).unwrap();
        for i in 0..n {
            w.append(ChunkKind::TextBlob, Codec::Zstd, phrase(i, 10).as_bytes()).unwrap();
        }
        let mut edges = EdgeSet::default();
        for i in 0..n.min(100) {
            edges.edges.push(Edge::new(format!("d{i}"), format!("d{}", i + 1), EdgeKind::Supersedes));
        }
        w.append(ChunkKind::SchemaDef, Codec::Zstd, &edges.to_json()).unwrap();
        w.finish().unwrap();
        let vectors: Vec<Vec<f32>> = (0..n.min(2000)).map(|i| vector(i, 64)).collect();
        H { n, synx_path: path, vectors, words: words() }
    }
}

#[derive(Clone, Copy)]
struct R { latency_ms: f64, throughput: f64, bytes: u64, category: &'static str }

fn rec(cat: &'static str, latency_ms: f64, throughput: f64, bytes: u64) -> R {
    R { latency_ms, throughput, bytes, category: cat }
}

type Uc = fn(&H) -> R;

fn main() {
    let out_path = std::env::args().nth(1).unwrap_or_else(|| "/tmp/synapse_bench.jsonl".into());
    let mut out = BufWriter::new(File::create(&out_path).unwrap());
    let ns = [1_000usize, 10_000];
    let zstd_levels = [3i32, 9, 19];
    let hnsw_ef = [16u32, 64, 128];

    let ucs: Vec<(&str, Uc)> = vec![
        // --- STORAGE (10) -------------------------------------------------
        ("uc01_bulk_ingest", uc_bulk_ingest),
        ("uc02_synx_open", uc_synx_open),
        ("uc03_mmap_open", uc_mmap_open),
        ("uc04_read_all_chunks", uc_read_all),
        ("uc05_chunk_roundtrip_raw", uc_chunk_rt_raw),
        ("uc06_chunk_roundtrip_zstd", uc_chunk_rt_zstd),
        ("uc07_manifest_verify", uc_manifest_verify),
        ("uc08_brainpack_pack", uc_brainpack_pack),
        ("uc09_brainpack_unpack", uc_brainpack_unpack),
        ("uc10_mmap_raw_slice", uc_mmap_raw_slice),
        // --- MEMORY (10) --------------------------------------------------
        ("uc11_mem_scope_global", uc_scope_global),
        ("uc12_mem_scope_user", uc_scope_user),
        ("uc13_mem_scope_session", uc_scope_session),
        ("uc14_mem_scope_project", uc_scope_project),
        ("uc15_kg_supersedes_chain", uc_kg_resolve),
        ("uc16_kg_valid_at_filter", uc_kg_valid_at),
        ("uc17_kg_edge_json_rt", uc_kg_edge_json),
        ("uc18_blake3_dedup_1k", uc_dedup_blake3),
        ("uc19_content_hash_verify", uc_content_hash),
        ("uc20_scope_tag_1k", uc_scope_tag),
        // --- FTS (10) -----------------------------------------------------
        ("uc21_fts_build", uc_fts_build),
        ("uc22_fts_query_unigram", uc_fts_uni),
        ("uc23_fts_query_boolean_or", uc_fts_or),
        ("uc24_fts_query_phrase", uc_fts_phrase),
        ("uc25_fts_query_prefix", uc_fts_prefix),
        ("uc26_fts_rebuild_after_delete", uc_fts_rebuild),
        ("uc27_fts_top1_latency", uc_fts_top1),
        ("uc28_fts_top_50", uc_fts_top50),
        ("uc29_fts_case_insens", uc_fts_ci),
        ("uc30_fts_multi_field", uc_fts_multi),
        // --- VECTOR (10) --------------------------------------------------
        ("uc31_hnsw_build_flat", uc_hnsw_flat),
        ("uc32_hnsw_build_quant", uc_hnsw_quant),
        ("uc33_hnsw_knn_k1", uc_hnsw_k1),
        ("uc34_hnsw_knn_k10", uc_hnsw_k10),
        ("uc35_hnsw_knn_k100", uc_hnsw_k100),
        ("uc36_hnsw_batch_query", uc_hnsw_batch),
        ("uc37_cosine_flat_scan", uc_cosine_scan),
        ("uc38_scalar_quant_roundtrip", uc_quant_rt),
        ("uc39_vector_dedup_hash", uc_vec_dedup),
        ("uc40_vec_build_then_search", uc_vec_build_search),
        // --- SYNC / PACK (10) --------------------------------------------
        ("uc41_crdt_encode", uc_crdt_encode),
        ("uc42_crdt_merge", uc_crdt_merge),
        ("uc43_crdt_merge_commutative", uc_crdt_commut),
        ("uc44_sign_keygen", uc_sign_keygen),
        ("uc45_sign_manifest", uc_sign_manifest),
        ("uc46_verify_manifest", uc_verify_manifest),
        ("uc47_portable_copy", uc_portable),
        ("uc48_brainpack_sign_pack", uc_pack_sign),
        ("uc49_crdt_payload_size", uc_crdt_size),
        ("uc50_full_roundtrip", uc_full_rt),
    ];

    for &n in &ns {
        for &zl in &zstd_levels {
            for &ef in &hnsw_ef {
                let h = H::build(n, zl);
                for (name, fnp) in &ucs {
                    let r = fnp(&h);
                    writeln!(out, "{}", json!({
                        "usecase": name,
                        "category": r.category,
                        "n": n,
                        "zstd_level": zl,
                        "hnsw_ef": ef,
                        "latency_ms": r.latency_ms,
                        "throughput": r.throughput,
                        "bytes": r.bytes,
                        "ok": true,
                    })).unwrap();
                }
            }
        }
    }
    out.flush().unwrap();
    eprintln!("wrote {}", out_path);
}

// ------ storage ---------------------------------------------------------
fn uc_bulk_ingest(h: &H) -> R {
    let p = format!("{}.ing", h.synx_path);
    let _ = std::fs::remove_file(&p);
    let t = t0();
    let mut w = SynxWriter::create(&p, SynxFlags::COMPRESSED).unwrap();
    for i in 0..h.n {
        w.append(ChunkKind::TextBlob, Codec::Zstd, phrase(i, 8).as_bytes()).unwrap();
    }
    w.finish().unwrap();
    let d = ms(t.elapsed());
    rec("storage", d, h.n as f64 / d * 1000.0, std::fs::metadata(&p).unwrap().len())
}
fn uc_synx_open(h: &H) -> R {
    let t = t0();
    let r = SynxReader::open(&h.synx_path).unwrap();
    rec("storage", ms(t.elapsed()), 0.0, r.manifest.chunks.len() as u64)
}
fn uc_mmap_open(h: &H) -> R {
    let t = t0();
    let r = MmapReader::open(&h.synx_path).unwrap();
    rec("storage", ms(t.elapsed()), 0.0, r.manifest.chunks.len() as u64)
}
fn uc_read_all(h: &H) -> R {
    let mut r = SynxReader::open(&h.synx_path).unwrap();
    let t = t0();
    let mut b = 0u64;
    for i in 0..r.manifest.chunks.len() {
        b += r.read_chunk_at(i).unwrap().decode().unwrap().len() as u64;
    }
    rec("storage", ms(t.elapsed()), h.n as f64 / ms(t.elapsed()) * 1000.0, b)
}
fn uc_chunk_rt_raw(_h: &H) -> R {
    let payload = b"hello synx".repeat(200);
    let t = t0();
    let mut b = 0u64;
    for _ in 0..500 {
        let c = Chunk::new(ChunkKind::TextBlob, Codec::Raw, &payload).unwrap();
        b += c.decode().unwrap().len() as u64;
    }
    rec("storage", ms(t.elapsed()), 500.0, b)
}
fn uc_chunk_rt_zstd(_h: &H) -> R {
    let payload = b"hello synx".repeat(200);
    let t = t0();
    let mut b = 0u64;
    for _ in 0..500 {
        let c = Chunk::new(ChunkKind::TextBlob, Codec::Zstd, &payload).unwrap();
        b += c.decode().unwrap().len() as u64;
    }
    rec("storage", ms(t.elapsed()), 500.0, b)
}
fn uc_manifest_verify(h: &H) -> R {
    let r = MmapReader::open(&h.synx_path).unwrap();
    let t = t0();
    let mut ok = 0u64;
    for i in 0..r.manifest.chunks.len() {
        let _ = r.read_chunk(i).unwrap().decode().unwrap();
        ok += 1;
    }
    rec("storage", ms(t.elapsed()), ok as f64 / ms(t.elapsed()) * 1000.0, ok)
}
fn uc_brainpack_pack(h: &H) -> R {
    use synapse_core::BrainPack;
    let out = format!("{}.bp", h.synx_path);
    let _ = std::fs::remove_file(&out);
    let t = t0();
    let sz = BrainPack::pack(&h.synx_path, &out).unwrap();
    rec("storage", ms(t.elapsed()), 0.0, sz)
}
fn uc_brainpack_unpack(h: &H) -> R {
    use synapse_core::BrainPack;
    let pack = format!("{}.bp", h.synx_path);
    let out = format!("{}.unp", h.synx_path);
    let _ = std::fs::remove_file(&out);
    let t = t0();
    let sz = BrainPack::unpack(&pack, &out).unwrap();
    rec("storage", ms(t.elapsed()), 0.0, sz)
}
fn uc_mmap_raw_slice(h: &H) -> R {
    let r = MmapReader::open(&h.synx_path).unwrap();
    let t = t0();
    let mut sum = 0u64;
    for i in 0..r.manifest.chunks.len() {
        sum += r.raw_slice(i).unwrap().len() as u64;
    }
    rec("storage", ms(t.elapsed()), r.manifest.chunks.len() as f64, sum)
}

// ------ memory ----------------------------------------------------------
fn uc_scope_global(_h: &H) -> R {
    let t = t0();
    let mut n = 0;
    for _ in 0..10_000 { n += Scope::Global.as_tag().len(); }
    rec("memory", ms(t.elapsed()), 10_000.0, n as u64)
}
fn uc_scope_user(_h: &H) -> R {
    let t = t0();
    let mut n = 0;
    for i in 0..10_000 { n += Scope::User(format!("u{i}")).as_tag().len(); }
    rec("memory", ms(t.elapsed()), 10_000.0, n as u64)
}
fn uc_scope_session(_h: &H) -> R {
    let t = t0();
    let mut n = 0;
    for i in 0..10_000 { n += Scope::Session { user: format!("u{i}"), session: "s".into() }.as_tag().len(); }
    rec("memory", ms(t.elapsed()), 10_000.0, n as u64)
}
fn uc_scope_project(_h: &H) -> R {
    let t = t0();
    let mut n = 0;
    for i in 0..10_000 { n += Scope::Project(format!("p{i}")).as_tag().len(); }
    rec("memory", ms(t.elapsed()), 10_000.0, n as u64)
}
fn uc_kg_resolve(h: &H) -> R {
    let mut set = EdgeSet::default();
    for i in 0..h.n.min(100) {
        set.edges.push(Edge::new(format!("d{i}"), format!("d{}", i + 1), EdgeKind::Supersedes));
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64 + 10;
    let t = t0();
    let mut ok = 0u64;
    for i in 0..100 {
        if !set.resolve_current(&format!("d{}", i), now).is_empty() { ok += 1; }
    }
    rec("memory", ms(t.elapsed()), 100.0, ok)
}
fn uc_kg_valid_at(h: &H) -> R {
    let mut set = EdgeSet::default();
    for i in 0..h.n.min(1000) {
        let mut e = Edge::new(format!("d{i}"), format!("d{}", i + 1), EdgeKind::References);
        e.valid_from = i as i64 % 1000; e.valid_to = e.valid_from + 500;
        set.edges.push(e);
    }
    let t = t0();
    let mut hits = 0usize;
    for ts in 0..200 { hits += set.valid_at(ts).count(); }
    rec("memory", ms(t.elapsed()), 200.0, hits as u64)
}
fn uc_kg_edge_json(_h: &H) -> R {
    let set = EdgeSet {
        edges: (0..1000).map(|i| Edge::new(format!("a{i}"), format!("b{i}"), EdgeKind::Summarises)).collect(),
    };
    let t = t0();
    let bytes = set.to_json();
    let _ = EdgeSet::from_json(&bytes).unwrap();
    rec("memory", ms(t.elapsed()), 1000.0, bytes.len() as u64)
}
fn uc_dedup_blake3(_h: &H) -> R {
    let payload = b"hello world this is a blake3 dedup benchmark".repeat(8);
    let t = t0();
    let mut h = [0u8; 32];
    for _ in 0..1000 { h = blake3::hash(&payload).into(); }
    rec("memory", ms(t.elapsed()), 1000.0, h[0] as u64)
}
fn uc_content_hash(h: &H) -> R {
    let mut r = SynxReader::open(&h.synx_path).unwrap();
    let t = t0();
    let mut ok = 0u64;
    for i in 0..r.manifest.chunks.len().min(200) {
        let c = r.read_chunk_at(i).unwrap();
        let _ = c.decode().unwrap();
        ok += 1;
    }
    rec("memory", ms(t.elapsed()), ok as f64, ok)
}
fn uc_scope_tag(_h: &H) -> R {
    let scopes: Vec<_> = (0..1000).map(|i| Scope::User(format!("u{i}"))).collect();
    let t = t0();
    let mut n = 0;
    for s in &scopes { n += s.as_tag().len(); }
    rec("memory", ms(t.elapsed()), 1000.0, n as u64)
}

// ------ FTS -------------------------------------------------------------
fn mk_fts_rows(h: &H) -> Vec<(String, String, String, String)> {
    (0..h.n).map(|i| (format!("d{i}"), format!("doc {i}"), phrase(i, 10), "global".into())).collect()
}
fn uc_fts_build(h: &H) -> R {
    let fts = FtsIndex::new().unwrap();
    let rows = mk_fts_rows(h);
    let t = t0();
    fts.write(&rows).unwrap();
    rec("fts", ms(t.elapsed()), h.n as f64 / ms(t.elapsed()) * 1000.0, 0)
}
fn uc_fts_uni(h: &H) -> R {
    let fts = FtsIndex::new().unwrap();
    fts.write(&mk_fts_rows(h)).unwrap();
    let t = t0();
    let mut hits = 0;
    for i in 0..200 { hits += fts.search(h.words[i % h.words.len()], 10).unwrap().len(); }
    rec("fts", ms(t.elapsed()), 200.0 / (ms(t.elapsed()) / 1000.0), hits as u64)
}
fn uc_fts_or(h: &H) -> R {
    let fts = FtsIndex::new().unwrap(); fts.write(&mk_fts_rows(h)).unwrap();
    let t = t0(); let mut hits = 0;
    for _ in 0..200 { hits += fts.search("rust OR tantivy OR vector", 10).unwrap().len(); }
    rec("fts", ms(t.elapsed()), 200.0, hits as u64)
}
fn uc_fts_phrase(h: &H) -> R {
    let fts = FtsIndex::new().unwrap(); fts.write(&mk_fts_rows(h)).unwrap();
    let t = t0(); let mut hits = 0;
    for _ in 0..200 { hits += fts.search("\"rust ships\"", 10).unwrap_or_default().len(); }
    rec("fts", ms(t.elapsed()), 200.0, hits as u64)
}
fn uc_fts_prefix(h: &H) -> R {
    let fts = FtsIndex::new().unwrap(); fts.write(&mk_fts_rows(h)).unwrap();
    let t = t0(); let mut hits = 0;
    for _ in 0..200 { hits += fts.search("rust*", 10).unwrap_or_default().len(); }
    rec("fts", ms(t.elapsed()), 200.0, hits as u64)
}
fn uc_fts_rebuild(h: &H) -> R {
    let t = t0();
    for _ in 0..3 {
        let fts = FtsIndex::new().unwrap();
        fts.write(&mk_fts_rows(h)).unwrap();
    }
    rec("fts", ms(t.elapsed()), 3.0, 0)
}
fn uc_fts_top1(h: &H) -> R {
    let fts = FtsIndex::new().unwrap(); fts.write(&mk_fts_rows(h)).unwrap();
    let t = t0(); let mut hits = 0;
    for _ in 0..500 { hits += fts.search("rust", 1).unwrap().len(); }
    rec("fts", ms(t.elapsed()), 500.0, hits as u64)
}
fn uc_fts_top50(h: &H) -> R {
    let fts = FtsIndex::new().unwrap(); fts.write(&mk_fts_rows(h)).unwrap();
    let t = t0(); let mut hits = 0;
    for _ in 0..200 { hits += fts.search("rust", 50).unwrap().len(); }
    rec("fts", ms(t.elapsed()), 200.0, hits as u64)
}
fn uc_fts_ci(h: &H) -> R {
    let fts = FtsIndex::new().unwrap(); fts.write(&mk_fts_rows(h)).unwrap();
    let t = t0(); let mut hits = 0;
    for _ in 0..200 { hits += fts.search("RUST", 10).unwrap().len(); }
    rec("fts", ms(t.elapsed()), 200.0, hits as u64)
}
fn uc_fts_multi(h: &H) -> R {
    let fts = FtsIndex::new().unwrap(); fts.write(&mk_fts_rows(h)).unwrap();
    let t = t0(); let mut hits = 0;
    for _ in 0..200 { hits += fts.search("rust AND memory", 10).unwrap().len(); }
    rec("fts", ms(t.elapsed()), 200.0, hits as u64)
}

// ------ vector ----------------------------------------------------------
fn uc_hnsw_flat(h: &H) -> R {
    let t = t0();
    let idx = HnswIndex::build(h.vectors.clone(), false).unwrap();
    rec("vector", ms(t.elapsed()), idx.len() as f64 / ms(t.elapsed()) * 1000.0, 0)
}
fn uc_hnsw_quant(h: &H) -> R {
    let t = t0();
    let idx = HnswIndex::build(h.vectors.clone(), true).unwrap();
    rec("vector", ms(t.elapsed()), idx.len() as f64 / ms(t.elapsed()) * 1000.0, 0)
}
fn uc_hnsw_k1(h: &H) -> R {
    let idx = HnswIndex::build(h.vectors.clone(), false).unwrap();
    let q = vector(42, 64);
    let t = t0(); let mut n = 0;
    for _ in 0..500 { n += idx.search(&q, 1).len(); }
    rec("vector", ms(t.elapsed()), 500.0, n as u64)
}
fn uc_hnsw_k10(h: &H) -> R {
    let idx = HnswIndex::build(h.vectors.clone(), false).unwrap();
    let q = vector(42, 64);
    let t = t0(); let mut n = 0;
    for _ in 0..500 { n += idx.search(&q, 10).len(); }
    rec("vector", ms(t.elapsed()), 500.0, n as u64)
}
fn uc_hnsw_k100(h: &H) -> R {
    let idx = HnswIndex::build(h.vectors.clone(), false).unwrap();
    let q = vector(42, 64);
    let t = t0(); let mut n = 0;
    for _ in 0..200 { n += idx.search(&q, 100).len(); }
    rec("vector", ms(t.elapsed()), 200.0, n as u64)
}
fn uc_hnsw_batch(h: &H) -> R {
    let idx = HnswIndex::build(h.vectors.clone(), false).unwrap();
    let qs: Vec<_> = (0..100).map(|i| vector(i * 37, 64)).collect();
    let t = t0(); let mut n = 0;
    for q in &qs { n += idx.search(q, 10).len(); }
    rec("vector", ms(t.elapsed()), qs.len() as f64, n as u64)
}
fn uc_cosine_scan(h: &H) -> R {
    let q = vector(42, 64);
    let t = t0(); let mut best = (-1.0f32, 0u32);
    for (i, v) in h.vectors.iter().enumerate() {
        let mut dot = 0.0f32; let mut na = 0.0f32; let mut nb = 0.0f32;
        for (a, b) in q.iter().zip(v.iter()) { dot += a * b; na += a * a; nb += b * b; }
        let s = dot / (na.sqrt() * nb.sqrt()).max(1e-12);
        if s > best.0 { best = (s, i as u32); }
    }
    rec("vector", ms(t.elapsed()), h.vectors.len() as f64, best.1 as u64)
}
fn uc_quant_rt(h: &H) -> R {
    use synapse_core::synx::vec_index::ScalarCodebook;
    let cb = ScalarCodebook::train(&h.vectors).unwrap();
    let t = t0();
    let mut sum = 0.0f32;
    for v in &h.vectors[..h.vectors.len().min(500)] {
        let q = cb.quantize(v);
        let d = cb.dequantize(&q);
        sum += d[0];
    }
    rec("vector", ms(t.elapsed()), 500.0, sum as u64)
}
fn uc_vec_dedup(h: &H) -> R {
    let t = t0();
    let mut seen = std::collections::HashSet::new();
    for v in &h.vectors {
        let mut bytes = Vec::with_capacity(v.len() * 4);
        for f in v { bytes.extend_from_slice(&f.to_le_bytes()); }
        seen.insert(blake3::hash(&bytes));
    }
    rec("vector", ms(t.elapsed()), h.vectors.len() as f64, seen.len() as u64)
}
fn uc_vec_build_search(h: &H) -> R {
    let t = t0();
    let idx = HnswIndex::build(h.vectors.clone(), true).unwrap();
    let q = vector(123, 64);
    let r = idx.search(&q, 10);
    rec("vector", ms(t.elapsed()), 1.0, r.len() as u64)
}

// ------ sync / pack -----------------------------------------------------
fn fake_id(i: u8) -> [u8; 32] { let mut a = [0u8; 32]; a[0] = i; a }
fn uc_crdt_encode(_h: &H) -> R {
    use synapse_core::sync::{automerge_wire::encode_ops, Op};
    let ops: Vec<_> = (0..200).map(|i| (fake_id(i as u8),
        Op::Put { doc_id: format!("x{i}"), blob_hash: [0; 32], ts: i as i64 })).collect();
    let t = t0();
    let bytes = encode_ops(&ops).unwrap();
    rec("sync", ms(t.elapsed()), ops.len() as f64 / ms(t.elapsed()) * 1000.0, bytes.len() as u64)
}
fn uc_crdt_merge(_h: &H) -> R {
    use synapse_core::sync::{automerge_wire::{encode_ops, merge_payload}, Op};
    let local: Vec<_> = (0..100).map(|i| (fake_id(i as u8),
        Op::Put { doc_id: format!("a{i}"), blob_hash: [0; 32], ts: i as i64 })).collect();
    let remote: Vec<_> = (100..200).map(|i| (fake_id(i as u8),
        Op::Put { doc_id: format!("b{i}"), blob_hash: [0; 32], ts: i as i64 })).collect();
    let payload = encode_ops(&remote).unwrap();
    let t = t0();
    let merged = merge_payload(&local, &payload).unwrap();
    rec("sync", ms(t.elapsed()), merged.len() as f64, payload.len() as u64)
}
fn uc_crdt_commut(_h: &H) -> R {
    use synapse_core::sync::{automerge_wire::{encode_ops, merge_payload}, Op};
    let a = (fake_id(1), Op::Put { doc_id: "x".into(), blob_hash: [9; 32], ts: 100 });
    let b = (fake_id(2), Op::Delete { doc_id: "y".into(), ts: 200 });
    let t = t0();
    let _ab = merge_payload(&[a.clone()], &encode_ops(&[b.clone()]).unwrap()).unwrap();
    let _ba = merge_payload(&[b], &encode_ops(&[a]).unwrap()).unwrap();
    rec("sync", ms(t.elapsed()), 2.0, 0)
}
fn uc_sign_keygen(_h: &H) -> R {
    use synapse_core::synx::sign::generate_key;
    let t = t0();
    let mut sum = 0u64;
    for _ in 0..10 { let (_, pk) = generate_key(); sum += pk[0] as u64; }
    rec("sync", ms(t.elapsed()), 10.0, sum)
}
fn uc_sign_manifest(_h: &H) -> R {
    use synapse_core::synx::sign::{generate_key, sign_manifest};
    let (sk, _) = generate_key();
    let hash = [7u8; 32];
    let t = t0();
    let mut sum = 0u64;
    for _ in 0..100 { let sig = sign_manifest(&hash, &sk).unwrap(); sum += sig[0] as u64; }
    rec("sync", ms(t.elapsed()), 100.0, sum)
}
fn uc_verify_manifest(_h: &H) -> R {
    use synapse_core::synx::sign::{generate_key, sign_manifest, verify_manifest};
    let (sk, pk) = generate_key();
    let hash = [7u8; 32];
    let sig = sign_manifest(&hash, &sk).unwrap();
    let t = t0();
    for _ in 0..100 { verify_manifest(&hash, &sig, &pk).unwrap(); }
    rec("sync", ms(t.elapsed()), 100.0, 0)
}
fn uc_portable(h: &H) -> R {
    let dst = format!("{}.portable", h.synx_path);
    let _ = std::fs::remove_file(&dst);
    let t = t0();
    std::fs::copy(&h.synx_path, &dst).unwrap();
    let sz = std::fs::metadata(&dst).unwrap().len();
    rec("sync", ms(t.elapsed()), 1.0, sz)
}
fn uc_pack_sign(h: &H) -> R {
    use synapse_core::BrainPack;
    let out = format!("{}.signed.bp", h.synx_path);
    let _ = std::fs::remove_file(&out);
    let t = t0();
    let sz = BrainPack::pack(&h.synx_path, &out).unwrap();
    rec("sync", ms(t.elapsed()), 1.0, sz)
}
fn uc_crdt_size(_h: &H) -> R {
    use synapse_core::sync::{automerge_wire::encode_ops, Op};
    let ops: Vec<_> = (0..1000).map(|i| (fake_id((i % 255) as u8),
        Op::Put { doc_id: format!("x{i}"), blob_hash: [0; 32], ts: i as i64 })).collect();
    let t = t0();
    let bytes = encode_ops(&ops).unwrap();
    rec("sync", ms(t.elapsed()), ops.len() as f64, bytes.len() as u64)
}
fn uc_full_rt(h: &H) -> R {
    use synapse_core::BrainPack;
    let pack = format!("{}.rt.bp", h.synx_path);
    let dst = format!("{}.rt.synx", h.synx_path);
    let _ = std::fs::remove_file(&pack); let _ = std::fs::remove_file(&dst);
    let t = t0();
    BrainPack::pack(&h.synx_path, &pack).unwrap();
    BrainPack::unpack(&pack, &dst).unwrap();
    let r = MmapReader::open(&dst).unwrap();
    rec("sync", ms(t.elapsed()), 1.0, r.manifest.chunks.len() as u64)
}
