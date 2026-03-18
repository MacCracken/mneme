//! Note consolidation — detect duplicates, staleness, and suggest merges.
//!
//! Keeps the vault healthy as it grows by identifying:
//! - Near-duplicate notes (high token overlap)
//! - Stale notes (not updated in a long time)
//! - Merge candidates (related notes that could be combined)

use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A note with its list of similar notes: (other_id, other_title, cosine_score).
pub type SimilarityEntry = (Uuid, String, Vec<(Uuid, String, f64)>);

/// A pair of notes detected as near-duplicates.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicatePair {
    pub note_a_id: Uuid,
    pub note_a_title: String,
    pub note_b_id: Uuid,
    pub note_b_title: String,
    /// Token overlap similarity (0.0–1.0).
    pub similarity: f64,
    /// Suggested action: "merge", "review", or "keep".
    pub suggestion: String,
    /// Detection method: "jaccard" or "semantic".
    pub detection_method: String,
}

/// LLM-generated merge suggestion for two duplicate notes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeSuggestion {
    pub note_a_id: Uuid,
    pub note_b_id: Uuid,
    pub keep_id: Uuid,
    pub merged_title: String,
    pub merged_content: String,
    pub rationale: String,
    pub confidence: f64,
}

/// A note identified as stale.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaleNote {
    pub note_id: Uuid,
    pub title: String,
    pub path: String,
    pub updated_at: DateTime<Utc>,
    pub days_since_update: i64,
    pub last_accessed: DateTime<Utc>,
    pub days_since_access: i64,
    /// Content freshness score: days_since_update / max(days_since_access, 1).
    /// >> 1.0 = accessed recently but old content (priority refresh);
    /// > > ~1.0 = normal; high absolute age = archive candidate.
    pub freshness_score: f64,
}

/// Summary of a consolidation pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsolidationReport {
    pub total_notes: usize,
    pub duplicates: Vec<DuplicatePair>,
    pub stale_notes: Vec<StaleNote>,
    pub duplicate_count: usize,
    pub stale_count: usize,
}

/// Input for duplicate detection: note ID, title, and content.
pub struct NoteContent {
    pub id: Uuid,
    pub title: String,
    pub path: String,
    pub content: String,
    pub updated_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
}

/// Detect near-duplicate note pairs based on token overlap.
///
/// Compares all pairs with O(n²) — suitable for vaults up to ~1000 notes.
/// For larger vaults, use embedding-based similarity via the semantic engine.
pub fn detect_duplicates(notes: &[NoteContent], threshold: f64) -> Vec<DuplicatePair> {
    let tokenized: Vec<(Uuid, &str, HashSet<String>)> = notes
        .iter()
        .map(|n| {
            let tokens = tokenize(&format!("{}\n{}", n.title, n.content));
            (n.id, n.title.as_str(), tokens)
        })
        .collect();

    let mut duplicates = Vec::new();

    for i in 0..tokenized.len() {
        for j in (i + 1)..tokenized.len() {
            let sim = jaccard_similarity(&tokenized[i].2, &tokenized[j].2);
            if sim >= threshold {
                let suggestion = if sim > 0.95 {
                    "merge"
                } else if sim > 0.85 {
                    "review"
                } else {
                    "keep"
                };
                duplicates.push(DuplicatePair {
                    note_a_id: tokenized[i].0,
                    note_a_title: tokenized[i].1.to_string(),
                    note_b_id: tokenized[j].0,
                    note_b_title: tokenized[j].1.to_string(),
                    similarity: sim,
                    suggestion: suggestion.into(),
                    detection_method: "jaccard".into(),
                });
            }
        }
    }

    duplicates.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
    duplicates
}

