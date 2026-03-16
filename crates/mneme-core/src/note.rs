//! Note types — the fundamental unit of knowledge in Mneme.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A note in the knowledge base.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub id: Uuid,
    pub title: String,
    /// Relative path from the vault root (e.g. "projects/agnos.md").
    pub path: String,
    /// SHA-256 hash of the Markdown content (hex-encoded).
    pub content_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
}

impl Note {
    pub fn new(title: String, path: String, content_hash: String) -> Self {
        let now = Utc::now();
        Self {
            id: Uuid::new_v4(),
            title,
            path,
            content_hash,
            created_at: now,
            updated_at: now,
            last_accessed: now,
        }
    }
}

/// Request to create a new note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateNote {
    pub title: String,
    /// Optional explicit path; if absent, derived from title.
    pub path: Option<String>,
    pub content: String,
    pub tags: Vec<String>,
}

/// Request to update an existing note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateNote {
    pub title: Option<String>,
    pub content: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// A note with its full content loaded.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoteWithContent {
    #[serde(flatten)]
    pub note: Note,
    pub content: String,
    pub tags: Vec<String>,
    pub backlinks: Vec<BacklinkInfo>,
}

/// Minimal info about a backlink (what links to this note).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BacklinkInfo {
    pub source_id: Uuid,
    pub source_title: String,
    pub link_text: String,
    pub context: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_note_has_matching_timestamps() {
        let note = Note::new("Test".into(), "test.md".into(), "abc123".into());
        assert_eq!(note.created_at, note.updated_at);
        assert_eq!(note.created_at, note.last_accessed);
        assert_eq!(note.title, "Test");
    }
}
