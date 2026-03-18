//! Semantic search types and hybrid merge logic.
//!
//! Provides vector-based similarity search results and the RRF
//! merge algorithm that combines full-text + semantic signals.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::retrieval_optimizer::BlendWeights;

/// A semantic search result with similarity score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticResult {
    pub note_id: Option<Uuid>,
    pub title: Option<String>,
    pub content: String,
    pub score: f64,
}

/// Hybrid search result combining full-text and semantic scores.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridResult {
    pub note_id: Uuid,
    pub title: String,
    pub path: String,
    pub snippet: String,
    /// Combined score from BM25 and semantic ranking.
    pub score: f64,
    /// Whether this result came from full-text, semantic, or both.
    pub source: ResultSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultSource {
    FullText,
    Semantic,
    Both,
}

/// Merge full-text and semantic results into a ranked hybrid list.
///
/// Uses reciprocal rank fusion (RRF) with the default blend weights.
pub fn hybrid_merge(
    fulltext_results: Vec<(Uuid, String, String, String, f32)>,
    semantic_results: Vec<SemanticResult>,
    limit: usize,
) -> Vec<HybridResult> {
    weighted_hybrid_merge(
        fulltext_results,
        semantic_results,
        limit,
        &BlendWeights::default(),
    )
}

/// Merge with explicit blend weights from the retrieval optimizer.
pub fn weighted_hybrid_merge(
    fulltext_results: Vec<(Uuid, String, String, String, f32)>,
    semantic_results: Vec<SemanticResult>,
    limit: usize,
    weights: &BlendWeights,
) -> Vec<HybridResult> {
    let k = 60.0_f64; // RRF constant

    let mut scores: HashMap<Uuid, HybridEntry> = HashMap::new();

    // Score full-text results by rank, weighted
    for (rank, (id, title, path, snippet, _ft_score)) in fulltext_results.iter().enumerate() {
        let rrf = weights.fulltext / (k + rank as f64 + 1.0);
        let entry = scores.entry(*id).or_insert_with(|| HybridEntry {
            title: title.clone(),
            path: path.clone(),
            snippet: snippet.clone(),
            score: 0.0,
            has_fulltext: false,
            has_semantic: false,
        });
        entry.score += rrf;
        entry.has_fulltext = true;
    }

    // Score semantic results by rank, weighted
    for (rank, result) in semantic_results.iter().enumerate() {
        if let Some(id) = result.note_id {
            let rrf = weights.semantic / (k + rank as f64 + 1.0);
            let entry = scores.entry(id).or_insert_with(|| HybridEntry {
                title: result.title.clone().unwrap_or_default(),
                path: String::new(),
                snippet: truncate(&result.content, 200),
                score: 0.0,
                has_fulltext: false,
                has_semantic: false,
            });
            entry.score += rrf;
            entry.has_semantic = true;
        }
    }

    // Apply recency boost (based on rank position — lower rank = more recent assumption)
    if weights.recency > 0.0 {
        let total = scores.len() as f64;
        for (i, (_id, entry)) in scores.iter_mut().enumerate() {
            // Boost inversely proportional to position
            let recency_factor = 1.0 - (i as f64 / total.max(1.0));
            entry.score += weights.recency * recency_factor;
        }
    }

    let mut results: Vec<HybridResult> = scores
        .into_iter()
        .map(|(id, entry)| {
            let source = match (entry.has_fulltext, entry.has_semantic) {
                (true, true) => ResultSource::Both,
                (true, false) => ResultSource::FullText,
                (false, true) => ResultSource::Semantic,
                (false, false) => ResultSource::FullText,
            };
            HybridResult {
                note_id: id,
                title: entry.title,
                path: entry.path,
                snippet: entry.snippet,
                score: entry.score,
                source,
            }
        })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    results.truncate(limit);
    results
}

struct HybridEntry {
    title: String,
    path: String,
    snippet: String,
    score: f64,
    has_fulltext: bool,
    has_semantic: bool,
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

    #[test]
    fn hybrid_merge_fulltext_only() {
        let id = Uuid::new_v4();
        let ft = vec![(id, "Note".into(), "note.md".into(), "content".into(), 1.0)];
        let results = hybrid_merge(ft, vec![], 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source, ResultSource::FullText);
    }

