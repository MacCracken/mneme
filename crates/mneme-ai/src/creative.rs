//! Creative suite integration — connect notes with AGNOS creative tools.
//!
//! Provides clients for:
//! - Tazama: video projects, shot lists
//! - Rasa: design assets, image annotations
//! - Shruti: audio transcription, podcast show notes
//! - BullShift: trade journal, research notes

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::client::DaimonClient;
use crate::AiError;

// --- Tazama (Video) ---

/// A shot list entry generated from note content.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShotListEntry {
    pub scene: String,
    pub description: String,
    pub duration_hint: Option<String>,
    pub notes: String,
}

/// A link between a note and a video project.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoProjectLink {
    pub note_id: Uuid,
    pub project_name: String,
    pub role: VideoRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VideoRole {
    Script,
    ShotList,
    Storyboard,
    Reference,
}

/// Generate a shot list from note content.
pub async fn generate_shot_list(
    client: &DaimonClient,
    content: &str,
) -> Result<Vec<ShotListEntry>, AiError> {
    if content.trim().is_empty() {
        return Err(AiError::EmptyContent);
    }

    let prompt = format!("Generate a shot list from the following script/outline:\n\n{content}");
    match client.rag_query(&prompt, Some(5)).await {
        Ok(resp) if !resp.chunks.is_empty() => {
            // Parse response into shot entries
            let entries: Vec<ShotListEntry> = resp
                .chunks
                .iter()
                .map(|chunk| ShotListEntry {
                    scene: format!(
                        "Scene {}",
                        chunk
                            .metadata
                            .get("index")
                            .cloned()
                            .unwrap_or_else(|| "1".into())
                    ),
                    description: chunk.content.clone(),
                    duration_hint: None,
                    notes: String::new(),
                })
                .collect();
            Ok(entries)
        }
        _ => {
            // Local fallback: extract paragraphs as scenes
            Ok(extract_scenes_locally(content))
        }
    }
}

fn extract_scenes_locally(content: &str) -> Vec<ShotListEntry> {
    content
        .split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .enumerate()
        .map(|(i, paragraph)| {
            let first_line = paragraph.lines().next().unwrap_or("").trim();
            let description = if first_line.starts_with('#') {
                first_line.trim_start_matches('#').trim().to_string()
            } else {
                first_line.chars().take(80).collect()
            };
            ShotListEntry {
                scene: format!("Scene {}", i + 1),
                description,
                duration_hint: None,
                notes: paragraph.trim().to_string(),
            }
        })
        .collect()
}

// --- Rasa (Design) ---

/// An image annotation linked to a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageAnnotation {
    pub image_path: String,
    pub note_id: Uuid,
    pub label: String,
    pub region: Option<AnnotationRegion>,
}

/// A rectangular region on an image.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// A design asset linked to a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DesignAssetLink {
    pub note_id: Uuid,
    pub asset_path: String,
    pub asset_type: DesignAssetType,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignAssetType {
    Sketch,
    Mockup,
    Wireframe,
    FinalDesign,
    Icon,
    Photo,
}

// --- Shruti (Audio) ---

/// Podcast show notes generated from a transcript.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowNotes {
    pub title: String,
    pub summary: String,
    pub timestamps: Vec<TimestampEntry>,
    pub key_topics: Vec<String>,
}

/// A timestamp entry in show notes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimestampEntry {
    pub time: String,
    pub label: String,
}

/// Generate show notes from a transcript.
pub async fn generate_show_notes(
    client: &DaimonClient,
    transcript: &str,
    title: &str,
) -> Result<ShowNotes, AiError> {
    if transcript.trim().is_empty() {
        return Err(AiError::EmptyContent);
    }

    let prompt = format!("Generate podcast show notes from this transcript:\n\n{transcript}");
    match client.rag_query(&prompt, Some(3)).await {
        Ok(resp) if !resp.formatted_context.is_empty() => Ok(ShowNotes {
            title: title.to_string(),
            summary: resp.formatted_context,
            timestamps: vec![],
            key_topics: resp
                .chunks
                .iter()
                .map(|c| c.content.chars().take(50).collect())
                .collect(),
        }),
        _ => {
            // Local fallback
            Ok(local_show_notes(transcript, title))
        }
    }
}

fn local_show_notes(transcript: &str, title: &str) -> ShowNotes {
    let paragraphs: Vec<&str> = transcript
        .split("\n\n")
        .filter(|p| !p.trim().is_empty())
        .collect();
    let summary = paragraphs
        .first()
        .map(|p| p.chars().take(200).collect())
        .unwrap_or_default();
    let key_topics: Vec<String> = paragraphs
        .iter()
        .take(5)
        .map(|p| p.lines().next().unwrap_or("").chars().take(60).collect())
        .collect();

    ShowNotes {
        title: title.to_string(),
        summary,
        timestamps: vec![],
        key_topics,
    }
}

// --- BullShift (Trading) ---

/// A trade journal entry linked to a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeJournalEntry {
    pub note_id: Uuid,
    pub ticker: String,
    pub action: TradeAction,
    pub thesis: String,
    pub outcome: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TradeAction {
    Buy,
    Sell,
    Hold,
    Research,
}

