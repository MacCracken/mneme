//! Unified semantic search engine — local embeddings + vector store.
//!
//! Provides a single facade for embedding notes and searching by meaning.
//! Falls back gracefully when models are unavailable.

use std::path::Path;
use std::sync::RwLock;

use uuid::Uuid;

use crate::SearchError;
use crate::semantic::SemanticResult;

#[cfg(feature = "local-vectors")]
use crate::embedder::{EMBEDDING_DIM, Embedder};
#[cfg(feature = "local-vectors")]
use crate::vector_store::VectorStore;

/// Semantic search engine combining local embeddings + ANN index.
pub struct SemanticEngine {
    #[cfg(feature = "local-vectors")]
    embedder: Option<Embedder>,
    #[cfg(feature = "local-vectors")]
    vector_store: Option<RwLock<VectorStore>>,
}

impl SemanticEngine {
    /// Initialize the semantic engine.
    ///
    /// Attempts to load models from `models_dir` and open/create a vector
    /// index at `vectors_dir`. If either fails, the engine operates in
    /// degraded mode (no local semantic search).
    pub fn open(models_dir: &Path, vectors_dir: &Path) -> Self {
        #[cfg(feature = "local-vectors")]
        {
            let embedder = match Embedder::open(models_dir) {
                Ok(e) => {
                    tracing::info!("Local embedder ready");
                    Some(e)
                }
                Err(e) => {
                    tracing::warn!("Local embedder unavailable: {e}");
                    None
                }
            };

            let vector_store = match VectorStore::open(vectors_dir, EMBEDDING_DIM) {
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
                embedder,
                vector_store,
            }
        }

        #[cfg(not(feature = "local-vectors"))]
        {
            let _ = (models_dir, vectors_dir);
            tracing::info!("Local vectors feature disabled");
            Self {}
        }
    }

    /// Create a no-op engine (for testing or when local vectors aren't needed).
    pub fn disabled() -> Self {
        #[cfg(feature = "local-vectors")]
        {
            Self {
                embedder: None,
                vector_store: None,
            }
        }

        #[cfg(not(feature = "local-vectors"))]
        {
            Self {}
        }
    }

    /// Whether local embedding + vector search is operational.
    pub fn is_available(&self) -> bool {
        #[cfg(feature = "local-vectors")]
        {
            self.embedder.is_some() && self.vector_store.is_some()
        }

        #[cfg(not(feature = "local-vectors"))]
        {
            false
        }
    }

    /// Index a note: embed its content and store the vector.
    pub fn index_note(
        &self,
        note_id: Uuid,
        title: &str,
        content: &str,
    ) -> Result<(), SearchError> {
        #[cfg(feature = "local-vectors")]
        {
            let embedder = match &self.embedder {
                Some(e) => e,
                None => return Ok(()), // silently skip if unavailable
            };
            let vector_store = match &self.vector_store {
                Some(vs) => vs,
                None => return Ok(()),
            };

            // Embed title + content together for better representation
            let text = format!("{title}\n\n{content}");
            let embedding = embedder.embed(&text)?;

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

    /// Find notes similar to the given text, filtered by a score threshold.
    ///
    /// Used for duplicate detection: embeds the text, searches the vector store,
    /// and returns only results at or above `threshold`.
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
    pub fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SemanticResult>, SearchError> {
        #[cfg(feature = "local-vectors")]
        {
            let embedder = match &self.embedder {
                Some(e) => e,
                None => return Ok(vec![]),
            };
            let vector_store = match &self.vector_store {
                Some(vs) => vs,
                None => return Ok(vec![]),
            };

            let query_embedding = embedder.embed(query)?;

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
        let engine = SemanticEngine::open(
            &PathBuf::from("/nonexistent/models"),
            dir.path(),
        );
        // Should not panic, just degrade
        assert!(!engine.is_available());
    }
}
