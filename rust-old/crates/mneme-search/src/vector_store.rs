//! Persistent ANN vector index using usearch.
//!
//! Stores embeddings keyed by note UUID with a sidecar metadata file.
//! Gated behind the `local-vectors` feature.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::SearchError;
use crate::semantic::SemanticResult;

/// Metadata associated with each stored vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct VectorMeta {
    note_id: Uuid,
    title: String,
    snippet: String,
}

/// In-process vector store backed by usearch.
pub struct VectorStore {
    index: usearch::Index,
    metadata: HashMap<u64, VectorMeta>,
    persist_dir: Option<PathBuf>,
    dim: usize,
    next_key: u64,
    /// Map from note UUID to usearch key for fast removal.
    note_keys: HashMap<Uuid, u64>,
}

impl VectorStore {
    /// Open or create a persistent vector store at the given directory.
    pub fn open(dir: &Path, dim: usize) -> Result<Self, SearchError> {
        std::fs::create_dir_all(dir).map_err(|e| SearchError::VectorStore(e.to_string()))?;

        let index_path = dir.join("vectors.usearch");
        let meta_path = dir.join("vectors.meta.json");

        let index = usearch::new_index(&usearch::IndexOptions {
            dimensions: dim,
            metric: usearch::MetricKind::Cos,
            quantization: usearch::ScalarKind::F32,
            ..Default::default()
        })
        .map_err(|e| SearchError::VectorStore(e.to_string()))?;

        // Reserve initial capacity
        index
            .reserve(1024)
            .map_err(|e| SearchError::VectorStore(e.to_string()))?;

        let mut store = Self {
            index,
            metadata: HashMap::new(),
            persist_dir: Some(dir.to_path_buf()),
            dim,
            next_key: 1,
            note_keys: HashMap::new(),
        };

        // Load existing data if present
        if index_path.exists() {
            store
                .index
                .load(index_path.to_str().unwrap())
                .map_err(|e| SearchError::VectorStore(e.to_string()))?;
        }
        if meta_path.exists() {
            let data = std::fs::read_to_string(&meta_path)
                .map_err(|e| SearchError::VectorStore(e.to_string()))?;
            let persisted: PersistedMeta = serde_json::from_str(&data).unwrap_or_default();
            store.metadata = persisted.entries;
            store.next_key = persisted.next_key;
            // Rebuild note_keys from metadata
            for (&key, meta) in &store.metadata {
                store.note_keys.insert(meta.note_id, key);
            }
        }

        tracing::info!(
            "Vector store opened at {} ({} vectors)",
            dir.display(),
            store.metadata.len()
        );

        Ok(store)
    }

    /// Create an in-memory vector store (for testing).
    pub fn in_memory(dim: usize) -> Result<Self, SearchError> {
        let index = usearch::new_index(&usearch::IndexOptions {
            dimensions: dim,
            metric: usearch::MetricKind::Cos,
            quantization: usearch::ScalarKind::F32,
            ..Default::default()
        })
        .map_err(|e| SearchError::VectorStore(e.to_string()))?;

        index
            .reserve(256)
            .map_err(|e| SearchError::VectorStore(e.to_string()))?;

        Ok(Self {
            index,
            metadata: HashMap::new(),
            persist_dir: None,
            dim,
            next_key: 1,
            note_keys: HashMap::new(),
        })
    }

    /// Insert or update a vector for a note.
    pub fn insert(
        &mut self,
        note_id: Uuid,
        title: &str,
        snippet: &str,
        embedding: &[f32],
    ) -> Result<(), SearchError> {
        if embedding.len() != self.dim {
            return Err(SearchError::VectorStore(format!(
                "expected {}-dim vector, got {}",
                self.dim,
                embedding.len()
            )));
        }

        // Remove existing entry for this note if present
        if let Some(&old_key) = self.note_keys.get(&note_id) {
            let _ = self.index.remove(old_key);
            self.metadata.remove(&old_key);
        }

        let key = self.next_key;
        self.next_key += 1;

        // Ensure capacity
        let capacity = self.index.capacity();
        if self.index.size() + 1 >= capacity {
            self.index
                .reserve(capacity * 2)
                .map_err(|e| SearchError::VectorStore(e.to_string()))?;
        }

        self.index
            .add(key, embedding)
            .map_err(|e| SearchError::VectorStore(e.to_string()))?;

        self.metadata.insert(
            key,
            VectorMeta {
                note_id,
                title: title.to_string(),
                snippet: truncate(snippet, 200),
            },
        );
        self.note_keys.insert(note_id, key);

        Ok(())
    }

    /// Search for the nearest vectors to the query embedding.
    pub fn search(
        &self,
        query_embedding: &[f32],
        limit: usize,
    ) -> Result<Vec<SemanticResult>, SearchError> {
        if self.index.size() == 0 {
            return Ok(vec![]);
        }

        let results = self
            .index
            .search(query_embedding, limit)
            .map_err(|e| SearchError::VectorStore(e.to_string()))?;

        let mut out = Vec::with_capacity(results.keys.len());
        for (key, distance) in results.keys.iter().zip(results.distances.iter()) {
            if let Some(meta) = self.metadata.get(key) {
                // usearch returns cosine distance; convert to similarity
                let score = 1.0 - (*distance as f64);
                out.push(SemanticResult {
                    note_id: Some(meta.note_id),
                    title: Some(meta.title.clone()),
                    content: meta.snippet.clone(),
                    score,
                });
            }
        }

        Ok(out)
    }

