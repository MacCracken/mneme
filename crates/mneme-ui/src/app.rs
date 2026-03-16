//! Application state and event loop.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use mneme_core::config::VaultInfo;
use mneme_core::graph::{
    EdgeRelation, GraphEdge, GraphLayout, GraphNode, NodeKind, Subgraph,
};
use mneme_core::note::Note;
use mneme_search::{ContextBuffer, SearchEngine, SemanticEngine};
use mneme_store::VaultManager;

/// Active panel in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    NoteList,
    NoteView,
    Search,
    Tags,
    Graph,
    SplitView,
    VaultPicker,
    Stale,
    Clusters,
}

/// State for one pane in split view.
pub struct PaneState {
    pub note_id: Option<Uuid>,
    pub title: String,
    pub content: String,
    pub tags: Vec<String>,
    pub backlinks: Vec<(String, Uuid)>,
    pub scroll: u16,
}

impl Default for PaneState {
    fn default() -> Self {
        Self {
            note_id: None,
            title: String::new(),
            content: String::new(),
            tags: Vec::new(),
            backlinks: Vec::new(),
            scroll: 0,
        }
    }
}

/// Application state for the TUI.
pub struct App {
    pub manager: VaultManager,
    pub engines: HashMap<Uuid, (SearchEngine, SemanticEngine)>,
    pub models_dir: PathBuf,
    pub panel: Panel,
    pub notes: Vec<Note>,
    pub selected_index: usize,
    pub selected_note_id: Option<Uuid>,
    pub note_content: String,
    pub note_tags: Vec<String>,
    pub note_backlinks: Vec<(String, Uuid)>,
    pub search_query: String,
    pub search_results: Vec<(Uuid, String, f32)>,
    pub tag_list: Vec<String>,
    pub status_message: String,
    pub should_quit: bool,
    // Graph state
    pub graph_layout: Option<GraphLayout>,
    pub graph_center: (f64, f64),
    pub graph_zoom: f64,
    pub graph_selected: Option<usize>,
    // Split view state
    pub split_panes: [PaneState; 2],
    pub active_pane: usize,
    pub split_pick_pane: Option<usize>,
    // Vault picker state
    pub vault_list: Vec<VaultInfo>,
    // Stale notes state
    pub stale_notes: Vec<(Uuid, String, i64, f64)>, // id, title, days_since_update, freshness
    pub stale_selected: usize,
    // Cluster state
    pub clusters: Vec<(String, Vec<(Uuid, String)>)>, // (label, [(note_id, title)])
    pub cluster_selected: usize,
    pub cluster_expanded: Option<usize>,
    // Context-aware retrieval
    pub context_buffer: ContextBuffer,
}

fn create_engines(vault_path: &Path, models_dir: &Path) -> (SearchEngine, SemanticEngine) {
    let search_dir = vault_path.join(".mneme").join("search-index");
    let search = SearchEngine::open(&search_dir)
        .unwrap_or_else(|_| SearchEngine::in_memory().unwrap());
    let vectors_dir = vault_path.join(".mneme").join("vectors");
    let semantic = SemanticEngine::open(models_dir, &vectors_dir);
    (search, semantic)
}

impl App {
    pub fn new(manager: VaultManager, models_dir: PathBuf) -> Self {
        let mut engines = HashMap::new();
        if let Some(ov) = manager.active() {
            let eng = create_engines(&ov.info.path, &models_dir);
            engines.insert(ov.info.id, eng);
        }

        Self {
            manager,
            engines,
            models_dir,
            panel: Panel::NoteList,
            notes: Vec::new(),
            selected_index: 0,
            selected_note_id: None,
            note_content: String::new(),
            note_tags: Vec::new(),
            note_backlinks: Vec::new(),
            search_query: String::new(),
            search_results: Vec::new(),
            tag_list: Vec::new(),
            status_message: "Press ? for help, v for vault picker".into(),
            should_quit: false,
            graph_layout: None,
            graph_center: (0.0, 0.0),
            graph_zoom: 1.0,
            graph_selected: None,
            split_panes: [PaneState::default(), PaneState::default()],
            active_pane: 0,
            split_pick_pane: None,
            vault_list: Vec::new(),
            stale_notes: Vec::new(),
            stale_selected: 0,
            clusters: Vec::new(),
            cluster_selected: 0,
            cluster_expanded: None,
            context_buffer: ContextBuffer::new(7),
        }
    }

    /// Get the active vault's name for display.
    pub fn active_vault_name(&self) -> &str {
        self.manager
            .active()
            .map(|ov| ov.info.name.as_str())
            .unwrap_or("(none)")
    }

    fn active_search(&self) -> Option<&SearchEngine> {
        let id = self.manager.active_id()?;
        self.engines.get(&id).map(|(s, _)| s)
    }

