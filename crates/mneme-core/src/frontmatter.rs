//! Frontmatter parsing — extract YAML metadata from Markdown notes.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Parsed YAML frontmatter from a Markdown note.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Frontmatter {
    pub title: Option<String>,
    pub tags: Vec<String>,
    /// Arbitrary key-value metadata.
    pub extra: HashMap<String, String>,
}

/// Splits a Markdown document into frontmatter (if present) and body.
///
/// Frontmatter is delimited by `---` at the start of the document:
/// ```text
/// ---
/// title: My Note
/// tags: [rust, agnos]
/// ---
/// Body content here.
/// ```
pub fn split_frontmatter(content: &str) -> (Option<&str>, &str) {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return (None, content);
    }

    // Skip the opening ---
    let after_open = &trimmed[3..];
    let after_open = after_open.strip_prefix('\n').unwrap_or(after_open);

    if let Some(end) = after_open.find("\n---") {
        let yaml = &after_open[..end];
        let body_start = end + 4; // skip \n---
        let body = after_open[body_start..]
            .strip_prefix('\n')
            .unwrap_or(&after_open[body_start..]);
        (Some(yaml), body)
    } else {
        // No closing ---, treat entire content as body
        (None, content)
    }
}

/// Parse frontmatter YAML into a `Frontmatter` struct.
///
/// This is a minimal parser that handles the common cases:
/// - `title: value`
/// - `tags: [a, b, c]` or `tags: a, b, c`
pub fn parse_frontmatter(yaml: &str) -> Frontmatter {
    let mut fm = Frontmatter::default();

    for line in yaml.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "title" => {
                    fm.title = Some(unquote(value).to_string());
                }
                "tags" => {
                    fm.tags = parse_tag_list(value);
                }
                _ => {
                    fm.extra.insert(key.to_string(), unquote(value).to_string());
                }
            }
        }
    }

    fm
}

/// Parse a tag list from either `[a, b, c]` or `a, b, c` format.
fn parse_tag_list(value: &str) -> Vec<String> {
    let value = value.trim();
    let inner = value
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .unwrap_or(value);

    inner
        .split(',')
        .map(|s| unquote(s.trim()).to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Remove surrounding quotes from a string value.
fn unquote(s: &str) -> &str {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        &s[1..s.len() - 1]
    } else {
        s
    }
}

/// Render frontmatter back to YAML string (for writing).
pub fn render_frontmatter(fm: &Frontmatter) -> String {
    let mut lines = Vec::new();

    if let Some(title) = &fm.title {
        lines.push(format!("title: {title}"));
    }

    if !fm.tags.is_empty() {
        let tags = fm.tags.join(", ");
        lines.push(format!("tags: [{tags}]"));
    }

    for (key, value) in &fm.extra {
        lines.push(format!("{key}: {value}"));
    }

    lines.join("\n")
}

/// Compose a full Markdown document with frontmatter and body.
pub fn compose_document(fm: &Frontmatter, body: &str) -> String {
    let yaml = render_frontmatter(fm);
    if yaml.is_empty() {
        body.to_string()
    } else {
        format!("---\n{yaml}\n---\n{body}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_with_frontmatter() {
        let doc = "---\ntitle: Hello\ntags: [a, b]\n---\nBody here.";
        let (yaml, body) = split_frontmatter(doc);
        assert_eq!(yaml, Some("title: Hello\ntags: [a, b]"));
        assert_eq!(body, "Body here.");
    }

    #[test]
    fn split_without_frontmatter() {
        let doc = "Just a regular document.";
        let (yaml, body) = split_frontmatter(doc);
        assert!(yaml.is_none());
        assert_eq!(body, doc);
    }

    #[test]
    fn split_unclosed_frontmatter() {
        let doc = "---\ntitle: Hello\nNo closing delimiter.";
        let (yaml, body) = split_frontmatter(doc);
        assert!(yaml.is_none());
        assert_eq!(body, doc);
    }

    #[test]
    fn parse_basic_frontmatter() {
        let yaml = "title: My Note\ntags: [rust, agnos, core]\nstatus: draft";
        let fm = parse_frontmatter(yaml);
        assert_eq!(fm.title, Some("My Note".into()));
        assert_eq!(fm.tags, vec!["rust", "agnos", "core"]);
        assert_eq!(fm.extra.get("status"), Some(&"draft".into()));
    }

    #[test]
    fn parse_quoted_values() {
        let yaml = "title: \"Hello World\"\ntags: ['a', \"b\"]";
        let fm = parse_frontmatter(yaml);
        assert_eq!(fm.title, Some("Hello World".into()));
        assert_eq!(fm.tags, vec!["a", "b"]);
    }

    #[test]
    fn parse_tags_without_brackets() {
        let yaml = "tags: rust, agnos";
        let fm = parse_frontmatter(yaml);
        assert_eq!(fm.tags, vec!["rust", "agnos"]);
    }

    #[test]
    fn roundtrip_compose() {
        let fm = Frontmatter {
            title: Some("Test Note".into()),
            tags: vec!["a".into(), "b".into()],
            extra: HashMap::new(),
        };
        let doc = compose_document(&fm, "Hello world.\n");
        assert!(doc.starts_with("---\n"));
        assert!(doc.contains("title: Test Note"));
        assert!(doc.contains("tags: [a, b]"));
        assert!(doc.ends_with("Hello world.\n"));
    }
}
