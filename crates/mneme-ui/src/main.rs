//! Mneme TUI entry point.

use std::path::PathBuf;

use anyhow::Result;
use crossterm::ExecutableCommand;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;

use mneme_search::SearchEngine;
use mneme_store::Vault;
use mneme_ui::app::{App, PaneState, Panel};
use mneme_ui::views;

#[tokio::main]
async fn main() -> Result<()> {
    let vault_dir = std::env::var("MNEME_VAULT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("mneme")
        });

    let vault = Vault::open(&vault_dir).await?;
    let search_dir = vault_dir.join(".mneme").join("search-index");
    let search = SearchEngine::open(&search_dir)?;

    let mut app = App::new(vault, search);
    app.load_notes().await;
    app.load_tags().await;

    // Terminal setup
    enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    loop {
        terminal.draw(|frame| views::render(frame, &app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            match app.panel {
                Panel::Search => match key.code {
                    KeyCode::Esc => {
                        app.panel = Panel::NoteList;
                        app.selected_index = 0;
                    }
                    KeyCode::Char(c) => {
                        app.search_query.push(c);
                        app.run_search();
                    }
                    KeyCode::Backspace => {
                        app.search_query.pop();
                        app.run_search();
                    }
                    KeyCode::Enter => {
                        if let Some((_id, _, _)) =
                            app.search_results.get(app.selected_index).cloned()
                        {
                            let id = _id;
                            // If we're picking for a split pane, load into that pane
                            if let Some(pane_idx) = app.split_pick_pane.take() {
                                app.load_pane(pane_idx, id).await;
                                app.panel = Panel::SplitView;
                            } else {
                                app.select_note(id).await;
                            }
                        }
                    }
                    KeyCode::Up => app.select_prev(),
                    KeyCode::Down => app.select_next(),
                    _ => {}
                },
                Panel::Graph => match key.code {
                    KeyCode::Char('q') => {
                        app.should_quit = true;
                    }
                    KeyCode::Esc => {
                        app.panel = Panel::NoteList;
                    }
                    KeyCode::Left => {
                        app.graph_center.0 -= 10.0 / app.graph_zoom;
                    }
                    KeyCode::Right => {
                        app.graph_center.0 += 10.0 / app.graph_zoom;
                    }
                    KeyCode::Up => {
                        app.graph_center.1 += 10.0 / app.graph_zoom;
                    }
                    KeyCode::Down => {
                        app.graph_center.1 -= 10.0 / app.graph_zoom;
                    }
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        app.graph_zoom = (app.graph_zoom * 1.2).min(10.0);
                    }
                    KeyCode::Char('-') => {
                        app.graph_zoom = (app.graph_zoom / 1.2).max(0.1);
                    }
                    KeyCode::Tab => {
                        // Cycle through nodes
                        if let Some(ref layout) = app.graph_layout {
                            if !layout.nodes.is_empty() {
                                let next = match app.graph_selected {
                                    Some(i) => (i + 1) % layout.nodes.len(),
                                    None => 0,
                                };
                                app.graph_selected = Some(next);
                            }
                        }
                    }
                    KeyCode::Enter => {
                        // Open selected node if it's a Note
                        if let (Some(sel), Some(layout)) =
                            (app.graph_selected, &app.graph_layout)
                        {
                            if let Some(node) = layout.nodes.get(sel) {
                                if node.kind == mneme_core::graph::NodeKind::Note {
                                    app.select_note(node.id).await;
                                }
                            }
                        }
                    }
                    _ => {}
                },
                Panel::SplitView => match key.code {
                    KeyCode::Char('q') => {
                        app.should_quit = true;
                    }
                    KeyCode::Esc => {
                        // Back to NoteView with left pane's note
                        if let Some(id) = app.split_panes[0].note_id {
                            app.select_note(id).await;
                        } else {
                            app.panel = Panel::NoteList;
                        }
                    }
                    KeyCode::Tab => {
                        app.active_pane = 1 - app.active_pane;
                        let pane_num = app.active_pane + 1;
                        app.status_message = format!("Active: Pane {pane_num}");
                    }
                    KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
                    KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                    KeyCode::Char('o') => {
                        // Open note picker for active pane
                        app.split_pick_pane = Some(app.active_pane);
                        app.panel = Panel::NoteList;
                        app.selected_index = 0;
                        app.load_notes().await;
                        app.status_message =
                            format!("Pick a note for Pane {}", app.active_pane + 1);
                    }
                    _ => {}
                },
                _ => match key.code {
                    KeyCode::Char('q') => {
                        app.should_quit = true;
                    }
                    KeyCode::Char('/') => {
                        app.panel = Panel::Search;
                        app.search_query.clear();
                        app.search_results.clear();
                        app.selected_index = 0;
                    }
                    KeyCode::Char('n') => {
                        app.panel = Panel::NoteList;
                        app.selected_index = 0;
                        app.load_notes().await;
                    }
                    KeyCode::Char('t') => {
                        app.panel = Panel::Tags;
                        app.selected_index = 0;
                        app.load_tags().await;
                    }
                    KeyCode::Char('g') => {
                        app.panel = Panel::Graph;
                        app.load_graph().await;
                    }
                    KeyCode::Char('s') => {
                        if app.panel == Panel::NoteView {
                            // Enter split view with current note in left pane
                            if let Some(id) = app.selected_note_id {
                                app.panel = Panel::SplitView;
                                app.active_pane = 0;
                                app.load_pane(0, id).await;
                                // Clear right pane
                                app.split_panes[1] = PaneState::default();
                                app.status_message = "Split view — Tab to switch panes, 'o' to open note".into();
                            }
                        }
                    }
                    KeyCode::Esc => {
                        if app.panel == Panel::NoteView {
                            app.panel = Panel::NoteList;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
                    KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                    KeyCode::Enter => {
                        if app.panel == Panel::NoteList {
                            if let Some(note) = app.notes.get(app.selected_index).cloned() {
                                // If picking for a split pane, load into that pane
                                if let Some(pane_idx) = app.split_pick_pane.take() {
                                    app.load_pane(pane_idx, note.id).await;
                                    app.panel = Panel::SplitView;
                                } else {
                                    app.select_note(note.id).await;
                                }
                            }
                        }
                    }
                    _ => {}
                },
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Cleanup
    disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
