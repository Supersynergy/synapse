//! IVF sharding for large brains (>50k docs).
//! Split: k-means on embeddings → N shard SQLite files.
//! Query: bloom prefilter → centroid-nearest shards → parallel fan-out → RRF merge.

use crate::error::{Error, Result};
use crate::types::{Hit, SearchMode, EMBED_DIM};
use crate::db::Store;
use anyhow::Context;
use base64::Engine as _;
use fastbloom::BloomFilter;
use linfa::prelude::{Fit, Predict};
use linfa::DatasetBase;
use linfa_clustering::KMeans;
use ndarray::{Array2, ArrayView1};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const AUTO_SHARD_THRESHOLD: usize = 50_000;
pub const DEFAULT_CENTROIDS: usize = 32;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardMeta {
    pub path: PathBuf,
    /// Base64-encoded little-endian f32 * EMBED_DIM
    pub centroid_b64: String,
    /// Base64-encoded serialized bloom filter bytes
    pub bloom_b64: String,
    pub doc_count: usize,
}

impl ShardMeta {
    pub fn centroid(&self) -> Result<[f32; EMBED_DIM]> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&self.centroid_b64)
            .map_err(|e| Error::Other(format!("base64 centroid: {e}")))?;
        if bytes.len() != EMBED_DIM * 4 {
            return Err(Error::Other(format!(
                "centroid bytes {} != {}",
                bytes.len(),
                EMBED_DIM * 4
            )));
        }
        let mut arr = [0f32; EMBED_DIM];
        for (i, chunk) in bytes.chunks_exact(4).enumerate() {
            arr[i] = f32::from_le_bytes(chunk.try_into().unwrap());
        }
        Ok(arr)
    }

    pub fn bloom_contains(&self, token: &str) -> Result<bool> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&self.bloom_b64)
            .map_err(|e| Error::Other(format!("base64 bloom: {e}")))?;
        let bloom: BloomFilter = bincode_bloom_from_bytes(&bytes)?;
        Ok(bloom.contains(token))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShardManifest {
    pub shards: Vec<ShardMeta>,
}

impl ShardManifest {
    pub fn load(path: &Path) -> Result<Self> {
        let s = std::fs::read_to_string(path)
            .with_context(|| format!("read manifest {}", path.display()))
            .map_err(|e| Error::Other(e.to_string()))?;
        toml::from_str(&s).map_err(|e| Error::Other(format!("parse manifest: {e}")))
    }

    pub fn save(&self, path: &Path) -> Result<()> {
        let s = toml::to_string_pretty(self)
            .map_err(|e| Error::Other(format!("serialize manifest: {e}")))?;
        std::fs::write(path, s)
            .with_context(|| format!("write manifest {}", path.display()))
            .map_err(|e| Error::Other(e.to_string()))
    }
}

pub struct ShardManager {
    pub manifest_path: PathBuf,
    pub shards: Vec<ShardMeta>,
}

impl ShardManager {
    pub fn open(manifest_path: PathBuf) -> Result<Self> {
        let manifest = ShardManifest::load(&manifest_path)?;
        Ok(Self { manifest_path, shards: manifest.shards })
    }

