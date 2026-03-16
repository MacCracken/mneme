//! Pluggable embedding backends — local ONNX or remote HTTP (OpenAI-compatible).
//!
//! Allows Mneme to use Synapse, Ollama, OpenAI, or any provider that exposes
//! an OpenAI-compatible `/v1/embeddings` endpoint, while falling back to
//! the bundled ONNX model when no remote service is available.

use serde::{Deserialize, Serialize};

use crate::SearchError;

/// Configuration for the embedding backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Backend type: "local" (ONNX) or "remote" (HTTP).
    /// Default: "auto" — try remote first, fall back to local.
    #[serde(default = "default_backend")]
    pub backend: String,
    /// URL for the remote embedding service (e.g. "http://127.0.0.1:8420").
    #[serde(default)]
    pub remote_url: Option<String>,
    /// Model name to request from the remote service.
    #[serde(default)]
    pub model: Option<String>,
    /// API key for the remote service (optional).
    #[serde(default)]
    pub api_key: Option<String>,
    /// Embedding dimension (must match vector store). Default: 384 for local ONNX.
    #[serde(default)]
    pub dimensions: Option<usize>,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            backend: "auto".into(),
            remote_url: None,
            model: None,
            api_key: None,
            dimensions: None,
        }
    }
}

fn default_backend() -> String {
    "auto".into()
}

/// Trait for embedding text into vectors.
pub trait EmbeddingBackend: Send + Sync {
    /// Embed a single text into a vector.
    fn embed(&self, text: &str) -> Result<Vec<f32>, SearchError>;

    /// Embed a batch of texts. Default implementation calls `embed` in a loop.
    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SearchError> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// The embedding dimension this backend produces.
    fn dimension(&self) -> usize;

    /// Human-readable name of the backend (for status reporting).
    fn name(&self) -> &str;
}

/// Remote embedding backend using the OpenAI-compatible `/v1/embeddings` API.
///
/// Works with Synapse, Ollama, OpenAI, Azure OpenAI, or any compatible provider.
pub struct RemoteHttpBackend {
    client: reqwest::Client,
    base_url: String,
    model: String,
    api_key: Option<String>,
    dimension: usize,
    rt: tokio::runtime::Handle,
}

impl RemoteHttpBackend {
    /// Create a new remote backend. Performs a health check to verify connectivity
    /// and determine the embedding dimension.
    pub fn new(
        base_url: &str,
        model: &str,
        api_key: Option<String>,
        expected_dim: Option<usize>,
    ) -> Result<Self, SearchError> {
        let client = reqwest::Client::new();
        let rt = tokio::runtime::Handle::try_current()
            .map_err(|e| SearchError::VectorStore(format!("No tokio runtime: {e}")))?;

        let mut backend = Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            model: model.to_string(),
            api_key,
            dimension: expected_dim.unwrap_or(384),
            rt,
        };

        // Probe dimension with a test embedding
        match backend.embed("dimension probe") {
            Ok(vec) => {
                backend.dimension = vec.len();
                tracing::info!(
                    "Remote embedding backend ready: {} ({}d, model={})",
                    backend.base_url,
                    backend.dimension,
                    backend.model,
                );
            }
            Err(e) => {
                tracing::warn!("Remote embedding probe failed: {e}");
                return Err(e);
            }
        }

        Ok(backend)
    }
}

impl EmbeddingBackend for RemoteHttpBackend {
    fn embed(&self, text: &str) -> Result<Vec<f32>, SearchError> {
        let result = self.rt.block_on(async {
            let url = format!("{}/v1/embeddings", self.base_url);
            let body = serde_json::json!({
                "model": self.model,
                "input": text,
            });

            let mut req = self.client.post(&url).json(&body);
            if let Some(key) = &self.api_key {
                req = req.bearer_auth(key);
            }

            let resp = req.send().await.map_err(|e| {
                SearchError::VectorStore(format!("Remote embed request failed: {e}"))
            })?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(SearchError::VectorStore(format!(
                    "Remote embed error {status}: {body}"
                )));
            }

            let parsed: EmbeddingResponse = resp.json().await.map_err(|e| {
                SearchError::VectorStore(format!("Remote embed parse error: {e}"))
            })?;

            parsed
                .data
                .into_iter()
                .next()
                .map(|d| d.embedding)
                .ok_or_else(|| SearchError::VectorStore("Empty embedding response".into()))
        });

        result
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SearchError> {
        self.rt.block_on(async {
            let url = format!("{}/v1/embeddings", self.base_url);
            let body = serde_json::json!({
                "model": self.model,
                "input": texts,
            });

            let mut req = self.client.post(&url).json(&body);
            if let Some(key) = &self.api_key {
                req = req.bearer_auth(key);
            }

            let resp = req.send().await.map_err(|e| {
                SearchError::VectorStore(format!("Remote batch embed failed: {e}"))
            })?;

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(SearchError::VectorStore(format!(
                    "Remote batch embed error {status}: {body}"
                )));
            }

            let parsed: EmbeddingResponse = resp.json().await.map_err(|e| {
                SearchError::VectorStore(format!("Remote batch embed parse error: {e}"))
            })?;

            Ok(parsed.data.into_iter().map(|d| d.embedding).collect())
        })
    }

    fn dimension(&self) -> usize {
        self.dimension
    }

    fn name(&self) -> &str {
        "remote-http"
    }
}

