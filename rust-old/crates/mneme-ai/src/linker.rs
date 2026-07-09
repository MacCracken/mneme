//! Auto-linking — suggest connections between related notes.
//!
//! Uses daimon RAG to find notes with similar content and suggest links.

use std::collections::HashMap;

use uuid::Uuid;

use crate::AiError;
use crate::client::DaimonClient;

/// A suggested link between two notes.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LinkSuggestion {
    pub source_id: Uuid,
    pub target_id: Uuid,
    pub target_title: String,
    pub reason: String,
    pub confidence: f64,
}

/// Auto-linker that discovers relationships between notes.
pub struct AutoLinker {
    client: DaimonClient,
}

impl AutoLinker {
    pub fn new(client: DaimonClient) -> Self {
        Self { client }
    }

    /// Find notes similar to the given note content.
    ///
    /// Returns suggested links ordered by confidence (highest first).
    pub async fn suggest_links(
        &self,
        note_id: Uuid,
        title: &str,
        content: &str,
        top_k: usize,
    ) -> Result<Vec<LinkSuggestion>, AiError> {
        if content.trim().is_empty() {
            return Ok(vec![]);
        }

        // Query RAG with the note's content to find related chunks
        let query = format!("{title}: {}", truncate_content(content, 500));
        let resp = self.client.rag_query(&query, Some(top_k + 5)).await?;

        let mut suggestions: Vec<LinkSuggestion> = resp
            .chunks
            .into_iter()
            .filter_map(|chunk| {
                let target_id = chunk
                    .metadata
                    .get("note_id")
                    .and_then(|id| Uuid::parse_str(id).ok())?;

                // Don't suggest linking to self
                if target_id == note_id {
                    return None;
                }

                let target_title = chunk
                    .metadata
                    .get("title")
                    .cloned()
                    .unwrap_or_else(|| "Untitled".into());

                Some(LinkSuggestion {
                    source_id: note_id,
                    target_id,
                    target_title,
                    reason: truncate_content(&chunk.content, 150),
                    confidence: chunk.score,
                })
            })
            .collect();

        // Deduplicate by target_id (keep highest confidence)
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        let mut seen = HashMap::new();
        suggestions.retain(|s| seen.insert(s.target_id, true).is_none());
        suggestions.truncate(top_k);

        Ok(suggestions)
    }
}

fn truncate_content(s: &str, max_len: usize) -> String {
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
    fn link_suggestion_serialization() {
        let suggestion = LinkSuggestion {
            source_id: Uuid::new_v4(),
            target_id: Uuid::new_v4(),
            target_title: "Related Note".into(),
            reason: "Similar content about Rust".into(),
            confidence: 0.85,
        };
        let json = serde_json::to_string(&suggestion).unwrap();
        assert!(json.contains("Related Note"));
        assert!(json.contains("0.85"));
    }

    #[test]
    fn truncate_content_short() {
        let result = super::truncate_content("short", 100);
        assert_eq!(result, "short");
    }

    #[test]
    fn truncate_content_long() {
        let long = "a".repeat(200);
        let result = super::truncate_content(&long, 50);
        assert!(result.ends_with("..."));
        assert!(result.len() < 60);
    }
}
