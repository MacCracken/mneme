//! Notion export importer.
//!
//! Imports from a Notion export directory (extracted zip):
//! - Cleans UUID-suffixed filenames (e.g., "My Page abc123.md" -> "My Page")
//! - Converts Notion-style links to standard Markdown
//! - Extracts properties from Notion's frontmatter
//! - Handles nested page directories

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use regex::Regex;
use tokio::fs;

use mneme_core::frontmatter::{parse_frontmatter, split_frontmatter};

use crate::IoError;
use crate::import_obsidian::ImportedNote;

/// Import statistics for Notion.
#[derive(Debug, Default)]
pub struct NotionImportStats {
    pub pages_found: usize,
    pub pages_imported: usize,
    pub csv_databases_found: usize,
    pub uuid_suffixes_cleaned: usize,
    pub errors: Vec<(PathBuf, String)>,
}

/// Import all pages from a Notion export directory.
pub async fn import_notion_export(
    export_path: &Path,
) -> Result<(Vec<ImportedNote>, NotionImportStats), IoError> {
    if !export_path.exists() {
        return Err(IoError::SourceNotFound(export_path.display().to_string()));
    }

    let mut notes = Vec::new();
    let mut stats = NotionImportStats::default();

    let files = collect_notion_files(export_path).await?;
    stats.pages_found = files
        .iter()
        .filter(|f| f.extension().is_some_and(|e| e == "md"))
        .count();
    stats.csv_databases_found = files
        .iter()
        .filter(|f| f.extension().is_some_and(|e| e == "csv"))
        .count();

    for file_path in &files {
        if file_path.extension().is_some_and(|ext| ext == "md") {
            match import_notion_page(export_path, file_path).await {
                Ok((note, cleaned)) => {
                    if cleaned {
                        stats.uuid_suffixes_cleaned += 1;
                    }
                    notes.push(note);
                    stats.pages_imported += 1;
                }
                Err(e) => {
                    stats.errors.push((file_path.clone(), e.to_string()));
                }
            }
        }
    }

    Ok((notes, stats))
}

async fn import_notion_page(
    export_root: &Path,
    file_path: &Path,
) -> Result<(ImportedNote, bool), IoError> {
    let raw = fs::read_to_string(file_path).await?;
    let relative = file_path
        .strip_prefix(export_root)
        .unwrap_or(file_path)
        .to_string_lossy()
        .to_string();

    let (yaml, body) = split_frontmatter(&raw);
    let fm = yaml.map(parse_frontmatter).unwrap_or_default();

    // Clean Notion's UUID-suffixed filename for title
    let raw_title = fm.title.unwrap_or_else(|| {
        file_path
            .file_stem()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "Untitled".into())
    });
    let (title, was_cleaned) = clean_notion_title(&raw_title);

    // Clean content: convert Notion-style links, clean up formatting
    let content = clean_notion_content(body);

    let tags = fm.tags;

    let modified_at = fs::metadata(file_path)
        .await
        .ok()
        .and_then(|m| m.modified().ok())
        .map(DateTime::<Utc>::from);

    Ok((
        ImportedNote {
            title,
            content,
            tags,
            source_path: file_path.to_path_buf(),
            relative_path: clean_notion_path(&relative),
            wikilinks: vec![],
            modified_at,
        },
        was_cleaned,
    ))
}

/// Remove Notion's UUID suffix from titles.
/// "My Page abc123def4567890abcdef1234567890" -> "My Page"
fn clean_notion_title(title: &str) -> (String, bool) {
    let re = Regex::new(r"\s+[a-f0-9]{32}$").unwrap();
    if re.is_match(title) {
        (re.replace(title, "").to_string(), true)
    } else {
        (title.to_string(), false)
    }
}

