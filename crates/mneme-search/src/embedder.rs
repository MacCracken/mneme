//! Local embedding engine using ONNX Runtime.
//!
//! Loads all-MiniLM-L6-v2 (384-dim) for sentence embeddings.
//! Gated behind the `local-vectors` feature.

use std::path::Path;

use ort::session::Session;

use crate::SearchError;

/// Dimension of all-MiniLM-L6-v2 embeddings.
pub const EMBEDDING_DIM: usize = 384;

fn ort_err(e: impl std::fmt::Display) -> SearchError {
    SearchError::Embedding(e.to_string())
}

/// Local ONNX-based sentence embedder.
pub struct Embedder {
    session: std::sync::Mutex<Session>,
    tokenizer: tokenizers::Tokenizer,
}

impl Embedder {
    /// Load model and tokenizer from the given directory.
    ///
    /// Expects `all-MiniLM-L6-v2.onnx` and `tokenizer.json` in `models_dir`.
    pub fn open(models_dir: &Path) -> Result<Self, SearchError> {
        let model_path = models_dir.join("all-MiniLM-L6-v2.onnx");
        let tokenizer_path = models_dir.join("tokenizer.json");

        if !model_path.exists() {
            return Err(SearchError::ModelNotFound(
                model_path.display().to_string(),
            ));
        }
        if !tokenizer_path.exists() {
            return Err(SearchError::ModelNotFound(
                tokenizer_path.display().to_string(),
            ));
        }

        let session = Session::builder()
            .map_err(ort_err)?
            .with_intra_threads(1)
            .map_err(ort_err)?
            .commit_from_file(&model_path)
            .map_err(ort_err)?;

        let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| SearchError::Embedding(e.to_string()))?;

        tracing::info!(
            "Loaded embedding model from {}",
            models_dir.display()
        );

        Ok(Self {
            session: std::sync::Mutex::new(session),
            tokenizer,
        })
    }

    /// Produce a 384-dim embedding for the given text.
    pub fn embed(&self, text: &str) -> Result<Vec<f32>, SearchError> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|e| SearchError::Embedding(e.to_string()))?;

        let ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        let attention: Vec<i64> = encoding
            .get_attention_mask()
            .iter()
            .map(|&m| m as i64)
            .collect();
        let type_ids: Vec<i64> = encoding
            .get_type_ids()
            .iter()
            .map(|&t| t as i64)
            .collect();

        let len = ids.len();

        // Create ort::Value tensors from ndarray
        let ids_arr = ndarray::Array2::from_shape_vec((1, len), ids).map_err(ort_err)?;
        let attn_arr =
            ndarray::Array2::from_shape_vec((1, len), attention.clone()).map_err(ort_err)?;
        let type_arr = ndarray::Array2::from_shape_vec((1, len), type_ids).map_err(ort_err)?;

        let ids_val = ort::value::Tensor::from_array(ids_arr).map_err(ort_err)?;
        let attn_val = ort::value::Tensor::from_array(attn_arr).map_err(ort_err)?;
        let type_val = ort::value::Tensor::from_array(type_arr).map_err(ort_err)?;

        let inputs = ort::inputs![ids_val, attn_val, type_val];

        let mut session = self
            .session
            .lock()
            .map_err(|e| SearchError::Embedding(e.to_string()))?;
        let outputs = session.run(inputs).map_err(ort_err)?;

        // Extract output as flat f32 slice: (shape, &[f32])
        let (shape, data) = outputs[0].try_extract_tensor::<f32>().map_err(ort_err)?;

        // shape: [1, seq_len, dim]
        let seq_len = shape[1] as usize;
        let dim = shape[2] as usize;

        // Mean pooling with attention mask
        let mut pooled = vec![0.0f32; dim];
        let mut mask_sum = 0.0f32;

        for t in 0..seq_len {
            let mask_val = attention[t] as f32;
            mask_sum += mask_val;
            let offset = t * dim;
            for d in 0..dim {
                pooled[d] += data[offset + d] * mask_val;
            }
        }

        if mask_sum > 0.0 {
            for val in &mut pooled {
                *val /= mask_sum;
            }
        }

        // L2 normalize
        let norm: f32 = pooled.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for val in &mut pooled {
                *val /= norm;
            }
        }

        Ok(pooled)
    }

    /// Embed multiple texts, returning one vector per input.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SearchError> {
        texts.iter().map(|t| self.embed(t)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_not_found_returns_error() {
        let result = Embedder::open(Path::new("/nonexistent/path"));
        assert!(matches!(result, Err(SearchError::ModelNotFound(_))));
    }

    #[test]
    fn embedding_dim_constant() {
        assert_eq!(EMBEDDING_DIM, 384);
    }
}
