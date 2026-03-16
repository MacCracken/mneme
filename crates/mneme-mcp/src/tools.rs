//! MCP tool handlers — business logic for each tool.

use std::collections::HashMap;
use std::path::Path;

use serde_json::Value;
use uuid::Uuid;

use mneme_core::graph::{EdgeRelation, GraphEdge, GraphNode, NodeKind, Subgraph};
use mneme_core::note::{CreateNote, UpdateNote};
use mneme_search::{SearchEngine, SemanticEngine};
use mneme_store::VaultManager;
use mneme_store::manager::OpenVault;

use crate::protocol::{mcp_error, mcp_success};

/// Search engines and optimizer for open vaults.
pub struct McpEngines {
    pub engines: HashMap<Uuid, (SearchEngine, SemanticEngine, std::sync::Mutex<mneme_search::RetrievalOptimizer>)>,
}

impl McpEngines {
    pub fn new() -> Self {
        Self {
            engines: HashMap::new(),
        }
    }

    pub fn ensure(&mut self, id: Uuid, vault_path: &Path, models_dir: &Path) {
        if !self.engines.contains_key(&id) {
            let search_dir = vault_path.join(".mneme").join("search-index");
            let search = SearchEngine::open(&search_dir)
                .unwrap_or_else(|_| SearchEngine::in_memory().unwrap());
            let vectors_dir = vault_path.join(".mneme").join("vectors");
            let semantic = SemanticEngine::open(models_dir, &vectors_dir);
            let optimizer = mneme_search::RetrievalOptimizer::new();
            self.engines.insert(id, (search, semantic, std::sync::Mutex::new(optimizer)));
        }
    }

    pub fn get(&self, id: Uuid) -> Option<(&SearchEngine, &SemanticEngine, &std::sync::Mutex<mneme_search::RetrievalOptimizer>)> {
        self.engines.get(&id).map(|(s, se, o)| (s, se, o))
    }
}

/// Resolved vault with engines for a tool call.
struct ResolvedVault<'a> {
    vault: &'a OpenVault,
    search: &'a SearchEngine,
    semantic: &'a SemanticEngine,
    optimizer: &'a std::sync::Mutex<mneme_search::RetrievalOptimizer>,
}

fn resolve<'a>(
    args: &Value,
    manager: &'a VaultManager,
    engines: &'a McpEngines,
) -> Result<ResolvedVault<'a>, String> {
    let ov = if let Some(vault_ref) = args.get("vault").and_then(|v| v.as_str()) {
        let info = manager
            .registry()
            .resolve(vault_ref)
            .ok_or_else(|| format!("Vault not found: {vault_ref}"))?;
        manager
            .get(info.id)
            .ok_or_else(|| format!("Vault not open: {vault_ref}"))?
    } else {
        manager
            .active()
            .ok_or_else(|| "No active vault".to_string())?
    };

    let (search, semantic, optimizer) = engines
        .get(ov.info.id)
        .ok_or_else(|| "Search engines not initialized".to_string())?;

    Ok(ResolvedVault {
        vault: ov,
        search,
        semantic,
        optimizer,
    })
}

/// Dispatch a tool call to the appropriate handler.
pub async fn handle_tool_call(
    id: &Value,
    tool: &str,
    args: &Value,
    manager: &mut VaultManager,
    engines: &McpEngines,
) -> Value {
    match tool {
        "mneme_create_note" => handle_create_note(id, args, manager, engines).await,
        "mneme_search" => handle_search(id, args, manager, engines),
        "mneme_get_note" => handle_get_note(id, args, manager, engines).await,
        "mneme_update_note" => handle_update_note(id, args, manager, engines).await,
        "mneme_query_graph" => handle_query_graph(id, args, manager, engines).await,
        "mneme_search_feedback" => handle_search_feedback(id, args, engines),
        "mneme_list_vaults" => handle_list_vaults(id, manager),
        "mneme_switch_vault" => handle_switch_vault(id, args, manager).await,
        _ => mcp_error(id, format!("Unknown tool: {tool}")),
    }
}

