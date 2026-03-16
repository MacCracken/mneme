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

use mneme_store::VaultManager;
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

    let models_dir = std::env::var("MNEME_MODELS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::data_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("mneme")
                .join("models")
        });

    let manager = VaultManager::single(&vault_dir).await?;

    let mut app = App::new(manager, models_dir);
    app.load_notes().await;
    app.load_tags().await;

    enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;

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
                        if let Some((id, _, _)) =
                            app.search_results.get(app.selected_index).cloned()
                        {
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
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Esc => app.panel = Panel::NoteList,
                    KeyCode::Left => app.graph_center.0 -= 10.0 / app.graph_zoom,
                    KeyCode::Right => app.graph_center.0 += 10.0 / app.graph_zoom,
                    KeyCode::Up => app.graph_center.1 += 10.0 / app.graph_zoom,
                    KeyCode::Down => app.graph_center.1 -= 10.0 / app.graph_zoom,
                    KeyCode::Char('+') | KeyCode::Char('=') => {
                        app.graph_zoom = (app.graph_zoom * 1.2).min(10.0);
                    }
                    KeyCode::Char('-') => {
                        app.graph_zoom = (app.graph_zoom / 1.2).max(0.1);
                    }
                    KeyCode::Tab => {
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
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Esc => {
                        if let Some(id) = app.split_panes[0].note_id {
                            app.select_note(id).await;
                        } else {
                            app.panel = Panel::NoteList;
                        }
                    }
                    KeyCode::Tab => {
                        app.active_pane = 1 - app.active_pane;
                        app.status_message = format!("Active: Pane {}", app.active_pane + 1);
                    }
                    KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
                    KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                    KeyCode::Char('o') => {
                        app.split_pick_pane = Some(app.active_pane);
                        app.panel = Panel::NoteList;
                        app.selected_index = 0;
                        app.load_notes().await;
                        app.status_message =
                            format!("Pick a note for Pane {}", app.active_pane + 1);
                    }
                    _ => {}
                },
                Panel::VaultPicker => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => app.panel = Panel::NoteList,
                    KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
                    KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                    KeyCode::Enter => {
                        let idx = app.selected_index;
                        app.switch_vault_by_index(idx).await;
                    }
                    _ => {}
                },
                Panel::Stale => match key.code {
                    KeyCode::Esc => app.panel = Panel::NoteList,
                    KeyCode::Char('q') => app.should_quit = true,
                    KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
                    KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                    KeyCode::Enter => {
                        if let Some((id, _, _, _)) =
                            app.stale_notes.get(app.selected_index).cloned()
                        {
                            app.select_note(id).await;
                        }
                    }
                    _ => {}
                },
                _ => match key.code {
                    KeyCode::Char('q') => app.should_quit = true,
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
                    KeyCode::Char('v') => {
                        app.panel = Panel::VaultPicker;
                        app.load_vault_list();
                    }
                    KeyCode::Char('!') => {
                        app.panel = Panel::Stale;
                        app.selected_index = 0;
                        app.load_stale_notes().await;
                    }
                    KeyCode::Char('s') => {
                        if app.panel == Panel::NoteView {
                            if let Some(id) = app.selected_note_id {
                                app.panel = Panel::SplitView;
                                app.active_pane = 0;
                                app.load_pane(0, id).await;
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

    disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}
