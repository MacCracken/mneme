//! Wikilink parser — converts `[[Target]]` and `[[Target|Display]]` to Markdown links.

use regex::Regex;
use std::sync::LazyLock;

static WIKILINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\[([^\]|]+?)(?:\|([^\]]+?))?\]\]").unwrap());

/// A parsed wikilink.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wikilink {
    /// The target note (e.g. "My Note" from `[[My Note]]`).
    pub target: String,
    /// Optional display text (e.g. "click here" from `[[My Note|click here]]`).
    pub display: Option<String>,
}

/// Extract all wikilinks from content.
pub fn extract_wikilinks(content: &str) -> Vec<Wikilink> {
    WIKILINK_RE
        .captures_iter(content)
        .map(|cap| Wikilink {
            target: cap[1].trim().to_string(),
            display: cap.get(2).map(|m| m.as_str().trim().to_string()),
        })
        .collect()
}

/// Convert wikilinks to standard Markdown links.
///
/// `[[Target]]` → `[Target](target.md)`
/// `[[Target|Display]]` → `[Display](target.md)`
pub fn wikilinks_to_markdown(content: &str) -> String {
    WIKILINK_RE
        .replace_all(content, |caps: &regex::Captures| {
            let target = caps[1].trim();
            let slug = title_to_slug(target);
            let display = caps
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_else(|| target.to_string());
            format!("[{display}]({slug}.md)")
        })
        .to_string()
}

/// Convert a note title to a slug for file paths.
fn title_to_slug(title: &str) -> String {
    title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_simple_wikilink() {
        let links = extract_wikilinks("See [[My Note]] for details.");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "My Note");
        assert_eq!(links[0].display, None);
    }

    #[test]
    fn extract_aliased_wikilink() {
        let links = extract_wikilinks("Check [[My Note|this page]].");
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].target, "My Note");
        assert_eq!(links[0].display, Some("this page".into()));
    }

    #[test]
    fn extract_multiple_wikilinks() {
        let links = extract_wikilinks("Link to [[A]] and [[B|beta]] and [[C]].");
        assert_eq!(links.len(), 3);
    }

    #[test]
    fn extract_no_wikilinks() {
        let links = extract_wikilinks("No links here.");
        assert!(links.is_empty());
    }

    #[test]
    fn convert_simple() {
        let result = wikilinks_to_markdown("See [[My Note]] for details.");
        assert_eq!(result, "See [My Note](my-note.md) for details.");
    }

    #[test]
    fn convert_aliased() {
        let result = wikilinks_to_markdown("Check [[My Note|this page]].");
        assert_eq!(result, "Check [this page](my-note.md).");
    }

    #[test]
    fn convert_multiple() {
        let result = wikilinks_to_markdown("[[A]] and [[B|beta]]");
        assert_eq!(result, "[A](a.md) and [beta](b.md)");
    }

    #[test]
    fn slug_generation() {
        assert_eq!(title_to_slug("Hello World"), "hello-world");
        assert_eq!(title_to_slug("What's New? (2026)"), "what-s-new-2026");
    }
}
