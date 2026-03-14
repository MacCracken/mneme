//! Application state and event loop.

use uuid::Uuid;

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

    /// Move selection up.
    pub fn select_prev(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        let max = match self.panel {
            Panel::NoteList => self.notes.len(),
            Panel::Search => self.search_results.len(),
            Panel::Tags => self.tag_list.len(),
            Panel::NoteView => return,
        };
        if self.selected_index + 1 < max {
            self.selected_index += 1;
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
}
