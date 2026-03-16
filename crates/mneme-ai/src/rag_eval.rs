//! RAG evaluation metrics — measure retrieval and generation quality.
//!
//! Provides lightweight, local-only scoring for RAG responses:
//! - **Faithfulness**: How well the answer is grounded in retrieved context (token overlap)
//! - **Answer relevance**: How relevant the answer is to the query (token overlap proxy)
//! - **Chunk utilization**: What fraction of retrieved context actually appears in the answer

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Quality scores for a single RAG response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RagEvalScores {
    /// Token overlap between answer and retrieved context (0.0–1.0).
    /// Higher means the answer is more grounded in the context.
    pub faithfulness: f64,
    /// Token overlap between answer and the original query (0.0–1.0).
    /// Higher means the answer addresses the query more directly.
    pub answer_relevance: f64,
    /// Fraction of retrieved context tokens that appear in the answer (0.0–1.0).
    /// Higher means more of the context was useful.
    pub chunk_utilization: f64,
    /// Overall quality score (weighted average of the above).
    pub overall: f64,
}

/// Aggregate RAG quality statistics over many queries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RagEvalAggregates {
    pub query_count: u64,
    pub avg_faithfulness: f64,
    pub avg_relevance: f64,
    pub avg_utilization: f64,
    pub avg_overall: f64,
    /// Running sums for incremental averaging.
    sum_faithfulness: f64,
    sum_relevance: f64,
    sum_utilization: f64,
    sum_overall: f64,
}

impl RagEvalAggregates {
    /// Record a new set of scores.
    pub fn record(&mut self, scores: &RagEvalScores) {
        self.query_count += 1;
        self.sum_faithfulness += scores.faithfulness;
        self.sum_relevance += scores.answer_relevance;
        self.sum_utilization += scores.chunk_utilization;
        self.sum_overall += scores.overall;

        let n = self.query_count as f64;
        self.avg_faithfulness = self.sum_faithfulness / n;
        self.avg_relevance = self.sum_relevance / n;
        self.avg_utilization = self.sum_utilization / n;
        self.avg_overall = self.sum_overall / n;
    }
}

/// Evaluate a RAG response's quality.
///
/// All scoring is done locally via token overlap — no LLM call required.
pub fn evaluate(query: &str, answer: &str, context_chunks: &[&str]) -> RagEvalScores {
    let query_tokens = tokenize(query);
    let answer_tokens = tokenize(answer);
    let context_tokens: HashSet<String> = context_chunks
        .iter()
        .flat_map(|c| tokenize(c))
        .collect();

    let faithfulness = if answer_tokens.is_empty() {
        0.0
    } else {
        let grounded = answer_tokens.intersection(&context_tokens).count();
        grounded as f64 / answer_tokens.len() as f64
    };

    let answer_relevance = if answer_tokens.is_empty() || query_tokens.is_empty() {
        0.0
    } else {
        let overlap = answer_tokens.intersection(&query_tokens).count();
        overlap as f64 / query_tokens.len() as f64
    };

    let chunk_utilization = if context_tokens.is_empty() {
        0.0
    } else {
        let used = context_tokens.intersection(&answer_tokens).count();
        used as f64 / context_tokens.len() as f64
    };

    // Weighted overall: faithfulness matters most
    let overall = faithfulness * 0.5 + answer_relevance * 0.3 + chunk_utilization * 0.2;

    RagEvalScores {
        faithfulness,
        answer_relevance,
        chunk_utilization,
        overall,
    }
}

