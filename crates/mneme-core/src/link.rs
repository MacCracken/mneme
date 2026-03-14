//! Link types — connections between notes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A directed link from one note to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    pub id: Uuid,
    /// The note containing the link.
    pub source_id: Uuid,
    /// The note being linked to.
    pub target_id: Uuid,
    /// The visible text of the link (e.g. "see also").
    pub link_text: String,
    /// Surrounding text for context (a sentence or paragraph).
    pub context: String,
    pub created_at: DateTime<Utc>,
}

impl Link {
    pub fn new(source_id: Uuid, target_id: Uuid, link_text: String, context: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            source_id,
            target_id,
            link_text,
            context,
            created_at: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_link_stores_endpoints() {
        let src = Uuid::new_v4();
        let tgt = Uuid::new_v4();
        let link = Link::new(src, tgt, "related".into(), "see related topic".into());
        assert_eq!(link.source_id, src);
        assert_eq!(link.target_id, tgt);
    }
}
