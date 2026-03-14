//! Spaced repetition — generate flashcards from notes.
//!
//! Extracts question/answer pairs from note content using
//! pattern recognition and AI assistance.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::client::DaimonClient;
use crate::AiError;

/// A flashcard generated from a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flashcard {
    pub id: Uuid,
    pub note_id: Uuid,
    pub front: String,
    pub back: String,
    pub card_type: CardType,
    pub created_at: DateTime<Utc>,
}

/// How the card was generated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CardType {
    /// Definition: "What is X?" -> definition
    Definition,
    /// Concept: heading -> content summary
    Concept,
    /// AI-generated question from content
    AiGenerated,
}

/// Spaced repetition scheduling state for a card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardSchedule {
    pub card_id: Uuid,
    pub ease_factor: f64,
    pub interval_days: u32,
    pub repetitions: u32,
    pub next_review: DateTime<Utc>,
    pub last_review: Option<DateTime<Utc>>,
}

/// User's rating of recall quality.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecallQuality {
    /// Complete failure to recall
    Again,
    /// Recalled with significant difficulty
    Hard,
    /// Recalled with some effort
    Good,
    /// Recalled instantly
    Easy,
}

impl CardSchedule {
    /// Create a new schedule for a card (first time).
    pub fn new(card_id: Uuid) -> Self {
        Self {
            card_id,
            ease_factor: 2.5,
            interval_days: 1,
            repetitions: 0,
            next_review: Utc::now(),
            last_review: None,
        }
    }

    /// Update schedule based on recall quality (SM-2 algorithm variant).
    pub fn review(&mut self, quality: RecallQuality) {
        let q = match quality {
            RecallQuality::Again => 0,
            RecallQuality::Hard => 1,
            RecallQuality::Good => 3,
            RecallQuality::Easy => 5,
        };

        if q < 2 {
            // Reset on failure
            self.repetitions = 0;
            self.interval_days = 1;
        } else {
            self.repetitions += 1;
            match self.repetitions {
                1 => self.interval_days = 1,
                2 => self.interval_days = 6,
                _ => {
                    self.interval_days = (self.interval_days as f64 * self.ease_factor) as u32;
                }
            }
        }

        // Update ease factor
        let q_f = q as f64;
        self.ease_factor += 0.1 - (5.0 - q_f) * (0.08 + (5.0 - q_f) * 0.02);
        if self.ease_factor < 1.3 {
            self.ease_factor = 1.3;
        }

        self.last_review = Some(Utc::now());
        self.next_review = Utc::now() + Duration::days(self.interval_days as i64);
    }

    /// Check if card is due for review.
    pub fn is_due(&self) -> bool {
        Utc::now() >= self.next_review
    }
}

/// Extract flashcards from note content.
pub fn extract_flashcards(note_id: Uuid, content: &str) -> Vec<Flashcard> {
    let mut cards = Vec::new();
    let now = Utc::now();

    // Pattern 1: Definition lists ("**Term**: definition" or "**Term** — definition")
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("**")
            && let Some(idx) = rest.find("**")
        {
            let term = &rest[..idx];
            let after = rest[idx + 2..].trim();
            let definition = after
                .strip_prefix(':')
                .or_else(|| after.strip_prefix('\u{2014}'))
                .or_else(|| after.strip_prefix('-'));
            if let Some(def) = definition {
                let def = def.trim();
                if !def.is_empty() {
                    cards.push(Flashcard {
                        id: Uuid::new_v5(
                            &Uuid::NAMESPACE_OID,
                            format!("{note_id}:def:{term}").as_bytes(),
                        ),
                        note_id,
                        front: format!("What is {term}?"),
                        back: def.to_string(),
                        card_type: CardType::Definition,
                        created_at: now,
                    });
                }
            }
        }
    }

    // Pattern 2: Heading -> first paragraph as concept cards
    let mut heading: Option<String> = None;
    let mut paragraph = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(h) = trimmed
            .strip_prefix("## ")
            .or_else(|| trimmed.strip_prefix("### "))
        {
            // Save previous heading+paragraph as card
            if let Some(ref h_text) = heading
                && !paragraph.trim().is_empty()
            {
                cards.push(Flashcard {
                    id: Uuid::new_v5(
                        &Uuid::NAMESPACE_OID,
                        format!("{note_id}:concept:{h_text}").as_bytes(),
                    ),
                    note_id,
                    front: format!("Explain: {h_text}"),
                    back: paragraph.trim().to_string(),
                    card_type: CardType::Concept,
                    created_at: now,
                });
            }
            heading = Some(h.to_string());
            paragraph.clear();
        } else if heading.is_some() && !trimmed.is_empty() && !trimmed.starts_with('#') {
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(trimmed);
        }
    }
    // Don't forget the last heading
    if let Some(ref h_text) = heading
        && !paragraph.trim().is_empty()
    {
        cards.push(Flashcard {
            id: Uuid::new_v5(
                &Uuid::NAMESPACE_OID,
                format!("{note_id}:concept:{h_text}").as_bytes(),
            ),
            note_id,
            front: format!("Explain: {h_text}"),
            back: paragraph.trim().to_string(),
            card_type: CardType::Concept,
            created_at: now,
        });
    }

    cards
}

