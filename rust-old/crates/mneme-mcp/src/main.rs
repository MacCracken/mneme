//! Mneme MCP server entry point — stdio JSON-RPC 2.0.

use std::path::PathBuf;

use anyhow::Result;
use serde_json::{Value, json};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

use mneme_mcp::protocol::{jsonrpc_error, tool_definitions};
use mneme_mcp::tools::{McpEngines, handle_tool_call};
use mneme_store::VaultManager;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("mneme_mcp=debug")
        .with_writer(std::io::stderr)
        .init();

    tracing::info!("mneme-mcp server starting (stdio)");

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

    let mut manager = VaultManager::single(&vault_dir).await?;

    // Create search engines for the default vault
    let mut engines = McpEngines::new();
    if let Some(ov) = manager.active() {
        engines.ensure(ov.info.id, &ov.info.path, &models_dir);
    }

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader.read_line(&mut line).await?;
        if n == 0 {
            break;
        }

        let request: Value = match serde_json::from_str(line.trim()) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("invalid JSON: {e}");
                continue;
            }
        };

        let response = handle_request(&request, &mut manager, &engines).await;
        let mut out = serde_json::to_string(&response)?;
        out.push('\n');
        stdout.write_all(out.as_bytes()).await?;
        stdout.flush().await?;
    }

    Ok(())
}

async fn handle_request(
    request: &Value,
    manager: &mut VaultManager,
    engines: &McpEngines,
) -> Value {
    let method = request.get("method").and_then(|m| m.as_str()).unwrap_or("");
    let id = request.get("id").cloned().unwrap_or(Value::Null);

    match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "mneme-mcp",
                    "version": env!("CARGO_PKG_VERSION"),
                },
                "capabilities": {
                    "tools": {}
                }
            }
        }),
        "tools/list" => {
            let tools: Vec<Value> = tool_definitions()
                .into_iter()
                .map(|t| json!({
                    "name": t.name,
                    "description": t.description,
                    "inputSchema": serde_json::to_value(&t.input_schema).unwrap_or_default(),
                }))
                .collect();
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "tools": tools
                }
            })
        }
        "tools/call" => {
            let params = request.get("params").cloned().unwrap_or(json!({}));
            let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            handle_tool_call(&id, tool_name, &args, manager, engines).await
        }
        _ => jsonrpc_error(&id, -32601, format!("unknown method: {method}")),
    }
}
