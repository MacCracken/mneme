//! Agnostic QA bridge — trigger quality assurance suites and import results.
//!
//! Communicates with the Agnostic QA platform to run automated checks
//! on knowledge base health: link validation, content assertions, and
//! retrieval performance benchmarks.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AiError;

/// A knowledge quality assertion generated from the vault.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeAssertion {
    pub description: String,
    pub assertion_type: AssertionType,
    pub note_id: Option<Uuid>,
    pub expected: String,
}

/// Types of knowledge quality assertions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssertionType {
    /// Note X should exist and contain concept Y.
    ContentContains,
    /// Tag Z should have at least N notes.
    TagMinCount,
    /// Note X should have at least N backlinks.
    BacklinkMinCount,
    /// No orphan notes (notes with zero links and zero tags).
    NoOrphans,
    /// No dead links (links pointing to deleted notes).
    NoDeadLinks,
}

/// Generate knowledge assertions from vault metadata.
///
/// These can be sent to Agnostic as a custom QA suite.
pub fn generate_assertions(
    notes: &[(Uuid, String, Vec<String>, usize)], // (id, title, tags, backlink_count)
    tag_counts: &[(String, usize)],
) -> Vec<KnowledgeAssertion> {
    let mut assertions = Vec::new();

    // Orphan detection: notes with no tags and no backlinks
    for (id, title, tags, backlinks) in notes {
        if tags.is_empty() && *backlinks == 0 {
            assertions.push(KnowledgeAssertion {
                description: format!("Note '{}' has no tags and no backlinks (orphan)", title),
                assertion_type: AssertionType::NoOrphans,
                note_id: Some(*id),
                expected: "at least 1 tag or backlink".into(),
            });
        }
    }

    // Tag health: tags should have reasonable coverage
    for (tag, count) in tag_counts {
        if *count < 2 {
            assertions.push(KnowledgeAssertion {
                description: format!("Tag '#{tag}' has only {count} note(s) — consider merging or removing"),
                assertion_type: AssertionType::TagMinCount,
                note_id: None,
                expected: "at least 2 notes per tag".into(),
            });
        }
    }

    assertions
}

/// Result of a QA run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaRunResult {
    pub run_id: String,
    pub status: String,
    pub total_assertions: usize,
    pub passed: usize,
    pub failed: usize,
    pub failures: Vec<QaFailure>,
}

/// A single QA failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaFailure {
    pub assertion: String,
    pub expected: String,
    pub actual: String,
}

/// Client for the Agnostic QA platform.
#[derive(Clone)]
pub struct AgnosticClient {
    client: reqwest::Client,
    base_url: String,
}

impl AgnosticClient {
    /// Create a new client.
    ///
    /// Defaults to `http://127.0.0.1:8000` if no URL provided.
    pub fn new(base_url: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.unwrap_or_else(|| "http://127.0.0.1:8000".into()),
        }
    }

    /// Check if Agnostic is reachable.
    pub async fn is_available(&self) -> bool {
        let url = format!("{}/health", self.base_url);
        self.client
            .get(&url)
            .send()
            .await
            .is_ok_and(|r| r.status().is_success())
    }

    /// Submit a QA run with custom assertions.
    pub async fn run_assertions(
        &self,
        assertions: &[KnowledgeAssertion],
        vault_name: &str,
    ) -> Result<String, AiError> {
        let body = serde_json::json!({
            "suite": "mneme-knowledge-qa",
            "description": format!("Knowledge QA for vault '{vault_name}'"),
            "assertions": assertions,
        });

        let url = format!("{}/api/v1/runs", self.base_url);
        let resp = self
            .client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| AiError::Unavailable {
                url: url.clone(),
                reason: e.to_string(),
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AiError::Daimon(format!("Agnostic QA: {status}: {body}")));
        }

        let result: serde_json::Value = resp.json().await.map_err(|e| {
            AiError::Daimon(format!("Agnostic QA parse: {e}"))
        })?;

        result
            .get("run_id")
            .and_then(|v| v.as_str())
            .map(String::from)
            .ok_or_else(|| AiError::Daimon("No run_id in response".into()))
    }

    /// Get the status/results of a QA run.
    pub async fn get_run_result(&self, run_id: &str) -> Result<QaRunResult, AiError> {
        let url = format!("{}/api/v1/runs/{}/report?format=json", self.base_url, run_id);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| AiError::Unavailable {
                url: url.clone(),
                reason: e.to_string(),
            })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AiError::Daimon(format!("QA result: {status}: {body}")));
        }

        resp.json().await.map_err(|e| {
            AiError::Daimon(format!("QA result parse: {e}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_orphan_assertions() {
        let notes = vec![
            (Uuid::new_v4(), "Tagged".into(), vec!["rust".into()], 1),
            (Uuid::new_v4(), "Orphan".into(), vec![], 0),
        ];
        let tags: Vec<(String, usize)> = vec![];
        let assertions = generate_assertions(&notes, &tags);
        assert_eq!(assertions.len(), 1);
        assert!(assertions[0].description.contains("Orphan"));
    }

    #[test]
    fn generate_tag_health_assertions() {
        let notes: Vec<(Uuid, String, Vec<String>, usize)> = vec![];
        let tags = vec![
            ("rust".into(), 5),
            ("lonely".into(), 1),
        ];
        let assertions = generate_assertions(&notes, &tags);
        assert_eq!(assertions.len(), 1);
        assert!(assertions[0].description.contains("lonely"));
    }

    #[test]
    fn no_assertions_for_healthy_vault() {
        let notes = vec![
            (Uuid::new_v4(), "A".into(), vec!["tag".into()], 2),
            (Uuid::new_v4(), "B".into(), vec!["tag".into()], 1),
        ];
        let tags = vec![("tag".into(), 2)];
        let assertions = generate_assertions(&notes, &tags);
        assert!(assertions.is_empty());
    }

    #[test]
    fn client_default_url() {
        let client = AgnosticClient::new(None);
        assert_eq!(client.base_url, "http://127.0.0.1:8000");
    }

    #[test]
    fn assertion_serde_roundtrip() {
        let a = KnowledgeAssertion {
            description: "test".into(),
            assertion_type: AssertionType::NoOrphans,
            note_id: Some(Uuid::new_v4()),
            expected: "at least 1".into(),
        };
        let json = serde_json::to_string(&a).unwrap();
        let parsed: KnowledgeAssertion = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.description, "test");
    }
}
