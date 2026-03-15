//! Application state and event loop.

use uuid::Uuid;

use mneme_core::graph::{
    EdgeRelation, GraphEdge, GraphLayout, GraphNode, NodeKind, Subgraph,
};
use mneme_core::note::Note;
use mneme_search::SearchEngine;
use mneme_store::Vault;

/// Active panel in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Panel {
    NoteList,
    NoteView,
    Search,
    Tags,
    Graph,
    SplitView,
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
    pub vault: Vault,
    pub search: SearchEngine,
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
    /// When picking a note for a split pane, remember which pane to load into.
    pub split_pick_pane: Option<usize>,
}

impl App {
    pub fn new(vault: Vault, search: SearchEngine) -> Self {
        Self {
            vault,
            search,
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
            status_message: "Press ? for help".into(),
            should_quit: false,
            graph_layout: None,
            graph_center: (0.0, 0.0),
            graph_zoom: 1.0,
            graph_selected: None,
            split_panes: [PaneState::default(), PaneState::default()],
            active_pane: 0,
            split_pick_pane: None,
        }
    }

    /// Load the note list from the vault.
    pub async fn load_notes(&mut self) {
        match self.vault.list_notes(100, 0).await {
            Ok(notes) => {
                self.notes = notes;
                if self.selected_index >= self.notes.len() && !self.notes.is_empty() {
                    self.selected_index = self.notes.len() - 1;
                }
            }
            Err(e) => self.status_message = format!("Error: {e}"),
        }
    }

    /// Load tags.
    pub async fn load_tags(&mut self) {
        match self.vault.list_tags().await {
            Ok(tags) => self.tag_list = tags.into_iter().map(|t| t.name).collect(),
            Err(e) => self.status_message = format!("Error: {e}"),
        }
    }

