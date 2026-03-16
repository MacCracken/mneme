//! HTTP client for daimon Agent Runtime API.

use std::collections::HashMap;

use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::AiError;

/// Client for daimon's REST API (RAG, vectors, knowledge).
#[derive(Clone)]
pub struct DaimonClient {
    client: Client,
    base_url: String,
    api_key: Option<String>,
}

impl DaimonClient {
    /// Create a new client pointing to daimon.
    ///
    /// Defaults to `http://127.0.0.1:8090` if no URL provided.
    pub fn new(base_url: Option<String>, api_key: Option<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or_else(|| "http://127.0.0.1:8090".into()),
            api_key,
        }
    }

    /// Check if daimon is reachable.
    pub async fn health_check(&self) -> Result<bool, AiError> {
        let resp = self
            .client
            .get(format!("{}/health", self.base_url))
            .send()
            .await;
        Ok(resp.is_ok_and(|r| r.status().is_success()))
    }

    // --- RAG ---

    /// Ingest text into the RAG pipeline.
    pub async fn rag_ingest(
        &self,
        text: &str,
        metadata: HashMap<String, String>,
    ) -> Result<RagIngestResponse, AiError> {
        let body = RagIngestRequest {
            text: text.to_string(),
            metadata,
            agent_id: Some("mneme".into()),
        };

        let resp = self
            .request_post("/v1/rag/ingest", &body)
            .await?
            .json::<RagIngestResponse>()
            .await?;
        Ok(resp)
    }

    /// Query the RAG pipeline.
    pub async fn rag_query(
        &self,
        query: &str,
        top_k: Option<usize>,
    ) -> Result<RagQueryResponse, AiError> {
        let body = RagQueryRequest {
            query: query.to_string(),
            top_k: top_k.unwrap_or(5),
        };

        let resp = self
            .request_post("/v1/rag/query", &body)
            .await?
            .json::<RagQueryResponse>()
            .await?;
        Ok(resp)
    }

    /// Get RAG pipeline stats.
    pub async fn rag_stats(&self) -> Result<RagStatsResponse, AiError> {
        let resp = self
            .request_get("/v1/rag/stats")
            .await?
            .json::<RagStatsResponse>()
            .await?;
        Ok(resp)
    }

    // --- Vectors ---

    /// Insert vectors into a collection.
    pub async fn vectors_insert(
        &self,
        collection: &str,
        vectors: Vec<VectorInsertItem>,
    ) -> Result<VectorInsertResponse, AiError> {
        let body = VectorInsertRequest {
            collection: collection.to_string(),
            vectors,
        };

        let resp = self
            .request_post("/v1/vectors/insert", &body)
            .await?
            .json::<VectorInsertResponse>()
            .await?;
        Ok(resp)
    }

    /// Search vectors by embedding similarity.
    pub async fn vectors_search(
        &self,
        embedding: Vec<f32>,
        collection: &str,
        top_k: usize,
    ) -> Result<VectorSearchResponse, AiError> {
        let body = VectorSearchRequest {
            embedding,
            top_k,
            collection: collection.to_string(),
            min_score: None,
        };

        let resp = self
            .request_post("/v1/vectors/search", &body)
            .await?
            .json::<VectorSearchResponse>()
            .await?;
        Ok(resp)
    }

    // --- Knowledge ---

    /// Search the knowledge base.
    pub async fn knowledge_search(
        &self,
        query: &str,
        limit: Option<usize>,
    ) -> Result<KnowledgeSearchResponse, AiError> {
        let body = KnowledgeSearchRequest {
            query: query.to_string(),
            source: None,
            limit: limit.unwrap_or(10),
        };

        let resp = self
            .request_post("/v1/knowledge/search", &body)
            .await?
            .json::<KnowledgeSearchResponse>()
            .await?;
        Ok(resp)
    }

    // --- Merge suggestions ---

    /// Ask the LLM to suggest how to merge two duplicate notes.
    pub async fn suggest_merge(
        &self,
        a_title: &str,
        a_content: &str,
        b_title: &str,
        b_content: &str,
    ) -> Result<MergeResponse, AiError> {
        let body = MergeRequest {
            note_a_title: a_title.to_string(),
            note_a_content: a_content.to_string(),
            note_b_title: b_title.to_string(),
            note_b_content: b_content.to_string(),
        };

        let resp = self
            .request_post("/v1/knowledge/merge", &body)
            .await?
            .json::<MergeResponse>()
            .await?;
        Ok(resp)
    }

    // --- HTTP helpers ---

    async fn request_post<T: Serialize>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<reqwest::Response, AiError> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.post(&url).json(body);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        let resp = req.send().await.map_err(|e| AiError::Unavailable {
            url: url.clone(),
            reason: e.to_string(),
        })?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AiError::Daimon(format!("{status}: {body}")));
        }
        Ok(resp)
    }

    async fn request_get(&self, path: &str) -> Result<reqwest::Response, AiError> {
        let url = format!("{}{}", self.base_url, path);
        let mut req = self.client.get(&url);
        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }
        let resp = req.send().await.map_err(|e| AiError::Unavailable {
            url: url.clone(),
            reason: e.to_string(),
        })?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(AiError::Daimon(format!("{status}: {body}")));
        }
        Ok(resp)
    }
}

