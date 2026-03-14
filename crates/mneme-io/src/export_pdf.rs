//! PDF export — generate PDF files from notes.
//!
//! Uses printpdf with standard PDF fonts for lightweight,
//! dependency-free PDF generation.

use std::path::Path;

use printpdf::*;
use tokio::fs;

use crate::IoError;

/// Options for PDF export.
#[derive(Debug, Clone)]
pub struct PdfExportOptions {
    pub page_width_mm: f32,
    pub page_height_mm: f32,
    pub margin_mm: f32,
    pub title_font_size: f32,
    pub body_font_size: f32,
    pub code_font_size: f32,
    pub line_spacing: f32,
}

impl Default for PdfExportOptions {
    fn default() -> Self {
        Self {
            page_width_mm: 210.0, // A4
            page_height_mm: 297.0,
            margin_mm: 25.0,
            title_font_size: 18.0,
            body_font_size: 11.0,
            code_font_size: 10.0,
            line_spacing: 1.4,
        }
    }
}

/// A note to export as PDF.
pub struct PdfNote {
    pub title: String,
    pub content_md: String,
    pub tags: Vec<String>,
}

/// Export a single note to a PDF file.
pub async fn export_note_to_pdf(
    note: &PdfNote,
    output_path: &Path,
    options: &PdfExportOptions,
) -> Result<(), IoError> {
    let pdf_bytes = render_pdf(note, options)?;
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).await?;
    }
    fs::write(output_path, pdf_bytes).await?;
    Ok(())
}

/// Export multiple notes to individual PDF files in a directory.
pub async fn export_notes_to_pdf(
    notes: &[PdfNote],
    output_dir: &Path,
    options: &PdfExportOptions,
) -> Result<usize, IoError> {
    fs::create_dir_all(output_dir).await?;
    let mut count = 0;
    for note in notes {
        let slug = slug(&note.title);
        let path = output_dir.join(format!("{slug}.pdf"));
        export_note_to_pdf(note, &path, options).await?;
        count += 1;
    }
    Ok(count)
}

