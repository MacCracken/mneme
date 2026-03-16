//! TUI view rendering with ratatui.

use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::widgets::canvas::{Canvas, Line as CanvasLine, Points};

use mneme_core::graph::NodeKind;

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
        Panel::Graph => render_graph(frame, app, chunks[0]),
        Panel::SplitView => render_split_view(frame, app, chunks[0]),
        Panel::VaultPicker => render_vault_picker(frame, app, chunks[0]),
        Panel::Stale => render_stale(frame, app, chunks[0]),
        Panel::Clusters => render_clusters(frame, app, chunks[0]),
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
            help_text(app.panel),
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

fn render_graph(frame: &mut Frame, app: &App, area: Rect) {
    let layout = match &app.graph_layout {
        Some(l) => l,
        None => {
            let empty = Paragraph::new("No graph data. Press 'g' to load.")
                .block(Block::default().borders(Borders::ALL).title(" Graph "));
            frame.render_widget(empty, area);
            return;
        }
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(30)])
        .split(area);

    // Build id->index map for edge drawing
    let id_to_idx: std::collections::HashMap<uuid::Uuid, usize> = layout
        .nodes
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id, i))
        .collect();

    // Determine bounds for canvas
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
    for node in &layout.nodes {
        min_x = min_x.min(node.x);
        max_x = max_x.max(node.x);
        min_y = min_y.min(node.y);
        max_y = max_y.max(node.y);
    }
    // Add padding
    let pad = 20.0;
    min_x -= pad;
    max_x += pad;
    min_y -= pad;
    max_y += pad;

    // Apply zoom and pan
    let cx = app.graph_center.0;
    let cy = app.graph_center.1;
    let z = app.graph_zoom;
    let view_min_x = cx + min_x / z;
    let view_max_x = cx + max_x / z;
    let view_min_y = cy + min_y / z;
    let view_max_y = cy + max_y / z;

    let selected_idx = app.graph_selected;

    let canvas = Canvas::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Knowledge Graph ")
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .x_bounds([view_min_x, view_max_x])
        .y_bounds([view_min_y, view_max_y])
        .paint(move |ctx| {
            // Draw edges
            for edge in &layout.edges {
                if let (Some(&si), Some(&ti)) = (id_to_idx.get(&edge.source), id_to_idx.get(&edge.target)) {
                    let s = &layout.nodes[si];
                    let t = &layout.nodes[ti];
                    ctx.draw(&CanvasLine {
                        x1: s.x,
                        y1: s.y,
                        x2: t.x,
                        y2: t.y,
                        color: Color::DarkGray,
                    });
                }
            }

            // Draw nodes as points
            for (i, node) in layout.nodes.iter().enumerate() {
                let color = if Some(i) == selected_idx {
                    Color::White
                } else {
                    match node.kind {
                        NodeKind::Note => Color::Cyan,
                        NodeKind::Tag => Color::Green,
                        NodeKind::Concept => Color::Yellow,
                    }
                };
                ctx.draw(&Points {
                    coords: &[(node.x, node.y)],
                    color,
                });
                ctx.print(node.x + 1.0, node.y, node.label.clone());
            }
        });

    frame.render_widget(canvas, chunks[0]);

    // Info sidebar
    let info_lines: Vec<Line> = if let Some(sel) = app.graph_selected {
        if let Some(ref layout) = app.graph_layout {
            if let Some(node) = layout.nodes.get(sel) {
                let kind_str = match node.kind {
                    NodeKind::Note => "Note",
                    NodeKind::Tag => "Tag",
                    NodeKind::Concept => "Concept",
                };
                let connections = layout
                    .edges
                    .iter()
                    .filter(|e| e.source == node.id || e.target == node.id)
                    .count();
                vec![
                    Line::from(Span::styled(
                        &node.label,
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(format!("Type: {kind_str}")),
                    Line::from(format!("Connections: {connections}")),
                    Line::from(format!("Position: ({:.0}, {:.0})", node.x, node.y)),
                ]
            } else {
                vec![Line::from("No node selected")]
            }
        } else {
            vec![Line::from("No graph data")]
        }
    } else {
        vec![Line::from("No node selected")]
    };

    let info = Paragraph::new(info_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Node Info "),
    );
    frame.render_widget(info, chunks[1]);
}

fn render_split_view(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    for (i, pane) in app.split_panes.iter().enumerate() {
        let is_active = i == app.active_pane;
        let border_color = if is_active { Color::Cyan } else { Color::DarkGray };
        let title = if pane.title.is_empty() {
            format!(" Pane {} (empty) ", i + 1)
        } else {
            format!(" {} ", pane.title)
        };

        let content = Paragraph::new(pane.content.as_str())
            .wrap(Wrap { trim: false })
            .scroll((pane.scroll, 0))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(Style::default().fg(border_color)),
            );
        frame.render_widget(content, chunks[i]);
    }
}