async fn handle_create_note(
    id: &Value,
    args: &Value,
    manager: &VaultManager,
    engines: &McpEngines,
) -> Value {
    let rv = match resolve(args, manager, engines) {
        Ok(rv) => rv,
        Err(e) => return mcp_error(id, e),
    };

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
        provenance: None,
    };

    match rv.vault.vault.create_note(req).await {
        Ok(note) => {
            let _ = rv.search.index_note(
                note.note.id,
                &note.note.title,
                &note.content,
                &note.tags,
                &note.note.path,
            );
            let _ = rv
                .semantic
                .index_note(note.note.id, &note.note.title, &note.content);

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

fn handle_search(
    id: &Value,
    args: &Value,
    manager: &VaultManager,
    engines: &McpEngines,
) -> Value {
    let rv = match resolve(args, manager, engines) {
        Ok(rv) => rv,
        Err(e) => return mcp_error(id, e),
    };

    let query = match args.get("query").and_then(|q| q.as_str()) {
        Some(q) => q,
        None => return mcp_error(id, "Missing required parameter: query"),
    };

    let limit = args.get("limit").and_then(|l| l.as_u64()).unwrap_or(10) as usize;

    // Select arm from optimizer
    let (arm_idx, weights) = rv
        .optimizer
        .lock()
        .map(|opt| opt.select_arm())
        .unwrap_or((0, mneme_search::retrieval_optimizer::BlendWeights::default()));

    let ft_results = match rv.search.search(query, limit) {
        Ok(r) => r,
        Err(e) => return mcp_error(id, format!("Search failed: {e}")),
    };

    let sem_results = rv.semantic.search(query, limit).unwrap_or_default();

    if ft_results.is_empty() && sem_results.is_empty() {
        return mcp_success(id, "No notes found matching your query.");
    }

    // Record search
    if let Ok(mut opt) = rv.optimizer.lock() {
        opt.record_search(arm_idx);
    }

    let search_id = format!("s:{arm_idx}");
    let mut text = format!("search_id: {search_id}\n\n");

    if !sem_results.is_empty() {
        let ft_tuples: Vec<_> = ft_results
            .into_iter()
            .map(|r| (r.note_id, r.title, r.path, r.snippet, r.score))
            .collect();
        let hybrid =
            mneme_search::semantic::weighted_hybrid_merge(ft_tuples, sem_results, limit, &weights);

        text.push_str(&format!("Found {} result(s) (hybrid):\n\n", hybrid.len()));
        for (i, r) in hybrid.iter().enumerate() {
            text.push_str(&format!(
                "{}. **{}** (ID: {}, score: {:.3}, source: {:?})\n   {}\n\n",
                i + 1,
                r.title,
                r.note_id,
                r.score,
                r.source,
                r.snippet
            ));
        }
    } else {
        text.push_str(&format!("Found {} result(s):\n\n", ft_results.len()));
        for (i, r) in ft_results.iter().enumerate() {
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
    }

    mcp_success(id, text)
}

async fn handle_get_note(
    id: &Value,
    args: &Value,
    manager: &VaultManager,
    engines: &McpEngines,
) -> Value {
    let rv = match resolve(args, manager, engines) {
        Ok(rv) => rv,
        Err(e) => return mcp_error(id, e),
    };

    let note_id = match parse_uuid(args, "id") {
        Ok(id) => id,
        Err(e) => return mcp_error(id, e),
    };

    match rv.vault.vault.get_note(note_id).await {
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
    manager: &VaultManager,
    engines: &McpEngines,
) -> Value {
    let rv = match resolve(args, manager, engines) {
        Ok(rv) => rv,
        Err(e) => return mcp_error(id, e),
    };

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

    match rv.vault.vault.update_note(note_id, req).await {
        Ok(note) => {
            let _ = rv.search.index_note(
                note.note.id,
                &note.note.title,
                &note.content,
                &note.tags,
                &note.note.path,
            );
            let _ = rv
                .semantic
                .index_note(note.note.id, &note.note.title, &note.content);

            mcp_success(
                id,
                format!("Updated note '{}' (ID: {})", note.note.title, note.note.id),
            )
        }
        Err(e) => mcp_error(id, format!("Failed to update note: {e}")),
    }
}

async fn handle_query_graph(
    id: &Value,
    args: &Value,
    manager: &VaultManager,
    engines: &McpEngines,
) -> Value {
    let rv = match resolve(args, manager, engines) {
        Ok(rv) => rv,
        Err(e) => return mcp_error(id, e),
    };

    let note_id = args
        .get("note_id")
        .and_then(|v| v.as_str())
        .and_then(|s| Uuid::parse_str(s).ok());
    let tag_filter = args.get("tag").and_then(|t| t.as_str());
    let _depth = args.get("depth").and_then(|d| d.as_u64()).unwrap_or(1);

    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    if let Some(nid) = note_id {
        match rv.vault.vault.get_note(nid).await {
            Ok(note) => {
                nodes.push(GraphNode {
                    id: nid,
                    label: note.note.title.clone(),
                    kind: NodeKind::Note,
                });

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

                let db = rv.vault.vault.db();
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
        match rv.vault.vault.db().list_tags().await {
            Ok(tags) => {
                if let Some(tag) = tags.iter().find(|t| t.name == tag_name) {
                    let tag_node_id = Uuid::new_v5(&Uuid::NAMESPACE_OID, tag_name.as_bytes());
                    nodes.push(GraphNode {
                        id: tag_node_id,
                        label: tag.name.clone(),
                        kind: NodeKind::Tag,
                    });

                    if let Ok(all_notes) = rv.vault.vault.list_notes(100, 0).await {
                        for note in all_notes {
                            if let Ok(note_tags) = rv.vault.vault.db().get_note_tags(note.id).await
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

fn handle_search_feedback(id: &Value, args: &Value, engines: &McpEngines) -> Value {
    let search_id = match args.get("search_id").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return mcp_error(id, "Missing required parameter: search_id"),
    };

    let _note_id = match args.get("note_id").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return mcp_error(id, "Missing required parameter: note_id"),
    };

    // Parse arm index from search_id
    let arm_idx: usize = search_id
        .strip_prefix("s:")
        .and_then(|s| s.split(':').next())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    // Record feedback in the first available optimizer
    for (_, (_, _, optimizer)) in &engines.engines {
        if let Ok(mut opt) = optimizer.lock() {
            opt.record_feedback(arm_idx);
            break;
        }
    }

    mcp_success(id, "Feedback recorded. Thank you!")
}

fn handle_list_vaults(id: &Value, manager: &VaultManager) -> Value {
    let vaults = manager.registry().list();
    let active_id = manager.active_id();

    if vaults.is_empty() {
        return mcp_success(id, "No vaults registered.");
    }

    let mut text = format!("{} vault(s):\n\n", vaults.len());
    for v in vaults {
        let active = if active_id == Some(v.id) { " (active)" } else { "" };
        let default = if v.is_default { " [default]" } else { "" };
        text.push_str(&format!(
            "- **{}**{}{}\n  ID: {}\n  Path: {}\n\n",
            v.name,
            active,
            default,
            v.id,
            v.path.display()
        ));
    }
    mcp_success(id, text)
}

async fn handle_switch_vault(
    id: &Value,
    args: &Value,
    manager: &mut VaultManager,
) -> Value {
    let vault_ref = match args.get("vault").and_then(|v| v.as_str()) {
        Some(v) => v,
        None => return mcp_error(id, "Missing required parameter: vault"),
    };

    let vault_id = match manager.registry().resolve(vault_ref) {
        Some(info) => info.id,
        None => return mcp_error(id, format!("Vault not found: {vault_ref}")),
    };

    match manager.switch_vault(vault_id).await {
        Ok(()) => {
            let name = manager
                .registry()
                .get_by_id(vault_id)
                .map(|v| v.name.clone())
                .unwrap_or_default();
            mcp_success(id, format!("Switched to vault '{name}'"))
        }
        Err(e) => mcp_error(id, format!("Failed to switch vault: {e}")),
    }
}

fn parse_uuid(args: &Value, field: &str) -> Result<Uuid, String> {
    let s = args
        .get(field)
        .and_then(|v| v.as_str())
        .ok_or_else(|| format!("Missing required parameter: {field}"))?;
    Uuid::parse_str(s).map_err(|_| format!("Invalid UUID for {field}: {s}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mneme_store::VaultManager;
    use tempfile::TempDir;

    async fn test_env() -> (VaultManager, McpEngines, TempDir) {
        let dir = TempDir::new().unwrap();
        let mgr = VaultManager::single(dir.path()).await.unwrap();
        let mut engines = McpEngines::new();
        let id = mgr.active_id().unwrap();
        // Use in-memory engines for testing
        engines.engines.insert(
            id,
            (
                SearchEngine::in_memory().unwrap(),
                SemanticEngine::disabled(),
                std::sync::Mutex::new(mneme_search::RetrievalOptimizer::new()),
            ),
        );
        (mgr, engines, dir)
    }

    #[tokio::test]
    async fn create_note_tool() {
        let (mut mgr, engines, _dir) = test_env().await;
        let id = serde_json::json!(1);
        let args = serde_json::json!({
            "title": "Test Note",
            "content": "Hello from MCP",
            "tags": ["mcp", "test"]
        });
        let result = handle_tool_call(&id, "mneme_create_note", &args, &mut mgr, &engines).await;
        assert!(
            result["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("Created note")
        );
    }

    #[tokio::test]
    async fn create_note_missing_title() {
        let (mut mgr, engines, _dir) = test_env().await;
        let id = serde_json::json!(1);
        let args = serde_json::json!({"content": "No title"});
        let result = handle_tool_call(&id, "mneme_create_note", &args, &mut mgr, &engines).await;
        assert_eq!(result["result"]["isError"], true);
    }

    #[tokio::test]
    async fn search_tool_no_results() {
        let (mut mgr, engines, _dir) = test_env().await;
        let id = serde_json::json!(1);
        let args = serde_json::json!({"query": "nonexistent", "limit": 5});
        let result = handle_tool_call(&id, "mneme_search", &args, &mut mgr, &engines).await;
        assert!(
            result["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("No notes found")
        );
    }

    #[tokio::test]
    async fn search_tool_with_results() {
        let (mut mgr, engines, _dir) = test_env().await;
        let create_args = serde_json::json!({
            "title": "Rust Guide",
            "content": "Rust programming language guide",
            "tags": ["rust"]
        });
        handle_tool_call(
            &serde_json::json!(1),
            "mneme_create_note",
            &create_args,
            &mut mgr,
            &engines,
        )
        .await;

        let id = serde_json::json!(2);
        let args = serde_json::json!({"query": "rust", "limit": 5});
        let result = handle_tool_call(&id, "mneme_search", &args, &mut mgr, &engines).await;
        let text = result["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("result(s)") || text.contains("Rust Guide"));
    }

    #[tokio::test]
    async fn get_note_tool() {
        let (mut mgr, engines, _dir) = test_env().await;
        let ov = mgr.active().unwrap();
        let note = ov
            .vault
            .create_note(CreateNote {
                title: "Get Test".into(),
                path: None,
                content: "Content here.".into(),
                tags: vec!["test".into()],
                provenance: None,
            })
            .await
            .unwrap();

        let id = serde_json::json!(1);
        let args = serde_json::json!({"id": note.note.id.to_string()});
        let result = handle_tool_call(&id, "mneme_get_note", &args, &mut mgr, &engines).await;
        let text = result["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("Get Test"));
    }

    #[tokio::test]
    async fn unknown_tool() {
        let (mut mgr, engines, _dir) = test_env().await;
        let id = serde_json::json!(1);
        let result = handle_tool_call(
            &id,
            "nonexistent_tool",
            &serde_json::json!({}),
            &mut mgr,
            &engines,
        )
        .await;
        assert_eq!(result["result"]["isError"], true);
    }

    #[tokio::test]
    async fn list_vaults_tool() {
        let (mut mgr, engines, _dir) = test_env().await;
        let id = serde_json::json!(1);
        let result = handle_tool_call(
            &id,
            "mneme_list_vaults",
            &serde_json::json!({}),
            &mut mgr,
            &engines,
        )
        .await;
        let text = result["result"]["content"][0]["text"].as_str().unwrap();
        assert!(text.contains("vault(s)"));
    }
}