fn render_pdf(note: &PdfNote, options: &PdfExportOptions) -> Result<Vec<u8>, IoError> {
    let (doc, page1, layer1) = PdfDocument::new(
        &note.title,
        Mm(options.page_width_mm),
        Mm(options.page_height_mm),
        "Layer 1",
    );

    let font_helvetica = doc.add_builtin_font(BuiltinFont::Helvetica).unwrap();
    let font_helvetica_bold = doc
        .add_builtin_font(BuiltinFont::HelveticaBold)
        .unwrap();
    let font_courier = doc.add_builtin_font(BuiltinFont::Courier).unwrap();

    let usable_width = options.page_width_mm - 2.0 * options.margin_mm;
    let chars_per_line = (usable_width / (options.body_font_size * 0.22)) as usize;

    let blocks = parse_markdown_blocks(&note.content_md);

    let mut current_layer = doc.get_page(page1).get_layer(layer1);
    let mut y = options.page_height_mm - options.margin_mm;
    let x = options.margin_mm;
    let line_height = options.body_font_size * options.line_spacing * 0.352778; // pt to mm

    // Title
    current_layer.use_text(
        &note.title,
        options.title_font_size,
        Mm(x),
        Mm(y),
        &font_helvetica_bold,
    );
    y -= options.title_font_size * 0.352778 * 2.0;

    // Tags line
    if !note.tags.is_empty() {
        let tag_line = note
            .tags
            .iter()
            .map(|t| format!("#{t}"))
            .collect::<Vec<_>>()
            .join("  ");
        current_layer.use_text(
            &tag_line,
            options.body_font_size - 1.0,
            Mm(x),
            Mm(y),
            &font_helvetica,
        );
        y -= line_height * 1.5;
    }

    // Separator
    y -= line_height * 0.5;

    // Content blocks
    for block in &blocks {
        // Check page break
        if y < options.margin_mm + line_height * 2.0 {
            let (new_page, new_layer) = doc.add_page(
                Mm(options.page_width_mm),
                Mm(options.page_height_mm),
                "Layer 1",
            );
            current_layer = doc.get_page(new_page).get_layer(new_layer);
            y = options.page_height_mm - options.margin_mm;
        }

        match block {
            MdBlock::Heading(level, text) => {
                let size = match level {
                    1 => options.title_font_size,
                    2 => options.title_font_size - 3.0,
                    _ => options.title_font_size - 5.0,
                };
                y -= line_height * 0.5; // space before heading
                current_layer.use_text(text, size, Mm(x), Mm(y), &font_helvetica_bold);
                y -= size * 0.352778 * 1.5;
            }
            MdBlock::Paragraph(text) => {
                let lines = wrap_text(text, chars_per_line);
                for line in &lines {
                    if y < options.margin_mm + line_height {
                        let (new_page, new_layer) = doc.add_page(
                            Mm(options.page_width_mm),
                            Mm(options.page_height_mm),
                            "Layer 1",
                        );
                        current_layer =
                            doc.get_page(new_page).get_layer(new_layer);
                        y = options.page_height_mm - options.margin_mm;
                    }
                    current_layer.use_text(
                        line,
                        options.body_font_size,
                        Mm(x),
                        Mm(y),
                        &font_helvetica,
                    );
                    y -= line_height;
                }
                y -= line_height * 0.3; // paragraph spacing
            }
            MdBlock::Code(text) => {
                let code_x = x + 5.0;
                let lines = text.lines();
                for line in lines {
                    if y < options.margin_mm + line_height {
                        let (new_page, new_layer) = doc.add_page(
                            Mm(options.page_width_mm),
                            Mm(options.page_height_mm),
                            "Layer 1",
                        );
                        current_layer =
                            doc.get_page(new_page).get_layer(new_layer);
                        y = options.page_height_mm - options.margin_mm;
                    }
                    current_layer.use_text(
                        line,
                        options.code_font_size,
                        Mm(code_x),
                        Mm(y),
                        &font_courier,
                    );
                    y -= line_height;
                }
                y -= line_height * 0.3;
            }
            MdBlock::ListItem(text) => {
                let bullet_text = format!("  * {text}");
                let lines = wrap_text(&bullet_text, chars_per_line);
                for line in &lines {
                    if y < options.margin_mm + line_height {
                        let (new_page, new_layer) = doc.add_page(
                            Mm(options.page_width_mm),
                            Mm(options.page_height_mm),
                            "Layer 1",
                        );
                        current_layer =
                            doc.get_page(new_page).get_layer(new_layer);
                        y = options.page_height_mm - options.margin_mm;
                    }
                    current_layer.use_text(
                        line,
                        options.body_font_size,
                        Mm(x),
                        Mm(y),
                        &font_helvetica,
                    );
                    y -= line_height;
                }
            }
            MdBlock::Empty => {
                y -= line_height * 0.5;
            }
        }
    }

    let bytes = doc.save_to_bytes().map_err(|e| IoError::Parse {
        path: "pdf".into(),
        reason: e.to_string(),
    })?;
    Ok(bytes)
}

#[derive(Debug)]
enum MdBlock {
    Heading(u8, String),
    Paragraph(String),
    Code(String),
    ListItem(String),
    Empty,
}

/// Parse markdown into blocks for PDF rendering.
fn parse_markdown_blocks(md: &str) -> Vec<MdBlock> {
    let mut blocks = Vec::new();
    let mut in_code = false;
    let mut code_buf = String::new();

    for line in md.lines() {
        if line.starts_with("```") {
            if in_code {
                blocks.push(MdBlock::Code(code_buf.clone()));
                code_buf.clear();
                in_code = false;
            } else {
                in_code = true;
            }
            continue;
        }

        if in_code {
            code_buf.push_str(line);
            code_buf.push('\n');
            continue;
        }

        let trimmed = line.trim();

        if trimmed.is_empty() {
            blocks.push(MdBlock::Empty);
        } else if let Some(rest) = trimmed.strip_prefix("### ") {
            blocks.push(MdBlock::Heading(3, strip_md_inline(rest)));
        } else if let Some(rest) = trimmed.strip_prefix("## ") {
            blocks.push(MdBlock::Heading(2, strip_md_inline(rest)));
        } else if let Some(rest) = trimmed.strip_prefix("# ") {
            blocks.push(MdBlock::Heading(1, strip_md_inline(rest)));
        } else if let Some(rest) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
        {
            blocks.push(MdBlock::ListItem(strip_md_inline(rest)));
        } else {
            blocks.push(MdBlock::Paragraph(strip_md_inline(trimmed)));
        }
    }

    blocks
}

