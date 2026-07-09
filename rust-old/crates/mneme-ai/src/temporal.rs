//! Temporal analysis — track how knowledge evolves over time.
//!
//! Analyzes note creation patterns, concept frequency changes,
//! and knowledge growth metrics.

use std::collections::HashMap;

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};

use crate::AiError;
use crate::concepts;

/// A snapshot of a note at a point in time.
#[derive(Debug, Clone)]
pub struct NoteSnapshot {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Activity for a specific time period.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodActivity {
    pub period: String,
    pub notes_created: usize,
    pub notes_updated: usize,
    pub total_words: usize,
}

/// A concept trend over time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptTrend {
    pub term: String,
    pub occurrences_by_period: Vec<(String, usize)>,
    pub trend: TrendDirection,
    pub total_occurrences: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrendDirection {
    Rising,
    Stable,
    Declining,
}

/// Overall temporal analysis result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalReport {
    pub total_notes: usize,
    pub date_range: Option<(String, String)>,
    pub activity_by_month: Vec<PeriodActivity>,
    pub concept_trends: Vec<ConceptTrend>,
    pub most_active_month: Option<String>,
    pub avg_words_per_note: usize,
}

/// Analyze temporal patterns across a collection of notes.
pub fn analyze_temporal(notes: &[NoteSnapshot]) -> Result<TemporalReport, AiError> {
    if notes.is_empty() {
        return Ok(TemporalReport {
            total_notes: 0,
            date_range: None,
            activity_by_month: vec![],
            concept_trends: vec![],
            most_active_month: None,
            avg_words_per_note: 0,
        });
    }

    // Activity by month
    let mut monthly: HashMap<String, PeriodActivity> = HashMap::new();
    let mut total_words = 0usize;

    for note in notes {
        let month_key = format!("{}-{:02}", note.created_at.year(), note.created_at.month());
        let words = note.content.split_whitespace().count();
        total_words += words;

        let entry = monthly
            .entry(month_key.clone())
            .or_insert_with(|| PeriodActivity {
                period: month_key.clone(),
                notes_created: 0,
                notes_updated: 0,
                total_words: 0,
            });
        entry.notes_created += 1;
        entry.total_words += words;

        // Count updates separately if updated != created
        if note.updated_at != note.created_at {
            let update_key = format!("{}-{:02}", note.updated_at.year(), note.updated_at.month());
            let update_entry =
                monthly
                    .entry(update_key.clone())
                    .or_insert_with(|| PeriodActivity {
                        period: update_key,
                        notes_created: 0,
                        notes_updated: 0,
                        total_words: 0,
                    });
            update_entry.notes_updated += 1;
        }
    }

    let mut activity_by_month: Vec<PeriodActivity> = monthly.into_values().collect();
    activity_by_month.sort_by(|a, b| a.period.cmp(&b.period));

    // Most active month
    let most_active_month = activity_by_month
        .iter()
        .max_by_key(|a| a.notes_created + a.notes_updated)
        .map(|a| a.period.clone());

    // Date range
    let min_date = notes.iter().map(|n| n.created_at).min();
    let max_date = notes.iter().map(|n| n.updated_at).max();
    let date_range = min_date.zip(max_date).map(|(min, max)| {
        (
            min.format("%Y-%m-%d").to_string(),
            max.format("%Y-%m-%d").to_string(),
        )
    });

    // Concept trends — extract concepts per month
    let mut concept_by_month: HashMap<String, HashMap<String, usize>> = HashMap::new();
    for note in notes {
        let month_key = format!("{}-{:02}", note.created_at.year(), note.created_at.month());
        if let Ok(extracted) = concepts::extract_concepts(&note.content) {
            let month_map = concept_by_month.entry(month_key).or_default();
            for concept in extracted {
                *month_map.entry(concept.term).or_default() += concept.occurrences;
            }
        }
    }

    // Build concept trends for top concepts
    let mut global_concepts: HashMap<String, usize> = HashMap::new();
    for month_concepts in concept_by_month.values() {
        for (term, count) in month_concepts {
            *global_concepts.entry(term.clone()).or_default() += count;
        }
    }

    let mut top_concepts: Vec<_> = global_concepts.into_iter().collect();
    top_concepts.sort_by(|a, b| b.1.cmp(&a.1));
    top_concepts.truncate(10);

    let sorted_months: Vec<String> = {
        let mut months: Vec<String> = concept_by_month.keys().cloned().collect();
        months.sort();
        months
    };

    let concept_trends: Vec<ConceptTrend> = top_concepts
        .into_iter()
        .map(|(term, total)| {
            let occurrences_by_period: Vec<(String, usize)> = sorted_months
                .iter()
                .map(|m| {
                    let count = concept_by_month
                        .get(m)
                        .and_then(|mc| mc.get(&term))
                        .copied()
                        .unwrap_or(0);
                    (m.clone(), count)
                })
                .collect();

            let trend = compute_trend(&occurrences_by_period);

            ConceptTrend {
                term,
                occurrences_by_period,
                trend,
                total_occurrences: total,
            }
        })
        .collect();

    let avg_words = total_words / notes.len();

    Ok(TemporalReport {
        total_notes: notes.len(),
        date_range,
        activity_by_month,
        concept_trends,
        most_active_month,
        avg_words_per_note: avg_words,
    })
}