    #[test]
    fn hybrid_merge_semantic_only() {
        let id = Uuid::new_v4();
        let sem = vec![SemanticResult {
            note_id: Some(id),
            title: Some("Note".into()),
            content: "semantic content".into(),
            score: 0.9,
        }];
        let results = hybrid_merge(vec![], sem, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source, ResultSource::Semantic);
    }

    #[test]
    fn hybrid_merge_both_sources() {
        let id = Uuid::new_v4();
        let ft = vec![(id, "Note".into(), "note.md".into(), "content".into(), 1.0)];
        let sem = vec![SemanticResult {
            note_id: Some(id),
            title: Some("Note".into()),
            content: "semantic content".into(),
            score: 0.9,
        }];
        let results = hybrid_merge(ft, sem, 10);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].source, ResultSource::Both);
        assert!(results[0].score > 1.0 / 61.0);
    }

    #[test]
    fn hybrid_merge_deduplicates() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let ft = vec![
            (id1, "A".into(), "a.md".into(), "a".into(), 2.0),
            (id2, "B".into(), "b.md".into(), "b".into(), 1.0),
        ];
        let sem = vec![SemanticResult {
            note_id: Some(id1),
            title: Some("A".into()),
            content: "a semantic".into(),
            score: 0.9,
        }];
        let results = hybrid_merge(ft, sem, 10);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].note_id, id1);
    }

    #[test]
    fn hybrid_merge_respects_limit() {
        let ft: Vec<_> = (0..20)
            .map(|i| {
                (
                    Uuid::new_v4(),
                    format!("Note {i}"),
                    format!("note{i}.md"),
                    format!("content {i}"),
                    1.0,
                )
            })
            .collect();
        let results = hybrid_merge(ft, vec![], 5);
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn weighted_merge_fulltext_heavy() {
        let id_ft = Uuid::new_v4();
        let id_sem = Uuid::new_v4();
        let ft = vec![(id_ft, "FT".into(), "ft.md".into(), "ft".into(), 1.0)];
        let sem = vec![SemanticResult {
            note_id: Some(id_sem),
            title: Some("Sem".into()),
            content: "semantic".into(),
            score: 0.9,
        }];
        let weights = BlendWeights {
            fulltext: 2.0,
            semantic: 0.5,
            recency: 0.0,
        };
        let results = weighted_hybrid_merge(ft, sem, 10, &weights);
        assert_eq!(results.len(), 2);
        // FT result should rank first due to higher weight
        assert_eq!(results[0].note_id, id_ft);
    }

    #[test]
    fn weighted_merge_semantic_heavy() {
        let id_ft = Uuid::new_v4();
        let id_sem = Uuid::new_v4();
        let ft = vec![(id_ft, "FT".into(), "ft.md".into(), "ft".into(), 1.0)];
        let sem = vec![SemanticResult {
            note_id: Some(id_sem),
            title: Some("Sem".into()),
            content: "semantic".into(),
            score: 0.9,
        }];
        let weights = BlendWeights {
            fulltext: 0.5,
            semantic: 2.0,
            recency: 0.0,
        };
        let results = weighted_hybrid_merge(ft, sem, 10, &weights);
        assert_eq!(results.len(), 2);
        // Semantic result should rank first
        assert_eq!(results[0].note_id, id_sem);
    }

    #[test]
    fn recency_boost_adds_to_score() {
        let id = Uuid::new_v4();
        let ft = vec![(id, "Note".into(), "n.md".into(), "c".into(), 1.0)];
        let no_boost = weighted_hybrid_merge(
            ft.clone(),
            vec![],
            10,
            &BlendWeights {
                fulltext: 1.0,
                semantic: 1.0,
                recency: 0.0,
            },
        );
        let with_boost = weighted_hybrid_merge(
            ft,
            vec![],
            10,
            &BlendWeights {
                fulltext: 1.0,
                semantic: 1.0,
                recency: 0.01,
            },
        );
        assert!(with_boost[0].score >= no_boost[0].score);
    }
}
