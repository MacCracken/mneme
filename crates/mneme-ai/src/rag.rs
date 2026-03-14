//! RAG (Retrieval-Augmented Generation) pipeline over the knowledge base.

use std::collections::HashMap;

use uuid::Uuid;

use crate::AiError;
use crate::client::{DaimonClient, RagQueryResponse};

/// Result from asking a question across the knowledge base.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RagAnswer {
    pub query: String,
    pub context: String,
    pub source_chunks: Vec<SourceChunk>,
    pub token_estimate: usize,
}

/// A chunk of context retrieved from the knowledge base.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SourceChunk {
    pub content: String,
    pub score: f64,
    pub note_id: Option<Uuid>,
    pub note_title: Option<String>,
}

/// RAG pipeline that ingests notes and answers questions.
pub struct RagPipeline {
    client: DaimonClient,
}

impl RagPipeline {
    pub fn new(client: DaimonClient) -> Self {
        Self { client }
    }

    /// Ingest a note into the RAG pipeline for later retrieval.
    pub async fn ingest_note(
        &self,
        note_id: Uuid,
        title: &str,
        content: &str,
    ) -> Result<usize, AiError> {
        if content.trim().is_empty() {
            return Err(AiError::EmptyContent);
        }

        let mut metadata = HashMap::new();
        metadata.insert("note_id".into(), note_id.to_string());
        metadata.insert("title".into(), title.to_string());
        metadata.insert("source".into(), "mneme".into());

        let resp = self.client.rag_ingest(content, metadata).await?;
        Ok(resp.chunks)
    }

    /// Ask a question across all ingested notes.
    pub async fn query(&self, question: &str, top_k: Option<usize>) -> Result<RagAnswer, AiError> {
        let resp: RagQueryResponse = self.client.rag_query(question, top_k).await?;

        let source_chunks = resp
            .chunks
            .into_iter()
            .map(|c| {
                let note_id = c
                    .metadata
                    .get("note_id")
                    .and_then(|id| Uuid::parse_str(id).ok());
                let note_title = c.metadata.get("title").cloned();
                SourceChunk {
                    content: c.content,
                    score: c.score,
                    note_id,
                    note_title,
                }
            })
            .collect();

        Ok(RagAnswer {
            query: resp.query,
            context: resp.formatted_context,
            source_chunks,
            token_estimate: resp.token_estimate,
        })
    }

    /// Get index statistics.
    pub async fn stats(&self) -> Result<usize, AiError> {
        let stats = self.client.rag_stats().await?;
        Ok(stats.index_size)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_chunk_serialization() {
        let chunk = SourceChunk {
            content: "test content".into(),
            score: 0.95,
            note_id: Some(Uuid::new_v4()),
            note_title: Some("Test Note".into()),
        };
        let json = serde_json::to_string(&chunk).unwrap();
        assert!(json.contains("test content"));
    }
}