/// Extract trade-related information from note content.
pub fn extract_trade_info(note_id: Uuid, content: &str) -> Vec<TradeJournalEntry> {
    let mut entries = Vec::new();
    let ticker_re = regex::Regex::new(r"\$([A-Z]{1,5})").unwrap();

    for caps in ticker_re.captures_iter(content) {
        let ticker = caps[1].to_string();
        let lower = content.to_lowercase();
        let action = if lower.contains("buy") || lower.contains("long") {
            TradeAction::Buy
        } else if lower.contains("sell") || lower.contains("short") {
            TradeAction::Sell
        } else if lower.contains("hold") {
            TradeAction::Hold
        } else {
            TradeAction::Research
        };

        // Extract thesis from context around the ticker mention
        let thesis = content
            .lines()
            .find(|l| l.contains(&format!("${ticker}")))
            .unwrap_or("")
            .trim()
            .to_string();

        if !entries.iter().any(|e: &TradeJournalEntry| e.ticker == ticker) {
            entries.push(TradeJournalEntry {
                note_id,
                ticker,
                action,
                thesis,
                outcome: None,
            });
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_scenes() {
        let content =
            "# Opening\n\nThe camera pans across the city.\n\n# Interview\n\nHost introduces the guest.";
        let scenes = extract_scenes_locally(content);
        // Splits into 4 paragraphs: heading, body, heading, body
        assert_eq!(scenes.len(), 4);
        assert_eq!(scenes[0].scene, "Scene 1");
        assert!(scenes[0].description.contains("Opening"));
    }

    #[test]
    fn local_show_notes_basic() {
        let transcript =
            "Welcome to the podcast.\n\nToday we discuss Rust.\n\nRust is a systems language.";
        let notes = local_show_notes(transcript, "Rust Podcast");
        assert_eq!(notes.title, "Rust Podcast");
        assert!(!notes.summary.is_empty());
        assert!(!notes.key_topics.is_empty());
    }

    #[test]
    fn extract_tickers() {
        let content = "Looking to buy $AAPL after earnings. Also researching $TSLA for potential entry. The $AAPL thesis is strong fundamentals.";
        let entries = extract_trade_info(Uuid::new_v4(), content);
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().any(|e| e.ticker == "AAPL"));
        assert!(entries.iter().any(|e| e.ticker == "TSLA"));
    }

    #[test]
    fn trade_action_detection() {
        let content = "Planning to buy $GOOG based on strong earnings.";
        let entries = extract_trade_info(Uuid::new_v4(), content);
        assert_eq!(entries[0].action, TradeAction::Buy);

        let content2 = "Considering selling $META position.";
        let entries2 = extract_trade_info(Uuid::new_v4(), content2);
        assert_eq!(entries2[0].action, TradeAction::Sell);
    }

    #[test]
    fn video_role_serialization() {
        let link = VideoProjectLink {
            note_id: Uuid::new_v4(),
            project_name: "Test".into(),
            role: VideoRole::ShotList,
        };
        let json = serde_json::to_string(&link).unwrap();
        assert!(json.contains("shot_list"));
    }

    #[test]
    fn design_asset_serialization() {
        let link = DesignAssetLink {
            note_id: Uuid::new_v4(),
            asset_path: "design.fig".into(),
            asset_type: DesignAssetType::Mockup,
            description: "Main screen".into(),
        };
        let json = serde_json::to_string(&link).unwrap();
        assert!(json.contains("mockup"));
    }

    #[test]
    fn extract_no_tickers() {
        let entries = extract_trade_info(Uuid::new_v4(), "No stock tickers here.");
        assert!(entries.is_empty());
    }

    #[test]
    fn show_notes_empty_transcript() {
        let notes = local_show_notes("", "Empty");
        assert_eq!(notes.title, "Empty");
    }

    #[test]
    fn trade_hold_action() {
        let content = "Decided to hold $MSFT position for now.";
        let entries = extract_trade_info(Uuid::new_v4(), content);
        assert_eq!(entries[0].action, TradeAction::Hold);
    }

    #[test]
    fn trade_research_default() {
        let content = "Analyzing $NVDA for potential opportunities.";
        let entries = extract_trade_info(Uuid::new_v4(), content);
        assert_eq!(entries[0].action, TradeAction::Research);
    }

    #[test]
    fn image_annotation_serialization() {
        let ann = ImageAnnotation {
            image_path: "test.png".into(),
            note_id: Uuid::new_v4(),
            label: "Diagram".into(),
            region: Some(AnnotationRegion { x: 10.0, y: 20.0, width: 100.0, height: 50.0 }),
        };
        let json = serde_json::to_string(&ann).unwrap();
        assert!(json.contains("Diagram"));
        assert!(json.contains("100"));
    }

    #[test]
    fn show_notes_serialization() {
        let notes = ShowNotes {
            title: "Test".into(),
            summary: "Summary".into(),
            timestamps: vec![TimestampEntry { time: "00:05".into(), label: "Intro".into() }],
            key_topics: vec!["topic1".into()],
        };
        let json = serde_json::to_string(&notes).unwrap();
        assert!(json.contains("00:05"));
    }

    #[test]
    fn trade_journal_serialization() {
        let entry = TradeJournalEntry {
            note_id: Uuid::new_v4(),
            ticker: "AAPL".into(),
            action: TradeAction::Buy,
            thesis: "Strong earnings".into(),
            outcome: Some("Profitable".into()),
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("buy"));
        assert!(json.contains("Profitable"));
    }
}