/// Detect near-duplicate note pairs from pre-computed semantic similarity results.
///
/// `similarity_map` is a list of (note_id, note_title, similar_notes) tuples,
/// where each similar_note is (other_id, cosine_score).
/// Deduplicates symmetric pairs (A,B) == (B,A).
pub fn detect_duplicates_semantic(
    similarity_map: &[SimilarityEntry],
    threshold: f64,
) -> Vec<DuplicatePair> {
    let mut seen = HashSet::new();
    let mut duplicates = Vec::new();

    for (note_id, note_title, similars) in similarity_map {
        for (other_id, other_title, score) in similars {
            if note_id == other_id || *score < threshold {
                continue;
            }
            // Canonical pair ordering to deduplicate
            let pair = if note_id < other_id {
                (*note_id, *other_id)
            } else {
                (*other_id, *note_id)
            };
            if !seen.insert(pair) {
                continue;
            }
            let suggestion = if *score > 0.95 {
                "merge"
            } else if *score > 0.85 {
                "review"
            } else {
                "keep"
            };
            duplicates.push(DuplicatePair {
                note_a_id: *note_id,
                note_a_title: note_title.clone(),
                note_b_id: *other_id,
                note_b_title: other_title.clone(),
                similarity: *score,
                suggestion: suggestion.into(),
                detection_method: "semantic".into(),
            });
        }
    }

    duplicates.sort_by(|a, b| b.similarity.partial_cmp(&a.similarity).unwrap());
    duplicates
}

/// Find notes that haven't been updated in `days` or more.
///
/// Computes a freshness score: `days_since_update / max(days_since_access, 1)`.
/// - Score >> 1.0: accessed recently but content is old → priority refresh
/// - Score ~1.0: normal staleness
/// - High absolute age with low access: archive candidate
pub fn detect_stale(notes: &[NoteContent], days: i64) -> Vec<StaleNote> {
    let now = Utc::now();
    let mut stale: Vec<StaleNote> = notes
        .iter()
        .filter_map(|n| {
            let age = (now - n.updated_at).num_days();
            if age >= days {
                let days_since_access = (now - n.last_accessed).num_days();
                let freshness_score = age as f64 / (days_since_access.max(1) as f64);
                Some(StaleNote {
                    note_id: n.id,
                    title: n.title.clone(),
                    path: n.path.clone(),
                    updated_at: n.updated_at,
                    days_since_update: age,
                    last_accessed: n.last_accessed,
                    days_since_access,
                    freshness_score,
                })
            } else {
                None
            }
        })
        .collect();

    stale.sort_by(|a, b| b.days_since_update.cmp(&a.days_since_update));
    stale
}

/// Run a full consolidation pass.
pub fn consolidate(
    notes: &[NoteContent],
    duplicate_threshold: f64,
    stale_days: i64,
) -> ConsolidationReport {
    let duplicates = detect_duplicates(notes, duplicate_threshold);
    let stale_notes = detect_stale(notes, stale_days);
    let duplicate_count = duplicates.len();
    let stale_count = stale_notes.len();

    ConsolidationReport {
        total_notes: notes.len(),
        duplicates,
        stale_notes,
        duplicate_count,
        stale_count,
    }
}

