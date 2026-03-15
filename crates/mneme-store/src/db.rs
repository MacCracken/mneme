//! SQLite database operations for notes, tags, and links.

use chrono::{DateTime, Utc};
use sqlx::Row;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use uuid::Uuid;

use mneme_core::link::Link;
use mneme_core::note::{BacklinkInfo, Note};
use mneme_core::tag::Tag;

use crate::StoreError;

/// Database connection pool and query methods.
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    /// Open (or create) a SQLite database at the given path.
    pub async fn open(path: &str) -> Result<Self, StoreError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&format!("sqlite:{path}?mode=rwc"))
            .await?;

        let db = Self { pool };
        db.run_migrations().await?;
        Ok(db)
    }

    /// Run schema migrations.
    async fn run_migrations(&self) -> Result<(), StoreError> {
        sqlx::query(include_str!("../migrations/001_initial.sql"))
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Notes ---

    /// Insert a new note record.
    pub async fn insert_note(&self, note: &Note) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO notes (id, title, path, content_hash, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(note.id.to_string())
        .bind(&note.title)
        .bind(&note.path)
        .bind(&note.content_hash)
        .bind(note.created_at.to_rfc3339())
        .bind(note.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get a note by ID.
    pub async fn get_note(&self, id: Uuid) -> Result<Note, StoreError> {
        let row = sqlx::query(
            "SELECT id, title, path, content_hash, created_at, updated_at
             FROM notes WHERE id = ?",
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StoreError::NoteNotFound(id.to_string()))?;

        row_to_note(&row)
    }

    /// Get a note by its file path.
    pub async fn get_note_by_path(&self, path: &str) -> Result<Note, StoreError> {
        let row = sqlx::query(
            "SELECT id, title, path, content_hash, created_at, updated_at
             FROM notes WHERE path = ?",
        )
        .bind(path)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(|| StoreError::NoteNotFound(path.to_string()))?;

        row_to_note(&row)
    }

    /// List all notes, ordered by updated_at descending.
    pub async fn list_notes(&self, limit: i64, offset: i64) -> Result<Vec<Note>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, title, path, content_hash, created_at, updated_at
             FROM notes ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_note).collect()
    }

    /// Update a note's title, content hash, and updated_at.
    pub async fn update_note(
        &self,
        id: Uuid,
        title: &str,
        content_hash: &str,
    ) -> Result<(), StoreError> {
        let result = sqlx::query(
            "UPDATE notes SET title = ?, content_hash = ?, updated_at = ?
             WHERE id = ?",
        )
        .bind(title)
        .bind(content_hash)
        .bind(Utc::now().to_rfc3339())
        .bind(id.to_string())
        .execute(&self.pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(StoreError::NoteNotFound(id.to_string()));
        }
        Ok(())
    }

    /// Delete a note and all its tag associations and links.
    pub async fn delete_note(&self, id: Uuid) -> Result<(), StoreError> {
        let result = sqlx::query("DELETE FROM notes WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(StoreError::NoteNotFound(id.to_string()));
        }
        Ok(())
    }

    /// Count total notes.
    pub async fn count_notes(&self) -> Result<i64, StoreError> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM notes")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get::<i64, _>("count"))
    }

    // --- Tags ---

    /// Insert or get an existing tag by name.
    pub async fn get_or_create_tag(&self, name: &str) -> Result<Tag, StoreError> {
        // Try to fetch existing
        if let Some(row) =
            sqlx::query("SELECT id, name, color, created_at FROM tags WHERE name = ?")
                .bind(name)
                .fetch_optional(&self.pool)
                .await?
        {
            return row_to_tag(&row);
        }

        // Create new
        let tag = Tag::new(name.to_string());
        sqlx::query("INSERT INTO tags (id, name, color, created_at) VALUES (?, ?, ?, ?)")
            .bind(tag.id.to_string())
            .bind(&tag.name)
            .bind(&tag.color)
            .bind(tag.created_at.to_rfc3339())
            .execute(&self.pool)
            .await?;
        Ok(tag)
    }

    /// List all tags.
    pub async fn list_tags(&self) -> Result<Vec<Tag>, StoreError> {
        let rows = sqlx::query("SELECT id, name, color, created_at FROM tags ORDER BY name")
            .fetch_all(&self.pool)
            .await?;
        rows.iter().map(row_to_tag).collect()
    }

    /// Delete a tag by ID.
    pub async fn delete_tag(&self, id: Uuid) -> Result<(), StoreError> {
        let result = sqlx::query("DELETE FROM tags WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await?;
        if result.rows_affected() == 0 {
            return Err(StoreError::TagNotFound(id.to_string()));
        }
        Ok(())
    }

    // --- Note-Tag associations ---

    /// Associate a tag with a note.
    pub async fn tag_note(&self, note_id: Uuid, tag_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("INSERT OR IGNORE INTO note_tags (note_id, tag_id) VALUES (?, ?)")
            .bind(note_id.to_string())
            .bind(tag_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Remove a tag from a note.
    pub async fn untag_note(&self, note_id: Uuid, tag_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM note_tags WHERE note_id = ? AND tag_id = ?")
            .bind(note_id.to_string())
            .bind(tag_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Remove all tags from a note.
    pub async fn clear_note_tags(&self, note_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM note_tags WHERE note_id = ?")
            .bind(note_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get all tag names for a note.
    pub async fn get_note_tags(&self, note_id: Uuid) -> Result<Vec<String>, StoreError> {
        let rows = sqlx::query(
            "SELECT t.name FROM tags t
             JOIN note_tags nt ON t.id = nt.tag_id
             WHERE nt.note_id = ?
             ORDER BY t.name",
        )
        .bind(note_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.iter().map(|r| r.get::<String, _>("name")).collect())
    }

    // --- Links ---

    /// Insert a link between two notes.
    pub async fn insert_link(&self, link: &Link) -> Result<(), StoreError> {
        sqlx::query(
            "INSERT INTO links (id, source_id, target_id, link_text, context, created_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(link.id.to_string())
        .bind(link.source_id.to_string())
        .bind(link.target_id.to_string())
        .bind(&link.link_text)
        .bind(&link.context)
        .bind(link.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Get all links originating from a note (outgoing).
    pub async fn get_outgoing_links(&self, note_id: Uuid) -> Result<Vec<Link>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, source_id, target_id, link_text, context, created_at
             FROM links WHERE source_id = ?",
        )
        .bind(note_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_link).collect()
    }

    /// Get backlinks — notes that link TO this note.
    pub async fn get_backlinks(&self, note_id: Uuid) -> Result<Vec<BacklinkInfo>, StoreError> {
        let rows = sqlx::query(
            "SELECT l.link_text, l.context, n.id as source_id, n.title as source_title
             FROM links l
             JOIN notes n ON l.source_id = n.id
             WHERE l.target_id = ?",
        )
        .bind(note_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        Ok(rows
            .iter()
            .map(|r| BacklinkInfo {
                source_id: Uuid::parse_str(&r.get::<String, _>("source_id")).unwrap(),
                source_title: r.get("source_title"),
                link_text: r.get("link_text"),
                context: r.get("context"),
            })
            .collect())
    }

    /// Fetch all links in the database (for graph building).
    pub async fn list_all_links(&self) -> Result<Vec<Link>, StoreError> {
        let rows = sqlx::query(
            "SELECT id, source_id, target_id, link_text, context, created_at FROM links",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_link).collect()
    }

    /// Delete all links originating from a note.
    pub async fn clear_note_links(&self, note_id: Uuid) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM links WHERE source_id = ?")
            .bind(note_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Get the underlying pool (for testing or advanced use).
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

// --- Row conversion helpers ---

fn row_to_note(row: &sqlx::sqlite::SqliteRow) -> Result<Note, StoreError> {
    Ok(Note {
        id: Uuid::parse_str(&row.get::<String, _>("id")).unwrap(),
        title: row.get("title"),
        path: row.get("path"),
        content_hash: row.get("content_hash"),
        created_at: parse_datetime(&row.get::<String, _>("created_at")),
        updated_at: parse_datetime(&row.get::<String, _>("updated_at")),
    })
}

fn row_to_tag(row: &sqlx::sqlite::SqliteRow) -> Result<Tag, StoreError> {
    Ok(Tag {
        id: Uuid::parse_str(&row.get::<String, _>("id")).unwrap(),
        name: row.get("name"),
        color: row.get("color"),
        created_at: parse_datetime(&row.get::<String, _>("created_at")),
    })
}

fn row_to_link(row: &sqlx::sqlite::SqliteRow) -> Result<Link, StoreError> {
    Ok(Link {
        id: Uuid::parse_str(&row.get::<String, _>("id")).unwrap(),
        source_id: Uuid::parse_str(&row.get::<String, _>("source_id")).unwrap(),
        target_id: Uuid::parse_str(&row.get::<String, _>("target_id")).unwrap(),
        link_text: row.get("link_text"),
        context: row.get("context"),
        created_at: parse_datetime(&row.get::<String, _>("created_at")),
    })
}

fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| {
            // Fallback for SQLite datetime format "YYYY-MM-DD HH:MM:SS"
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .map(|ndt| ndt.and_utc())
                .unwrap_or_else(|_| Utc::now())
        })
}
