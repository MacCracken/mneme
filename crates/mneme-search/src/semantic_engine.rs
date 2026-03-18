//! Unified semantic search engine — pluggable embeddings + vector store.
//!
//! Provides a single facade for embedding notes and searching by meaning.
//! Supports local ONNX or remote HTTP (Synapse/Ollama/OpenAI) backends,
//! with automatic fallback.

use std::path::Path;
use std::sync::RwLock;

use uuid::Uuid;

use crate::SearchError;
use crate::embedding_backend::{EmbeddingBackend, EmbeddingConfig, build_backend};
use crate::semantic::SemanticResult;

#[cfg(feature = "local-vectors")]
use crate::vector_store::VectorStore;

/// Semantic search engine combining pluggable embeddings + ANN index.
pub struct SemanticEngine {
    backend: Option<Box<dyn EmbeddingBackend>>,
    #[cfg(feature = "local-vectors")]
    vector_store: Option<RwLock<VectorStore>>,
}

impl SemanticEngine {
    /// Initialize the semantic engine with default config (local ONNX).
    ///
    /// Attempts to load models from `models_dir` and open/create a vector
    /// index at `vectors_dir`. If either fails, the engine operates in
    /// degraded mode (no local semantic search).
    pub fn open(models_dir: &Path, vectors_dir: &Path) -> Self {
        Self::open_with_config(models_dir, vectors_dir, &EmbeddingConfig::default())
    }

    /// Initialize with explicit embedding config.
    pub fn open_with_config(
        models_dir: &Path,
        vectors_dir: &Path,
        config: &EmbeddingConfig,
    ) -> Self {
        let backend = build_backend(config, models_dir);
        let dim = backend.as_ref().map(|b| b.dimension()).unwrap_or(384);

        if let Some(ref b) = backend {
            tracing::info!("Embedding backend: {} ({}d)", b.name(), b.dimension());
        } else {
            tracing::warn!("No embedding backend available");
        }

        #[cfg(feature = "local-vectors")]
        {
            let vector_store = match VectorStore::open(vectors_dir, dim) {
                Ok(vs) => {
                    tracing::info!("Vector store ready ({} vectors)", vs.len());
                    Some(RwLock::new(vs))
                }
                Err(e) => {
                    tracing::warn!("Vector store unavailable: {e}");
                    None
                }
            };

            Self {
                backend,
                vector_store,
            }
        }

        #[cfg(not(feature = "local-vectors"))]
        {
            let _ = vectors_dir;
            Self { backend }
        }
    }

    /// Create a no-op engine (for testing or when local vectors aren't needed).
    pub fn disabled() -> Self {
        #[cfg(feature = "local-vectors")]
        {
            Self {
                backend: None,
                vector_store: None,
            }
        }

        #[cfg(not(feature = "local-vectors"))]
        {
            Self { backend: None }
        }
    }

    /// Whether embedding + vector search is operational.
    pub fn is_available(&self) -> bool {
        #[cfg(feature = "local-vectors")]
        {
            self.backend.is_some() && self.vector_store.is_some()
        }

        #[cfg(not(feature = "local-vectors"))]
        {
            self.backend.is_some()
        }
    }

    /// Name of the active embedding backend (for status reporting).
    pub fn backend_name(&self) -> &str {
        self.backend.as_ref().map(|b| b.name()).unwrap_or("none")
    }

    /// Embedding dimension of the active backend.
    pub fn embedding_dimension(&self) -> usize {
        self.backend.as_ref().map(|b| b.dimension()).unwrap_or(0)
    }

