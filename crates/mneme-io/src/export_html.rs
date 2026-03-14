//! HTML export — generate a static site from notes.
//!
//! Produces self-contained HTML files with embedded CSS,
//! navigation, and inter-note links.

use std::path::Path;

use comrak::{markdown_to_html, Options};
use tokio::fs;

use crate::IoError;

/// Options for HTML export.
#[derive(Debug, Clone)]
pub struct ExportOptions {
    pub site_title: String,
    pub include_tags: bool,
    pub include_backlinks: bool,
}

impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            site_title: "Mneme".into(),
            include_tags: true,
            include_backlinks: true,
        }
    }
}

/// A note prepared for HTML export.
pub struct ExportNote {
    pub title: String,
    pub slug: String,
    pub content_md: String,
    pub tags: Vec<String>,
    pub backlink_titles: Vec<String>,
}

/// Export a collection of notes to HTML files in the output directory.
pub async fn export_to_html(
    notes: &[ExportNote],
    output_dir: &Path,
    options: &ExportOptions,
) -> Result<usize, IoError> {
    fs::create_dir_all(output_dir).await?;

    // Generate index page
    let index_html = render_index(notes, options);
    fs::write(output_dir.join("index.html"), index_html).await?;

    // Generate each note page
    for note in notes {
        let html = render_note(note, options);
        fs::write(output_dir.join(format!("{}.html", note.slug)), html).await?;
    }

    // Write CSS
    fs::write(output_dir.join("style.css"), CSS).await?;

    Ok(notes.len() + 1) // notes + index
}

fn render_index(notes: &[ExportNote], options: &ExportOptions) -> String {
    let mut links = String::new();
    for note in notes {
        links.push_str(&format!(
            "    <li><a href=\"{slug}.html\">{title}</a>",
            slug = note.slug,
            title = html_escape(&note.title),
        ));
        if options.include_tags && !note.tags.is_empty() {
            let tags: Vec<String> = note
                .tags
                .iter()
                .map(|t| format!("<span class=\"tag\">#{t}</span>"))
                .collect();
            links.push_str(&format!(" {}", tags.join(" ")));
        }
        links.push_str("</li>\n");
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <link rel="stylesheet" href="style.css">
</head>
<body>
  <header><h1>{title}</h1></header>
  <main>
    <h2>Notes ({count})</h2>
    <ul class="note-list">
{links}    </ul>
  </main>
</body>
</html>"#,
        title = html_escape(&options.site_title),
        count = notes.len(),
        links = links,
    )
}

fn render_note(note: &ExportNote, options: &ExportOptions) -> String {
    let mut comrak_opts = Options::default();
    comrak_opts.extension.table = true;
    comrak_opts.extension.strikethrough = true;
    comrak_opts.extension.tasklist = true;
    comrak_opts.extension.autolink = true;
    let body_html = markdown_to_html(&note.content_md, &comrak_opts);

    let mut sidebar = String::new();

    if options.include_tags && !note.tags.is_empty() {
        sidebar.push_str("<div class=\"sidebar-section\"><h3>Tags</h3><ul>");
        for tag in &note.tags {
            sidebar.push_str(&format!("<li class=\"tag\">#{tag}</li>"));
        }
        sidebar.push_str("</ul></div>");
    }

    if options.include_backlinks && !note.backlink_titles.is_empty() {
        sidebar.push_str("<div class=\"sidebar-section\"><h3>Backlinks</h3><ul>");
        for bl in &note.backlink_titles {
            sidebar.push_str(&format!("<li>← {}</li>", html_escape(bl)));
        }
        sidebar.push_str("</ul></div>");
    }

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <link rel="stylesheet" href="style.css">
</head>
<body>
  <header>
    <a href="index.html">← Back</a>
    <h1>{title}</h1>
  </header>
  <div class="layout">
    <main>{body}</main>
    <aside>{sidebar}</aside>
  </div>
</body>
</html>"#,
        title = html_escape(&note.title),
        body = body_html,
        sidebar = sidebar,
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

const CSS: &str = r#"
:root { --bg: #1a1a2e; --fg: #e0e0e0; --accent: #0abab5; --muted: #888; }
* { box-sizing: border-box; margin: 0; padding: 0; }
body { font-family: system-ui, sans-serif; background: var(--bg); color: var(--fg); line-height: 1.6; max-width: 960px; margin: 0 auto; padding: 2rem; }
header { margin-bottom: 2rem; }
header a { color: var(--accent); text-decoration: none; }
h1, h2, h3 { color: var(--accent); margin-bottom: 0.5rem; }
a { color: var(--accent); }
.layout { display: grid; grid-template-columns: 3fr 1fr; gap: 2rem; }
main { min-width: 0; }
aside { font-size: 0.9rem; color: var(--muted); }
.sidebar-section { margin-bottom: 1.5rem; }
.sidebar-section ul { list-style: none; padding-left: 0; }
.sidebar-section li { padding: 0.2rem 0; }
.note-list { list-style: none; padding-left: 0; }
.note-list li { padding: 0.4rem 0; border-bottom: 1px solid #333; }
.tag { color: var(--accent); font-size: 0.85rem; margin-left: 0.5rem; }
pre { background: #111; padding: 1rem; border-radius: 4px; overflow-x: auto; }
code { font-family: monospace; background: #111; padding: 0.1rem 0.3rem; border-radius: 2px; }
table { border-collapse: collapse; width: 100%; margin: 1rem 0; }
th, td { border: 1px solid #333; padding: 0.5rem; text-align: left; }
th { background: #222; }
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn export_basic_site() {
        let dir = TempDir::new().unwrap();
        let notes = vec![
            ExportNote {
                title: "First Note".into(),
                slug: "first-note".into(),
                content_md: "# Hello\n\nThis is **bold**.".into(),
                tags: vec!["test".into()],
                backlink_titles: vec![],
            },
            ExportNote {
                title: "Second Note".into(),
                slug: "second-note".into(),
                content_md: "Another note with [a link](first-note.html).".into(),
                tags: vec![],
                backlink_titles: vec!["First Note".into()],
            },
        ];

        let count = export_to_html(&notes, dir.path(), &ExportOptions::default())
            .await
            .unwrap();
        assert_eq!(count, 3); // 2 notes + index

        // Check files exist
        assert!(dir.path().join("index.html").exists());
        assert!(dir.path().join("first-note.html").exists());
        assert!(dir.path().join("second-note.html").exists());
        assert!(dir.path().join("style.css").exists());

        // Check index contains links
        let index = fs::read_to_string(dir.path().join("index.html"))
            .await
            .unwrap();
        assert!(index.contains("first-note.html"));
        assert!(index.contains("First Note"));
    }

    #[tokio::test]
    async fn export_renders_markdown() {
        let dir = TempDir::new().unwrap();
        let notes = vec![ExportNote {
            title: "MD Test".into(),
            slug: "md-test".into(),
            content_md: "**bold** and *italic*".into(),
            tags: vec![],
            backlink_titles: vec![],
        }];

        export_to_html(&notes, dir.path(), &ExportOptions::default())
            .await
            .unwrap();

        let html = fs::read_to_string(dir.path().join("md-test.html"))
            .await
            .unwrap();
        assert!(html.contains("<strong>bold</strong>"));
        assert!(html.contains("<em>italic</em>"));
    }

    #[test]
    fn html_escape_special_chars() {
        assert_eq!(html_escape("<script>alert('xss')"), "&lt;script&gt;alert('xss')");
        assert_eq!(html_escape("A & B"), "A &amp; B");
    }
}
