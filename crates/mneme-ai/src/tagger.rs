//! Automatic tag suggestions based on note content.
//!
//! Uses concept extraction and existing tag vocabulary to suggest
//! relevant tags for a note.

use std::collections::HashSet;

use crate::concepts::{self, ConceptKind};
use crate::AiError;

/// A tag suggestion with confidence score.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TagSuggestion {
    pub tag: String,
    pub confidence: f64,
    pub reason: TagReason,
}

/// Why this tag was suggested.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagReason {
    /// Tag matches an extracted concept.
    ConceptMatch,
    /// Tag exists in vocabulary and content mentions it.
    VocabularyMatch,
}

/// Suggest tags for a note based on its content and existing tags in the vault.
pub fn suggest_tags(
    content: &str,
    existing_tags: &[String],
    max_suggestions: usize,
) -> Result<Vec<TagSuggestion>, AiError> {
    let concepts = concepts::extract_concepts(content)?;
    let existing: HashSet<&str> = existing_tags.iter().map(|s| s.as_str()).collect();

    let mut suggestions = Vec::new();

    // Match extracted concepts against existing tags
    for concept in &concepts {
        let term = &concept.term;
        if existing.contains(term.as_str()) {
            suggestions.push(TagSuggestion {
                tag: term.clone(),
                confidence: concept.score.min(1.0),
                reason: TagReason::VocabularyMatch,
            });
        }

        // Check partial matches (concept is substring of existing tag)
        for tag in &existing {
            if tag.contains(term.as_str()) && *tag != term.as_str() {
                suggestions.push(TagSuggestion {
                    tag: tag.to_string(),
                    confidence: concept.score * 0.7,
                    reason: TagReason::VocabularyMatch,
                });
            }
        }
    }

    // Suggest new tags from high-scoring concepts (topics)
    for concept in &concepts {
        if concept.kind == ConceptKind::Topic && concept.score > 0.01 {
            if !existing.contains(concept.term.as_str())
                && !suggestions.iter().any(|s| s.tag == concept.term)
            {
                suggestions.push(TagSuggestion {
                    tag: concept.term.clone(),
                    confidence: concept.score * 0.5,
                    reason: TagReason::ConceptMatch,
                });
            }
        }
    }

    // Sort by confidence, deduplicate, limit
    suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
    let mut seen = HashSet::new();
    suggestions.retain(|s| seen.insert(s.tag.clone()));
    suggestions.truncate(max_suggestions);

    Ok(suggestions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggest_from_vocabulary() {
        let content = "Rust is a systems programming language with a borrow checker. \
                        Rust ensures memory safety without garbage collection. \
                        The Rust compiler is very strict.";
        let existing = vec!["rust".into(), "programming".into(), "python".into()];

        let suggestions = suggest_tags(content, &existing, 5).unwrap();
        assert!(!suggestions.is_empty());
        assert!(suggestions.iter().any(|s| s.tag == "rust"));
    }

    #[test]
    fn suggest_new_concepts() {
        let content = "Machine learning algorithms require training data. \
                        Neural networks learn patterns from training examples. \
                        Deep learning extends traditional machine learning.";
        let existing: Vec<String> = vec![];

        let suggestions = suggest_tags(content, &existing, 5).unwrap();
        assert!(!suggestions.is_empty());
        // Should suggest topic-level concepts
        assert!(suggestions.iter().all(|s| matches!(s.reason, TagReason::ConceptMatch)));
    }

    #[test]
    fn empty_content_error() {
        let result = suggest_tags("", &[], 5);
        assert!(result.is_err());
    }

    #[test]
    fn respects_limit() {
        let content = "Rust Go Python Java TypeScript Kotlin Swift \
                        Rust Go Python Java TypeScript Kotlin Swift \
                        Rust Go Python Java TypeScript Kotlin Swift";
        let suggestions = suggest_tags(content, &[], 3).unwrap();
        assert!(suggestions.len() <= 3);
    }
}