    /// Index a note: embed its content and store the vector.
    pub fn index_note(&self, note_id: Uuid, title: &str, content: &str) -> Result<(), SearchError> {
        #[cfg(feature = "local-vectors")]
        {
            let backend = match &self.backend {
                Some(b) => b,
                None => return Ok(()),
            };
            let vector_store = match &self.vector_store {
                Some(vs) => vs,
                None => return Ok(()),
            };

            let text = format!("{title}\n\n{content}");
            let embedding = backend.embed(&text)?;

            let snippet = if content.len() > 200 {
                &content[..200]
            } else {
                content
            };

            vector_store
                .write()
                .map_err(|e| SearchError::VectorStore(e.to_string()))?
                .insert(note_id, title, snippet, &embedding)?;

            Ok(())
        }

        #[cfg(not(feature = "local-vectors"))]
        {
            let _ = (note_id, title, content);
            Ok(())
        }
    }

    /// Embed raw text into a vector. Returns None if the engine is unavailable.
    pub fn embed(&self, text: &str) -> Result<Option<Vec<f32>>, SearchError> {
        match &self.backend {
            Some(b) => Ok(Some(b.embed(text)?)),
            None => Ok(None),
        }
    }

    /// Batch-embed multiple texts. Returns None if the engine is unavailable.
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Option<Vec<Vec<f32>>>, SearchError> {
        match &self.backend {
            Some(b) => Ok(Some(b.embed_batch(texts)?)),
            None => Ok(None),
        }
    }

    /// List all note IDs currently in the vector store.
    pub fn indexed_note_ids(&self) -> Vec<Uuid> {
        #[cfg(feature = "local-vectors")]
        {
            if let Some(vs) = &self.vector_store
                && let Ok(store) = vs.read()
            {
                return store.note_ids();
            }
            vec![]
        }

        #[cfg(not(feature = "local-vectors"))]
        {
            vec![]
        }
    }

    /// Context-aware search: fuse query embedding with context embedding before searching.
    pub fn context_search(
        &self,
        query: &str,
        context_emb: &[f32],
        query_weight: f64,
        limit: usize,
    ) -> Result<Vec<SemanticResult>, SearchError> {
        #[cfg(feature = "local-vectors")]
        {
            let backend = match &self.backend {
                Some(b) => b,
                None => return Ok(vec![]),
            };
            let vector_store = match &self.vector_store {
                Some(vs) => vs,
                None => return Ok(vec![]),
            };

            let query_emb = backend.embed(query)?;
            let fused =
                crate::context_buffer::fuse_embeddings(&query_emb, context_emb, query_weight);

            vector_store
                .read()
                .map_err(|e| SearchError::VectorStore(e.to_string()))?
                .search(&fused, limit)
        }

        #[cfg(not(feature = "local-vectors"))]
        {
            let _ = (query, context_emb, query_weight, limit);
            Ok(vec![])
        }
    }

    /// Find notes similar to the given text, filtered by a score threshold.
    pub fn find_similar_to(
        &self,
        text: &str,
        threshold: f64,
        limit: usize,
    ) -> Result<Vec<SemanticResult>, SearchError> {
        let mut results = self.search(text, limit)?;
        results.retain(|r| r.score >= threshold);
        Ok(results)
    }

    /// Search for notes similar to the query text.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<SemanticResult>, SearchError> {
        #[cfg(feature = "local-vectors")]
        {
            let backend = match &self.backend {
                Some(b) => b,
                None => return Ok(vec![]),
            };
            let vector_store = match &self.vector_store {
                Some(vs) => vs,
                None => return Ok(vec![]),
            };

            let query_embedding = backend.embed(query)?;

            vector_store
                .read()
                .map_err(|e| SearchError::VectorStore(e.to_string()))?
                .search(&query_embedding, limit)
        }

