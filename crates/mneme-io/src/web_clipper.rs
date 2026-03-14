//! Web clipper — save web pages as Markdown notes.
//!
//! Converts HTML content to Markdown, extracts metadata,
//! and creates importable note structures.

use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::IoError;

/// A clipped web page ready for import.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClippedPage {
    pub title: String,
    pub url: String,
    pub content_md: String,
    pub excerpt: String,
    pub tags: Vec<String>,
    pub clipped_at: String,
}

/// Options for web clipping.
#[derive(Debug, Clone)]
pub struct ClipOptions {
    pub include_source_link: bool,
    pub include_clip_date: bool,
    pub auto_tag: bool,
    pub max_length: Option<usize>,
}

impl Default for ClipOptions {
    fn default() -> Self {
        Self {
            include_source_link: true,
            include_clip_date: true,
            auto_tag: true,
            max_length: None,
        }
    }
}

/// Clip HTML content into a Markdown note.
pub fn clip_html(html: &str, url: &str, options: &ClipOptions) -> Result<ClippedPage, IoError> {
    if html.trim().is_empty() {
        return Err(IoError::Parse {
            path: url.to_string(),
            reason: "Empty HTML content".into(),
        });
    }

    let title = extract_title(html).unwrap_or_else(|| title_from_url(url));
    let content = html_to_markdown(html);
    let excerpt = extract_excerpt(&content, 200);

    let mut md = String::new();

    if options.include_source_link {
        md.push_str(&format!("> Source: <{url}>\n\n"));
    }
    if options.include_clip_date {
        md.push_str(&format!(
            "> Clipped: {}\n\n",
            Utc::now().format("%Y-%m-%d %H:%M")
        ));
    }

    md.push_str("---\n\n");
    md.push_str(&content);

    if let Some(max) = options.max_length
        && md.len() > max
    {
        md.truncate(max);
        md.push_str("\n\n[...truncated]");
    }

    let tags = if options.auto_tag {
        vec!["clipped".to_string()]
    } else {
        vec![]
    };

    Ok(ClippedPage {
        title,
        url: url.to_string(),
        content_md: md,
        excerpt,
        tags,
        clipped_at: Utc::now().format("%Y-%m-%d %H:%M").to_string(),
    })
}

/// Clip a plain text bookmark (URL + optional description).
pub fn clip_bookmark(url: &str, title: Option<&str>, description: Option<&str>) -> ClippedPage {
    let title = title.unwrap_or(url).to_string();
    let mut content = format!("# {title}\n\n");
    content.push_str(&format!("URL: <{url}>\n\n"));

    if let Some(desc) = description {
        content.push_str(&format!("{desc}\n\n"));
    }

    content.push_str(&format!("Saved: {}\n", Utc::now().format("%Y-%m-%d")));

    ClippedPage {
        title,
        url: url.to_string(),
        content_md: content,
        excerpt: description.unwrap_or("").to_string(),
        tags: vec!["bookmark".to_string()],
        clipped_at: Utc::now().format("%Y-%m-%d %H:%M").to_string(),
    }
}

/// Extract page title from HTML.
fn extract_title(html: &str) -> Option<String> {
    let re = Regex::new(r"(?i)<title[^>]*>([^<]+)</title>").unwrap();
    re.captures(html)
        .map(|caps| html_decode(&caps[1]).trim().to_string())
        .filter(|t| !t.is_empty())
}

/// Generate a title from a URL.
fn title_from_url(url: &str) -> String {
    // Extract domain + path
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);

    let path = without_scheme
        .split('/')
        .rfind(|s| !s.is_empty() && (!s.contains('.') || s.len() > 10))
        .unwrap_or(without_scheme);

    path.replace(['-', '_'], " ")
        .split('.')
        .next()
        .unwrap_or(path)
        .to_string()
}