    /// Load the note list from the active vault.
    pub async fn load_notes(&mut self) {
        let Some(ov) = self.manager.active() else {
            self.status_message = "No active vault".into();
            return;
        };
        match ov.vault.list_notes(100, 0).await {
            Ok(notes) => {
                self.notes = notes;
                if self.selected_index >= self.notes.len() && !self.notes.is_empty() {
                    self.selected_index = self.notes.len() - 1;
                }
            }
            Err(e) => self.status_message = format!("Error: {e}"),
        }
    }

    /// Load tags from the active vault.
    pub async fn load_tags(&mut self) {
        let Some(ov) = self.manager.active() else { return };
        match ov.vault.list_tags().await {
            Ok(tags) => self.tag_list = tags.into_iter().map(|t| t.name).collect(),
            Err(e) => self.status_message = format!("Error: {e}"),
        }
    }

    /// Select and load a note.
    pub async fn select_note(&mut self, id: Uuid) {
        let Some(ov) = self.manager.active() else { return };
        match ov.vault.get_note(id).await {
            Ok(note) => {
                self.selected_note_id = Some(id);
                self.note_content = note.content;
                self.note_tags = note.tags;
                self.note_backlinks = note
                    .backlinks
                    .into_iter()
                    .map(|bl| (bl.source_title, bl.source_id))
                    .collect();
                self.panel = Panel::NoteView;
                self.status_message = format!("Viewing: {}", note.note.title);
                // Track in context buffer for context-aware retrieval
                self.context_buffer.push(id);
            }
            Err(e) => self.status_message = format!("Error: {e}"),
        }
    }