        #[cfg(not(feature = "local-vectors"))]
        {
            let _ = (query, limit);
            Ok(vec![])
        }
    }

    /// Remove a note from the vector index.
    pub fn remove_note(&self, note_id: Uuid) -> Result<(), SearchError> {
        #[cfg(feature = "local-vectors")]
        {
            if let Some(vs) = &self.vector_store {
                vs.write()
                    .map_err(|e| SearchError::VectorStore(e.to_string()))?
                    .remove(note_id)?;
            }
            Ok(())
        }

        #[cfg(not(feature = "local-vectors"))]
        {
            let _ = note_id;
            Ok(())
        }
    }

    /// Persist the vector index to disk.
    pub fn save(&self) -> Result<(), SearchError> {
        #[cfg(feature = "local-vectors")]
        {
            if let Some(vs) = &self.vector_store {
                vs.read()
                    .map_err(|e| SearchError::VectorStore(e.to_string()))?
                    .save()?;
            }
            Ok(())
        }

        #[cfg(not(feature = "local-vectors"))]
        {
            Ok(())
        }
    }

    /// Number of indexed vectors.
    pub fn vector_count(&self) -> usize {
        #[cfg(feature = "local-vectors")]
        {
            self.vector_store
                .as_ref()
                .and_then(|vs| vs.read().ok())
                .map(|vs| vs.len())
                .unwrap_or(0)
        }

        #[cfg(not(feature = "local-vectors"))]
        {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn disabled_engine_returns_empty() {
        let engine = SemanticEngine::disabled();
        assert!(!engine.is_available());
        assert_eq!(engine.vector_count(), 0);
        assert_eq!(engine.backend_name(), "none");
        assert_eq!(engine.embedding_dimension(), 0);

        let results = engine.search("test", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn disabled_engine_index_is_noop() {
        let engine = SemanticEngine::disabled();
        assert!(engine.index_note(Uuid::new_v4(), "Test", "Content").is_ok());
        assert!(engine.remove_note(Uuid::new_v4()).is_ok());
        assert!(engine.save().is_ok());
    }

    #[test]
    fn open_with_missing_models_degrades() {
        let dir = tempfile::TempDir::new().unwrap();
        let engine = SemanticEngine::open(&PathBuf::from("/nonexistent/models"), dir.path());
        // Should not panic, just degrade
        assert!(!engine.is_available());
    }

    #[test]
    fn embed_returns_none_when_disabled() {
        let engine = SemanticEngine::disabled();
        assert!(engine.embed("test").unwrap().is_none());
        assert!(engine.embed_batch(&["a", "b"]).unwrap().is_none());
    }

    #[test]
    fn open_with_config_degrades_gracefully() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = EmbeddingConfig {
            backend: "remote".into(),
            remote_url: Some("http://127.0.0.1:99999".into()),
            model: Some("test".into()),
            ..Default::default()
        };
        // Remote is unreachable, should degrade
        let engine =
            SemanticEngine::open_with_config(&PathBuf::from("/nonexistent"), dir.path(), &config);
        assert!(!engine.is_available());
    }

    #[test]
    fn find_similar_to_empty_when_disabled() {
        let engine = SemanticEngine::disabled();
        let results = engine.find_similar_to("test", 0.5, 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn context_search_empty_when_disabled() {
        let engine = SemanticEngine::disabled();
        let context = vec![0.0f32; 384];
        let results = engine.context_search("test", &context, 0.7, 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn indexed_note_ids_empty_when_disabled() {
        let engine = SemanticEngine::disabled();
        assert!(engine.indexed_note_ids().is_empty());
    }

    #[test]
    fn open_with_local_config_nonexistent_models() {
        let dir = tempfile::TempDir::new().unwrap();
        let config = EmbeddingConfig {
            backend: "local".into(),
            ..Default::default()
        };
        let engine = SemanticEngine::open_with_config(
            &PathBuf::from("/nonexistent/models"),
            dir.path(),
            &config,
        );
        assert!(!engine.is_available());
        assert_eq!(engine.backend_name(), "none");
        assert_eq!(engine.embedding_dimension(), 0);
    }

    #[test]
    fn save_disabled_engine() {
        let engine = SemanticEngine::disabled();
        assert!(engine.save().is_ok());
    }
}
