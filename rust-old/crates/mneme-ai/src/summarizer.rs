//! Note summarization pipeline.
//!
//! Uses daimon RAG to generate summaries by querying with
//! summarization-oriented prompts.

use crate::AiError;
use crate::client::DaimonClient;

/// Summary of a note or collection of notes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NoteSummary {
    pub summary: String,
    pub key_points: Vec<String>,
    pub word_count: usize,
}

/// Summarization pipeline.
pub struct Summarizer {
    client: DaimonClient,
}

impl Summarizer {
    pub fn new(client: DaimonClient) -> Self {
        Self { client }
    }

    /// Generate a summary of a note's content.
    ///
    /// This extracts key points and produces a condensed version.
    /// When daimon has a local LLM available (via Synapse), this
    /// uses RAG query to generate a natural language summary.
    /// Otherwise, it falls back to extractive summarization.
    pub async fn summarize(&self, content: &str) -> Result<NoteSummary, AiError> {
        if content.trim().is_empty() {
            return Err(AiError::EmptyContent);
        }

        // Try daimon RAG query for abstractive summarization
        let query = format!("Summarize the following text concisely: {content}");
        match self.client.rag_query(&query, Some(3)).await {
            Ok(resp) if !resp.chunks.is_empty() => {
                let summary = resp.formatted_context;
                let key_points = resp
                    .chunks
                    .iter()
                    .map(|c| truncate(&c.content, 200))
                    .collect();
                Ok(NoteSummary {
                    summary,
                    key_points,
                    word_count: content.split_whitespace().count(),
                })
            }
            _ => {
                // Fallback: extractive summarization (first sentences)
                Ok(extractive_summary(content))
            }
        }
    }
}

/// Simple extractive summarization — takes the first few sentences
/// and extracts key points from paragraph starts.
fn extractive_summary(content: &str) -> NoteSummary {
    let sentences: Vec<&str> = content
        .split(['.', '!', '?'])
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && s.len() > 10)
        .collect();

    let summary = sentences
        .iter()
        .take(3)
        .map(|s| format!("{s}."))
        .collect::<Vec<_>>()
        .join(" ");

    let key_points = sentences.iter().take(5).map(|s| s.to_string()).collect();

    NoteSummary {
        summary,
        key_points,
        word_count: content.split_whitespace().count(),
    }
}

fn truncate(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extractive_summary_basic() {
        let content = "Rust is a systems programming language. It focuses on safety and performance. The borrow checker prevents data races. Memory safety without garbage collection. Zero-cost abstractions are a key feature.";
        let summary = extractive_summary(content);
        assert!(!summary.summary.is_empty());
        assert!(!summary.key_points.is_empty());
        assert!(summary.word_count > 0);
    }

    #[test]
    fn extractive_summary_short_content() {
        let content = "A single sentence about Rust programming.";
        let summary = extractive_summary(content);
        assert_eq!(summary.key_points.len(), 1);
    }

    #[test]
    fn truncate_long() {
        assert_eq!(truncate("hello world", 5), "hello...");
    }

    #[test]
    fn truncate_short() {
        assert_eq!(truncate("hi", 10), "hi");
    }
}
