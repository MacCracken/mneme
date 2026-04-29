//! MCP JSON-RPC 2.0 protocol helpers.

use std::collections::HashMap;

use bote::{ToolDef, ToolSchema};
use serde_json::{Value, json};

/// Build a successful MCP tool result.
pub fn mcp_success(id: &Value, text: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{
                "type": "text",
                "text": text.into()
            }]
        }
    })
}

/// Build an error MCP tool result.
pub fn mcp_error(id: &Value, msg: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": {
            "content": [{
                "type": "text",
                "text": msg.into()
            }],
            "isError": true
        }
    })
}

/// Build a JSON-RPC error (protocol-level, not tool-level).
pub fn jsonrpc_error(id: &Value, code: i32, message: impl Into<String>) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": code,
            "message": message.into()
        }
    })
}

/// Tool schema definitions for `tools/list`.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef::new(
            "mneme_create_note",
            "Create a new note in the Mneme knowledge base with title, content, and optional tags",
            ToolSchema::new(
                "object",
                HashMap::from([
                    ("title".into(), json!({ "type": "string", "description": "Note title" })),
                    ("content".into(), json!({ "type": "string", "description": "Markdown content" })),
                    ("tags".into(), json!({ "type": "array", "items": { "type": "string" }, "description": "Tags to apply (e.g. [\"rust\", \"project/agnos\"])" })),
                    ("path".into(), json!({ "type": "string", "description": "Optional file path relative to vault (auto-generated from title if omitted)" })),
                    ("vault".into(), json!({ "type": "string", "description": "Optional vault name or ID (default: active vault)" })),
                ]),
                vec!["title".into(), "content".into()],
            ),
        ),
        ToolDef::new(
            "mneme_search",
            "Search notes by keyword, full-text query, or tag filter. Uses hybrid FTS + semantic search when available.",
            ToolSchema::new(
                "object",
                HashMap::from([
                    ("query".into(), json!({ "type": "string", "description": "Search query" })),
                    ("limit".into(), json!({ "type": "integer", "description": "Max results (default: 10)", "default": 10 })),
                    ("vault".into(), json!({ "type": "string", "description": "Optional vault name or ID (default: active vault)" })),
                ]),
                vec!["query".into()],
            ),
        ),
        ToolDef::new(
            "mneme_get_note",
            "Retrieve a note by ID with its full content, tags, and backlinks",
            ToolSchema::new(
                "object",
                HashMap::from([
                    ("id".into(), json!({ "type": "string", "description": "Note UUID" })),
                    ("vault".into(), json!({ "type": "string", "description": "Optional vault name or ID (default: active vault)" })),
                ]),
                vec!["id".into()],
            ),
        ),
        ToolDef::new(
            "mneme_update_note",
            "Update a note's title, content, or tags",
            ToolSchema::new(
                "object",
                HashMap::from([
                    ("id".into(), json!({ "type": "string", "description": "Note UUID" })),
                    ("title".into(), json!({ "type": "string", "description": "New title (optional)" })),
                    ("content".into(), json!({ "type": "string", "description": "New markdown content (optional)" })),
                    ("tags".into(), json!({ "type": "array", "items": { "type": "string" }, "description": "Replace tags (optional)" })),
                    ("vault".into(), json!({ "type": "string", "description": "Optional vault name or ID (default: active vault)" })),
                ]),
                vec!["id".into()],
            ),
        ),
        ToolDef::new(
            "mneme_query_graph",
            "Query the knowledge graph for notes related to a given note, tag, or concept",
            ToolSchema::new(
                "object",
                HashMap::from([
                    ("note_id".into(), json!({ "type": "string", "description": "Center node UUID (optional)" })),
                    ("tag".into(), json!({ "type": "string", "description": "Filter by tag name (optional)" })),
                    ("depth".into(), json!({ "type": "integer", "description": "Traversal depth (default: 1)", "default": 1 })),
                    ("vault".into(), json!({ "type": "string", "description": "Optional vault name or ID (default: active vault)" })),
                ]),
                vec![],
            ),
        ),
        ToolDef::new(
            "mneme_search_feedback",
            "Record that a search result was useful (clicked/opened). Improves future search ranking.",
            ToolSchema::new(
                "object",
                HashMap::from([
                    ("search_id".into(), json!({ "type": "string", "description": "The search_id from a previous mneme_search result" })),
                    ("note_id".into(), json!({ "type": "string", "description": "The note UUID that was useful" })),
                ]),
                vec!["search_id".into(), "note_id".into()],
            ),
        ),
        ToolDef::new(
            "mneme_list_vaults",
            "List all registered vaults with their status",
            ToolSchema::new("object", HashMap::new(), vec![]),
        ),
        ToolDef::new(
            "mneme_switch_vault",
            "Switch the active vault by name or ID",
            ToolSchema::new(
                "object",
                HashMap::from([
                    ("vault".into(), json!({ "type": "string", "description": "Vault name or UUID" })),
                ]),
                vec!["vault".into()],
            ),
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_success_format() {
        let resp = mcp_success(&json!(1), "hello");
        assert_eq!(resp["jsonrpc"], "2.0");
        assert_eq!(resp["id"], 1);
        assert_eq!(resp["result"]["content"][0]["text"], "hello");
    }

    #[test]
    fn mcp_error_format() {
        let resp = mcp_error(&json!(2), "something failed");
        assert_eq!(resp["result"]["isError"], true);
        assert_eq!(resp["result"]["content"][0]["text"], "something failed");
    }

    #[test]
    fn tool_definitions_count() {
        let tools = tool_definitions();
        assert_eq!(tools.len(), 8);
    }

    #[test]
    fn jsonrpc_error_format() {
        let resp = jsonrpc_error(&json!(3), -32601, "Method not found");
        assert_eq!(resp["jsonrpc"], "2.0");
        assert_eq!(resp["id"], 3);
        assert_eq!(resp["error"]["code"], -32601);
        assert_eq!(resp["error"]["message"], "Method not found");
    }

    #[test]
    fn tool_definitions_have_schemas() {
        let tools = tool_definitions();
        for tool in &tools {
            assert!(!tool.name.is_empty());
            assert!(!tool.description.is_empty());
            assert_eq!(tool.input_schema.schema_type, "object");
        }
    }

    #[test]
    fn tool_definitions_required_fields() {
        let tools = tool_definitions();
        // create_note requires title and content
        let create = tools.iter().find(|t| t.name == "mneme_create_note").unwrap();
        assert!(create.input_schema.required.contains(&"title".to_string()));
        assert!(create.input_schema.required.contains(&"content".to_string()));
        // search requires query
        let search = tools.iter().find(|t| t.name == "mneme_search").unwrap();
        assert!(search.input_schema.required.contains(&"query".to_string()));
    }

    #[test]
    fn tool_definitions_names() {
        let tools = tool_definitions();
        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"mneme_create_note"));
        assert!(names.contains(&"mneme_search"));
        assert!(names.contains(&"mneme_get_note"));
        assert!(names.contains(&"mneme_update_note"));
        assert!(names.contains(&"mneme_query_graph"));
        assert!(names.contains(&"mneme_list_vaults"));
        assert!(names.contains(&"mneme_switch_vault"));
    }

    #[test]
    fn vault_tools_exist() {
        let tools = tool_definitions();
        let list_vaults = tools.iter().find(|t| t.name == "mneme_list_vaults");
        assert!(list_vaults.is_some());
        let switch = tools.iter().find(|t| t.name == "mneme_switch_vault").unwrap();
        assert!(switch.input_schema.required.contains(&"vault".to_string()));
    }
}