    /// Remove a note's vector from the index.
    pub fn remove(&mut self, note_id: Uuid) -> Result<(), SearchError> {
        if let Some(key) = self.note_keys.remove(&note_id) {
            let _ = self.index.remove(key);
            self.metadata.remove(&key);
        }
        Ok(())
    }

    /// Number of stored vectors.
    pub fn len(&self) -> usize {
        self.metadata.len()
    }

    /// All note IDs currently stored.
    pub fn note_ids(&self) -> Vec<Uuid> {
        self.note_keys.keys().copied().collect()
    }

    /// Whether the store is empty.
    pub fn is_empty(&self) -> bool {
        self.metadata.is_empty()
    }

    /// Persist index and metadata to disk.
    pub fn save(&self) -> Result<(), SearchError> {
        let dir = match &self.persist_dir {
            Some(d) => d,
            None => return Ok(()), // in-memory, nothing to save
        };

        let index_path = dir.join("vectors.usearch");
        self.index
            .save(index_path.to_str().unwrap())
            .map_err(|e| SearchError::VectorStore(e.to_string()))?;

        let meta_path = dir.join("vectors.meta.json");
        let persisted = PersistedMeta {
            next_key: self.next_key,
            entries: self.metadata.clone(),
        };
        let data = serde_json::to_string(&persisted)
            .map_err(|e| SearchError::VectorStore(e.to_string()))?;
        std::fs::write(&meta_path, data).map_err(|e| SearchError::VectorStore(e.to_string()))?;

        Ok(())
    }
}

impl Drop for VectorStore {
    fn drop(&mut self) {
        if self.persist_dir.is_some()
            && let Err(e) = self.save()
        {
            tracing::warn!("Failed to persist vector store on drop: {e}");
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct PersistedMeta {
    next_key: u64,
    entries: HashMap<u64, VectorMeta>,
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> VectorStore {
        VectorStore::in_memory(3).unwrap()
    }

    #[test]
    fn insert_and_search() {
        let mut store = test_store();
        let id = Uuid::new_v4();
        store
            .insert(id, "Test Note", "Some content", &[1.0, 0.0, 0.0])
            .unwrap();

        let results = store.search(&[1.0, 0.0, 0.0], 5).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].note_id, Some(id));
        assert!(results[0].score > 0.99); // near-perfect match
    }

    #[test]
    fn insert_updates_existing() {
        let mut store = test_store();
        let id = Uuid::new_v4();
        store
            .insert(id, "V1", "content v1", &[1.0, 0.0, 0.0])
            .unwrap();
        store
            .insert(id, "V2", "content v2", &[0.0, 1.0, 0.0])
            .unwrap();

        assert_eq!(store.len(), 1);
        let results = store.search(&[0.0, 1.0, 0.0], 5).unwrap();
        assert_eq!(results[0].title, Some("V2".into()));
    }

    #[test]
    fn remove_note() {
        let mut store = test_store();
        let id = Uuid::new_v4();
        store
            .insert(id, "Remove Me", "content", &[1.0, 0.0, 0.0])
            .unwrap();
        assert_eq!(store.len(), 1);

        store.remove(id).unwrap();
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn search_empty_store() {
        let store = test_store();
        let results = store.search(&[1.0, 0.0, 0.0], 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn wrong_dimension_rejected() {
        let mut store = test_store(); // 3-dim
        let result = store.insert(Uuid::new_v4(), "Bad", "x", &[1.0, 0.0]);
        assert!(result.is_err());
    }

    #[test]
    fn multiple_notes_ranked() {
        let mut store = test_store();

        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let id3 = Uuid::new_v4();

        store.insert(id1, "North", "up", &[0.0, 1.0, 0.0]).unwrap();
        store
            .insert(id2, "East", "right", &[1.0, 0.0, 0.0])
            .unwrap();
        store
            .insert(id3, "Northeast", "diagonal", &[0.707, 0.707, 0.0])
            .unwrap();

        // Query: mostly north
        let results = store.search(&[0.0, 1.0, 0.0], 3).unwrap();
        assert_eq!(results.len(), 3);
        // First result should be "North" (exact match)
        assert_eq!(results[0].note_id, Some(id1));
    }

    #[test]
    fn persist_and_reload() {
        let dir = tempfile::TempDir::new().unwrap();
        let id = Uuid::new_v4();

        {
            let mut store = VectorStore::open(dir.path(), 3).unwrap();
            store
                .insert(id, "Persisted", "data", &[1.0, 0.0, 0.0])
                .unwrap();
            store.save().unwrap();
        }

        // Reopen
        let store = VectorStore::open(dir.path(), 3).unwrap();
        assert_eq!(store.len(), 1);
        let results = store.search(&[1.0, 0.0, 0.0], 5).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].note_id, Some(id));
        assert_eq!(results[0].title, Some("Persisted".into()));
    }

    #[test]
    fn remove_nonexistent_is_ok() {
        let mut store = test_store();
        assert!(store.remove(Uuid::new_v4()).is_ok());
    }

    #[test]
    fn len_and_is_empty() {
        let mut store = test_store();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);

        store
            .insert(Uuid::new_v4(), "A", "a", &[1.0, 0.0, 0.0])
            .unwrap();
        assert!(!store.is_empty());
        assert_eq!(store.len(), 1);
    }
}
