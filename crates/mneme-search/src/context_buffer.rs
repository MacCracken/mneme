//! Working-memory context buffer for context-aware retrieval.
//!
//! Tracks the most recently accessed note IDs and computes a context embedding
//! by averaging their embeddings. This context is fused with the query embedding
//! to bias search results toward the user's current working context.

use std::collections::VecDeque;

use uuid::Uuid;

/// A bounded buffer of recent note IDs for session context tracking.
#[derive(Debug, Clone)]
pub struct ContextBuffer {
    /// Recent note IDs, most recent at the back.
    recent: VecDeque<Uuid>,
    /// Maximum capacity (default: 7).
    capacity: usize,
}

impl ContextBuffer {
    /// Create a new context buffer with the given capacity.
    pub fn new(capacity: usize) -> Self {
        Self {
            recent: VecDeque::with_capacity(capacity),
            capacity: capacity.max(1),
        }
    }

    /// Record that a note was accessed. Deduplicates (moves to most recent).
    pub fn push(&mut self, note_id: Uuid) {
        // Remove if already present (we'll re-add at the end)
        self.recent.retain(|&id| id != note_id);
        if self.recent.len() >= self.capacity {
            self.recent.pop_front();
        }
        self.recent.push_back(note_id);
    }

    /// Get the recent note IDs, most recent last.
    pub fn recent_ids(&self) -> &VecDeque<Uuid> {
        &self.recent
    }

    /// Number of notes in the buffer.
    pub fn len(&self) -> usize {
        self.recent.len()
    }

    /// Whether the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.recent.is_empty()
    }

    /// Clear the buffer.
    pub fn clear(&mut self) {
        self.recent.clear();
    }

    /// Compute a context embedding by averaging the given per-note embeddings.
    ///
    /// `embeddings` maps note IDs to their embedding vectors. Notes not in
    /// the map are skipped. Returns None if no embeddings are available.
    ///
    /// More recent notes get higher weight (linear decay from 1.0 to 0.5).
    pub fn context_embedding(
        &self,
        embeddings: &[(Uuid, Vec<f32>)],
    ) -> Option<Vec<f32>> {
        if self.recent.is_empty() {
            return None;
        }

        let emb_map: std::collections::HashMap<Uuid, &Vec<f32>> =
            embeddings.iter().map(|(id, e)| (*id, e)).collect();

        let n = self.recent.len();
        let mut matched = Vec::new();
        for (i, id) in self.recent.iter().enumerate() {
            if let Some(emb) = emb_map.get(id) {
                // Recency weight: linearly from 0.5 (oldest) to 1.0 (newest)
                let weight = 0.5 + 0.5 * (i as f64 / n.max(1) as f64);
                matched.push((weight, *emb));
            }
        }

        if matched.is_empty() {
            return None;
        }

        let dim = matched[0].1.len();
        let total_weight: f64 = matched.iter().map(|(w, _)| w).sum();
        let mut result = vec![0.0f32; dim];
        for (weight, emb) in &matched {
            for d in 0..dim {
                result[d] += (*weight as f32 / total_weight as f32) * emb[d];
            }
        }

        // L2 normalize
        let norm: f32 = result.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut result {
                *v /= norm;
            }
        }

        Some(result)
    }
}

