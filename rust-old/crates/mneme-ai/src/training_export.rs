//! Training data export — capture user signals for model fine-tuning.
//!
//! Collects search feedback, note edits, and trust overrides into JSONL
//! format suitable for Synapse fine-tuning or external training pipelines.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single training data record.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TrainingRecord {
    /// User clicked a search result (positive relevance signal).
    #[serde(rename = "search_click")]
    SearchClick {
        timestamp: DateTime<Utc>,
        query: String,
        clicked_note_id: Uuid,
        clicked_note_title: String,
        search_arm: String,
        position: usize,
    },
    /// User edited a note shortly after searching (implicit relevance).
    #[serde(rename = "edit_after_search")]
    EditAfterSearch {
        timestamp: DateTime<Utc>,
        query: String,
        note_id: Uuid,
        note_title: String,
        /// Number of seconds between search and edit.
        latency_secs: i64,
    },
    /// User overrode the trust/provenance of a note.
    #[serde(rename = "trust_override")]
    TrustOverride {
        timestamp: DateTime<Utc>,
        note_id: Uuid,
        old_provenance: String,
        new_trust: f64,
    },
    /// A note was created with content (for embedding training pairs).
    #[serde(rename = "note_content")]
    NoteContent {
        timestamp: DateTime<Utc>,
        note_id: Uuid,
        title: String,
        content: String,
        tags: Vec<String>,
    },
}

/// Append-only log of training records stored on disk.
pub struct TrainingLog {
    path: std::path::PathBuf,
}

impl TrainingLog {
    /// Open or create a training log at the given path.
    pub fn open(path: std::path::PathBuf) -> Self {
        Self { path }
    }

    /// Append a record to the log.
    pub fn append(&self, record: &TrainingRecord) -> Result<(), std::io::Error> {
        use std::io::Write;
        let line = serde_json::to_string(record)?;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)?;
        writeln!(file, "{line}")?;
        Ok(())
    }

    /// Read all records from the log.
    pub fn read_all(&self) -> Result<Vec<TrainingRecord>, std::io::Error> {
        if !self.path.exists() {
            return Ok(vec![]);
        }
        let content = std::fs::read_to_string(&self.path)?;
        let records: Vec<TrainingRecord> = content
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        Ok(records)
    }

    /// Read records, filtering by type and optional date range.
    pub fn read_filtered(
        &self,
        record_type: Option<&str>,
        since: Option<DateTime<Utc>>,
    ) -> Result<Vec<TrainingRecord>, std::io::Error> {
        let all = self.read_all()?;
        let filtered = all
            .into_iter()
            .filter(|r| {
                if let Some(rt) = record_type {
                    let json = serde_json::to_value(r).unwrap_or_default();
                    json.get("type").and_then(|t| t.as_str()) == Some(rt)
                } else {
                    true
                }
            })
            .filter(|r| {
                if let Some(since) = since {
                    record_timestamp(r) >= since
                } else {
                    true
                }
            })
            .collect();
        Ok(filtered)
    }

    /// Number of records in the log.
    pub fn count(&self) -> Result<usize, std::io::Error> {
        if !self.path.exists() {
            return Ok(0);
        }
        let content = std::fs::read_to_string(&self.path)?;
        Ok(content.lines().filter(|l| !l.trim().is_empty()).count())
    }

    /// Export all records as a single JSONL string.
    pub fn export_jsonl(&self) -> Result<String, std::io::Error> {
        if !self.path.exists() {
            return Ok(String::new());
        }
        std::fs::read_to_string(&self.path)
    }

    /// Clear the log.
    pub fn clear(&self) -> Result<(), std::io::Error> {
        if self.path.exists() {
            std::fs::remove_file(&self.path)?;
        }
        Ok(())
    }
}