    /// Select and load a note.
    pub async fn select_note(&mut self, id: Uuid) {
        match self.vault.get_note(id).await {
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
            }
            Err(e) => self.status_message = format!("Error: {e}"),
        }
    }

    /// Run a search query.
    pub fn run_search(&mut self) {
        if self.search_query.is_empty() {
            self.search_results.clear();
            return;
        }

        match self.search.search(&self.search_query, 20) {
            Ok(results) => {
                self.search_results = results
                    .into_iter()
                    .map(|r| (r.note_id, r.title, r.score))
                    .collect();
                self.status_message = format!("{} result(s)", self.search_results.len());
            }
            Err(e) => self.status_message = format!("Search error: {e}"),
        }
    }

    /// Build and lay out the knowledge graph from all notes, tags, and links.
    pub async fn load_graph(&mut self) {
        let notes = match self.vault.list_notes(1000, 0).await {
            Ok(n) => n,
            Err(e) => {
                self.status_message = format!("Graph error: {e}");
                return;
            }
        };
        let tags = match self.vault.list_tags().await {
            Ok(t) => t,
            Err(e) => {
                self.status_message = format!("Graph error: {e}");
                return;
            }
        };
        let links = match self.vault.list_all_links().await {
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

        // Note-to-note edges from links
        let mut edges: Vec<GraphEdge> = links
            .iter()
            .map(|l| GraphEdge {
                source: l.source_id,
                target: l.target_id,
                relation: EdgeRelation::LinksTo,
            })
            .collect();

        // Note-tag edges: query tags for each note
        for note in &notes {
            if let Ok(note_tags) = self.vault.db().get_note_tags(note.id).await {
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
        match self.vault.get_note(note_id).await {
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
    use mneme_search::SearchEngine;
    use mneme_store::Vault;
    use tempfile::TempDir;

    async fn test_app() -> (App, TempDir) {
        let dir = TempDir::new().unwrap();
        let vault = Vault::open(dir.path()).await.unwrap();
        let search = SearchEngine::in_memory().unwrap();
        (App::new(vault, search), dir)
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
    async fn select_prev_at_zero() {
        let (mut app, _dir) = test_app().await;
        app.select_prev();
        assert_eq!(app.selected_index, 0); // stays at 0
    }

    #[tokio::test]
    async fn select_next_empty_list() {
        let (mut app, _dir) = test_app().await;
        app.select_next();
        assert_eq!(app.selected_index, 0); // stays at 0 when empty
    }

    #[tokio::test]
    async fn load_notes_populates_list() {
        let (mut app, _dir) = test_app().await;
        // Create some notes first
        use mneme_core::note::CreateNote;
        for i in 0..3 {
            app.vault
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
        for i in 0..3 {
            app.vault
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
        assert_eq!(app.selected_index, 2); // can't go past end
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
    async fn load_tags_works() {
        let (mut app, _dir) = test_app().await;
        use mneme_core::note::CreateNote;
        app.vault
            .create_note(CreateNote {
                title: "Tagged".into(),
                path: None,
                content: "Content".into(),
                tags: vec!["alpha".into(), "beta".into()],
            })
            .await
            .unwrap();
        app.load_tags().await;
        assert_eq!(app.tag_list.len(), 2);
    }

    #[tokio::test]
    async fn select_note_loads_content() {
        let (mut app, _dir) = test_app().await;
        use mneme_core::note::CreateNote;
        let note = app
            .vault
            .create_note(CreateNote {
                title: "View Me".into(),
                path: None,
                content: "Detailed content here.".into(),
                tags: vec!["tag1".into()],
            })
            .await
            .unwrap();

        app.select_note(note.note.id).await;
        assert_eq!(app.panel, Panel::NoteView);
        assert_eq!(app.selected_note_id, Some(note.note.id));
        assert_eq!(app.note_content, "Detailed content here.");
        assert_eq!(app.note_tags, vec!["tag1"]);
        assert!(app.status_message.contains("View Me"));
    }

    #[tokio::test]
    async fn select_note_not_found() {
        let (mut app, _dir) = test_app().await;
        app.select_note(uuid::Uuid::new_v4()).await;
        assert!(app.status_message.contains("Error"));
    }

    #[tokio::test]
    async fn run_search_with_results() {
        let (mut app, _dir) = test_app().await;
        use mneme_core::note::CreateNote;
        let note = app
            .vault
            .create_note(CreateNote {
                title: "Rust Guide".into(),
                path: None,
                content: "Rust programming language guide.".into(),
                tags: vec!["rust".into()],
            })
            .await
            .unwrap();
        // Index it
        let _ = app.search.index_note(
            note.note.id,
            &note.note.title,
            &note.content,
            &note.tags,
            &note.note.path,
        );

        app.search_query = "rust".into();
        app.run_search();
        assert!(!app.search_results.is_empty());
        assert!(app.status_message.contains("result"));
    }

    #[tokio::test]
    async fn select_next_in_search_panel() {
        let (mut app, _dir) = test_app().await;
        app.panel = Panel::Search;
        app.search_results = vec![
            (uuid::Uuid::new_v4(), "R1".into(), 1.0),
            (uuid::Uuid::new_v4(), "R2".into(), 0.5),
        ];
        app.selected_index = 0;
        app.select_next();
        assert_eq!(app.selected_index, 1);
        app.select_next();
        assert_eq!(app.selected_index, 1); // can't go past end
    }

    #[tokio::test]
    async fn select_next_in_tags_panel() {
        let (mut app, _dir) = test_app().await;
        app.panel = Panel::Tags;
        app.tag_list = vec!["a".into(), "b".into()];
        app.selected_index = 0;
        app.select_next();
        assert_eq!(app.selected_index, 1);
    }

    #[tokio::test]
    async fn select_next_in_note_view_noop() {
        let (mut app, _dir) = test_app().await;
        app.panel = Panel::NoteView;
        app.selected_index = 0;
        app.select_next();
        assert_eq!(app.selected_index, 0); // no-op in NoteView
    }
}