// --- RAG types ---

#[derive(Serialize)]
struct RagIngestRequest {
    text: String,
    metadata: HashMap<String, String>,
    agent_id: Option<String>,
}

#[derive(Serialize)]
struct RagQueryRequest {
    query: String,
    top_k: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RagIngestResponse {
    pub status: String,
    pub chunks: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RagQueryResponse {
    pub query: String,
    pub chunks: Vec<RagChunk>,
    pub formatted_context: String,
    pub token_estimate: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RagChunk {
    pub content: String,
    pub score: f64,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RagStatsResponse {
    pub index_size: usize,
    pub config: RagConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RagConfig {
    pub top_k: usize,
    pub chunk_size: usize,
    pub overlap: usize,
    pub min_relevance_score: f64,
}

// --- Vector types ---

#[derive(Serialize)]
struct VectorInsertRequest {
    collection: String,
    vectors: Vec<VectorInsertItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorInsertItem {
    pub embedding: Vec<f32>,
    pub content: String,
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VectorInsertResponse {
    pub status: String,
    pub collection: String,
    pub ids: Vec<String>,
    pub count: usize,
}

#[derive(Serialize)]
struct VectorSearchRequest {
    embedding: Vec<f32>,
    top_k: usize,
    collection: String,
    min_score: Option<f32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VectorSearchResponse {
    pub collection: String,
    pub results: Vec<VectorSearchResult>,
    pub total: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VectorSearchResult {
    pub id: String,
    pub score: f64,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

// --- Knowledge types ---

#[derive(Serialize)]
struct KnowledgeSearchRequest {
    query: String,
    source: Option<String>,
    limit: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KnowledgeSearchResponse {
    pub query: String,
    pub results: Vec<KnowledgeResult>,
    pub total: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KnowledgeResult {
    pub id: String,
    pub source: String,
    pub path: String,
    pub relevance: f64,
    pub content_preview: String,
}

// --- Merge types ---

#[derive(Serialize)]
struct MergeRequest {
    note_a_title: String,
    note_a_content: String,
    note_b_title: String,
    note_b_content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MergeResponse {
    pub keep: String,
    pub merged_title: String,
    pub merged_content: String,
    pub rationale: String,
    pub confidence: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_default_url() {
        let client = DaimonClient::new(None, None);
        assert_eq!(client.base_url, "http://127.0.0.1:8090");
    }

    #[test]
    fn client_custom_url() {
        let client = DaimonClient::new(Some("http://localhost:9090".into()), None);
        assert_eq!(client.base_url, "http://localhost:9090");
    }
}
