//! Obsidian vault importer.
//!
//! Imports an Obsidian vault by:
//! 1. Walking the directory for `.md` files
//! 2. Parsing YAML frontmatter
//! 3. Converting `[[wikilinks]]` to standard Markdown links
//! 4. Producing ImportedNote structs ready for vault insertion

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use tokio::fs;

use mneme_core::frontmatter::{parse_frontmatter, split_frontmatter};

use crate::wikilink::{extract_wikilinks, wikilinks_to_markdown};
use crate::IoError;

/// A note imported from an Obsidian vault.
#[derive(Debug, Clone)]
pub struct ImportedNote {
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub source_path: PathBuf,
    pub relative_path: String,
    pub wikilinks: Vec<String>,
    pub modified_at: Option<DateTime<Utc>>,
}

/// Import statistics.
#[derive(Debug, Default)]
pub struct ImportStats {
    pub notes_found: usize,
    pub notes_imported: usize,
    pub wikilinks_converted: usize,
    pub errors: Vec<(PathBuf, String)>,
}

/// Import all `.md` files from an Obsidian vault directory.
pub async fn import_obsidian_vault(
    vault_path: &Path,
    convert_wikilinks: bool,
) -> Result<(Vec<ImportedNote>, ImportStats), IoError> {
    if !vault_path.exists() {
        return Err(IoError::SourceNotFound(
            vault_path.display().to_string(),
        ));
    }

    let mut notes = Vec::new();
    let mut stats = ImportStats::default();

    let md_files = collect_markdown_files(vault_path).await?;
    stats.notes_found = md_files.len();

    for file_path in md_files {
        match import_single_note(vault_path, &file_path, convert_wikilinks).await {
            Ok(note) => {
                stats.wikilinks_converted += note.wikilinks.len();
                notes.push(note);
                stats.notes_imported += 1;
            }
            Err(e) => {
                stats
                    .errors
                    .push((file_path.clone(), e.to_string()));
            }
        }
    }

    Ok((notes, stats))
}

async fn import_single_note(
    vault_root: &Path,
    file_path: &Path,
    convert_wikilinks: bool,
) -> Result<ImportedNote, IoError> {
    let raw_content = fs::read_to_string(file_path).await?;

    let relative = file_path
        .strip_prefix(vault_root)
        .unwrap_or(file_path)
        .to_string_lossy()
        .to_string();

    // Parse frontmatter
    let (yaml, body) = split_frontmatter(&raw_content);
    let fm = yaml.map(parse_frontmatter).unwrap_or_default();

    // Title: from frontmatter, or filename
    let title = fm.title.unwrap_or_else(|| {
        file_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".into())
    });

    // Extract wikilinks before conversion
    let wikilinks: Vec<String> = extract_wikilinks(body)
        .into_iter()
        .map(|wl| wl.target)
        .collect();

    // Convert wikilinks to standard Markdown
    let content = if convert_wikilinks {
        wikilinks_to_markdown(body)
    } else {
        body.to_string()
    };

    // Tags from frontmatter
    let mut tags = fm.tags;

    // Also extract Obsidian inline tags (#tag)
    for word in body.split_whitespace() {
        if let Some(tag) = word.strip_prefix('#') {
            let tag = tag.trim_matches(|c: char| !c.is_alphanumeric() && c != '/' && c != '-');
            if !tag.is_empty() && !tags.contains(&tag.to_string()) {
                tags.push(tag.to_string());
            }
        }
    }

    // File modification time
    let modified_at = fs::metadata(file_path)
        .await
        .ok()
        .and_then(|m| m.modified().ok())
        .map(DateTime::<Utc>::from);

    Ok(ImportedNote {
        title,
        content,
        tags,
        source_path: file_path.to_path_buf(),
        relative_path: relative,
        wikilinks,
        modified_at,
    })
}

/// Recursively collect all `.md` files, skipping `.obsidian/` and `.trash/`.
async fn collect_markdown_files(dir: &Path) -> Result<Vec<PathBuf>, IoError> {
    let mut files = Vec::new();
    let mut stack = vec![dir.to_path_buf()];

    while let Some(current) = stack.pop() {
        let mut entries = fs::read_dir(&current).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden dirs and Obsidian internals
            if name.starts_with('.') || name == "node_modules" {
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
    async fn import_simple_vault() {
        let dir = TempDir::new().unwrap();
        write_file(
            dir.path(),
            "note1.md",
            "---\ntitle: Note One\ntags: [test]\n---\nHello world.",
        )
        .await;
        write_file(dir.path(), "note2.md", "No frontmatter here.").await;

        let (notes, stats) = import_obsidian_vault(dir.path(), true).await.unwrap();
        assert_eq!(stats.notes_found, 2);
        assert_eq!(stats.notes_imported, 2);
        assert_eq!(notes[0].title, "Note One");
    }

    #[tokio::test]
    async fn import_wikilinks() {
        let dir = TempDir::new().unwrap();
        write_file(
            dir.path(),
            "main.md",
            "See [[Other Note]] and [[Third|see third]].",
        )
        .await;

        let (notes, stats) = import_obsidian_vault(dir.path(), true).await.unwrap();
        assert_eq!(stats.wikilinks_converted, 2);
        assert!(notes[0].content.contains("[Other Note](other-note.md)"));
        assert!(notes[0].content.contains("[see third](third.md)"));
    }

    #[tokio::test]
    async fn import_inline_tags() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "tagged.md", "Content with #rust and #project/agnos tags.").await;

        let (notes, _) = import_obsidian_vault(dir.path(), true).await.unwrap();
        assert!(notes[0].tags.contains(&"rust".to_string()));
        assert!(notes[0].tags.contains(&"project/agnos".to_string()));
    }

    #[tokio::test]
    async fn import_nested_directories() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "root.md", "Root note.").await;
        write_file(dir.path(), "sub/nested.md", "Nested note.").await;

        let (notes, stats) = import_obsidian_vault(dir.path(), true).await.unwrap();
        assert_eq!(stats.notes_found, 2);
        assert!(notes.iter().any(|n| n.relative_path.contains("sub/")));
    }

    #[tokio::test]
    async fn import_skips_hidden() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "visible.md", "Yes.").await;
        write_file(dir.path(), ".obsidian/config.json", "{}").await;

        let (notes, stats) = import_obsidian_vault(dir.path(), true).await.unwrap();
        assert_eq!(stats.notes_found, 1);
        assert_eq!(notes.len(), 1);
    }

    #[tokio::test]
    async fn import_nonexistent_dir() {
        let result = import_obsidian_vault(Path::new("/nonexistent"), true).await;
        assert!(matches!(result, Err(IoError::SourceNotFound(_))));
    }
}
