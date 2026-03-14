//! Note versioning — track changes over time.
//!
//! Maintains a version history for notes, enabling diff views,
//! rollback, and integration with Delta (AGNOS version control).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A version snapshot of a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteVersion {
    pub id: Uuid,
    pub note_id: Uuid,
    pub version_number: u32,
    pub title: String,
    pub content: String,
    pub content_hash: String,
    pub created_at: DateTime<Utc>,
    pub message: Option<String>,
}

/// A diff between two versions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionDiff {
    pub from_version: u32,
    pub to_version: u32,
    pub changes: Vec<DiffChange>,
    pub lines_added: usize,
    pub lines_removed: usize,
}

/// A single change in a diff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffChange {
    pub kind: ChangeKind,
    pub line_number: usize,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Removed,
    Context,
}

/// Version history for a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionHistory {
    pub note_id: Uuid,
    pub versions: Vec<NoteVersion>,
    pub current_version: u32,
}

/// In-memory version store (backed by vault files).
/// In production, this integrates with Delta for persistent VCS.
#[derive(Debug, Default)]
pub struct VersionStore {
    versions: std::collections::HashMap<Uuid, Vec<NoteVersion>>,
}

impl VersionStore {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new version of a note.
    pub fn record_version(
        &mut self,
        note_id: Uuid,
        title: &str,
        content: &str,
        content_hash: &str,
        message: Option<String>,
    ) -> NoteVersion {
        let versions = self.versions.entry(note_id).or_default();
        let version_number = versions.len() as u32 + 1;

        let version = NoteVersion {
            id: Uuid::new_v4(),
            note_id,
            version_number,
            title: title.to_string(),
            content: content.to_string(),
            content_hash: content_hash.to_string(),
            created_at: Utc::now(),
            message,
        };

        versions.push(version.clone());
        version
    }

    /// Get version history for a note.
    pub fn get_history(&self, note_id: Uuid) -> VersionHistory {
        let versions = self.versions.get(&note_id).cloned().unwrap_or_default();
        let current = versions.len() as u32;
        VersionHistory {
            note_id,
            versions,
            current_version: current,
        }
    }

    /// Get a specific version of a note.
    pub fn get_version(&self, note_id: Uuid, version: u32) -> Option<&NoteVersion> {
        self.versions
            .get(&note_id)
            .and_then(|versions| versions.iter().find(|v| v.version_number == version))
    }

    /// Compute diff between two versions.
    pub fn diff(&self, note_id: Uuid, from: u32, to: u32) -> Option<VersionDiff> {
        let from_version = self.get_version(note_id, from)?;
        let to_version = self.get_version(note_id, to)?;

        Some(compute_diff(
            &from_version.content,
            &to_version.content,
            from,
            to,
        ))
    }
}

/// Compute a line-level diff between two texts.
pub fn compute_diff(old: &str, new: &str, from_ver: u32, to_ver: u32) -> VersionDiff {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let mut changes = Vec::new();
    let mut lines_added = 0;
    let mut lines_removed = 0;

    let mut old_idx = 0;
    let mut new_idx = 0;

    while old_idx < old_lines.len() || new_idx < new_lines.len() {
        if old_idx < old_lines.len() && new_idx < new_lines.len() {
            if old_lines[old_idx] == new_lines[new_idx] {
                changes.push(DiffChange {
                    kind: ChangeKind::Context,
                    line_number: new_idx + 1,
                    content: new_lines[new_idx].to_string(),
                });
                old_idx += 1;
                new_idx += 1;
            } else {
                // Check if old line was removed
                changes.push(DiffChange {
                    kind: ChangeKind::Removed,
                    line_number: old_idx + 1,
                    content: old_lines[old_idx].to_string(),
                });
                lines_removed += 1;
                old_idx += 1;

                if new_idx < new_lines.len() {
                    changes.push(DiffChange {
                        kind: ChangeKind::Added,
                        line_number: new_idx + 1,
                        content: new_lines[new_idx].to_string(),
                    });
                    lines_added += 1;
                    new_idx += 1;
                }
            }
        } else if old_idx < old_lines.len() {
            changes.push(DiffChange {
                kind: ChangeKind::Removed,
                line_number: old_idx + 1,
                content: old_lines[old_idx].to_string(),
            });
            lines_removed += 1;
            old_idx += 1;
        } else {
            changes.push(DiffChange {
                kind: ChangeKind::Added,
                line_number: new_idx + 1,
                content: new_lines[new_idx].to_string(),
            });
            lines_added += 1;
            new_idx += 1;
        }
    }

    VersionDiff {
        from_version: from_ver,
        to_version: to_ver,
        changes,
        lines_added,
        lines_removed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_retrieve() {
        let mut store = VersionStore::new();
        let note_id = Uuid::new_v4();

        let v1 = store.record_version(note_id, "Title", "Content v1", "hash1", None);
        assert_eq!(v1.version_number, 1);

        let v2 = store.record_version(
            note_id,
            "Title",
            "Content v2",
            "hash2",
            Some("Updated".into()),
        );
        assert_eq!(v2.version_number, 2);

        let history = store.get_history(note_id);
        assert_eq!(history.versions.len(), 2);
        assert_eq!(history.current_version, 2);
    }

    #[test]
    fn get_specific_version() {
        let mut store = VersionStore::new();
        let note_id = Uuid::new_v4();

        store.record_version(note_id, "T", "First", "h1", None);
        store.record_version(note_id, "T", "Second", "h2", None);

        let v1 = store.get_version(note_id, 1).unwrap();
        assert_eq!(v1.content, "First");

        let v2 = store.get_version(note_id, 2).unwrap();
        assert_eq!(v2.content, "Second");

        assert!(store.get_version(note_id, 3).is_none());
    }

    #[test]
    fn diff_between_versions() {
        let mut store = VersionStore::new();
        let note_id = Uuid::new_v4();

        store.record_version(note_id, "T", "Line 1\nLine 2\nLine 3", "h1", None);
        store.record_version(
            note_id,
            "T",
            "Line 1\nModified\nLine 3\nLine 4",
            "h2",
            None,
        );

        let diff = store.diff(note_id, 1, 2).unwrap();
        assert!(diff.lines_added > 0 || diff.lines_removed > 0);
        assert_eq!(diff.from_version, 1);
        assert_eq!(diff.to_version, 2);
    }

    #[test]
    fn compute_diff_added_lines() {
        let diff = compute_diff("A\nB", "A\nB\nC\nD", 1, 2);
        assert_eq!(diff.lines_added, 2);
        assert_eq!(diff.lines_removed, 0);
    }

    #[test]
    fn compute_diff_removed_lines() {
        let diff = compute_diff("A\nB\nC", "A", 1, 2);
        assert!(diff.lines_removed >= 2);
    }

    #[test]
    fn empty_history() {
        let store = VersionStore::new();
        let history = store.get_history(Uuid::new_v4());
        assert!(history.versions.is_empty());
        assert_eq!(history.current_version, 0);
    }

    #[test]
    fn version_serialization() {
        let v = NoteVersion {
            id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            version_number: 1,
            title: "Test".into(),
            content: "Content".into(),
            content_hash: "abc".into(),
            created_at: Utc::now(),
            message: Some("Initial".into()),
        };
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("version_number"));
    }
}