    /// Run a search query, with optional context-aware semantic boost.
    pub fn run_search(&mut self) {
        if self.search_query.is_empty() {
            self.search_results.clear();
            return;
        }

        let Some(search) = self.active_search() else { return };

        match search.search(&self.search_query, 20) {
            Ok(results) => {
                self.search_results = results
                    .into_iter()
                    .map(|r| (r.note_id, r.title, r.score))
                    .collect();

                // Attempt context-aware semantic re-ranking
                if !self.context_buffer.is_empty() {
                    if let Some(semantic) = self.active_semantic() {
                        // Build context embedding from recent notes
                        let recent_ids: Vec<Uuid> =
                            self.context_buffer.recent_ids().iter().copied().collect();
                        let mut embeddings = Vec::new();
                        for note in &self.notes {
                            if recent_ids.contains(&note.id) {
                                if let Ok(Some(emb)) = semantic.embed(&note.title) {
                                    embeddings.push((note.id, emb));
                                }
                            }
                        }
                        if let Some(ctx_emb) = self.context_buffer.context_embedding(&embeddings) {
                            if let Ok(ctx_results) = semantic.context_search(
                                &self.search_query,
                                &ctx_emb,
                                0.7,
                                20,
                            ) {
                                // Merge semantic context results with fulltext
                                for sr in ctx_results {
                                    if let Some(id) = sr.note_id {
                                        if !self.search_results.iter().any(|(rid, _, _)| *rid == id) {
                                            let title = sr.title.unwrap_or_default();
                                            self.search_results.push((id, title, sr.score as f32));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                self.status_message = format!("{} result(s)", self.search_results.len());
            }
            Err(e) => self.status_message = format!("Search error: {e}"),
        }
    }

    /// Build and lay out the knowledge graph.
    pub async fn load_graph(&mut self) {
        let Some(ov) = self.manager.active() else { return };

        let notes = match ov.vault.list_notes(1000, 0).await {
            Ok(n) => n,
            Err(e) => {
                self.status_message = format!("Graph error: {e}");
                return;
            }
        };
        let tags = match ov.vault.list_tags().await {
            Ok(t) => t,
            Err(e) => {
                self.status_message = format!("Graph error: {e}");
                return;
            }
        };
        let links = match ov.vault.list_all_links().await {
            Ok(l) => l,
            Err(e) => {
                self.status_message = format!("Graph error: {e}");
                return;
            }
        };

        let mut nodes: Vec<GraphNode> = notes
            .iter()
            .map(|n| GraphNode {
                id: n.id,
                label: n.title.clone(),
                kind: NodeKind::Note,
            })
            .collect();

        for tag in &tags {
            nodes.push(GraphNode {
                id: tag.id,
                label: tag.name.clone(),
                kind: NodeKind::Tag,
            });
        }

        let mut edges: Vec<GraphEdge> = links
            .iter()
            .map(|l| GraphEdge {
                source: l.source_id,
                target: l.target_id,
                relation: EdgeRelation::LinksTo,
            })
            .collect();

        for note in &notes {
            if let Ok(note_tags) = ov.vault.db().get_note_tags(note.id).await {
                for tag_name in &note_tags {
                    if let Some(tag) = tags.iter().find(|t| &t.name == tag_name) {
                        edges.push(GraphEdge {
                            source: note.id,
                            target: tag.id,
                            relation: EdgeRelation::TaggedWith,
                        });
                    }
                }
            }
        }

        let subgraph = Subgraph { nodes, edges };
        let layout = GraphLayout::from_subgraph(&subgraph);

        let node_count = layout.nodes.len();
        let edge_count = layout.edges.len();
        self.graph_layout = Some(layout);
        self.graph_center = (0.0, 0.0);
        self.graph_zoom = 1.0;
        self.graph_selected = if node_count > 0 { Some(0) } else { None };
        self.status_message = format!("Graph: {node_count} nodes, {edge_count} edges");
    }

    /// Load a note into a specific split pane.
    pub async fn load_pane(&mut self, pane_idx: usize, note_id: Uuid) {
        let Some(ov) = self.manager.active() else { return };
        match ov.vault.get_note(note_id).await {
            Ok(note) => {
                let pane = &mut self.split_panes[pane_idx];
                pane.note_id = Some(note_id);
                pane.title = note.note.title;
                pane.content = note.content;
                pane.tags = note.tags;
                pane.backlinks = note
                    .backlinks
                    .into_iter()
                    .map(|bl| (bl.source_title, bl.source_id))
                    .collect();
                pane.scroll = 0;
                self.status_message = format!("Pane {}: {}", pane_idx + 1, self.split_panes[pane_idx].title);
            }
            Err(e) => self.status_message = format!("Error: {e}"),
        }
    }

    fn active_semantic(&self) -> Option<&SemanticEngine> {
        let id = self.manager.active_id()?;
        self.engines.get(&id).map(|(_, s)| s)
    }

    /// Load clusters from the active vault using K-means++ on embeddings.
    pub async fn load_clusters(&mut self) {
        let Some(ov) = self.manager.active() else {
            self.status_message = "No active vault".into();
            return;
        };
        let notes = match ov.vault.list_notes(1000, 0).await {
            Ok(n) => n,
            Err(e) => {
                self.status_message = format!("Error: {e}");
                return;
            }
        };

        let Some(semantic) = self.active_semantic() else {
            self.status_message = "Semantic engine unavailable".into();
            return;
        };

        // Embed each note
        let mut note_embeddings = Vec::new();
        for note in &notes {
            let text = format!("{}\n", note.title);
            if let Ok(Some(emb)) = semantic.embed(&text) {
                note_embeddings.push(mneme_ai::clustering::NoteEmbedding {
                    id: note.id,
                    title: note.title.clone(),
                    embedding: emb,
                });
            }
        }

        if note_embeddings.is_empty() {
            self.status_message = "No embeddings available for clustering".into();
            return;
        }

        let result = mneme_ai::clustering::cluster_notes(&note_embeddings, None, 8);
        self.clusters = result
            .clusters
            .into_iter()
            .map(|c| {
                let notes: Vec<(Uuid, String)> =
                    c.note_ids.into_iter().zip(c.note_titles).collect();
                (c.label, notes)
            })
            .collect();
        self.cluster_selected = 0;
        self.cluster_expanded = None;
        self.status_message = format!("{} cluster(s) found", self.clusters.len());
    }

    /// Load stale notes from the active vault.
    pub async fn load_stale_notes(&mut self) {
        let Some(ov) = self.manager.active() else {
            self.status_message = "No active vault".into();
            return;
        };
        let notes = match ov.vault.list_notes(1000, 0).await {
            Ok(n) => n,
            Err(e) => {
                self.status_message = format!("Error: {e}");
                return;
            }
        };

        // Build NoteContent entries (use note metadata only — no full content needed for staleness)
        let note_contents: Vec<mneme_ai::consolidation::NoteContent> = notes
            .iter()
            .map(|n| mneme_ai::consolidation::NoteContent {
                id: n.id,
                title: n.title.clone(),
                path: n.path.clone(),
                content: String::new(),
                updated_at: n.updated_at,
                last_accessed: n.last_accessed,
            })
            .collect();

        let stale = mneme_ai::consolidation::detect_stale(&note_contents, 30);
        self.stale_notes = stale
            .into_iter()
            .map(|s| (s.note_id, s.title, s.days_since_update, s.freshness_score))
            .collect();
        self.stale_selected = 0;
        self.status_message = format!("{} stale note(s)", self.stale_notes.len());
    }

    /// Load the vault list for the picker.
    pub fn load_vault_list(&mut self) {
        self.vault_list = self.manager.registry().list().to_vec();
        self.selected_index = 0;
    }

    /// Switch to a vault by its index in the vault list.
    pub async fn switch_vault_by_index(&mut self, index: usize) {
        if let Some(info) = self.vault_list.get(index).cloned() {
            match self.manager.switch_vault(info.id).await {
                Ok(()) => {
                    // Create engines if needed
                    if !self.engines.contains_key(&info.id) {
                        let eng = create_engines(&info.path, &self.models_dir);
                        self.engines.insert(info.id, eng);
                    }
                    self.status_message = format!("Switched to vault '{}'", info.name);
                    self.panel = Panel::NoteList;
                    self.load_notes().await;
                    self.load_tags().await;
                }
                Err(e) => self.status_message = format!("Error: {e}"),
            }
        }
    }

    /// Move selection up.
    pub fn select_prev(&mut self) {
        match self.panel {
            Panel::Graph => {
                if self.graph_layout.is_some() {
                    if let Some(sel) = self.graph_selected {
                        if sel > 0 {
                            self.graph_selected = Some(sel - 1);
                        }
                    }
                }
            }
            Panel::SplitView => {
                let pane = &mut self.split_panes[self.active_pane];
                pane.scroll = pane.scroll.saturating_sub(1);
            }
            _ => {
                if self.selected_index > 0 {
                    self.selected_index -= 1;
                }
            }
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        match self.panel {
            Panel::Graph => {
                if let Some(ref layout) = self.graph_layout {
                    if let Some(sel) = self.graph_selected {
                        if sel + 1 < layout.nodes.len() {
                            self.graph_selected = Some(sel + 1);
                        }
                    }
                }
            }
            Panel::SplitView => {
                let pane = &mut self.split_panes[self.active_pane];
                pane.scroll = pane.scroll.saturating_add(1);
            }
            _ => {
                let max = match self.panel {
                    Panel::NoteList => self.notes.len(),
                    Panel::Search => self.search_results.len(),
                    Panel::Tags => self.tag_list.len(),
                    Panel::VaultPicker => self.vault_list.len(),
                    Panel::Stale => self.stale_notes.len(),
                    Panel::Clusters => {
                        if let Some(idx) = self.cluster_expanded {
                            self.clusters.get(idx).map(|(_, n)| n.len()).unwrap_or(0)
                        } else {
                            self.clusters.len()
                        }
                    }
                    _ => return,
                };
                if self.selected_index + 1 < max {
                    self.selected_index += 1;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mneme_store::VaultManager;
    use tempfile::TempDir;

    async fn test_app() -> (App, TempDir) {
        let dir = TempDir::new().unwrap();
        let manager = VaultManager::single(dir.path()).await.unwrap();
        let models_dir = PathBuf::from("/nonexistent"); // no models for tests
        (App::new(manager, models_dir), dir)
    }

    #[tokio::test]
    async fn new_app_defaults() {
        let (app, _dir) = test_app().await;
        assert_eq!(app.panel, Panel::NoteList);
        assert_eq!(app.selected_index, 0);
        assert!(app.notes.is_empty());
        assert!(!app.should_quit);
    }

    #[tokio::test]
    async fn active_vault_name() {
        let (app, _dir) = test_app().await;
        assert_eq!(app.active_vault_name(), "default");
    }

    #[tokio::test]
    async fn load_notes_populates_list() {
        let (mut app, _dir) = test_app().await;
        use mneme_core::note::CreateNote;
        let ov = app.manager.active().unwrap();
        for i in 0..3 {
            ov.vault
                .create_note(CreateNote {
                    title: format!("Note {i}"),
                    path: None,
                    content: format!("Content {i}"),
                    tags: vec![],
                })
                .await
                .unwrap();
        }
        app.load_notes().await;
        assert_eq!(app.notes.len(), 3);
    }

    #[tokio::test]
    async fn select_navigation() {
        let (mut app, _dir) = test_app().await;
        use mneme_core::note::CreateNote;
        let ov = app.manager.active().unwrap();
        for i in 0..3 {
            ov.vault
                .create_note(CreateNote {
                    title: format!("Note {i}"),
                    path: None,
                    content: format!("Content {i}"),
                    tags: vec![],
                })
                .await
                .unwrap();
        }
        app.load_notes().await;

        app.select_next();
        assert_eq!(app.selected_index, 1);
        app.select_next();
        assert_eq!(app.selected_index, 2);
        app.select_next();
        assert_eq!(app.selected_index, 2);
        app.select_prev();
        assert_eq!(app.selected_index, 1);
    }

    #[tokio::test]
    async fn search_empty_query_clears() {
        let (mut app, _dir) = test_app().await;
        app.search_query = String::new();
        app.run_search();
        assert!(app.search_results.is_empty());
    }

    #[tokio::test]
    async fn vault_picker_navigation() {
        let (mut app, _dir) = test_app().await;
        app.load_vault_list();
        assert_eq!(app.vault_list.len(), 1);
        app.panel = Panel::VaultPicker;
        app.select_next();
        assert_eq!(app.selected_index, 0);
    }
}