fn compute_trend(data: &[(String, usize)]) -> TrendDirection {
    if data.len() < 2 {
        return TrendDirection::Stable;
    }

    let mid = data.len() / 2;
    let first_half: usize = data[..mid].iter().map(|(_, c)| c).sum();
    let second_half: usize = data[mid..].iter().map(|(_, c)| c).sum();

    if second_half > first_half + (first_half / 3) {
        TrendDirection::Rising
    } else if first_half > second_half + (second_half / 3) {
        TrendDirection::Declining
    } else {
        TrendDirection::Stable
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn make_snapshot(title: &str, content: &str, year: i32, month: u32, day: u32) -> NoteSnapshot {
        let dt = Utc.with_ymd_and_hms(year, month, day, 12, 0, 0).unwrap();
        NoteSnapshot {
            title: title.into(),
            content: content.into(),
            tags: vec![],
            created_at: dt,
            updated_at: dt,
        }
    }

    #[test]
    fn empty_notes() {
        let report = analyze_temporal(&[]).unwrap();
        assert_eq!(report.total_notes, 0);
        assert!(report.date_range.is_none());
    }

    #[test]
    fn single_note_analysis() {
        let notes = vec![make_snapshot(
            "Test",
            "Hello world test content.",
            2026,
            3,
            1,
        )];
        let report = analyze_temporal(&notes).unwrap();
        assert_eq!(report.total_notes, 1);
        assert_eq!(report.activity_by_month.len(), 1);
        assert_eq!(report.activity_by_month[0].notes_created, 1);
    }

    #[test]
    fn multi_month_activity() {
        let notes = vec![
            make_snapshot(
                "Jan Note",
                "January content about Rust programming Rust.",
                2026,
                1,
                15,
            ),
            make_snapshot(
                "Feb Note 1",
                "February content about Rust systems Rust.",
                2026,
                2,
                10,
            ),
            make_snapshot(
                "Feb Note 2",
                "More February content about Rust design Rust.",
                2026,
                2,
                20,
            ),
            make_snapshot(
                "Mar Note",
                "March content about Python programming Python.",
                2026,
                3,
                5,
            ),
        ];
        let report = analyze_temporal(&notes).unwrap();
        assert_eq!(report.total_notes, 4);
        assert_eq!(report.activity_by_month.len(), 3);
        assert_eq!(report.most_active_month, Some("2026-02".into()));
    }

    #[test]
    fn trend_detection() {
        let rising = vec![
            ("2026-01".into(), 1usize),
            ("2026-02".into(), 2),
            ("2026-03".into(), 5),
            ("2026-04".into(), 10),
        ];
        assert!(matches!(compute_trend(&rising), TrendDirection::Rising));

        let declining = vec![
            ("2026-01".into(), 10usize),
            ("2026-02".into(), 8),
            ("2026-03".into(), 2),
            ("2026-04".into(), 1),
        ];
        assert!(matches!(
            compute_trend(&declining),
            TrendDirection::Declining
        ));

        let stable = vec![
            ("2026-01".into(), 5usize),
            ("2026-02".into(), 5),
            ("2026-03".into(), 5),
            ("2026-04".into(), 5),
        ];
        assert!(matches!(compute_trend(&stable), TrendDirection::Stable));
    }

    #[test]
    fn report_serialization() {
        let report = TemporalReport {
            total_notes: 5,
            date_range: Some(("2026-01-01".into(), "2026-03-13".into())),
            activity_by_month: vec![],
            concept_trends: vec![],
            most_active_month: Some("2026-02".into()),
            avg_words_per_note: 150,
        };
        let json = serde_json::to_string(&report).unwrap();
        assert!(json.contains("2026-02"));
    }
}
