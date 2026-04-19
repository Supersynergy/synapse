//! Embedding pipeline: fastembed-rs (BGE-small-en-v1.5 ONNX, 384-dim) + redb BLAKE3 cache.
//!
//! Cache is persistent — identical text -> zero recompute across daemon restarts.

use crate::error::{Error, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use redb::{Database, ReadableTableMetadata, TableDefinition};
use std::path::{Path, PathBuf};
use std::sync::Arc;

const EMB_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("emb_cache_v1");

pub struct Embedder {
    model: TextEmbedding,
    cache: Option<Arc<Database>>,
}

impl Embedder {
    pub fn new() -> Result<Self> {
        Self::new_with_cache::<PathBuf>(None)
    }

    pub fn new_with_cache<P: AsRef<Path>>(cache_path: Option<P>) -> Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGESmallENV15).with_show_download_progress(false),
        ).map_err(|e| Error::Other(format!("fastembed init: {e}")))?;
        let cache = match cache_path {
            Some(p) => {
                if let Some(parent) = p.as_ref().parent() { std::fs::create_dir_all(parent).ok(); }
                let db = Database::create(p.as_ref())
                    .map_err(|e| Error::Other(format!("redb create: {e}")))?;
                let wtx = db.begin_write().map_err(|e| Error::Other(format!("redb wtx: {e}")))?;
                { let _ = wtx.open_table(EMB_TABLE).map_err(|e| Error::Other(format!("redb open: {e}")))?; }
                wtx.commit().map_err(|e| Error::Other(format!("redb commit: {e}")))?;
                Some(Arc::new(db))
            }
            None => None,
        };
        Ok(Self { model, cache })
    }

    pub fn embed_batch(&mut self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if self.cache.is_some() {
            let cache = self.cache.clone().unwrap();
            return self.embed_batch_cached(&cache, texts);
        }
        self.model.embed(texts.to_vec(), None)
            .map_err(|e| Error::Other(format!("embed: {e}")))
    }

    fn embed_batch_cached(&mut self, cache: &Database, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        let hashes: Vec<[u8; 32]> = texts.iter()
            .map(|t| *blake3::hash(t.as_bytes()).as_bytes()).collect();
        let mut out: Vec<Option<Vec<f32>>> = vec![None; texts.len()];
        let mut miss_idx: Vec<usize> = Vec::new();
        {
            let rtx = cache.begin_read().map_err(|e| Error::Other(format!("redb rtx: {e}")))?;
            let t = rtx.open_table(EMB_TABLE).map_err(|e| Error::Other(format!("redb tbl: {e}")))?;
            for (i, h) in hashes.iter().enumerate() {
                if let Some(v) = t.get(h.as_slice()).map_err(|e| Error::Other(format!("redb get: {e}")))? {
                    let bytes = v.value();
                    let mut v = Vec::with_capacity(bytes.len() / 4);
                    for chunk in bytes.chunks_exact(4) {
                        v.push(f32::from_le_bytes(chunk.try_into().unwrap()));
                    }
                    out[i] = Some(v);
                } else {
                    miss_idx.push(i);
                }
            }
        }
        if !miss_idx.is_empty() {
            let miss_texts: Vec<String> = miss_idx.iter().map(|&i| texts[i].clone()).collect();
            let new_embs = self.model.embed(miss_texts, None)
                .map_err(|e| Error::Other(format!("embed: {e}")))?;
            let wtx = cache.begin_write().map_err(|e| Error::Other(format!("redb wtx: {e}")))?;
            {
                let mut t = wtx.open_table(EMB_TABLE).map_err(|e| Error::Other(format!("redb tbl: {e}")))?;
                for (emb, &i) in new_embs.iter().zip(miss_idx.iter()) {
                    let bytes: Vec<u8> = emb.iter().flat_map(|f| f.to_le_bytes()).collect();
                    t.insert(hashes[i].as_slice(), bytes.as_slice())
                        .map_err(|e| Error::Other(format!("redb ins: {e}")))?;
                    out[i] = Some(emb.clone());
                }
            }
            wtx.commit().map_err(|e| Error::Other(format!("redb commit: {e}")))?;
        }
        Ok(out.into_iter().map(|o| o.unwrap()).collect())
    }

    pub fn embed_one(&mut self, text: &str) -> Result<Vec<f32>> {
        let mut out = self.embed_batch(&[text.to_string()])?;
        out.pop().ok_or_else(|| Error::Other("empty embed".into()))
    }

    pub fn cache_stats(&self) -> Result<Option<u64>> {
        let Some(ref c) = self.cache else { return Ok(None); };
        let rtx = c.begin_read().map_err(|e| Error::Other(format!("{e}")))?;
        let t = rtx.open_table(EMB_TABLE).map_err(|e| Error::Other(format!("{e}")))?;
        Ok(Some(t.len().map_err(|e| Error::Other(format!("{e}")))?))
    }
}
