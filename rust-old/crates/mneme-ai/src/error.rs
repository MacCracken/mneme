//! AI pipeline error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("daimon API error: {0}")]
    Daimon(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("daimon unavailable at {url}: {reason}")]
    Unavailable { url: String, reason: String },

    #[error("no content to process")]
    EmptyContent,
}
