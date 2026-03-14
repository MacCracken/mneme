//! AI-assisted writing — completions, rewording, and expansion.
//!
//! Delegates generation to daimon's inference endpoints.
//! Falls back to simple heuristics when daimon is unavailable.

use serde::{Deserialize, Serialize};

use crate::AiError;
use crate::client::DaimonClient;

/// Type of writing assistance requested.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteAction {
    /// Continue writing from the given text
    Complete,
    /// Rephrase/reword the given text
    Reword,
    /// Expand/elaborate on the given text
    Expand,
}

/// Request for AI writing assistance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteRequest {
    pub action: WriteAction,
    pub text: String,
    pub context: Option<String>,
}

/// Result of AI writing assistance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WriteResult {
    pub original: String,
    pub result: String,
    pub action: WriteAction,
    pub source: WriteSource,
}

/// Where the writing result came from.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriteSource {
    Daimon,
    Local,
}

/// AI writing assistant.
pub struct Writer {
    client: DaimonClient,
}

impl Writer {
    pub fn new(client: DaimonClient) -> Self {
        Self { client }
    }

    /// Generate writing assistance based on the action type.
    pub async fn assist(&self, req: &WriteRequest) -> Result<WriteResult, AiError> {
        if req.text.trim().is_empty() {
            return Err(AiError::EmptyContent);
        }

        // Build a prompt for daimon
        let prompt = match req.action {
            WriteAction::Complete => format!(
                "Continue writing the following text naturally:\n\n{}",
                req.text
            ),
            WriteAction::Reword => format!(
                "Rewrite the following text with different wording while preserving the meaning:\n\n{}",
                req.text
            ),
            WriteAction::Expand => format!(
                "Expand on the following text with more detail and examples:\n\n{}",
                req.text
            ),
        };

        // Try daimon
        match self.client.rag_query(&prompt, Some(3)).await {
            Ok(resp) if !resp.formatted_context.is_empty() => {
                Ok(WriteResult {
                    original: req.text.clone(),
                    result: resp.formatted_context,
                    action: req.action,
                    source: WriteSource::Daimon,
                })
            }
            _ => {
                // Fallback to local heuristics
                let result = local_assist(req);
                Ok(WriteResult {
                    original: req.text.clone(),
                    result,
                    action: req.action,
                    source: WriteSource::Local,
                })
            }
        }
    }
}

/// Local fallback writing assistance (no AI).
fn local_assist(req: &WriteRequest) -> String {
    match req.action {
        WriteAction::Complete => {
            // Simple continuation: suggest an ending sentence
            let sentences: Vec<&str> = req.text.split('.').filter(|s| !s.trim().is_empty()).collect();
            if sentences.is_empty() {
                return req.text.clone();
            }
            let last = sentences.last().unwrap().trim();
            format!("{last} Furthermore, this topic warrants additional exploration and analysis.")
        }
        WriteAction::Reword => {
            // Basic synonym-level rewriting: swap simple words
            let mut result = req.text.clone();
            let swaps = [
                ("important", "significant"),
                ("significant", "notable"),
                ("however", "nevertheless"),
                ("therefore", "consequently"),
                ("because", "since"),
                ("also", "additionally"),
                ("but", "yet"),
                ("big", "large"),
                ("small", "compact"),
                ("good", "effective"),
                ("bad", "problematic"),
                ("use", "utilize"),
                ("show", "demonstrate"),
                ("help", "assist"),
                ("make", "create"),
                ("get", "obtain"),
            ];
            for (from, to) in &swaps {
                // Only replace whole words (simple approach)
                result = result.replace(&format!(" {from} "), &format!(" {to} "));
            }
            result
        }
        WriteAction::Expand => {
            // Extract key sentences and elaborate
            let sentences: Vec<&str> = req.text.split('.')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            let mut expanded = String::new();
            for sentence in &sentences {
                expanded.push_str(sentence);
                expanded.push_str(". ");
                expanded.push_str(&format!("To elaborate on this point, {} is worth examining in greater detail. ", sentence.to_lowercase()));
            }
            expanded.trim().to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_complete() {
        let req = WriteRequest {
            action: WriteAction::Complete,
            text: "Rust is a systems programming language.".into(),
            context: None,
        };
        let result = local_assist(&req);
        assert!(!result.is_empty());
        assert!(result.contains("exploration"));
    }

    #[test]
    fn local_reword() {
        let req = WriteRequest {
            action: WriteAction::Reword,
            text: "This is also important because it helps show the big picture.".into(),
            context: None,
        };
        let result = local_assist(&req);
        assert!(result.contains("additionally") || result.contains("significant") || result.contains("demonstrate"));
    }

    #[test]
    fn local_expand() {
        let req = WriteRequest {
            action: WriteAction::Expand,
            text: "Rust ensures memory safety. The borrow checker prevents data races.".into(),
            context: None,
        };
        let result = local_assist(&req);
        assert!(result.len() > req.text.len());
        assert!(result.contains("elaborate"));
    }

    #[test]
    fn local_complete_empty_sentences() {
        let req = WriteRequest {
            action: WriteAction::Complete,
            text: "no periods here".into(),
            context: None,
        };
        let result = local_assist(&req);
        assert!(result.contains("Furthermore"));
    }

    #[test]
    fn local_expand_single_sentence() {
        let req = WriteRequest {
            action: WriteAction::Expand,
            text: "Rust is fast.".into(),
            context: None,
        };
        let result = local_assist(&req);
        assert!(result.len() > req.text.len());
    }

    #[test]
    fn write_result_serialization() {
        let result = WriteResult {
            original: "test".into(),
            result: "tested".into(),
            action: WriteAction::Reword,
            source: WriteSource::Local,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("reword"));
        assert!(json.contains("local"));
    }
}