fn record_timestamp(r: &TrainingRecord) -> DateTime<Utc> {
    match r {
        TrainingRecord::SearchClick { timestamp, .. } => *timestamp,
        TrainingRecord::EditAfterSearch { timestamp, .. } => *timestamp,
        TrainingRecord::TrustOverride { timestamp, .. } => *timestamp,
        TrainingRecord::NoteContent { timestamp, .. } => *timestamp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn training_record_serde_roundtrip() {
        let record = TrainingRecord::SearchClick {
            timestamp: Utc::now(),
            query: "rust ownership".into(),
            clicked_note_id: Uuid::new_v4(),
            clicked_note_title: "Rust Guide".into(),
            search_arm: "balanced".into(),
            position: 0,
        };
        let json = serde_json::to_string(&record).unwrap();
        let parsed: TrainingRecord = serde_json::from_str(&json).unwrap();
        match parsed {
            TrainingRecord::SearchClick { query, .. } => assert_eq!(query, "rust ownership"),
            _ => panic!("wrong variant"),
        }
    }

    #[test]
    fn training_log_append_and_read() {
        let dir = tempfile::TempDir::new().unwrap();
        let log = TrainingLog::open(dir.path().join("train.jsonl"));

        let r1 = TrainingRecord::SearchClick {
            timestamp: Utc::now(),
            query: "test".into(),
            clicked_note_id: Uuid::new_v4(),
            clicked_note_title: "Note".into(),
            search_arm: "balanced".into(),
            position: 0,
        };
        let r2 = TrainingRecord::TrustOverride {
            timestamp: Utc::now(),
            note_id: Uuid::new_v4(),
            old_provenance: "generated".into(),
            new_trust: 0.9,
        };

        log.append(&r1).unwrap();
        log.append(&r2).unwrap();

        let all = log.read_all().unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(log.count().unwrap(), 2);
    }

    #[test]
    fn training_log_empty() {
        let dir = tempfile::TempDir::new().unwrap();
        let log = TrainingLog::open(dir.path().join("nonexistent.jsonl"));
        assert_eq!(log.count().unwrap(), 0);
        assert!(log.read_all().unwrap().is_empty());
        assert!(log.export_jsonl().unwrap().is_empty());
    }

    #[test]
    fn training_log_filter_by_type() {
        let dir = tempfile::TempDir::new().unwrap();
        let log = TrainingLog::open(dir.path().join("train.jsonl"));

        log.append(&TrainingRecord::SearchClick {
            timestamp: Utc::now(),
            query: "q1".into(),
            clicked_note_id: Uuid::new_v4(),
            clicked_note_title: "N1".into(),
            search_arm: "balanced".into(),
            position: 0,
        })
        .unwrap();
        log.append(&TrainingRecord::TrustOverride {
            timestamp: Utc::now(),
            note_id: Uuid::new_v4(),
            old_provenance: "manual".into(),
            new_trust: 0.5,
        })
        .unwrap();

        let clicks = log.read_filtered(Some("search_click"), None).unwrap();
        assert_eq!(clicks.len(), 1);
        let overrides = log.read_filtered(Some("trust_override"), None).unwrap();
        assert_eq!(overrides.len(), 1);
    }

    #[test]
    fn training_log_clear() {
        let dir = tempfile::TempDir::new().unwrap();
        let log = TrainingLog::open(dir.path().join("train.jsonl"));
        log.append(&TrainingRecord::SearchClick {
            timestamp: Utc::now(),
            query: "q".into(),
            clicked_note_id: Uuid::new_v4(),
            clicked_note_title: "N".into(),
            search_arm: "balanced".into(),
            position: 0,
        })
        .unwrap();
        assert_eq!(log.count().unwrap(), 1);
        log.clear().unwrap();
        assert_eq!(log.count().unwrap(), 0);
    }

    #[test]
    fn export_jsonl_format() {
        let dir = tempfile::TempDir::new().unwrap();
        let log = TrainingLog::open(dir.path().join("train.jsonl"));
        log.append(&TrainingRecord::NoteContent {
            timestamp: Utc::now(),
            note_id: Uuid::new_v4(),
            title: "Test".into(),
            content: "Content here".into(),
            tags: vec!["tag1".into()],
        })
        .unwrap();
        let jsonl = log.export_jsonl().unwrap();
        assert!(jsonl.contains("\"type\":\"note_content\""));
        assert!(jsonl.contains("\"title\":\"Test\""));
        // Should be a single line
        assert_eq!(jsonl.lines().count(), 1);
    }
}