/// Fuse a query embedding with a context embedding.
///
/// `search_vec = λ * query_emb + (1 - λ) * context_emb`
/// The result is L2-normalized.
pub fn fuse_embeddings(query: &[f32], context: &[f32], query_weight: f64) -> Vec<f32> {
    let lambda = query_weight.clamp(0.0, 1.0) as f32;
    let mut fused: Vec<f32> = query
        .iter()
        .zip(context.iter())
        .map(|(q, c)| lambda * q + (1.0 - lambda) * c)
        .collect();

    // L2 normalize
    let norm: f32 = fused.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for v in &mut fused {
            *v /= norm;
        }
    }

    fused
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_and_capacity() {
        let mut buf = ContextBuffer::new(3);
        let ids: Vec<Uuid> = (0..5).map(|_| Uuid::new_v4()).collect();

        for id in &ids {
            buf.push(*id);
        }

        assert_eq!(buf.len(), 3);
        // Should contain the last 3
        let recent: Vec<Uuid> = buf.recent_ids().iter().copied().collect();
        assert_eq!(recent, vec![ids[2], ids[3], ids[4]]);
    }

    #[test]
    fn push_deduplicates() {
        let mut buf = ContextBuffer::new(5);
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        buf.push(id1);
        buf.push(id2);
        buf.push(id1); // re-access id1

        assert_eq!(buf.len(), 2);
        let recent: Vec<Uuid> = buf.recent_ids().iter().copied().collect();
        assert_eq!(recent, vec![id2, id1]); // id1 is now most recent
    }

    #[test]
    fn empty_buffer() {
        let buf = ContextBuffer::new(7);
        assert!(buf.is_empty());
        assert_eq!(buf.len(), 0);
        assert!(buf.context_embedding(&[]).is_none());
    }

    #[test]
    fn clear_buffer() {
        let mut buf = ContextBuffer::new(7);
        buf.push(Uuid::new_v4());
        buf.clear();
        assert!(buf.is_empty());
    }

    #[test]
    fn context_embedding_weighted_average() {
        let mut buf = ContextBuffer::new(3);
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        buf.push(id1);
        buf.push(id2);

        let embeddings = vec![
            (id1, vec![1.0, 0.0, 0.0]),
            (id2, vec![0.0, 1.0, 0.0]),
        ];

        let ctx = buf.context_embedding(&embeddings).unwrap();
        assert_eq!(ctx.len(), 3);
        // Should be normalized
        let norm: f32 = ctx.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn context_embedding_skips_missing() {
        let mut buf = ContextBuffer::new(3);
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        buf.push(id1);
        buf.push(id2);

        // Only id1 has an embedding
        let embeddings = vec![(id1, vec![1.0, 0.0, 0.0])];
        let ctx = buf.context_embedding(&embeddings).unwrap();
        // Should be just id1's embedding (normalized)
        assert!((ctx[0] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn context_embedding_none_when_no_matches() {
        let mut buf = ContextBuffer::new(3);
        buf.push(Uuid::new_v4());
        // No matching embeddings
        assert!(buf.context_embedding(&[]).is_none());
    }

    #[test]
    fn fuse_embeddings_pure_query() {
        let query = vec![1.0, 0.0, 0.0];
        let context = vec![0.0, 1.0, 0.0];
        let fused = fuse_embeddings(&query, &context, 1.0);
        // λ=1.0 → pure query
        assert!((fused[0] - 1.0).abs() < 1e-5);
        assert!(fused[1].abs() < 1e-5);
    }

    #[test]
    fn fuse_embeddings_pure_context() {
        let query = vec![1.0, 0.0, 0.0];
        let context = vec![0.0, 1.0, 0.0];
        let fused = fuse_embeddings(&query, &context, 0.0);
        // λ=0.0 → pure context
        assert!(fused[0].abs() < 1e-5);
        assert!((fused[1] - 1.0).abs() < 1e-5);
    }

    #[test]
    fn fuse_embeddings_balanced() {
        let query = vec![1.0, 0.0];
        let context = vec![0.0, 1.0];
        let fused = fuse_embeddings(&query, &context, 0.5);
        // Should be normalized 50/50 blend
        assert!((fused[0] - fused[1]).abs() < 1e-5);
        let norm: f32 = fused.iter().map(|v| v * v).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-5);
    }

    #[test]
    fn fuse_embeddings_clamped() {
        let query = vec![1.0, 0.0];
        let context = vec![0.0, 1.0];
        // Out-of-range λ should be clamped
        let fused = fuse_embeddings(&query, &context, 2.0);
        assert!((fused[0] - 1.0).abs() < 1e-5);
    }
}