/// Clean Notion-specific content patterns.
fn clean_notion_content(content: &str) -> String {
    // First decode %20 so link regex can match space-separated UUIDs
    let mut result = content.replace("%20", " ");

    // Convert Notion's internal links: [Title](Title uuid.md) -> [Title](slug.md)
    let link_re = Regex::new(r"\[([^\]]+)\]\(([^)]+)\s+[a-f0-9]{32}\.md\)").unwrap();
    result = link_re
        .replace_all(&result, |caps: &regex::Captures| {
            let text = &caps[1];
            let slug = slug_path(text);
            format!("[{text}]({slug}.md)")
        })
        .to_string();

    // Remove Notion's empty toggle blocks
    let toggle_re = Regex::new(r"<details>\s*<summary></summary>\s*</details>\n?").unwrap();
    result = toggle_re.replace_all(&result, "").to_string();

    result
}

/// Clean UUID suffixes from path components.
fn clean_notion_path(path: &str) -> String {
    let re = Regex::new(r"\s+[a-f0-9]{32}").unwrap();
    re.replace_all(path, "").to_string()
}

fn slug_path(title: &str) -> String {
    title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '/' {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

async fn collect_notion_files(dir: &Path) -> Result<Vec<PathBuf>, IoError> {
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
            } else {
                let ext = path.extension().and_then(|e| e.to_str());
                if matches!(ext, Some("md" | "csv")) {
                    files.push(path);
                }
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

    #[test]
    fn clean_uuid_suffix() {
        let (title, cleaned) = clean_notion_title("My Page abc123def4567890abcdef1234567890");
        assert_eq!(title, "My Page");
        assert!(cleaned);
    }

    #[test]
    fn clean_no_suffix() {
        let (title, cleaned) = clean_notion_title("Normal Title");
        assert_eq!(title, "Normal Title");
        assert!(!cleaned);
    }

    #[test]
    fn clean_notion_links() {
        let content =
            "See [Other Page](Other%20Page%20abc123def4567890abcdef1234567890.md) for details.";
        let cleaned = clean_notion_content(content);
        assert!(cleaned.contains("[Other Page](other-page.md)"));
    }

    #[test]
    fn path_cleaning() {
        let path =
            "Projects abc123def4567890abcdef1234567890/Sub abc123def4567890abcdef1234567890.md";
        let cleaned = clean_notion_path(path);
        assert_eq!(cleaned, "Projects/Sub.md");
    }

    #[tokio::test]
    async fn import_notion_directory() {
        let dir = TempDir::new().unwrap();
        write_file(
            dir.path(),
            "My Note abc123def4567890abcdef1234567890.md",
            "# My Note\n\nSome content here.",
        )
        .await;
        write_file(
            dir.path(),
            "Another abc123def4567890abcdef1234567890.md",
            "---\ntitle: Another Page\ntags: [notion]\n---\nMore content.",
        )
        .await;

        let (notes, stats) = import_notion_export(dir.path()).await.unwrap();
        assert_eq!(stats.pages_found, 2);
        assert_eq!(stats.pages_imported, 2);
        assert!(notes.iter().any(|n| n.title == "My Note"));
    }

    #[tokio::test]
    async fn import_notion_nested() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "root.md", "Root page.").await;
        write_file(dir.path(), "subdir/child.md", "Child page.").await;

        let (notes, stats) = import_notion_export(dir.path()).await.unwrap();
        assert_eq!(stats.pages_found, 2);
        assert_eq!(notes.len(), 2);
    }

    #[tokio::test]
    async fn import_notion_nonexistent() {
        let result = import_notion_export(Path::new("/nonexistent")).await;
        assert!(matches!(result, Err(IoError::SourceNotFound(_))));
    }

    #[tokio::test]
    async fn import_counts_csv_databases() {
        let dir = TempDir::new().unwrap();
        write_file(dir.path(), "page.md", "Content.").await;
        write_file(dir.path(), "database.csv", "col1,col2\na,b").await;

        let (_, stats) = import_notion_export(dir.path()).await.unwrap();
        assert_eq!(stats.csv_databases_found, 1);
        assert_eq!(stats.pages_found, 1);
    }
}