/// Basic HTML to Markdown conversion.
fn html_to_markdown(html: &str) -> String {
    let mut text = html.to_string();

    // Remove script and style tags
    let script_re = Regex::new(r"(?is)<script[^>]*>.*?</script>").unwrap();
    text = script_re.replace_all(&text, "").to_string();

    let style_re = Regex::new(r"(?is)<style[^>]*>.*?</style>").unwrap();
    text = style_re.replace_all(&text, "").to_string();

    // Convert headings
    for i in (1..=6).rev() {
        let heading_re = Regex::new(&format!(r"(?i)<h{i}[^>]*>(.*?)</h{i}>")).unwrap();
        let prefix = "#".repeat(i);
        text = heading_re
            .replace_all(&text, format!("\n{prefix} $1\n"))
            .to_string();
    }

    // Convert paragraphs
    let p_re = Regex::new(r"(?is)<p[^>]*>(.*?)</p>").unwrap();
    text = p_re.replace_all(&text, "\n$1\n").to_string();

    // Convert links
    let link_re = Regex::new(r#"(?i)<a[^>]*href="([^"]*)"[^>]*>(.*?)</a>"#).unwrap();
    text = link_re.replace_all(&text, "[$2]($1)").to_string();

    // Convert bold/strong
    let bold_re = Regex::new(r"(?i)<(?:strong|b)>(.*?)</(?:strong|b)>").unwrap();
    text = bold_re.replace_all(&text, "**$1**").to_string();

    // Convert italic/em
    let italic_re = Regex::new(r"(?i)<(?:em|i)>(.*?)</(?:em|i)>").unwrap();
    text = italic_re.replace_all(&text, "*$1*").to_string();

    // Convert code
    let code_re = Regex::new(r"(?i)<code>(.*?)</code>").unwrap();
    text = code_re.replace_all(&text, "`$1`").to_string();

    // Convert line breaks
    let br_re = Regex::new(r"(?i)<br\s*/?>").unwrap();
    text = br_re.replace_all(&text, "\n").to_string();

    // Convert list items
    let li_re = Regex::new(r"(?i)<li[^>]*>(.*?)</li>").unwrap();
    text = li_re.replace_all(&text, "- $1").to_string();

    // Remove remaining HTML tags
    let tag_re = Regex::new(r"<[^>]+>").unwrap();
    text = tag_re.replace_all(&text, "").to_string();

    // Decode HTML entities
    text = html_decode(&text);

    // Clean up whitespace
    let multi_newline = Regex::new(r"\n{3,}").unwrap();
    text = multi_newline.replace_all(&text, "\n\n").to_string();

    text.trim().to_string()
}

/// Decode common HTML entities.
fn html_decode(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&#x27;", "'")
        .replace("&mdash;", "\u{2014}")
        .replace("&ndash;", "\u{2013}")
        .replace("&hellip;", "...")
}

/// Extract an excerpt from content.
fn extract_excerpt(content: &str, max_len: usize) -> String {
    let plain: String = content
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('#') && !l.starts_with('>'))
        .take(3)
        .collect::<Vec<_>>()
        .join(" ");

    if plain.len() <= max_len {
        plain
    } else {
        format!("{}...", &plain[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_basic_html() {
        let html = "<html><head><title>Test Page</title></head><body><h1>Hello</h1><p>World</p></body></html>";
        let result =
            clip_html(html, "https://example.com/test", &ClipOptions::default()).unwrap();
        assert_eq!(result.title, "Test Page");
        assert!(result.content_md.contains("Hello"));
        assert!(result.content_md.contains("World"));
        assert!(result.tags.contains(&"clipped".to_string()));
    }

    #[test]
    fn clip_extracts_links() {
        let html = r#"<p>Visit <a href="https://rust-lang.org">Rust</a> for more.</p>"#;
        let result = clip_html(html, "https://example.com", &ClipOptions::default()).unwrap();
        assert!(result.content_md.contains("[Rust](https://rust-lang.org)"));
    }

    #[test]
    fn clip_converts_formatting() {
        let html = "<p><strong>bold</strong> and <em>italic</em> and <code>code</code></p>";
        let result = clip_html(html, "https://example.com", &ClipOptions::default()).unwrap();
        assert!(result.content_md.contains("**bold**"));
        assert!(result.content_md.contains("*italic*"));
        assert!(result.content_md.contains("`code`"));
    }

    #[test]
    fn clip_removes_scripts() {
        let html = "<p>Content</p><script>alert('xss')</script><p>More</p>";
        let result = clip_html(html, "https://example.com", &ClipOptions::default()).unwrap();
        assert!(!result.content_md.contains("alert"));
        assert!(result.content_md.contains("Content"));
    }

    #[test]
    fn clip_empty_html_error() {
        let result = clip_html("", "https://example.com", &ClipOptions::default());
        assert!(result.is_err());
    }

    #[test]
    fn bookmark_basic() {
        let clip = clip_bookmark("https://example.com", Some("Example"), Some("A test site"));
        assert_eq!(clip.title, "Example");
        assert!(clip.content_md.contains("example.com"));
        assert!(clip.tags.contains(&"bookmark".to_string()));
    }

    #[test]
    fn title_from_url_extraction() {
        assert_eq!(
            title_from_url("https://example.com/my-article"),
            "my article"
        );
        assert_eq!(
            title_from_url("https://example.com/blog/rust_tips"),
            "rust tips"
        );
    }

    #[test]
    fn html_entity_decoding() {
        assert_eq!(html_decode("&amp; &lt; &gt;"), "& < >");
        assert_eq!(
            html_decode("&mdash; &hellip;"),
            "\u{2014} ..."
        );
    }

    #[test]
    fn excerpt_truncation() {
        let content = "This is a very long piece of content that should be truncated after a certain number of characters to create a short excerpt.";
        let excerpt = extract_excerpt(content, 30);
        assert!(excerpt.len() <= 35); // 30 + "..."
        assert!(excerpt.ends_with("..."));
    }

    #[test]
    fn max_length_truncation() {
        let html = "<p>A</p>".repeat(100);
        let options = ClipOptions {
            max_length: Some(100),
            ..ClipOptions::default()
        };
        let result = clip_html(&html, "https://example.com", &options).unwrap();
        assert!(result.content_md.contains("[...truncated]"));
    }
}
