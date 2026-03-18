//! File-backed Markdown note storage.

use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::fs;

use crate::StoreError;

/// Manages reading and writing Markdown files in the vault directory.
pub struct FileStore {
    notes_dir: PathBuf,
}

impl FileStore {
    pub fn new(vault_root: &Path) -> Self {
        Self {
            notes_dir: vault_root.join("notes"),
        }
    }

    pub fn notes_dir(&self) -> &Path {
        &self.notes_dir
    }

    /// Ensure the notes directory exists.
    pub async fn init(&self) -> Result<(), StoreError> {
        fs::create_dir_all(&self.notes_dir).await?;
        Ok(())
    }

    /// Read a note's Markdown content from disk.
    pub async fn read_note(&self, relative_path: &str) -> Result<String, StoreError> {
        let full_path = self.notes_dir.join(relative_path);
        fs::read_to_string(&full_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StoreError::NoteNotFound(relative_path.to_string())
            } else {
                StoreError::Io(e)
            }
        })
    }

    /// Write a note's Markdown content to disk.
    pub async fn write_note(&self, relative_path: &str, content: &str) -> Result<(), StoreError> {
        let full_path = self.notes_dir.join(relative_path);

        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).await?;
        }

        fs::write(&full_path, content).await?;
        Ok(())
    }

    /// Delete a note file from disk.
    pub async fn delete_note(&self, relative_path: &str) -> Result<(), StoreError> {
        let full_path = self.notes_dir.join(relative_path);
        fs::remove_file(&full_path).await.map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                StoreError::NoteNotFound(relative_path.to_string())
            } else {
                StoreError::Io(e)
            }
        })
    }

    /// Check if a note file exists.
    pub async fn exists(&self, relative_path: &str) -> bool {
        self.notes_dir.join(relative_path).exists()
    }
}

/// Compute a SHA-256 hash of content, hex-encoded.
pub fn content_hash(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}

/// Derive a filesystem path from a note title.
///
/// Converts to lowercase, replaces spaces/special chars with hyphens,
/// and appends `.md`.
pub fn title_to_path(title: &str) -> String {
    let slug: String = title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();

    // Collapse multiple hyphens
    let mut result = String::with_capacity(slug.len());
    let mut prev_hyphen = false;
    for c in slug.chars() {
        if c == '-' {
            if !prev_hyphen && !result.is_empty() {
                result.push('-');
            }
            prev_hyphen = true;
        } else {
            result.push(c);
            prev_hyphen = false;
        }
    }

    // Trim trailing hyphen
    let result = result.trim_end_matches('-');
    format!("{result}.md")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_deterministic() {
        let h1 = content_hash("hello world");
        let h2 = content_hash("hello world");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex
    }

    #[test]
    fn hash_differs() {
        assert_ne!(content_hash("a"), content_hash("b"));
    }

    #[test]
    fn title_to_path_basic() {
        assert_eq!(title_to_path("Hello World"), "hello-world.md");
    }

    #[test]
    fn title_to_path_special_chars() {
        assert_eq!(title_to_path("What's New? (2026)"), "what-s-new-2026.md");
    }

    #[test]
    fn title_to_path_collapses_hyphens() {
        assert_eq!(title_to_path("a   b"), "a-b.md");
    }

    #[tokio::test]
    async fn filestore_notes_dir() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = FileStore::new(dir.path());
        assert_eq!(store.notes_dir(), dir.path().join("notes"));
    }

    #[tokio::test]
    async fn filestore_write_and_read() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = FileStore::new(dir.path());
        store.init().await.unwrap();

        store.write_note("test.md", "# Hello").await.unwrap();
        let content = store.read_note("test.md").await.unwrap();
        assert_eq!(content, "# Hello");
    }

    #[tokio::test]
    async fn filestore_read_not_found() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = FileStore::new(dir.path());
        store.init().await.unwrap();

        let result = store.read_note("nonexistent.md").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn filestore_delete() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = FileStore::new(dir.path());
        store.init().await.unwrap();

        store.write_note("to-delete.md", "temp").await.unwrap();
        assert!(store.exists("to-delete.md").await);
        store.delete_note("to-delete.md").await.unwrap();
        assert!(!store.exists("to-delete.md").await);
    }

    #[tokio::test]
    async fn filestore_delete_not_found() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = FileStore::new(dir.path());
        store.init().await.unwrap();

        let result = store.delete_note("nonexistent.md").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn filestore_write_subdirectory() {
        let dir = tempfile::TempDir::new().unwrap();
        let store = FileStore::new(dir.path());
        store.init().await.unwrap();

        store
            .write_note("subdir/nested.md", "nested content")
            .await
            .unwrap();
        let content = store.read_note("subdir/nested.md").await.unwrap();
        assert_eq!(content, "nested content");
    }
}
