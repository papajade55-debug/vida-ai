use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use vida_core::{McpServerInfo, McpTool, McpToolResult, VidaEngine};
use vida_db::McpServerConfigRow;

#[tauri::command]
pub async fn start_mcp_server(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    name: String,
) -> Result<Vec<McpTool>, String> {
    let mut e = engine.write().await;
    e.start_mcp_server(&name)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_mcp_server(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    name: String,
) -> Result<(), String> {
    let mut e = engine.write().await;
    e.stop_mcp_server(&name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_mcp_servers(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<Vec<McpServerInfo>, String> {
    let e = engine.read().await;
    e.list_mcp_servers()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_mcp_tools(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<Vec<McpTool>, String> {
    let e = engine.read().await;
    Ok(e.list_mcp_tools())
}

#[tauri::command]
pub async fn call_mcp_tool(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    tool_name: String,
    arguments: serde_json::Value,
) -> Result<McpToolResult, String> {
    let mut e = engine.write().await;
    e.call_mcp_tool(&tool_name, arguments)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_mcp_server_config(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    config: McpServerConfigRow,
) -> Result<(), String> {
    let e = engine.read().await;
    e.save_mcp_server_config(&config)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_mcp_server_config(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    id: String,
) -> Result<(), String> {
    let mut e = engine.write().await;
    e.delete_mcp_server_config(&id)
        .await
        .map_err(|e| e.to_string())
}
