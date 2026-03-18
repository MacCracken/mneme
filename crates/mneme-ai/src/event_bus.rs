//! Daimon event bus client — publish/subscribe via HTTP SSE.
//!
//! Publishes note lifecycle events and subscribes to cross-agent context.
//! Uses daimon's `/v1/events/publish` and `/v1/events/subscribe` endpoints.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AiError;

/// Event types published by Mneme.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum MnemeEvent {
    #[serde(rename = "note.created")]
    NoteCreated {
        vault_id: Uuid,
        note_id: Uuid,
        title: String,
        tags: Vec<String>,
    },
    #[serde(rename = "note.updated")]
    NoteUpdated {
        vault_id: Uuid,
        note_id: Uuid,
        title: String,
    },
    #[serde(rename = "note.deleted")]
    NoteDeleted { vault_id: Uuid, note_id: Uuid },
    #[serde(rename = "concept.extracted")]
    ConceptExtracted {
        vault_id: Uuid,
        note_id: Uuid,
        concepts: Vec<String>,
    },
    #[serde(rename = "search.executed")]
    SearchExecuted {
        vault_id: Uuid,
        query: String,
        result_count: usize,
    },
}

impl MnemeEvent {
    /// The topic string for this event.
    pub fn topic(&self) -> &'static str {
        match self {
            MnemeEvent::NoteCreated { .. } => "mneme.note.created",
            MnemeEvent::NoteUpdated { .. } => "mneme.note.updated",
            MnemeEvent::NoteDeleted { .. } => "mneme.note.deleted",
            MnemeEvent::ConceptExtracted { .. } => "mneme.concept.extracted",
            MnemeEvent::SearchExecuted { .. } => "mneme.search.executed",
        }
    }
}

/// Client for daimon's event bus.
#[derive(Clone)]
pub struct EventBusClient {
    client: reqwest::Client,
    base_url: String,
    sender_name: String,
}

impl EventBusClient {
    /// Create a new event bus client.
    ///
    /// `base_url` defaults to daimon at `http://127.0.0.1:8090`.
    pub fn new(base_url: Option<String>, sender_name: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.unwrap_or_else(|| "http://127.0.0.1:8090".into()),
            sender_name: sender_name.unwrap_or_else(|| "mneme".into()),
        }
    }

    /// Publish an event to the daimon topic broker.
    ///
    /// Returns the number of agents the event was delivered to,
    /// or 0 if daimon is unavailable (fire-and-forget).
    pub async fn publish(&self, event: &MnemeEvent) -> usize {
        let payload = match serde_json::to_value(event) {
            Ok(v) => v,
            Err(_) => return 0,
        };

        let body = serde_json::json!({
            "topic": event.topic(),
            "sender": self.sender_name,
            "payload": payload,
        });

        let url = format!("{}/v1/events/publish", self.base_url);
        match self.client.post(&url).json(&body).send().await {
            Ok(resp) if resp.status().is_success() => resp
                .json::<PublishResponse>()
                .await
                .map(|r| r.delivered_to)
                .unwrap_or(0),
            Ok(resp) => {
                tracing::debug!("Event bus publish failed: {}", resp.status());
                0
            }
            Err(e) => {
                tracing::debug!("Event bus unavailable: {e}");
                0
            }
        }
    }

    /// List active topics on the event bus.
    pub async fn list_topics(&self) -> Result<Vec<TopicInfo>, AiError> {
        let url = format!("{}/v1/events/topics", self.base_url);
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
            return Err(AiError::Daimon(format!("Topics: {}", resp.status())));
        }

        let body: TopicsResponse = resp
            .json()
            .await
            .map_err(|e| AiError::Daimon(format!("Topics parse: {e}")))?;
        Ok(body.topics)
    }

    /// Check if the event bus is reachable.
    pub async fn is_available(&self) -> bool {
        self.list_topics().await.is_ok()
    }
}

#[derive(Deserialize)]
struct PublishResponse {
    delivered_to: usize,
}

#[derive(Deserialize)]
struct TopicsResponse {
    topics: Vec<TopicInfo>,
}

/// Info about an active topic on the event bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicInfo {
    pub topic: String,
    pub subscribers: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_topics() {
        let event = MnemeEvent::NoteCreated {
            vault_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            title: "Test".into(),
            tags: vec!["tag".into()],
        };
        assert_eq!(event.topic(), "mneme.note.created");
    }

    #[test]
    fn event_serialization() {
        let event = MnemeEvent::NoteUpdated {
            vault_id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            title: "Updated".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"event_type\":\"note.updated\""));
    }

    #[test]
    fn all_event_variants_have_topics() {
        let vid = Uuid::new_v4();
        let nid = Uuid::new_v4();
        let events = vec![
            MnemeEvent::NoteCreated {
                vault_id: vid,
                note_id: nid,
                title: "t".into(),
                tags: vec![],
            },
            MnemeEvent::NoteUpdated {
                vault_id: vid,
                note_id: nid,
                title: "t".into(),
            },
            MnemeEvent::NoteDeleted {
                vault_id: vid,
                note_id: nid,
            },
            MnemeEvent::ConceptExtracted {
                vault_id: vid,
                note_id: nid,
                concepts: vec![],
            },
            MnemeEvent::SearchExecuted {
                vault_id: vid,
                query: "q".into(),
                result_count: 0,
            },
        ];
        for e in &events {
            assert!(e.topic().starts_with("mneme."));
        }
    }

    #[test]
    fn client_default_url() {
        let client = EventBusClient::new(None, None);
        assert_eq!(client.base_url, "http://127.0.0.1:8090");
        assert_eq!(client.sender_name, "mneme");
    }
}
