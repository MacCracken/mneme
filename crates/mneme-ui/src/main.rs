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
use mneme_ui::app::{App, Panel};
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
                            app.select_note(id).await;
                        }
                    }
                    KeyCode::Up => app.select_prev(),
                    KeyCode::Down => app.select_next(),
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
                    KeyCode::Esc => {
                        if app.panel == Panel::NoteView {
                            app.panel = Panel::NoteList;
                        }
                    }
                    KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
                    KeyCode::Down | KeyCode::Char('j') => app.select_next(),
                    KeyCode::Enter => {
                        if app.panel == Panel::NoteList
                            && let Some(note) = app.notes.get(app.selected_index).cloned()
                        {
                            app.select_note(note.id).await;
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
