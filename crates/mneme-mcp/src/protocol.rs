//! MCP JSON-RPC 2.0 protocol helpers.

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
pub fn tool_definitions() -> Value {
    json!([
        {
            "name": "mneme_create_note",
            "description": "Create a new note in the Mneme knowledge base with title, content, and optional tags",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title": { "type": "string", "description": "Note title" },
                    "content": { "type": "string", "description": "Markdown content" },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Tags to apply (e.g. [\"rust\", \"project/agnos\"])"
                    },
                    "path": { "type": "string", "description": "Optional file path relative to vault (auto-generated from title if omitted)" },
                    "vault": { "type": "string", "description": "Optional vault name or ID (default: active vault)" }
                },
                "required": ["title", "content"]
            }
        },
        {
            "name": "mneme_search",
            "description": "Search notes by keyword, full-text query, or tag filter. Uses hybrid FTS + semantic search when available.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query" },
                    "limit": { "type": "integer", "description": "Max results (default: 10)", "default": 10 },
                    "vault": { "type": "string", "description": "Optional vault name or ID (default: active vault)" }
                },
                "required": ["query"]
            }
        },
        {
            "name": "mneme_get_note",
            "description": "Retrieve a note by ID with its full content, tags, and backlinks",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Note UUID" },
                    "vault": { "type": "string", "description": "Optional vault name or ID (default: active vault)" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "mneme_update_note",
            "description": "Update a note's title, content, or tags",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Note UUID" },
                    "title": { "type": "string", "description": "New title (optional)" },
                    "content": { "type": "string", "description": "New markdown content (optional)" },
                    "tags": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Replace tags (optional)"
                    },
                    "vault": { "type": "string", "description": "Optional vault name or ID (default: active vault)" }
                },
                "required": ["id"]
            }
        },
        {
            "name": "mneme_query_graph",
            "description": "Query the knowledge graph for notes related to a given note, tag, or concept",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "note_id": { "type": "string", "description": "Center node UUID (optional)" },
                    "tag": { "type": "string", "description": "Filter by tag name (optional)" },
                    "depth": { "type": "integer", "description": "Traversal depth (default: 1)", "default": 1 },
                    "vault": { "type": "string", "description": "Optional vault name or ID (default: active vault)" }
                }
            }
        },
        {
            "name": "mneme_list_vaults",
            "description": "List all registered vaults with their status",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        },
        {
            "name": "mneme_switch_vault",
            "description": "Switch the active vault by name or ID",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "vault": { "type": "string", "description": "Vault name or UUID" }
                },
                "required": ["vault"]
            }
        }
    ])
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
        assert_eq!(tools.as_array().unwrap().len(), 7);
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
        for tool in tools.as_array().unwrap() {
            assert!(tool["name"].is_string());
            assert!(tool["description"].is_string());
            assert!(tool["inputSchema"]["type"].as_str() == Some("object"));
        }
    }

    #[test]
    fn tool_definitions_required_fields() {
        let tools = tool_definitions();
        let arr = tools.as_array().unwrap();
        // create_note requires title and content
        let create = arr
            .iter()
            .find(|t| t["name"] == "mneme_create_note")
            .unwrap();
        let required = create["inputSchema"]["required"].as_array().unwrap();
        assert!(required.iter().any(|r| r == "title"));
        assert!(required.iter().any(|r| r == "content"));
        // search requires query
        let search = arr.iter().find(|t| t["name"] == "mneme_search").unwrap();
        let required = search["inputSchema"]["required"].as_array().unwrap();
        assert!(required.iter().any(|r| r == "query"));
    }

    #[test]
    fn tool_definitions_names() {
        let tools = tool_definitions();
        let names: Vec<&str> = tools
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
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
        let arr = tools.as_array().unwrap();
        let list_vaults = arr.iter().find(|t| t["name"] == "mneme_list_vaults");
        assert!(list_vaults.is_some());
        let switch = arr.iter().find(|t| t["name"] == "mneme_switch_vault");
        assert!(switch.is_some());
        let required = switch.unwrap()["inputSchema"]["required"]
            .as_array()
            .unwrap();
        assert!(required.iter().any(|r| r == "vault"));
    }
}