/// Jaccard similarity between two token sets.
fn jaccard_similarity(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    let intersection = a.intersection(b).count();
    let union = a.union(b).count();
    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Tokenize text: lowercase, split on non-alphanumeric, filter short words.
fn tokenize(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2)
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn make_note(title: &str, content: &str, days_ago: i64) -> NoteContent {
        let updated = Utc::now() - Duration::days(days_ago);
        NoteContent {
            id: Uuid::new_v4(),
            title: title.into(),
            path: format!("{}.md", title.to_lowercase().replace(' ', "-")),
            content: content.into(),
            updated_at: updated,
            last_accessed: updated, // default: accessed when updated
        }
    }

    #[test]
    fn detect_exact_duplicates() {
        let notes = vec![
            make_note("Rust Guide", "Rust is a systems programming language", 0),
            make_note("Rust Guide", "Rust is a systems programming language", 0),
        ];
        let dups = detect_duplicates(&notes, 0.7);
        assert_eq!(dups.len(), 1);
        assert!(dups[0].similarity > 0.95);
        assert_eq!(dups[0].suggestion, "merge");
    }

    #[test]
    fn no_duplicates_for_different_content() {
        let notes = vec![
            make_note(
                "Rust Guide",
                "Rust is a systems programming language with ownership",
                0,
            ),
            make_note(
                "Python Guide",
                "Python is a dynamic interpreted language with GC",
                0,
            ),
        ];
        let dups = detect_duplicates(&notes, 0.7);
        assert!(dups.is_empty());
    }

    #[test]
    fn partial_overlap_detected() {
        let notes = vec![
            make_note(
                "Rust Basics",
                "Rust programming language provides memory safety through ownership and borrowing rules",
                0,
            ),
            make_note(
                "Rust Safety",
                "Rust language provides memory safety via ownership rules and borrow checker",
                0,
            ),
        ];
        let dups = detect_duplicates(&notes, 0.5);
        assert_eq!(dups.len(), 1);
        assert!(dups[0].similarity >= 0.5);
    }

    #[test]
    fn threshold_filters_low_similarity() {
        let notes = vec![
            make_note("A", "alpha bravo charlie delta echo", 0),
            make_note("B", "alpha bravo foxtrot golf hotel", 0),
        ];
        // Some overlap but not high
        let high = detect_duplicates(&notes, 0.9);
        assert!(high.is_empty());
        let low = detect_duplicates(&notes, 0.2);
        assert_eq!(low.len(), 1);
    }

    #[test]
    fn detect_stale_notes() {
        let notes = vec![
            make_note("Fresh", "content", 5),
            make_note("Old", "content", 100),
            make_note("Very Old", "content", 365),
        ];
        let stale = detect_stale(&notes, 90);
        assert_eq!(stale.len(), 2);
        assert_eq!(stale[0].title, "Very Old"); // sorted by age desc
        assert!(stale[0].days_since_update >= 365);
    }

    #[test]
    fn no_stale_notes_when_fresh() {
        let notes = vec![
            make_note("Fresh 1", "content", 1),
            make_note("Fresh 2", "content", 10),
        ];
        let stale = detect_stale(&notes, 90);
        assert!(stale.is_empty());
    }

    #[test]
    fn consolidation_report() {
        let notes = vec![
            make_note("Rust Guide", "Rust is a systems programming language", 200),
            make_note("Rust Guide v2", "Rust is a systems programming language", 5),
            make_note("Python Guide", "Python is interpreted and dynamic", 10),
        ];
        let report = consolidate(&notes, 0.7, 90);
        assert_eq!(report.total_notes, 3);
        assert!(report.duplicate_count >= 1);
        assert!(report.stale_count >= 1);
    }

    #[test]
    fn empty_vault_consolidation() {
        let report = consolidate(&[], 0.7, 90);
        assert_eq!(report.total_notes, 0);
        assert_eq!(report.duplicate_count, 0);
        assert_eq!(report.stale_count, 0);
    }

    #[test]
    fn single_note_no_duplicates() {
        let notes = vec![make_note("Only", "solo content", 0)];
        let dups = detect_duplicates(&notes, 0.5);
        assert!(dups.is_empty());
    }

    #[test]
    fn suggestion_levels() {
        // > 0.95 = merge, > 0.85 = review, else keep
        let notes = vec![
            make_note("A", "exactly the same tokens here now", 0),
            make_note("B", "exactly the same tokens here now", 0), // identical
        ];
        let dups = detect_duplicates(&notes, 0.5);
        assert_eq!(dups[0].suggestion, "merge");
    }

    #[test]
    fn jaccard_empty_sets() {
        let a: HashSet<String> = HashSet::new();
        let b: HashSet<String> = HashSet::new();
        assert_eq!(jaccard_similarity(&a, &b), 1.0);
    }

    #[test]
    fn jaccard_disjoint_sets() {
        let a: HashSet<String> = ["alpha", "bravo"].iter().map(|s| s.to_string()).collect();
        let b: HashSet<String> = ["charlie", "delta"].iter().map(|s| s.to_string()).collect();
        assert_eq!(jaccard_similarity(&a, &b), 0.0);
    }

    #[test]
    fn jaccard_identical_sets() {
        let a: HashSet<String> = ["alpha", "bravo"].iter().map(|s| s.to_string()).collect();
        let b = a.clone();
        assert_eq!(jaccard_similarity(&a, &b), 1.0);
    }

    #[test]
    fn duplicates_sorted_by_similarity_desc() {
        let notes = vec![
            make_note("A", "alpha bravo charlie delta echo foxtrot golf", 0),
            make_note("B", "alpha bravo charlie delta echo foxtrot golf hotel", 0), // very similar
            make_note("C", "alpha bravo charlie india juliet kilo lima", 0),        // less similar
        ];
        let dups = detect_duplicates(&notes, 0.3);
        if dups.len() >= 2 {
            assert!(dups[0].similarity >= dups[1].similarity);
        }
    }
}
