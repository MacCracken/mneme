//! Vault — the top-level store combining DB and file operations.

use std::path::{Path, PathBuf};
use uuid::Uuid;

use mneme_core::frontmatter::{
    Frontmatter, compose_document, parse_frontmatter, split_frontmatter,
};
use mneme_core::note::{CreateNote, Note, NoteWithContent, UpdateNote};
use mneme_core::tag::Tag;

use crate::StoreError;
use crate::db::Database;
use crate::files::{FileStore, content_hash, title_to_path};

/// A vault encapsulates a notes directory + SQLite database.
pub struct Vault {
    db: Database,
    files: FileStore,
    root: PathBuf,
}

impl Vault {
    /// Open a vault at the given directory path.
    pub async fn open(root: &Path) -> Result<Self, StoreError> {
        let db_path = root.join(".mneme").join("db.sqlite");
        tokio::fs::create_dir_all(root.join(".mneme")).await?;

        let db = Database::open(db_path.to_str().unwrap()).await?;
        let files = FileStore::new(root);
        files.init().await?;

        Ok(Self {
            db,
            files,
            root: root.to_path_buf(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn db(&self) -> &Database {
        &self.db
    }

    // --- Note operations ---

    /// Create a new note.
    pub async fn create_note(&self, req: CreateNote) -> Result<NoteWithContent, StoreError> {
        let path = req.path.unwrap_or_else(|| title_to_path(&req.title));

        // Check for path conflict
        if self.files.exists(&path).await {
            return Err(StoreError::PathConflict(path));
        }

        // Build frontmatter
        let fm = Frontmatter {
            title: Some(req.title.clone()),
            tags: req.tags.clone(),
            ..Default::default()
        };
        let document = compose_document(&fm, &req.content);
        let hash = content_hash(&document);

        // Write file
        self.files.write_note(&path, &document).await?;

        // Create DB record
        let note = Note::new(req.title, path, hash);
        self.db.insert_note(&note).await?;

        // Create tags and associations
        for tag_name in &req.tags {
            let tag = self.db.get_or_create_tag(tag_name).await?;
            self.db.tag_note(note.id, tag.id).await?;
        }

        Ok(NoteWithContent {
            note,
            content: req.content,
            tags: req.tags,
            backlinks: vec![],
        })
    }

    /// Get a note with its full content.
    pub async fn get_note(&self, id: Uuid) -> Result<NoteWithContent, StoreError> {
        let note = self.db.get_note(id).await?;
        let document = self.files.read_note(&note.path).await?;
        let (yaml, body) = split_frontmatter(&document);

        let tags = self.db.get_note_tags(note.id).await?;
        let backlinks = self.db.get_backlinks(note.id).await?;

        let fm = yaml.map(parse_frontmatter);
        let _ = fm; // frontmatter already reflected in DB

        Ok(NoteWithContent {
            note,
            content: body.to_string(),
            tags,
            backlinks,
        })
    }

    /// List notes with pagination.
    pub async fn list_notes(&self, limit: i64, offset: i64) -> Result<Vec<Note>, StoreError> {
        self.db.list_notes(limit, offset).await
    }

    /// Update a note's content and/or title.
    pub async fn update_note(
        &self,
        id: Uuid,
        req: UpdateNote,
    ) -> Result<NoteWithContent, StoreError> {
        let note = self.db.get_note(id).await?;
        let document = self.files.read_note(&note.path).await?;
        let (yaml, body) = split_frontmatter(&document);

        let mut fm = yaml.map(parse_frontmatter).unwrap_or_default();

        let new_title = req.title.unwrap_or(note.title.clone());
        let new_content = req.content.unwrap_or_else(|| body.to_string());

        fm.title = Some(new_title.clone());

        // Update tags if provided
        if let Some(new_tags) = &req.tags {
            fm.tags = new_tags.clone();
            self.db.clear_note_tags(id).await?;
            for tag_name in new_tags {
                let tag = self.db.get_or_create_tag(tag_name).await?;
                self.db.tag_note(id, tag.id).await?;
            }
        }

        let new_document = compose_document(&fm, &new_content);
        let hash = content_hash(&new_document);

        self.files.write_note(&note.path, &new_document).await?;
        self.db.update_note(id, &new_title, &hash).await?;

        let tags = self.db.get_note_tags(id).await?;
        let backlinks = self.db.get_backlinks(id).await?;

        Ok(NoteWithContent {
            note: Note {
                title: new_title,
                content_hash: hash,
                ..note
            },
            content: new_content,
            tags,
            backlinks,
        })
    }

    /// Delete a note from both DB and filesystem.
    pub async fn delete_note(&self, id: Uuid) -> Result<(), StoreError> {
        let note = self.db.get_note(id).await?;
        self.db.delete_note(id).await?;
        // File deletion is best-effort
        let _ = self.files.delete_note(&note.path).await;
        Ok(())
    }

    /// Count total notes in the vault.
    pub async fn count_notes(&self) -> Result<i64, StoreError> {
        self.db.count_notes().await
    }

    // --- Tag operations ---

    /// List all tags.
    pub async fn list_tags(&self) -> Result<Vec<Tag>, StoreError> {
        self.db.list_tags().await
    }

    /// Delete a tag.
    pub async fn delete_tag(&self, id: Uuid) -> Result<(), StoreError> {
        self.db.delete_tag(id).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn test_vault() -> (Vault, TempDir) {
        let dir = TempDir::new().unwrap();
        let vault = Vault::open(dir.path()).await.unwrap();
        (vault, dir)
    }

    #[tokio::test]
    async fn create_and_get_note() {
        let (vault, _dir) = test_vault().await;

        let created = vault
            .create_note(CreateNote {
                title: "Hello World".into(),
                path: None,
                content: "This is a test note.".into(),
                tags: vec!["test".into(), "demo".into()],
            })
            .await
            .unwrap();

        assert_eq!(created.note.title, "Hello World");
        assert_eq!(created.tags.len(), 2);
        assert!(created.tags.contains(&"test".to_string()));
        assert!(created.tags.contains(&"demo".to_string()));
        assert_eq!(created.content, "This is a test note.");

        // Retrieve it
        let fetched = vault.get_note(created.note.id).await.unwrap();
        assert_eq!(fetched.note.title, "Hello World");
        assert_eq!(fetched.tags.len(), 2);
    }

    #[tokio::test]
    async fn update_note() {
        let (vault, _dir) = test_vault().await;

        let created = vault
            .create_note(CreateNote {
                title: "Original".into(),
                path: None,
                content: "Original content.".into(),
                tags: vec![],
            })
            .await
            .unwrap();

        let updated = vault
            .update_note(
                created.note.id,
                UpdateNote {
                    title: Some("Updated Title".into()),
                    content: Some("New content.".into()),
                    tags: Some(vec!["updated".into()]),
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.note.title, "Updated Title");
        assert_eq!(updated.content, "New content.");
        assert_eq!(updated.tags, vec!["updated"]);
    }

    #[tokio::test]
    async fn delete_note() {
        let (vault, _dir) = test_vault().await;

        let created = vault
            .create_note(CreateNote {
                title: "To Delete".into(),
                path: None,
                content: "Goodbye.".into(),
                tags: vec![],
            })
            .await
            .unwrap();

        vault.delete_note(created.note.id).await.unwrap();
        assert!(vault.get_note(created.note.id).await.is_err());
    }

    #[tokio::test]
    async fn list_and_count() {
        let (vault, _dir) = test_vault().await;

        for i in 0..5 {
            vault
                .create_note(CreateNote {
                    title: format!("Note {i}"),
                    path: None,
                    content: format!("Content {i}"),
                    tags: vec![],
                })
                .await
                .unwrap();
        }

        assert_eq!(vault.count_notes().await.unwrap(), 5);
        let notes = vault.list_notes(3, 0).await.unwrap();
        assert_eq!(notes.len(), 3);
    }

    #[tokio::test]
    async fn path_conflict() {
        let (vault, _dir) = test_vault().await;

        vault
            .create_note(CreateNote {
                title: "Same".into(),
                path: Some("same.md".into()),
                content: "First".into(),
                tags: vec![],
            })
            .await
            .unwrap();

        let err = vault
            .create_note(CreateNote {
                title: "Same Again".into(),
                path: Some("same.md".into()),
                content: "Second".into(),
                tags: vec![],
            })
            .await;

        assert!(matches!(err, Err(StoreError::PathConflict(_))));
    }

    #[tokio::test]
    async fn tag_operations() {
        let (vault, _dir) = test_vault().await;
        vault
            .create_note(CreateNote {
                title: "Tagged".into(),
                path: None,
                content: "Content.".into(),
                tags: vec!["alpha".into(), "beta".into()],
            })
            .await
            .unwrap();

        let tags = vault.list_tags().await.unwrap();
        assert_eq!(tags.len(), 2);

        let tag_id = tags.iter().find(|t| t.name == "alpha").unwrap().id;
        vault.delete_tag(tag_id).await.unwrap();
        let tags = vault.list_tags().await.unwrap();
        assert_eq!(tags.len(), 1);
    }

    #[tokio::test]
    async fn update_note_partial() {
        let (vault, _dir) = test_vault().await;
        let created = vault
            .create_note(CreateNote {
                title: "Original".into(),
                path: None,
                content: "Original content.".into(),
                tags: vec!["tag1".into()],
            })
            .await
            .unwrap();

        // Update only title
        let updated = vault
            .update_note(
                created.note.id,
                UpdateNote {
                    title: Some("New Title".into()),
                    content: None,
                    tags: None,
                },
            )
            .await
            .unwrap();
        assert_eq!(updated.note.title, "New Title");
        assert_eq!(updated.content, "Original content.");
        assert_eq!(updated.tags, vec!["tag1"]);
    }

    #[tokio::test]
    async fn count_notes_empty() {
        let (vault, _dir) = test_vault().await;
        assert_eq!(vault.count_notes().await.unwrap(), 0);
    }

    #[tokio::test]
    async fn delete_nonexistent_note() {
        let (vault, _dir) = test_vault().await;
        let result = vault.delete_note(uuid::Uuid::new_v4()).await;
        assert!(result.is_err());
    }
}