/// Wrapper around the local ONNX embedder to implement `EmbeddingBackend`.
#[cfg(feature = "local-vectors")]
pub struct LocalOnnxBackend {
    embedder: crate::embedder::Embedder,
}

#[cfg(feature = "local-vectors")]
impl LocalOnnxBackend {
    pub fn new(embedder: crate::embedder::Embedder) -> Self {
        Self { embedder }
    }
}

#[cfg(feature = "local-vectors")]
impl EmbeddingBackend for LocalOnnxBackend {
    fn embed(&self, text: &str) -> Result<Vec<f32>, SearchError> {
        self.embedder.embed(text)
    }

    fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, SearchError> {
        self.embedder.embed_batch(texts)
    }

    fn dimension(&self) -> usize {
        crate::embedder::EMBEDDING_DIM
    }

    fn name(&self) -> &str {
        "local-onnx"
    }
}

/// Build the best available embedding backend based on config.
///
/// Strategy for `backend = "auto"`:
/// 1. Try remote HTTP if `remote_url` is configured
/// 2. Fall back to local ONNX if available
/// 3. Return None if neither works
pub fn build_backend(
    config: &EmbeddingConfig,
    #[allow(unused_variables)] models_dir: &std::path::Path,
) -> Option<Box<dyn EmbeddingBackend>> {
    match config.backend.as_str() {
        "remote" => build_remote(config),
        "local" => build_local(config, models_dir),
        _ => {
            // "auto": try remote first, then local
            if let Some(backend) = build_remote(config) {
                Some(backend)
            } else {
                build_local(config, models_dir)
            }
        }
    }
}

fn build_remote(config: &EmbeddingConfig) -> Option<Box<dyn EmbeddingBackend>> {
    let url = config.remote_url.as_deref()?;
    let model = config.model.as_deref().unwrap_or("default");
    match RemoteHttpBackend::new(url, model, config.api_key.clone(), config.dimensions) {
        Ok(b) => Some(Box::new(b)),
        Err(e) => {
            tracing::warn!("Remote embedding backend unavailable: {e}");
            None
        }
    }
}

#[allow(unused_variables)]
fn build_local(
    config: &EmbeddingConfig,
    models_dir: &std::path::Path,
) -> Option<Box<dyn EmbeddingBackend>> {
    #[cfg(feature = "local-vectors")]
    {
        match crate::embedder::Embedder::open(models_dir) {
            Ok(embedder) => {
                tracing::info!("Local ONNX embedder ready");
                Some(Box::new(LocalOnnxBackend::new(embedder)))
            }
            Err(e) => {
                tracing::warn!("Local ONNX embedder unavailable: {e}");
                None
            }
        }
    }

    #[cfg(not(feature = "local-vectors"))]
    {
        None
    }
}

// --- OpenAI-compatible response types ---

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = EmbeddingConfig::default();
        assert_eq!(config.backend, "auto");
        assert!(config.remote_url.is_none());
        assert!(config.model.is_none());
    }

    #[test]
    fn config_serde_roundtrip() {
        let config = EmbeddingConfig {
            backend: "remote".into(),
            remote_url: Some("http://localhost:8420".into()),
            model: Some("all-MiniLM-L6-v2".into()),
            api_key: None,
            dimensions: Some(384),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: EmbeddingConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.backend, "remote");
        assert_eq!(parsed.remote_url.unwrap(), "http://localhost:8420");
        assert_eq!(parsed.dimensions.unwrap(), 384);
    }

    #[cfg(feature = "local-vectors")]
    #[test]
    fn local_backend_unavailable_returns_none() {
        let config = EmbeddingConfig {
            backend: "local".into(),
            ..Default::default()
        };
        let backend = build_backend(&config, std::path::Path::new("/nonexistent"));
        assert!(backend.is_none());
    }

    #[test]
    fn remote_backend_no_url_returns_none() {
        let config = EmbeddingConfig {
            backend: "remote".into(),
            remote_url: None,
            ..Default::default()
        };
        let backend = build_remote(&config);
        assert!(backend.is_none());
    }

    #[test]
    fn auto_backend_without_remote_tries_local() {
        let config = EmbeddingConfig::default();
        // No remote URL, local models missing — should return None gracefully
        let backend = build_backend(&config, std::path::Path::new("/nonexistent"));
        // May be None (no local models) or Some (if ONNX happens to be around)
        // Either way, should not panic
        let _ = backend;
    }
}