fn render_vault_picker(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .vault_list
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let active = if app.manager.active_id() == Some(v.id) {
                " (active)"
            } else {
                ""
            };
            let default = if v.is_default { " [default]" } else { "" };
            let style = if i == app.selected_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(format!(
                "  {} {}{}",
                v.name, active, default
            )))
            .style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Vaults — Enter to switch, Esc to cancel "),
    );
    frame.render_widget(list, area);
}

fn render_clusters(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(expanded_idx) = app.cluster_expanded {
        // Show notes within a cluster
        let cluster = &app.clusters[expanded_idx];
        let items: Vec<ListItem> = cluster
            .1
            .iter()
            .enumerate()
            .map(|(i, (_, title))| {
                let style = if i == app.selected_index {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Span::styled(format!("  {title}"), style))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " {} ({} notes) — Esc to go back ",
                    cluster.0,
                    cluster.1.len()
                )),
        );
        frame.render_widget(list, area);
    } else {
        // Show cluster list
        let items: Vec<ListItem> = app
            .clusters
            .iter()
            .enumerate()
            .map(|(i, (label, notes))| {
                let style = if i == app.selected_index {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:>3}  ", notes.len()),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(label, style),
                ]))
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(
                    " Clusters ({}) — Enter to expand ",
                    app.clusters.len()
                )),
        );
        frame.render_widget(list, area);
    }
}

fn render_stale(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .stale_notes
        .iter()
        .enumerate()
        .map(|(i, (_, title, days, freshness))| {
            let style = if i == app.selected_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(
                    format!("{days:>4}d  "),
                    Style::default().fg(Color::Red),
                ),
                Span::styled(
                    format!("{freshness:>5.1}  "),
                    Style::default().fg(Color::Magenta),
                ),
                Span::styled(title, style),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(
                " Stale Notes ({}) — days | freshness | title ",
                app.stale_notes.len()
            )),
    );
    frame.render_widget(list, area);
}

fn panel_name(panel: Panel) -> &'static str {
    match panel {
        Panel::NoteList => "NOTES",
        Panel::NoteView => "VIEW",
        Panel::Search => "SEARCH",
        Panel::Tags => "TAGS",
        Panel::Graph => "GRAPH",
        Panel::SplitView => "SPLIT",
        Panel::VaultPicker => "VAULTS",
        Panel::Stale => "STALE",
        Panel::Clusters => "CLUSTERS",
    }
}

fn help_text(panel: Panel) -> &'static str {
    match panel {
        Panel::Graph => "[q]uit [Esc]back [arrows]pan [+/-]zoom [Tab]cycle [Enter]open",
        Panel::SplitView => "[q]uit [Esc]back [Tab]switch pane [j/k]scroll [o]open note",
        Panel::Stale => "[q]uit [Esc]back [Enter]open [j/k]navigate",
        Panel::Clusters => "[q]uit [Esc]back [Enter]expand/open [j/k]navigate",
        _ => "[q]uit [/]search [n]otes [t]ags [g]raph [s]plit [!]stale [c]lusters [?]help",
    }
}
