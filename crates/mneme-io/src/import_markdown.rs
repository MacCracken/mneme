//! Plain Markdown directory importer.
//!
//! Imports any directory of `.md` files, preserving directory structure.
//! Simpler than Obsidian import — no wikilink conversion needed.

use std::path::{Path, PathBuf};

use tokio::fs;

use mneme_core::frontmatter::{parse_frontmatter, split_frontmatter};

use crate::IoError;
use crate::import_obsidian::ImportedNote;

/// Import statistics for plain Markdown import.
#[derive(Debug, Default)]
pub struct MarkdownImportStats {
    pub files_found: usize,
    pub files_imported: usize,
    pub skipped: usize,
    pub errors: Vec<(PathBuf, String)>,
}

/// Import all `.md` files from a directory tree.
pub async fn import_markdown_dir(
    source_dir: &Path,
) -> Result<(Vec<ImportedNote>, MarkdownImportStats), IoError> {
    if !source_dir.exists() {
        return Err(IoError::SourceNotFound(source_dir.display().to_string()));
    }

    let mut notes = Vec::new();
    let mut stats = MarkdownImportStats::default();

    let files = collect_md_files(source_dir).await?;
    stats.files_found = files.len();

    for file_path in files {
        match import_md_file(source_dir, &file_path).await {
            Ok(note) => {
                notes.push(note);
                stats.files_imported += 1;
            }
            Err(e) => {
                stats.errors.push((file_path.clone(), e.to_string()));
            }
        }
    }

    Ok((notes, stats))
}

async fn import_md_file(root: &Path, path: &Path) -> Result<ImportedNote, IoError> {
    let raw = fs::read_to_string(path).await?;

    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();

    let (yaml, body) = split_frontmatter(&raw);
    let fm = yaml.map(parse_frontmatter).unwrap_or_default();

    let title = fm.title.unwrap_or_else(|| {
        path.file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".into())
    });

    let modified_at = fs::metadata(path)
        .await
        .ok()
        .and_then(|m| m.modified().ok())
        .map(chrono::DateTime::<chrono::Utc>::from);

    Ok(ImportedNote {
        title,
        content: body.to_string(),
        tags: fm.tags,
        source_path: path.to_path_buf(),
        relative_path: relative,
        wikilinks: vec![],
        modified_at,
    })
}

async fn collect_md_files(dir: &Path) -> Result<Vec<PathBuf>, IoError> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        let mut entries = fs::read_dir(&current).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            if name.starts_with('.') {
                continue;
            }

            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "md") {
                files.push(path);
            }
        }
    }

    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    async fn write_file(dir: &Path, name: &str, content: &str) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await.unwrap();
        }
        fs::write(path, content).await.unwrap();
    }

    #[tokio::test]
    async fn import_flat_dir() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "a.md", "Note A content.").await;
        write_file(dir.path(), "b.md", "---\ntitle: Note B\n---\nContent B.").await;

        let (notes, stats) = import_markdown_dir(dir.path()).await.unwrap();
        assert_eq!(stats.files_found, 2);
        assert_eq!(stats.files_imported, 2);
        assert!(notes.iter().any(|n| n.title == "Note B"));
        assert!(notes.iter().any(|n| n.title == "a")); // from filename
    }

    #[tokio::test]
    async fn import_preserves_structure() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "root.md", "Root.").await;
        write_file(dir.path(), "project/plan.md", "Plan.").await;
        write_file(dir.path(), "project/sub/detail.md", "Detail.").await;

        let (notes, stats) = import_markdown_dir(dir.path()).await.unwrap();
        assert_eq!(stats.files_found, 3);
        assert!(
            notes
                .iter()
                .any(|n| n.relative_path.contains("project/sub/"))
        );
    }
}