    /// Query: bloom-prefilter tokens → top-2 centroid-nearest shards → fan-out → RRF merge.
    pub fn query(
        &self,
        q: &str,
        query_emb: &[f32; EMBED_DIM],
        mode: SearchMode,
        limit: usize,
    ) -> Result<Vec<Hit>> {
        let tokens: Vec<&str> = q.split_whitespace().collect();

        // Bloom prefilter: keep shards that contain at least one query token
        let candidate_shards: Vec<&ShardMeta> = if tokens.is_empty() {
            self.shards.iter().collect()
        } else {
            let mut candidates: Vec<&ShardMeta> = self
                .shards
                .iter()
                .filter(|s| {
                    tokens.iter().any(|t| s.bloom_contains(t).unwrap_or(true))
                })
                .collect();
            if candidates.is_empty() {
                candidates = self.shards.iter().collect();
            }
            candidates
        };

        // Centroid-nearest: rank candidates by cosine similarity to query embedding, take top-2
        let mut scored: Vec<(f32, &ShardMeta)> = candidate_shards
            .iter()
            .map(|s| {
                let c = s.centroid().unwrap_or([0f32; EMBED_DIM]);
                let sim = cosine_sim(query_emb, &c);
                (sim, *s)
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        let top_shards: Vec<&ShardMeta> = scored.iter().take(2).map(|x| x.1).collect();

        // Parallel fan-out
        let per_shard_hits: Vec<Vec<Hit>> = top_shards
            .par_iter()
            .map(|shard| query_shard(shard, q, query_emb, mode, limit * 2).unwrap_or_default())
            .collect();

        // RRF merge
        Ok(rrf_merge(per_shard_hits, limit))
    }
}

fn query_shard(
    shard: &ShardMeta,
    q: &str,
    query_emb: &[f32; EMBED_DIM],
    mode: SearchMode,
    limit: usize,
) -> Result<Vec<Hit>> {
    let store = Store::open(&shard.path)?;
    let emb_slice: &[f32] = query_emb;
    match mode {
        SearchMode::Lex => store.search(q, mode, None, limit),
        SearchMode::Vec | SearchMode::Hybrid => store.search(q, mode, Some(emb_slice), limit),
    }
}

fn rrf_merge(lists: Vec<Vec<Hit>>, limit: usize) -> Vec<Hit> {
    use std::collections::HashMap;
    let k = 60.0f64;
    let mut scores: HashMap<i64, f64> = HashMap::new();
    let mut hits_map: HashMap<i64, Hit> = HashMap::new();
    for list in &lists {
        for (rank, hit) in list.iter().enumerate() {
            let rrf = 1.0 / (k + rank as f64 + 1.0);
            *scores.entry(hit.id).or_default() += rrf;
            hits_map.entry(hit.id).or_insert_with(|| hit.clone());
        }
    }
    let mut merged: Vec<Hit> = hits_map
        .into_values()
        .map(|mut h| {
            h.score = scores[&h.id];
            h
        })
        .collect();
    merged.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    merged.truncate(limit);
    merged
}

/// Split a brain.db into N shard files via k-means on embeddings.
pub fn split(
    source_db: &Path,
    out_dir: &Path,
    n_shards: Option<usize>,
) -> Result<ShardManifest> {
    std::fs::create_dir_all(out_dir)
        .map_err(|e| Error::Other(format!("create out_dir: {e}")))?;

    let store = Store::open(source_db)?;
    let (ids, texts, embeddings) = load_all_docs_with_embeddings(&store)?;

    if embeddings.is_empty() {
        return Err(Error::Other("no embedded docs to shard".into()));
    }

    let k = n_shards.unwrap_or_else(|| {
        let n = embeddings.len();
        (n / 5000).max(2).min(DEFAULT_CENTROIDS)
    });

    // Build ndarray matrix [n_docs x EMBED_DIM]
    let n = embeddings.len();
    let flat: Vec<f32> = embeddings.iter().flatten().copied().collect();
    let matrix = Array2::from_shape_vec((n, EMBED_DIM), flat)
        .map_err(|e| Error::Other(format!("ndarray: {e}")))?;

    // K-means clustering
    let dataset = DatasetBase::from(matrix.mapv(|x| x as f64));
    let model = KMeans::params(k)
        .fit(&dataset)
        .map_err(|e| Error::Other(format!("kmeans: {e}")))?;

    let assignments = model.predict(&dataset);
    let centroids = model.centroids();

    // Group doc indices by shard assignment
    let mut shard_doc_indices: Vec<Vec<usize>> = vec![vec![]; k];
    for (doc_idx, &shard_id) in assignments.iter().enumerate() {
        shard_doc_indices[shard_id].push(doc_idx);
    }

    let mut shard_metas = Vec::new();

    for (shard_id, doc_indices) in shard_doc_indices.iter().enumerate() {
        if doc_indices.is_empty() {
            continue;
        }

        let shard_path = out_dir.join(format!("shard_{:04}.db", shard_id));
        let mut shard_store = Store::open(&shard_path)?;

        // Extract centroid for this shard
        let centroid_row: ArrayView1<f64> = centroids.row(shard_id);
        let centroid: [f32; EMBED_DIM] = {
            let mut arr = [0f32; EMBED_DIM];
            for (i, v) in centroid_row.iter().enumerate() {
                arr[i] = *v as f32;
            }
            arr
        };
        let centroid_bytes: Vec<u8> = centroid.iter().flat_map(|f| f.to_le_bytes()).collect();
        let centroid_b64 =
            base64::engine::general_purpose::STANDARD.encode(&centroid_bytes);

        // Build bloom filter from all tokens in shard docs
        let mut bloom =
            BloomFilter::with_false_pos(0.01).expected_items(doc_indices.len() * 50 + 1);
        let mut shard_reqs = Vec::new();

        for &doc_idx in doc_indices {
            let text = &texts[doc_idx];
            for token in text.split_whitespace() {
                bloom.insert(token);
            }
            let emb = embeddings[doc_idx].clone();
            let req = crate::types::PutRequest {
                uri: None,
                title: None,
                text: text.clone(),
                meta: None,
                embedding: Some(emb),
            };
            shard_reqs.push(req);
        }

        shard_store.put_batch(&shard_reqs)?;

        let bloom_bytes = bloom_to_bytes(&bloom)?;
        let bloom_b64 = base64::engine::general_purpose::STANDARD.encode(&bloom_bytes);

        shard_metas.push(ShardMeta {
            path: shard_path,
            centroid_b64,
            bloom_b64,
            doc_count: doc_indices.len(),
        });

        tracing::info!(
            "shard {shard_id}: {} docs → {}",
            doc_indices.len(),
            out_dir.join(format!("shard_{:04}.db", shard_id)).display()
        );
    }

    let manifest = ShardManifest { shards: shard_metas };
    let _ = ids; // suppress unused warning — ids kept for future use
    Ok(manifest)
}

fn load_all_docs_with_embeddings(
    store: &Store,
) -> Result<(Vec<i64>, Vec<String>, Vec<Vec<f32>>)> {
    let mut ids = Vec::new();
    let mut texts = Vec::new();
    let mut embeddings = Vec::new();

    let mut stmt = store.conn.prepare(
        "SELECT d.id, d.text, v.embedding FROM docs d
         JOIN docs_vec v ON v.id = d.id
         ORDER BY d.id",
    )?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let id: i64 = row.get(0)?;
        let text: String = row.get(1)?;
        let bytes: Vec<u8> = row.get(2)?;
        if bytes.len() == EMBED_DIM * 4 {
            let emb: Vec<f32> = bytes
                .chunks_exact(4)
                .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
                .collect();
            ids.push(id);
            texts.push(text);
            embeddings.push(emb);
        }
    }
    Ok((ids, texts, embeddings))
}

fn cosine_sim(a: &[f32; EMBED_DIM], b: &[f32; EMBED_DIM]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let na: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na == 0.0 || nb == 0.0 { 0.0 } else { dot / (na * nb) }
}

/// Serialize: [u32 LE num_hashes][u64 LE bits...]
fn bloom_to_bytes(bloom: &BloomFilter) -> Result<Vec<u8>> {
    let num_hashes = bloom.num_hashes();
    let bits: Vec<u64> = bloom.iter().collect();
    let mut out = Vec::with_capacity(4 + bits.len() * 8);
    out.extend_from_slice(&num_hashes.to_le_bytes());
    for b in &bits {
        out.extend_from_slice(&b.to_le_bytes());
    }
    Ok(out)
}

fn bincode_bloom_from_bytes(bytes: &[u8]) -> Result<BloomFilter> {
    if bytes.len() < 4 {
        return Err(Error::Other("bloom bytes too short".into()));
    }
    let num_hashes = u32::from_le_bytes(bytes[..4].try_into().unwrap());
    let bits: Vec<u64> = bytes[4..]
        .chunks_exact(8)
        .map(|c| u64::from_le_bytes(c.try_into().unwrap()))
        .collect();
    if bits.is_empty() {
        return Err(Error::Other("bloom bits empty".into()));
    }
    Ok(BloomFilter::from_vec(bits).hashes(num_hashes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_random_embedding() -> Vec<f32> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        (0..EMBED_DIM)
            .map(|i| {
                let mut h = DefaultHasher::new();
                i.hash(&mut h);
                let v = h.finish() as f32 / u64::MAX as f32;
                v * 2.0 - 1.0
            })
            .collect()
    }

    fn make_seeded_embedding(seed: u64) -> Vec<f32> {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        (0..EMBED_DIM)
            .map(|i| {
                let mut h = DefaultHasher::new();
                (seed, i).hash(&mut h);
                let v = h.finish() as f32 / u64::MAX as f32;
                v * 2.0 - 1.0
            })
            .collect()
    }

    #[test]
    fn test_shard_split_and_query_recall() {
        let dir = TempDir::new().unwrap();
        let db_path = dir.path().join("brain.db");
        let mut store = Store::open(&db_path).unwrap();

        // Insert 100 synthetic docs with embeddings
        let n_docs = 100usize;
        let mut all_reqs = Vec::new();
        for i in 0..n_docs {
            let emb = make_seeded_embedding(i as u64);
            all_reqs.push(crate::types::PutRequest {
                uri: Some(format!("doc:{}", i)),
                title: Some(format!("Title {}", i)),
                text: format!("synthetic document number {} about topic {}", i, i % 10),
                meta: None,
                embedding: Some(emb),
            });
        }
        store.put_batch(&all_reqs).unwrap();

        // Split into shards
        let shard_dir = dir.path().join("shards");
        let manifest = split(&db_path, &shard_dir, Some(2)).unwrap();
        assert_eq!(manifest.shards.len(), 2);
        let total: usize = manifest.shards.iter().map(|s| s.doc_count).sum();
        assert_eq!(total, n_docs);

        // Save manifest
        let manifest_path = shard_dir.join("brain.shards.toml");
        manifest.save(&manifest_path).unwrap();

        // Full-scan baseline: all doc ids
        let baseline_ids: std::collections::HashSet<i64> = {
            let store = Store::open(&db_path).unwrap();
            let q_emb = make_seeded_embedding(5);
            let hits = store.search("document", SearchMode::Hybrid, Some(&q_emb), n_docs).unwrap();
            hits.iter().map(|h| h.id).collect()
        };

        // Shard query
        let manager = ShardManager::open(manifest_path).unwrap();
        let q_emb_arr: [f32; EMBED_DIM] = make_seeded_embedding(5).try_into().unwrap();
        let shard_hits = manager.query("document", &q_emb_arr, SearchMode::Hybrid, n_docs).unwrap();
        let shard_ids: std::collections::HashSet<i64> = shard_hits.iter().map(|h| h.id).collect();

        // Recall: shard results should contain ≥95% of baseline
        let intersection = baseline_ids.intersection(&shard_ids).count();
        let recall = if baseline_ids.is_empty() {
            1.0
        } else {
            intersection as f64 / baseline_ids.len() as f64
        };
        assert!(
            recall >= 0.95,
            "recall={:.2} < 0.95 (baseline={} shard={})",
            recall,
            baseline_ids.len(),
            shard_ids.len()
        );
    }
}