/// Strip inline markdown formatting for plain text rendering.
fn strip_md_inline(text: &str) -> String {
    let text = text.replace("**", "").replace("__", "");
    let text = text.replace(['*', '_'], "");
    let text = text.replace('`', "");
    // Convert [text](url) -> text
    let link_re = regex::Regex::new(r"\[([^\]]+)\]\([^)]+\)").unwrap();
    link_re.replace_all(&text, "$1").to_string()
}

fn wrap_text(text: &str, max_width: usize) -> Vec<String> {
    if text.len() <= max_width {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();

    for word in text.split_whitespace() {
        if current.is_empty() {
            current = word.to_string();
        } else if current.len() + 1 + word.len() > max_width {
            lines.push(current);
            current = word.to_string();
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }

    lines
}

fn slug(s: &str) -> String {
    s.chars()
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
    use tempfile::TempDir;

    #[test]
    fn parse_heading_blocks() {
        let blocks =
            parse_markdown_blocks("# Title\n\nParagraph text.\n\n## Section");
        assert!(matches!(blocks[0], MdBlock::Heading(1, _)));
        assert!(matches!(blocks[2], MdBlock::Paragraph(_)));
        assert!(matches!(blocks[4], MdBlock::Heading(2, _)));
    }

    #[test]
    fn parse_code_block() {
        let md = "text\n```\ncode line\n```\nmore text";
        let blocks = parse_markdown_blocks(md);
        assert!(matches!(blocks[1], MdBlock::Code(_)));
    }

    #[test]
    fn parse_list_items() {
        let blocks =
            parse_markdown_blocks("- item one\n- item two\n* item three");
        assert!(matches!(blocks[0], MdBlock::ListItem(_)));
        assert!(matches!(blocks[1], MdBlock::ListItem(_)));
        assert!(matches!(blocks[2], MdBlock::ListItem(_)));
    }

    #[test]
    fn strip_inline_markdown() {
        assert_eq!(
            strip_md_inline("**bold** and *italic*"),
            "bold and italic"
        );
        assert_eq!(strip_md_inline("`code`"), "code");
        assert_eq!(
            strip_md_inline("[link](http://example.com)"),
            "link"
        );
    }

    #[test]
    fn word_wrap() {
        let lines =
            wrap_text("hello world this is a test of word wrapping", 20);
        assert!(lines.len() > 1);
        for line in &lines {
            assert!(line.len() <= 25); // allow some slack for whole words
        }
    }

    #[tokio::test]
    async fn export_single_pdf() {
        let dir = TempDir::new().unwrap();
        let note = PdfNote {
            title: "Test Note".into(),
            content_md: "# Hello\n\nThis is a **test** note.\n\n## Section\n\n- item 1\n- item 2\n\n```\ncode block\n```".into(),
            tags: vec!["test".into(), "pdf".into()],
        };

        let path = dir.path().join("test.pdf");
        export_note_to_pdf(&note, &path, &PdfExportOptions::default())
            .await
            .unwrap();
        assert!(path.exists());

        let bytes = fs::read(&path).await.unwrap();
        assert!(bytes.len() > 100); // has actual content
        assert!(bytes.starts_with(b"%PDF")); // valid PDF header
    }

    #[tokio::test]
    async fn export_multiple_pdfs() {
        let dir = TempDir::new().unwrap();
        let notes = vec![
            PdfNote {
                title: "Note One".into(),
                content_md: "Content one.".into(),
                tags: vec![],
            },
            PdfNote {
                title: "Note Two".into(),
                content_md: "Content two.".into(),
                tags: vec![],
            },
        ];

        let count = export_notes_to_pdf(&notes, dir.path(), &PdfExportOptions::default())
            .await
            .unwrap();
        assert_eq!(count, 2);
        assert!(dir.path().join("note-one.pdf").exists());
        assert!(dir.path().join("note-two.pdf").exists());
    }

    #[tokio::test]
    async fn export_pdf_with_page_break() {
        let dir = TempDir::new().unwrap();
        // Generate content long enough for page break
        let long_content = (0..100)
            .map(|i| {
                format!(
                    "This is paragraph number {i} with enough text to fill space."
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        let note = PdfNote {
            title: "Long Note".into(),
            content_md: long_content,
            tags: vec![],
        };

        let path = dir.path().join("long.pdf");
        export_note_to_pdf(&note, &path, &PdfExportOptions::default())
            .await
            .unwrap();
        assert!(path.exists());
    }
}
