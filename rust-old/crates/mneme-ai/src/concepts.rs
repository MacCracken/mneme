//! Concept extraction — entities, topics, and key terms from notes.
//!
//! Extracts concepts using a combination of:
//! - Statistical methods (TF-IDF-like term scoring)
//! - Daimon knowledge base cross-referencing

use std::collections::HashMap;

use crate::AiError;

/// An extracted concept from a note.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Concept {
    pub term: String,
    pub kind: ConceptKind,
    pub score: f64,
    pub occurrences: usize,
}

/// Classification of extracted concepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConceptKind {
    Topic,
    Entity,
    KeyTerm,
}

/// Extract concepts from note content.
///
/// Uses statistical term frequency analysis to identify significant
/// terms. Words that appear frequently in the document but are not
/// common stop words are scored as potential concepts.
pub fn extract_concepts(content: &str) -> Result<Vec<Concept>, AiError> {
    if content.trim().is_empty() {
        return Err(AiError::EmptyContent);
    }

    let words = tokenize(content);
    let mut freq: HashMap<String, usize> = HashMap::new();

    for word in &words {
        if !is_stop_word(word) && word.len() >= 3 {
            *freq.entry(word.clone()).or_default() += 1;
        }
    }

    let total_words = words.len() as f64;
    let mut concepts: Vec<Concept> = freq
        .into_iter()
        .filter(|(_, count)| *count >= 2)
        .map(|(term, count)| {
            let tf = count as f64 / total_words;
            // Boost longer terms (more specific)
            let length_boost = (term.len() as f64 / 10.0).min(1.5);
            let score = tf * length_boost;

            let kind = classify_term(&term);

            Concept {
                term,
                kind,
                score,
                occurrences: count,
            }
        })
        .collect();

    concepts.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    concepts.truncate(20);

    Ok(concepts)
}

/// Simple tokenization — lowercase, split on non-alphanumeric.
fn tokenize(text: &str) -> Vec<String> {
    text.split(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
        .filter(|w| !w.is_empty())
        .map(|w| w.to_lowercase())
        .collect()
}

/// Classify a term based on heuristics.
fn classify_term(term: &str) -> ConceptKind {
    // Capitalized in original → likely entity (simplified check)
    if term.chars().next().is_some_and(|c| c.is_uppercase()) {
        ConceptKind::Entity
    } else if term.len() >= 6 {
        ConceptKind::Topic
    } else {
        ConceptKind::KeyTerm
    }
}

/// Common English stop words to filter out.
fn is_stop_word(word: &str) -> bool {
    matches!(
        word,
        "the"
            | "a"
            | "an"
            | "and"
            | "or"
            | "but"
            | "in"
            | "on"
            | "at"
            | "to"
            | "for"
            | "of"
            | "with"
            | "by"
            | "from"
            | "as"
            | "is"
            | "was"
            | "are"
            | "were"
            | "be"
            | "been"
            | "being"
            | "have"
            | "has"
            | "had"
            | "do"
            | "does"
            | "did"
            | "will"
            | "would"
            | "could"
            | "should"
            | "may"
            | "might"
            | "shall"
            | "can"
            | "this"
            | "that"
            | "these"
            | "those"
            | "it"
            | "its"
            | "not"
            | "no"
            | "nor"
            | "so"
            | "if"
            | "then"
            | "than"
            | "too"
            | "very"
            | "just"
            | "about"
            | "above"
            | "after"
            | "again"
            | "all"
            | "also"
            | "any"
            | "because"
            | "before"
            | "between"
            | "both"
            | "each"
            | "few"
            | "more"
            | "most"
            | "other"
            | "some"
            | "such"
            | "through"
            | "under"
            | "until"
            | "when"
            | "where"
            | "which"
            | "while"
            | "who"
            | "whom"
            | "why"
            | "how"
            | "what"
            | "there"
            | "here"
            | "into"
            | "over"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_from_technical_content() {
        let content = "Rust is a systems programming language focused on safety. \
            The borrow checker in Rust ensures memory safety. Rust programs \
            compile to native code. The compiler catches many bugs at compile time. \
            Rust has zero-cost abstractions and pattern matching.";

        let concepts = extract_concepts(content).unwrap();
        assert!(!concepts.is_empty());

        let terms: Vec<&str> = concepts.iter().map(|c| c.term.as_str()).collect();
        assert!(terms.contains(&"rust"));
    }

    #[test]
    fn extract_empty_content() {
        assert!(extract_concepts("").is_err());
        assert!(extract_concepts("   ").is_err());
    }

    #[test]
    fn stop_words_filtered() {
        assert!(is_stop_word("the"));
        assert!(is_stop_word("and"));
        assert!(!is_stop_word("rust"));
        assert!(!is_stop_word("programming"));
    }

    #[test]
    fn tokenize_basic() {
        let tokens = tokenize("Hello, World! This is a test.");
        assert!(tokens.contains(&"hello".to_string()));
        assert!(tokens.contains(&"world".to_string()));
        assert!(tokens.contains(&"test".to_string()));
    }

    #[test]
    fn concept_serialization() {
        let concept = Concept {
            term: "rust".into(),
            kind: ConceptKind::KeyTerm,
            score: 0.5,
            occurrences: 3,
        };
        let json = serde_json::to_string(&concept).unwrap();
        assert!(json.contains("key_term"));
    }
}
