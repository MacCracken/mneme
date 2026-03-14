//! TUI view rendering with ratatui.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::app::{App, Panel};

/// Render the full TUI layout.
pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Main content
            Constraint::Length(1), // Status bar
        ])
        .split(frame.area());

    match app.panel {
        Panel::NoteList => render_note_list(frame, app, chunks[0]),
        Panel::NoteView => render_note_view(frame, app, chunks[0]),
        Panel::Search => render_search(frame, app, chunks[0]),
        Panel::Tags => render_tags(frame, app, chunks[0]),
    }

    // Status bar
    let status = Paragraph::new(Line::from(vec![
        Span::styled(
            format!(" {} ", panel_name(app.panel)),
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::raw(&app.status_message),
        Span::raw("  "),
        Span::styled(
            "[q]uit [/]search [n]otes [t]ags [?]help",
            Style::default().fg(Color::DarkGray),
        ),
    ]));
    frame.render_widget(status, chunks[1]);
}

fn render_note_list(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .notes
        .iter()
        .enumerate()
        .map(|(i, note)| {
            let style = if i == app.selected_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let date = note.updated_at.format("%m/%d");
            ListItem::new(Line::from(vec![
                Span::styled(format!("{date}  "), Style::default().fg(Color::DarkGray)),
                Span::styled(&note.title, style),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Notes ({}) ", app.notes.len())),
    );

    frame.render_widget(list, area);
}

fn render_note_view(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(75), // Note content
            Constraint::Percentage(25), // Sidebar (tags + backlinks)
        ])
        .split(area);

    // Note content
    let content = Paragraph::new(app.note_content.as_str())
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Note ")
                .border_style(Style::default().fg(Color::Cyan)),
        );
    frame.render_widget(content, chunks[0]);

    // Sidebar
    let sidebar_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40), // Tags
            Constraint::Percentage(60), // Backlinks
        ])
        .split(chunks[1]);

    // Tags
    let tag_items: Vec<ListItem> = app
        .note_tags
        .iter()
        .map(|t| {
            ListItem::new(Span::styled(
                format!("  #{t}"),
                Style::default().fg(Color::Green),
            ))
        })
        .collect();
    let tags_list = List::new(tag_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Tags ({}) ", app.note_tags.len())),
    );
    frame.render_widget(tags_list, sidebar_chunks[0]);

    // Backlinks
    let bl_items: Vec<ListItem> = app
        .note_backlinks
        .iter()
        .map(|(title, _id)| {
            ListItem::new(Span::styled(
                format!("  ← {title}"),
                Style::default().fg(Color::Magenta),
            ))
        })
        .collect();
    let bl_list = List::new(bl_items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Backlinks ({}) ", app.note_backlinks.len())),
    );
    frame.render_widget(bl_list, sidebar_chunks[1]);
}

fn render_search(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search input
            Constraint::Min(1),    // Results
        ])
        .split(area);

    // Search input
    let input = Paragraph::new(format!("  {}", app.search_query)).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Search ")
            .border_style(Style::default().fg(Color::Yellow)),
    );
    frame.render_widget(input, chunks[0]);

    // Results
    let items: Vec<ListItem> = app
        .search_results
        .iter()
        .enumerate()
        .map(|(i, (_id, title, score))| {
            let style = if i == app.selected_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{score:.2}  "),
                    Style::default().fg(Color::DarkGray),
                ),
                Span::styled(title, style),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Results ({}) ", app.search_results.len())),
    );
    frame.render_widget(list, chunks[1]);
}

fn render_tags(frame: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let items: Vec<ListItem> = app
        .tag_list
        .iter()
        .enumerate()
        .map(|(i, tag)| {
            let style = if i == app.selected_index {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Green)
            };
            ListItem::new(Span::styled(format!("  #{tag}"), style))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Tags ({}) ", app.tag_list.len())),
    );
    frame.render_widget(list, area);
}

fn panel_name(panel: Panel) -> &'static str {
    match panel {
        Panel::NoteList => "NOTES",
        Panel::NoteView => "VIEW",
        Panel::Search => "SEARCH",
        Panel::Tags => "TAGS",
    }
}