/// Generate flashcards using AI (via daimon).
pub async fn generate_flashcards_ai(
    client: &DaimonClient,
    note_id: Uuid,
    content: &str,
) -> Result<Vec<Flashcard>, AiError> {
    if content.trim().is_empty() {
        return Err(AiError::EmptyContent);
    }

    // Try daimon
    let prompt = format!("Generate study flashcards (question/answer pairs) from:\n\n{content}");
    match client.rag_query(&prompt, Some(5)).await {
        Ok(resp) if !resp.chunks.is_empty() => {
            let now = Utc::now();
            let cards: Vec<Flashcard> = resp
                .chunks
                .iter()
                .enumerate()
                .map(|(i, chunk)| Flashcard {
                    id: Uuid::new_v5(
                        &Uuid::NAMESPACE_OID,
                        format!("{note_id}:ai:{i}").as_bytes(),
                    ),
                    note_id,
                    front: chunk
                        .metadata
                        .get("question")
                        .cloned()
                        .unwrap_or_else(|| format!("Question {}", i + 1)),
                    back: chunk.content.clone(),
                    card_type: CardType::AiGenerated,
                    created_at: now,
                })
                .collect();
            Ok(cards)
        }
        _ => {
            // Fallback to local extraction
            Ok(extract_flashcards(note_id, content))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_definition_cards() {
        let content = "**Rust**: A systems programming language focused on safety.\n**Borrow Checker**: The compiler component that enforces ownership rules.";
        let cards = extract_flashcards(Uuid::new_v4(), content);
        assert_eq!(cards.len(), 2);
        assert!(cards[0].front.contains("Rust"));
        assert!(cards[0].back.contains("systems programming"));
        assert_eq!(cards[0].card_type, CardType::Definition);
    }

    #[test]
    fn extract_concept_cards() {
        let content = "# Title\n\n## Memory Safety\n\nRust prevents null pointer dereferences and buffer overflows.\n\n## Concurrency\n\nRust's type system prevents data races at compile time.";
        let cards = extract_flashcards(Uuid::new_v4(), content);
        let concept_cards: Vec<_> = cards
            .iter()
            .filter(|c| c.card_type == CardType::Concept)
            .collect();
        assert_eq!(concept_cards.len(), 2);
        assert!(concept_cards[0].front.contains("Memory Safety"));
    }

    #[test]
    fn sm2_good_review() {
        let mut schedule = CardSchedule::new(Uuid::new_v4());
        assert!(schedule.is_due());

        schedule.review(RecallQuality::Good);
        assert_eq!(schedule.repetitions, 1);
        assert_eq!(schedule.interval_days, 1);

        schedule.review(RecallQuality::Good);
        assert_eq!(schedule.repetitions, 2);
        assert_eq!(schedule.interval_days, 6);

        schedule.review(RecallQuality::Good);
        assert_eq!(schedule.repetitions, 3);
        assert!(schedule.interval_days > 6);
    }

    #[test]
    fn sm2_fail_resets() {
        let mut schedule = CardSchedule::new(Uuid::new_v4());
        schedule.review(RecallQuality::Good);
        schedule.review(RecallQuality::Good);
        assert_eq!(schedule.repetitions, 2);

        schedule.review(RecallQuality::Again);
        assert_eq!(schedule.repetitions, 0);
        assert_eq!(schedule.interval_days, 1);
    }

    #[test]
    fn ease_factor_minimum() {
        let mut schedule = CardSchedule::new(Uuid::new_v4());
        // Multiple failures should not drop ease below 1.3
        for _ in 0..10 {
            schedule.review(RecallQuality::Hard);
        }
        assert!(schedule.ease_factor >= 1.3);
    }

    #[test]
    fn flashcard_serialization() {
        let card = Flashcard {
            id: Uuid::new_v4(),
            note_id: Uuid::new_v4(),
            front: "Q".into(),
            back: "A".into(),
            card_type: CardType::Definition,
            created_at: Utc::now(),
        };
        let json = serde_json::to_string(&card).unwrap();
        assert!(json.contains("definition"));
    }

    #[test]
    fn no_cards_from_empty() {
        let cards = extract_flashcards(Uuid::new_v4(), "");
        assert!(cards.is_empty());
    }
}
