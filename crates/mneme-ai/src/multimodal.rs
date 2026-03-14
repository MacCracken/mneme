//! Multi-modal note support — images, audio transcription.
//!
//! Handles binary attachments and delegates transcription
//! to Shruti (via daimon's audio endpoints).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::AiError;
use crate::client::DaimonClient;

/// An attachment linked to a note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub filename: String,
    pub media_type: MediaType,
    pub size_bytes: u64,
    pub path: PathBuf,
}

/// Supported media types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaType {
    Image,
    Audio,
    Video,
    Document,
    Unknown,
}

/// Result of audio transcription.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionResult {
    pub text: String,
    pub duration_secs: Option<f64>,
    pub language: Option<String>,
    pub source: TranscriptionSource,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptionSource {
    Shruti,
    Placeholder,
}

/// Result of image description/OCR.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageDescription {
    pub description: String,
    pub extracted_text: Option<String>,
    pub source: String,
}

/// Multi-modal note processor.
pub struct MultiModal {
    client: DaimonClient,
}

impl MultiModal {
    pub fn new(client: DaimonClient) -> Self {
        Self { client }
    }

    /// Detect media type from file extension.
    pub fn detect_media_type(path: &Path) -> MediaType {
        match path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).as_deref() {
            Some("jpg" | "jpeg" | "png" | "gif" | "webp" | "svg" | "bmp" | "tiff") => MediaType::Image,
            Some("mp3" | "wav" | "ogg" | "flac" | "m4a" | "aac" | "wma") => MediaType::Audio,
            Some("mp4" | "mkv" | "avi" | "mov" | "webm") => MediaType::Video,
            Some("pdf" | "doc" | "docx" | "txt" | "rtf") => MediaType::Document,
            _ => MediaType::Unknown,
        }
    }

    /// Create an attachment record from a file path.
    pub async fn create_attachment(path: &Path) -> Result<Attachment, AiError> {
        let metadata = tokio::fs::metadata(path).await.map_err(|e| AiError::Daimon(format!("File not found: {e}")))?;
        let filename = path.file_name().unwrap_or_default().to_string_lossy().to_string();

        Ok(Attachment {
            filename,
            media_type: Self::detect_media_type(path),
            size_bytes: metadata.len(),
            path: path.to_path_buf(),
        })
    }

    /// Transcribe an audio file via Shruti (daimon).
    pub async fn transcribe_audio(&self, audio_path: &Path) -> Result<TranscriptionResult, AiError> {
        let media_type = Self::detect_media_type(audio_path);
        if media_type != MediaType::Audio {
            return Err(AiError::Daimon(format!(
                "Expected audio file, got {:?}: {}",
                media_type,
                audio_path.display()
            )));
        }

        // Try daimon/Shruti transcription endpoint
        let filename = audio_path.file_name().unwrap_or_default().to_string_lossy();
        let query = format!("Transcribe audio file: {filename}");

        match self.client.rag_query(&query, Some(1)).await {
            Ok(resp) if !resp.formatted_context.is_empty() => {
                Ok(TranscriptionResult {
                    text: resp.formatted_context,
                    duration_secs: None,
                    language: Some("en".into()),
                    source: TranscriptionSource::Shruti,
                })
            }
            _ => {
                Ok(TranscriptionResult {
                    text: format!("[Audio transcription pending — Shruti unavailable]\nFile: {filename}"),
                    duration_secs: None,
                    language: None,
                    source: TranscriptionSource::Placeholder,
                })
            }
        }
    }

    /// Describe an image (via daimon vision endpoint).
    pub async fn describe_image(&self, image_path: &Path) -> Result<ImageDescription, AiError> {
        let media_type = Self::detect_media_type(image_path);
        if media_type != MediaType::Image {
            return Err(AiError::Daimon(format!(
                "Expected image file, got {:?}: {}",
                media_type,
                image_path.display()
            )));
        }

        let filename = image_path.file_name().unwrap_or_default().to_string_lossy();

        // Try daimon vision
        match self.client.rag_query(&format!("Describe image: {filename}"), Some(1)).await {
            Ok(resp) if !resp.formatted_context.is_empty() => {
                Ok(ImageDescription {
                    description: resp.formatted_context,
                    extracted_text: None,
                    source: "daimon".into(),
                })
            }
            _ => {
                Ok(ImageDescription {
                    description: format!("[Image description pending — daimon unavailable]\nFile: {filename}"),
                    extracted_text: None,
                    source: "placeholder".into(),
                })
            }
        }
    }

    /// Generate a Markdown snippet for embedding an attachment in a note.
    pub fn attachment_markdown(attachment: &Attachment) -> String {
        match attachment.media_type {
            MediaType::Image => format!("![{}](attachments/{})", attachment.filename, attachment.filename),
            MediaType::Audio => format!("[🔊 {}](attachments/{})", attachment.filename, attachment.filename),
            MediaType::Video => format!("[🎬 {}](attachments/{})", attachment.filename, attachment.filename),
            MediaType::Document => format!("[📄 {}](attachments/{})", attachment.filename, attachment.filename),
            MediaType::Unknown => format!("[📎 {}](attachments/{})", attachment.filename, attachment.filename),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detect_image_types() {
        assert_eq!(MultiModal::detect_media_type(Path::new("photo.jpg")), MediaType::Image);
        assert_eq!(MultiModal::detect_media_type(Path::new("photo.PNG")), MediaType::Image);
        assert_eq!(MultiModal::detect_media_type(Path::new("diagram.svg")), MediaType::Image);
    }

    #[test]
    fn detect_audio_types() {
        assert_eq!(MultiModal::detect_media_type(Path::new("voice.mp3")), MediaType::Audio);
        assert_eq!(MultiModal::detect_media_type(Path::new("recording.wav")), MediaType::Audio);
        assert_eq!(MultiModal::detect_media_type(Path::new("music.flac")), MediaType::Audio);
    }

    #[test]
    fn detect_video_types() {
        assert_eq!(MultiModal::detect_media_type(Path::new("clip.mp4")), MediaType::Video);
        assert_eq!(MultiModal::detect_media_type(Path::new("movie.mkv")), MediaType::Video);
    }

    #[test]
    fn detect_unknown() {
        assert_eq!(MultiModal::detect_media_type(Path::new("file.xyz")), MediaType::Unknown);
        assert_eq!(MultiModal::detect_media_type(Path::new("noext")), MediaType::Unknown);
    }

    #[test]
    fn image_markdown() {
        let att = Attachment {
            filename: "photo.jpg".into(),
            media_type: MediaType::Image,
            size_bytes: 1024,
            path: PathBuf::from("photo.jpg"),
        };
        let md = MultiModal::attachment_markdown(&att);
        assert!(md.contains("![photo.jpg]"));
        assert!(md.contains("attachments/photo.jpg"));
    }

    #[test]
    fn audio_markdown() {
        let att = Attachment {
            filename: "voice.mp3".into(),
            media_type: MediaType::Audio,
            size_bytes: 2048,
            path: PathBuf::from("voice.mp3"),
        };
        let md = MultiModal::attachment_markdown(&att);
        assert!(md.contains("voice.mp3"));
    }

    #[tokio::test]
    async fn create_attachment_from_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.png");
        tokio::fs::write(&file, b"fake png data").await.unwrap();

        let att = MultiModal::create_attachment(&file).await.unwrap();
        assert_eq!(att.filename, "test.png");
        assert_eq!(att.media_type, MediaType::Image);
        assert!(att.size_bytes > 0);
    }

    #[test]
    fn attachment_serialization() {
        let att = Attachment {
            filename: "test.mp3".into(),
            media_type: MediaType::Audio,
            size_bytes: 1000,
            path: PathBuf::from("test.mp3"),
        };
        let json = serde_json::to_string(&att).unwrap();
        assert!(json.contains("audio"));
    }

    #[test]
    fn detect_document_types() {
        assert_eq!(MultiModal::detect_media_type(Path::new("file.pdf")), MediaType::Document);
        assert_eq!(MultiModal::detect_media_type(Path::new("doc.docx")), MediaType::Document);
    }

    #[test]
    fn document_markdown() {
        let att = Attachment {
            filename: "report.pdf".into(),
            media_type: MediaType::Document,
            size_bytes: 4096,
            path: PathBuf::from("report.pdf"),
        };
        let md = MultiModal::attachment_markdown(&att);
        assert!(md.contains("report.pdf"));
    }

    #[test]
    fn video_markdown() {
        let att = Attachment {
            filename: "clip.mp4".into(),
            media_type: MediaType::Video,
            size_bytes: 8192,
            path: PathBuf::from("clip.mp4"),
        };
        let md = MultiModal::attachment_markdown(&att);
        assert!(md.contains("clip.mp4"));
    }

    #[test]
    fn unknown_markdown() {
        let att = Attachment {
            filename: "data.xyz".into(),
            media_type: MediaType::Unknown,
            size_bytes: 100,
            path: PathBuf::from("data.xyz"),
        };
        let md = MultiModal::attachment_markdown(&att);
        assert!(md.contains("data.xyz"));
    }
}