/// Simple tokenizer: lowercase, split on non-alphanumeric, filter stopwords.
fn tokenize(text: &str) -> HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2)
        .filter(|w| !STOPWORDS.contains(w))
        .map(String::from)
        .collect()
}

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "are", "but", "not", "you", "all", "can", "has", "her",
    "was", "one", "our", "out", "his", "had", "hot", "how", "its", "let", "may",
    "who", "did", "get", "got", "him", "too", "own", "say", "she", "use", "way",
    "each", "than", "them", "then", "this", "that", "with", "have", "from",
    "been", "were", "what", "when", "will", "more", "some", "very", "just",
    "about", "also", "into", "does", "could", "would", "should", "their",
    "which", "there", "these", "those", "being", "other",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perfect_faithfulness() {
        let context = ["Rust is a systems programming language"];
        let scores = evaluate(
            "What is Rust?",
            "Rust is a systems programming language",
            &context,
        );
        assert!(scores.faithfulness > 0.8);
    }

    #[test]
    fn zero_faithfulness_ungrounded() {
        let context = ["Python is an interpreted language"];
        let scores = evaluate(
            "What is Rust?",
            "Rust was created by Mozilla for web browsers",
            &context,
        );
        assert!(scores.faithfulness < 0.3);
    }

    #[test]
    fn answer_relevance_high() {
        let context = ["Rust has a borrow checker"];
        let scores = evaluate(
            "borrow checker in Rust",
            "Rust borrow checker ensures memory safety",
            &context,
        );
        assert!(scores.answer_relevance > 0.5);
    }

    #[test]
    fn answer_relevance_low() {
        let context = ["Weather today is sunny"];
        let scores = evaluate(
            "borrow checker in Rust",
            "The weather today is sunny and warm",
            &context,
        );
        assert!(scores.answer_relevance < 0.2);
    }

    #[test]
    fn chunk_utilization_full() {
        let context = ["Rust memory safety"];
        let scores = evaluate(
            "What about Rust?",
            "Rust provides memory safety guarantees",
            &context,
        );
        // Both "rust" and "memory" and "safety" should appear in answer
        assert!(scores.chunk_utilization > 0.5);
    }

    #[test]
    fn chunk_utilization_low() {
        let context = [
            "Rust has ownership semantics and borrowing rules",
            "The type system prevents data races at compile time",
            "Cargo is the Rust package manager",
        ];
        // Answer only uses a tiny fraction of the context
        let scores = evaluate("Tell me about Rust", "Rust is nice", &context);
        assert!(scores.chunk_utilization < 0.2);
    }

    #[test]
    fn empty_answer_scores_zero() {
        let context = ["Some context"];
        let scores = evaluate("query", "", &context);
        assert_eq!(scores.faithfulness, 0.0);
        assert_eq!(scores.answer_relevance, 0.0);
        assert_eq!(scores.overall, 0.0);
    }

    #[test]
    fn empty_context_scores_zero_utilization() {
        let scores = evaluate("query", "some answer text here", &[]);
        assert_eq!(scores.chunk_utilization, 0.0);
        assert_eq!(scores.faithfulness, 0.0);
    }

    #[test]
    fn overall_is_weighted_average() {
        let context = ["Rust programming"];
        let scores = evaluate("Rust", "Rust programming language", &context);
        let expected =
            scores.faithfulness * 0.5 + scores.answer_relevance * 0.3 + scores.chunk_utilization * 0.2;
        assert!((scores.overall - expected).abs() < 1e-10);
    }

    #[test]
    fn scores_bounded_zero_to_one() {
        let context = ["alpha bravo charlie delta echo foxtrot"];
        let scores = evaluate(
            "alpha bravo",
            "alpha bravo charlie delta echo foxtrot golf hotel india",
            &context,
        );
        assert!(scores.faithfulness >= 0.0 && scores.faithfulness <= 1.0);
        assert!(scores.answer_relevance >= 0.0 && scores.answer_relevance <= 1.0);
        assert!(scores.chunk_utilization >= 0.0 && scores.chunk_utilization <= 1.0);
        assert!(scores.overall >= 0.0 && scores.overall <= 1.0);
    }

    #[test]
    fn aggregates_track_running_average() {
        let mut agg = RagEvalAggregates::default();
        assert_eq!(agg.query_count, 0);

        let s1 = RagEvalScores {
            faithfulness: 0.8,
            answer_relevance: 0.6,
            chunk_utilization: 0.4,
            overall: 0.66,
        };
        agg.record(&s1);
        assert_eq!(agg.query_count, 1);
        assert!((agg.avg_faithfulness - 0.8).abs() < 1e-10);

        let s2 = RagEvalScores {
            faithfulness: 0.4,
            answer_relevance: 0.2,
            chunk_utilization: 0.6,
            overall: 0.36,
        };
        agg.record(&s2);
        assert_eq!(agg.query_count, 2);
        assert!((agg.avg_faithfulness - 0.6).abs() < 1e-10);
        assert!((agg.avg_overall - 0.51).abs() < 1e-10);
    }

    #[test]
    fn aggregates_serde_roundtrip() {
        let mut agg = RagEvalAggregates::default();
        agg.record(&RagEvalScores {
            faithfulness: 0.7,
            answer_relevance: 0.5,
            chunk_utilization: 0.3,
            overall: 0.55,
        });
        let json = serde_json::to_string(&agg).unwrap();
        let restored: RagEvalAggregates = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.query_count, 1);
        assert!((restored.avg_faithfulness - 0.7).abs() < 1e-10);
    }

    #[test]
    fn tokenize_filters_stopwords() {
        let tokens = tokenize("the quick brown fox jumps over the lazy dog");
        assert!(!tokens.contains("the"));
        assert!(tokens.contains("quick"));
        assert!(tokens.contains("brown"));
        assert!(tokens.contains("jumps"));
    }

    #[test]
    fn tokenize_lowercases() {
        let tokens = tokenize("Rust PROGRAMMING Language");
        assert!(tokens.contains("rust"));
        assert!(tokens.contains("programming"));
        assert!(tokens.contains("language"));
    }

    #[test]
    fn multiple_context_chunks() {
        let context = [
            "Rust provides memory safety",
            "The borrow checker enforces ownership",
            "Cargo manages dependencies",
        ];
        let scores = evaluate(
            "How does Rust ensure safety?",
            "Rust provides memory safety through the borrow checker which enforces ownership rules",
            &context,
        );
        // Should be well-grounded in context
        assert!(scores.faithfulness > 0.5);
        assert!(scores.chunk_utilization > 0.3);
    }
}
