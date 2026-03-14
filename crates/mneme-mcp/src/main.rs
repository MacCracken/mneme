//! Mneme MCP server entry point — stdio JSON-RPC 2.0.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use serde_json::{Value, json};
use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

use mneme_mcp::protocol::{jsonrpc_error, tool_definitions};
use mneme_mcp::tools::handle_tool_call;
use mneme_search::SearchEngine;
use mneme_store::Vault;

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

    let vault = Vault::open(&vault_dir).await?;
    let search_dir = vault_dir.join(".mneme").join("search-index");
    let search = SearchEngine::open(&search_dir)?;

    let vault = Arc::new(vault);
    let search = Arc::new(search);

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

        let response = handle_request(&request, &vault, &search).await;
        let mut out = serde_json::to_string(&response)?;
        out.push('\n');
        stdout.write_all(out.as_bytes()).await?;
        stdout.flush().await?;
    }

    Ok(())
}

async fn handle_request(request: &Value, vault: &Vault, search: &SearchEngine) -> Value {
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
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": tool_definitions()
            }
        }),
        "tools/call" => {
            let params = request.get("params").cloned().unwrap_or(json!({}));
            let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = params.get("arguments").cloned().unwrap_or(json!({}));
            handle_tool_call(&id, tool_name, &args, vault, search).await
        }
        _ => jsonrpc_error(&id, -32601, format!("unknown method: {method}")),
    }
}
