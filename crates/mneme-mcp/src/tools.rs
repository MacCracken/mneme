//! MCP tool handlers — business logic for each tool.

use serde_json::Value;
use uuid::Uuid;

use mneme_core::graph::{EdgeRelation, GraphEdge, GraphNode, NodeKind, Subgraph};
use mneme_core::note::{CreateNote, UpdateNote};
use mneme_search::SearchEngine;
use mneme_store::Vault;

use crate::protocol::{mcp_error, mcp_success};

/// Dispatch a tool call to the appropriate handler.
pub async fn handle_tool_call(
    id: &Value,
    tool: &str,
    args: &Value,
    vault: &Vault,
    search: &SearchEngine,
) -> Value {
    match tool {
        "mneme_create_note" => handle_create_note(id, args, vault, search).await,
        "mneme_search" => handle_search(id, args, search),
        "mneme_get_note" => handle_get_note(id, args, vault).await,
        "mneme_update_note" => handle_update_note(id, args, vault, search).await,
        "mneme_query_graph" => handle_query_graph(id, args, vault).await,
        _ => mcp_error(id, format!("Unknown tool: {tool}")),
    }
}

async fn handle_create_note(
    id: &Value,
    args: &Value,
    vault: &Vault,
    search: &SearchEngine,
) -> Value {
    let title = match args.get("title").and_then(|t| t.as_str()) {
        Some(t) => t.to_string(),
        None => return mcp_error(id, "Missing required parameter: title"),
    };

    let content = match args.get("content").and_then(|c| c.as_str()) {
        Some(c) => c.to_string(),
        None => return mcp_error(id, "Missing required parameter: content"),
    };

    let tags: Vec<String> = args
        .get("tags")
        .and_then(|t| t.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let path = args.get("path").and_then(|p| p.as_str()).map(String::from);

    let req = CreateNote {
        title,
        path,
        content,
        tags,
    };

    match vault.create_note(req).await {
        Ok(note) => {
            // Index for search
            let _ = search.index_note(
                note.note.id,
                &note.note.title,
                &note.content,
                &note.tags,
                &note.note.path,
            );

            mcp_success(
                id,
                format!(
                    "Created note '{}' (ID: {}, path: {})",
                    note.note.title, note.note.id, note.note.path
                ),
            )
        }
        Err(e) => mcp_error(id, format!("Failed to create note: {e}")),
    }
}

fn handle_search(id: &Value, args: &Value, search: &SearchEngine) -> Value {
    let query = match args.get("query").and_then(|q| q.as_str()) {
        Some(q) => q,
        None => return mcp_error(id, "Missing required parameter: query"),
    };

    let limit = args.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as usize;

    match search.search(query, limit) {
        Ok(results) => {
            if results.is_empty() {
                return mcp_success(id, "No notes found matching your query.");
            }

            let mut text = format!("Found {} result(s):\n\n", results.len());
            for (i, r) in results.iter().enumerate() {
                text.push_str(&format!(
                    "{}. **{}** (ID: {}, score: {:.2})\n   Path: {}\n   {}\n\n",
                    i + 1,
                    r.title,
                    r.note_id,
                    r.score,
                    r.path,
                    r.snippet
                ));
            }
            mcp_success(id, text)
        }
        Err(e) => mcp_error(id, format!("Search failed: {e}")),
    }
}

async fn handle_get_note(id: &Value, args: &Value, vault: &Vault) -> Value {
    let note_id = match parse_uuid(args, "id") {
        Ok(id) => id,
        Err(e) => return mcp_error(id, e),
    };

    match vault.get_note(note_id).await {
        Ok(note) => {
            let mut text = format!(
                "# {}\n\nID: {}\nPath: {}\nTags: {}\nCreated: {}\nUpdated: {}\n",
                note.note.title,
                note.note.id,
                note.note.path,
                if note.tags.is_empty() {
                    "(none)".to_string()
                } else {
                    note.tags.join(", ")
                },
                note.note.created_at.format("%Y-%m-%d %H:%M"),
                note.note.updated_at.format("%Y-%m-%d %H:%M"),
            );

            if !note.backlinks.is_empty() {
                text.push_str(&format!("\nBacklinks ({}):\n", note.backlinks.len()));
                for bl in &note.backlinks {
                    text.push_str(&format!("  - {} ({})\n", bl.source_title, bl.link_text));
                }
            }

            text.push_str(&format!("\n---\n\n{}", note.content));
            mcp_success(id, text)
        }
        Err(e) => mcp_error(id, format!("Note not found: {e}")),
    }
}

async fn handle_update_note(
    id: &Value,
    args: &Value,
    vault: &Vault,
    search: &SearchEngine,
) -> Value {
    let note_id = match parse_uuid(args, "id") {
        Ok(id) => id,
        Err(e) => return mcp_error(id, e),
    };

    let title = args.get("title").and_then(|t| t.as_str()).map(String::from);
    let content = args
        .get("content")
        .and_then(|c| c.as_str())
        .map(String::from);
    let tags = args.get("tags").and_then(|t| t.as_array()).map(|arr| {
        arr.iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect()
    });

    if title.is_none() && content.is_none() && tags.is_none() {
        return mcp_error(
            id,
            "At least one of title, content, or tags must be provided",
        );
    }

    let req = UpdateNote {
        title,
        content,
        tags,
    };

    match vault.update_note(note_id, req).await {
        Ok(note) => {
            // Re-index
            let _ = search.index_note(
                note.note.id,
                &note.note.title,
                &note.content,
                &note.tags,
                &note.note.path,
            );

            mcp_success(
                id,
                format!("Updated note '{}' (ID: {})", note.note.title, note.note.id),
            )
        }
        Err(e) => mcp_error(id, format!("Failed to update note: {e}")),
    }
}

async fn handle_query_graph(id: &Value, args: &Value, vault: &Vault) -> Value {
    let note_id = args
        .get("note_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok());
    let tag_filter = args.get("tag").and_then(|t| t.as_str());
    let _depth = args.get("depth").and_then(|d| d.as_u64()).unwrap_or(1);

    // Build a subgraph from the vault data
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    if let Some(nid) = note_id {
        // Get the center note and its connections
        match vault.get_note(nid).await {
            Ok(note) => {
                nodes.push(GraphNode {
                    id: nid,
                    label: note.note.title.clone(),
                    kind: NodeKind::Note,
                });

                // Add tag nodes and edges
                for tag_name in &note.tags {
                    let tag_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, tag_name.as_bytes());
                    nodes.push(GraphNode {
                        id: tag_id,
                        label: tag_name.clone(),
                        kind: NodeKind::Tag,
                    });
                    edges.push(GraphEdge {
                        source: nid,
                        target: tag_id,
                        relation: EdgeRelation::TaggedWith,
                    });
                }

                // Add backlink edges
                for bl in &note.backlinks {
                    nodes.push(GraphNode {
                        id: bl.source_id,
                        label: bl.source_title.clone(),
                        kind: NodeKind::Note,
                    });
                    edges.push(GraphEdge {
                        source: bl.source_id,
                        target: nid,
                        relation: EdgeRelation::LinksTo,
                    });
                }

                // Add outgoing links
                let db = vault.db();
                if let Ok(links) = db.get_outgoing_links(nid).await {
                    for link in links {
                        if let Ok(target_note) = db.get_note(link.target_id).await {
                            nodes.push(GraphNode {
                                id: link.target_id,
                                label: target_note.title.clone(),
                                kind: NodeKind::Note,
                            });
                            edges.push(GraphEdge {
                                source: nid,
                                target: link.target_id,
                                relation: EdgeRelation::LinksTo,
                            });
                        }
                    }
                }
            }
            Err(e) => return mcp_error(id, format!("Note not found: {e}")),
        }
    } else if let Some(tag_name) = tag_filter {
        // List all notes with this tag
        // For now, search by tag name
        match vault.db().list_tags().await {
            Ok(tags) => {
                if let Some(tag) = tags.iter().find(|t| t.name == tag_name) {
                    let tag_node_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, tag_name.as_bytes());
                    nodes.push(GraphNode {
                        id: tag_node_id,
                        label: tag.name.clone(),
                        kind: NodeKind::Tag,
                    });

                    // Get all notes — for proper implementation we'd query by tag
                    if let Ok(all_notes) = vault.list_notes(100, 0).await {
                        for note in all_notes {
                            if let Ok(note_tags) = vault.db().get_note_tags(note.id).await
                                && note_tags.contains(&tag_name.to_string())
                            {
                                nodes.push(GraphNode {
                                    id: note.id,
                                    label: note.title.clone(),
                                    kind: NodeKind::Note,
                                });
                                edges.push(GraphEdge {
                                    source: note.id,
                                    target: tag_node_id,
                                    relation: EdgeRelation::TaggedWith,
                                });
                            }
                        }
                    }
                } else {
                    return mcp_error(id, format!("Tag not found: {tag_name}"));
                }
            }
            Err(e) => return mcp_error(id, format!("Failed to query tags: {e}")),
        }
    } else {
        return mcp_error(id, "Provide either note_id or tag to query the graph");
    }

    let graph = Subgraph { nodes, edges };
    let json = serde_json::to_string_pretty(&graph).unwrap_or_default();
    mcp_success(
        id,
        format!(
            "Graph: {} nodes, {} edges\n\n{}",
            graph.nodes.len(),
            graph.edges.len(),
            json
        ),
    )
}

fn parse_uuid(args: &Value, field: &str) -> Result<Uuid, String> {
    let s = args
        .get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("Missing required parameter: {field}"))?;
    Uuid::parse_str(s).map_err(|_| format!("Invalid UUID for {field}: {s}"))
}
