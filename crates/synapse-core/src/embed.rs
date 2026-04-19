//! Embedding pipeline via fastembed-rs (BGE-small-en-v1.5, 384-dim ONNX).

use crate::error::{Error, Result};
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

pub struct Embedder {
    model: TextEmbedding,
}

impl Embedder {
    pub fn new() -> Result<Self> {
        let model = TextEmbedding::try_new(
            InitOptions::new(EmbeddingModel::BGESmallENV15).with_show_download_progress(false),
        ).map_err(|e| Error::Other(format!("fastembed init: {e}")))?;
        Ok(Self { model })
    }

    pub fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.model.embed(texts.to_vec(), None)
            .map_err(|e| Error::Other(format!("embed: {e}")))
    }

    pub fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        let mut out = self.embed_batch(&[text.to_string()])?;
        out.pop().ok_or_else(|| Error::Other("empty embed".into()))
    }
}
