//! Translation pipeline — translate notes via daimon.
//!
//! Supports per-note and batch translation while preserving
//! Markdown structure.

use serde::{Deserialize, Serialize};

use crate::AiError;
use crate::client::DaimonClient;

/// Supported languages (ISO 639-1 codes).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Language {
    pub code: String,
    pub name: String,
}

/// Translation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateRequest {
    pub content: String,
    pub target_language: String,
    pub source_language: Option<String>,
    pub preserve_formatting: bool,
}

/// Translation result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslateResult {
    pub original: String,
    pub translated: String,
    pub source_language: String,
    pub target_language: String,
    pub word_count: usize,
    pub source: TranslateSource,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranslateSource {
    Daimon,
    Placeholder,
}

/// Translation pipeline.
pub struct Translator {
    client: DaimonClient,
}

impl Translator {
    pub fn new(client: DaimonClient) -> Self {
        Self { client }
    }

    /// Translate note content to the target language.
    pub async fn translate(&self, req: &TranslateRequest) -> Result<TranslateResult, AiError> {
        if req.content.trim().is_empty() {
            return Err(AiError::EmptyContent);
        }

        let source_lang = req.source_language.clone().unwrap_or_else(|| "auto".into());
        let prompt = format!(
            "Translate the following text from {} to {}. Preserve all Markdown formatting:\n\n{}",
            source_lang, req.target_language, req.content
        );

        match self.client.rag_query(&prompt, Some(1)).await {
            Ok(resp) if !resp.formatted_context.is_empty() => Ok(TranslateResult {
                original: req.content.clone(),
                translated: resp.formatted_context,
                source_language: source_lang,
                target_language: req.target_language.clone(),
                word_count: req.content.split_whitespace().count(),
                source: TranslateSource::Daimon,
            }),
            _ => {
                // Fallback: return a placeholder translation
                let translated = if req.preserve_formatting {
                    preserve_markdown_structure(&req.content, &req.target_language)
                } else {
                    format!(
                        "[Translation to {} pending — daimon unavailable]\n\n{}",
                        req.target_language, req.content
                    )
                };

                Ok(TranslateResult {
                    original: req.content.clone(),
                    translated,
                    source_language: source_lang,
                    target_language: req.target_language.clone(),
                    word_count: req.content.split_whitespace().count(),
                    source: TranslateSource::Placeholder,
                })
            }
        }
    }

    /// Translate multiple texts in batch.
    pub async fn translate_batch(
        &self,
        contents: &[String],
        target_language: &str,
    ) -> Result<Vec<TranslateResult>, AiError> {
        let mut results = Vec::with_capacity(contents.len());
        for content in contents {
            let req = TranslateRequest {
                content: content.clone(),
                target_language: target_language.to_string(),
                source_language: None,
                preserve_formatting: true,
            };
            results.push(self.translate(&req).await?);
        }
        Ok(results)
    }

    /// List commonly supported languages.
    pub fn supported_languages() -> Vec<Language> {
        vec![
            Language {
                code: "en".into(),
                name: "English".into(),
            },
            Language {
                code: "es".into(),
                name: "Spanish".into(),
            },
            Language {
                code: "fr".into(),
                name: "French".into(),
            },
            Language {
                code: "de".into(),
                name: "German".into(),
            },
            Language {
                code: "it".into(),
                name: "Italian".into(),
            },
            Language {
                code: "pt".into(),
                name: "Portuguese".into(),
            },
            Language {
                code: "zh".into(),
                name: "Chinese".into(),
            },
            Language {
                code: "ja".into(),
                name: "Japanese".into(),
            },
            Language {
                code: "ko".into(),
                name: "Korean".into(),
            },
            Language {
                code: "ar".into(),
                name: "Arabic".into(),
            },
            Language {
                code: "ru".into(),
                name: "Russian".into(),
            },
            Language {
                code: "hi".into(),
                name: "Hindi".into(),
            },
        ]
    }
}

/// Preserve Markdown structure in placeholder translation.
fn preserve_markdown_structure(content: &str, target_lang: &str) -> String {
    let mut result = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with('#')
            || trimmed.starts_with("```")
            || trimmed.starts_with("---")
            || trimmed.starts_with("- [")
        {
            // Keep structural elements as-is
            result.push_str(line);
        } else {
            result.push_str(&format!("/* {target_lang} */ {line}"));
        }
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_languages_nonempty() {
        let langs = Translator::supported_languages();
        assert!(langs.len() >= 10);
        assert!(langs.iter().any(|l| l.code == "en"));
        assert!(langs.iter().any(|l| l.code == "ja"));
    }

    #[test]
    fn preserve_structure_keeps_headings() {
        let content = "# Title\n\nSome text here.\n\n## Section\n\n- [ ] Todo item";
        let result = preserve_markdown_structure(content, "es");
        assert!(result.contains("# Title"));
        assert!(result.contains("## Section"));
        assert!(result.contains("- [ ] Todo item"));
        assert!(result.contains("/* es */"));
    }

    #[test]
    fn preserve_structure_code_blocks() {
        let content = "Text line\n```\ncode here\n```\n---\nMore text";
        let result = preserve_markdown_structure(content, "fr");
        assert!(result.contains("```"));
        assert!(result.contains("---"));
        assert!(result.contains("/* fr */"));
    }

    #[test]
    fn language_serialization() {
        let lang = Language {
            code: "ja".into(),
            name: "Japanese".into(),
        };
        let json = serde_json::to_string(&lang).unwrap();
        assert!(json.contains("ja"));
        assert!(json.contains("Japanese"));
    }

    #[test]
    fn translate_result_serialization() {
        let result = TranslateResult {
            original: "hello".into(),
            translated: "hola".into(),
            source_language: "en".into(),
            target_language: "es".into(),
            word_count: 1,
            source: TranslateSource::Placeholder,
        };
        let json = serde_json::to_string(&result).unwrap();
        assert!(json.contains("placeholder"));
    }
}
