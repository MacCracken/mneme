//! Application state shared across handlers.

use std::sync::Arc;

use mneme_search::SearchEngine;
use mneme_store::Vault;
use tokio::sync::RwLock;

/// Shared application state.
#[derive(Clone)]
pub struct AppState {
    pub vault: Arc<RwLock<Vault>>,
    pub search: Arc<SearchEngine>,
}
