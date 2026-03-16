//! Cross-vault search — fan out queries across multiple vaults and merge results.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::engine::SearchResult;
use crate::semantic::SemanticResult;

/// A search result annotated with its source vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrossVaultResult {
    pub vault_id: Uuid,
    pub vault_name: String,
    pub note_id: Uuid,
    pub title: String,
    pub path: String,
    pub snippet: String,
    pub score: f64,
    pub source: CrossVaultSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrossVaultSource {
    FullText,
    Semantic,
    Both,
}

/// Results collected from a single vault before merging.
pub struct VaultResults {
    pub vault_id: Uuid,
    pub vault_name: String,
    pub weight: f64,
    pub fulltext: Vec<SearchResult>,
    pub semantic: Vec<SemanticResult>,
}

/// Merge results from multiple vaults using weighted RRF.
///
/// Each vault's results are RRF-ranked internally, then the vault weight
/// is applied as a multiplier.
pub fn cross_vault_merge(
    vault_results: Vec<VaultResults>,
    limit: usize,
) -> Vec<CrossVaultResult> {
    let k = 60.0_f64;
    let mut all: Vec<CrossVaultResult> = Vec::new();

    for vr in &vault_results {
        let mut note_scores: std::collections::HashMap<Uuid, CrossVaultEntry> =
            std::collections::HashMap::new();

        // Score fulltext results by rank
        for (rank, result) in vr.fulltext.iter().enumerate() {
            let rrf = 1.0 / (k + rank as f64 + 1.0);
            let entry = note_scores.entry(result.note_id).or_insert_with(|| {
                CrossVaultEntry {
                    title: result.title.clone(),
                    path: result.path.clone(),
                    snippet: result.snippet.clone(),
                    score: 0.0,
                    has_fulltext: false,
                    has_semantic: false,
                }
            });
            entry.score += rrf;
            entry.has_fulltext = true;
        }

        // Score semantic results by rank
        for (rank, result) in vr.semantic.iter().enumerate() {
            if let Some(note_id) = result.note_id {
                let rrf = 1.0 / (k + rank as f64 + 1.0);
                let entry = note_scores.entry(note_id).or_insert_with(|| {
                    CrossVaultEntry {
                        title: result.title.clone().unwrap_or_default(),
                        path: String::new(),
                        snippet: truncate(&result.content, 200),
                        score: 0.0,
                        has_fulltext: false,
                        has_semantic: false,
                    }
                });
                entry.score += rrf;
                entry.has_semantic = true;
            }
        }

        // Apply vault weight and collect
        for (note_id, entry) in note_scores {
            let source = match (entry.has_fulltext, entry.has_semantic) {
                (true, true) => CrossVaultSource::Both,
                (true, false) => CrossVaultSource::FullText,
                (false, true) => CrossVaultSource::Semantic,
                (false, false) => CrossVaultSource::FullText,
            };

            all.push(CrossVaultResult {
                vault_id: vr.vault_id,
                vault_name: vr.vault_name.clone(),
                note_id,
                title: entry.title,
                path: entry.path,
                snippet: entry.snippet,
                score: entry.score * vr.weight,
                source,
            });
        }
    }

    all.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    all.truncate(limit);
    all
}

struct CrossVaultEntry {
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

    fn make_ft(id: Uuid, title: &str, score: f32) -> SearchResult {
        SearchResult {
            note_id: id,
            title: title.into(),
            path: format!("{title}.md"),
            snippet: format!("Content of {title}"),
            score,
        }
    }

    fn make_sem(id: Uuid, title: &str, score: f64) -> SemanticResult {
        SemanticResult {
            note_id: Some(id),
            title: Some(title.into()),
            content: format!("Semantic content of {title}"),
            score,
        }
    }

    #[test]
    fn single_vault_merge() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let vault_id = Uuid::new_v4();

        let results = cross_vault_merge(
            vec![VaultResults {
                vault_id,
                vault_name: "main".into(),
                weight: 1.0,
                fulltext: vec![make_ft(id1, "Note A", 2.0), make_ft(id2, "Note B", 1.0)],
                semantic: vec![make_sem(id1, "Note A", 0.9)],
            }],
            10,
        );

        assert_eq!(results.len(), 2);
        // id1 appears in both → higher score
        assert_eq!(results[0].note_id, id1);
        assert_eq!(results[0].source, CrossVaultSource::Both);
        assert_eq!(results[0].vault_name, "main");
    }

    #[test]
    fn multi_vault_merge() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();
        let v1 = Uuid::new_v4();
        let v2 = Uuid::new_v4();

        let results = cross_vault_merge(
            vec![
                VaultResults {
                    vault_id: v1,
                    vault_name: "work".into(),
                    weight: 1.0,
                    fulltext: vec![make_ft(id1, "Work Note", 2.0)],
                    semantic: vec![],
                },
                VaultResults {
                    vault_id: v2,
                    vault_name: "personal".into(),
                    weight: 1.0,
                    fulltext: vec![make_ft(id2, "Personal Note", 2.0)],
                    semantic: vec![],
                },
            ],
            10,
        );

        assert_eq!(results.len(), 2);
        let vault_names: Vec<&str> = results.iter().map(|r| r.vault_name.as_str()).collect();
        assert!(vault_names.contains(&"work"));
        assert!(vault_names.contains(&"personal"));
    }

    #[test]
    fn vault_weight_affects_ranking() {
        let id1 = Uuid::new_v4();
        let id2 = Uuid::new_v4();

        let results = cross_vault_merge(
            vec![
                VaultResults {
                    vault_id: Uuid::new_v4(),
                    vault_name: "high".into(),
                    weight: 2.0,
                    fulltext: vec![make_ft(id1, "Boosted", 1.0)],
                    semantic: vec![],
                },
                VaultResults {
                    vault_id: Uuid::new_v4(),
                    vault_name: "low".into(),
                    weight: 0.5,
                    fulltext: vec![make_ft(id2, "Dampened", 1.0)],
                    semantic: vec![],
                },
            ],
            10,
        );

        assert_eq!(results[0].vault_name, "high");
        assert!(results[0].score > results[1].score);
    }

    #[test]
    fn respects_limit() {
        let vault_id = Uuid::new_v4();
        let ft: Vec<_> = (0..20).map(|i| make_ft(Uuid::new_v4(), &format!("N{i}"), 1.0)).collect();

        let results = cross_vault_merge(
            vec![VaultResults {
                vault_id,
                vault_name: "big".into(),
                weight: 1.0,
                fulltext: ft,
                semantic: vec![],
            }],
            5,
        );

        assert_eq!(results.len(), 5);
    }

    #[test]
    fn empty_vaults() {
        let results = cross_vault_merge(vec![], 10);
        assert!(results.is_empty());
    }
}
