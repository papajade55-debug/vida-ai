use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::RwLock;
use vida_core::{
    classify_path, infer_tool_action, AccessAction, AccessResource, AgentToolContext,
    McpServerInfo, McpTool, McpToolResult, PermissionType, VidaEngine,
};
use vida_db::McpServerConfigRow;

use super::access::require_access;
use super::permissions::{require_permission, PermissionState};

fn infer_tool_permission(tool_name: &str) -> PermissionType {
    let lower = tool_name.to_ascii_lowercase();

    if [
        "write", "edit", "create", "delete", "remove", "move", "rename", "save",
    ]
    .iter()
    .any(|needle| lower.contains(needle))
    {
        PermissionType::FileWrite
    } else if ["read", "list", "find", "search", "glob", "stat"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        PermissionType::FileRead
    } else if ["http", "web", "fetch", "download", "request", "url"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        PermissionType::NetworkAccess
    } else {
        PermissionType::ShellExecute
    }
}

fn extract_tool_path(arguments: &serde_json::Value) -> Option<String> {
    arguments
        .get("path")
        .or_else(|| arguments.get("file"))
        .or_else(|| arguments.get("filepath"))
        .and_then(|value| value.as_str())
        .map(str::to_string)
}

fn classify_tool_resource(engine: &VidaEngine, path: Option<&str>) -> AccessResource {
    let Some(path) = path else {
        return AccessResource::ProjectFiles;
    };

    let context = AgentToolContext {
        workspace_root: engine.workspace_path.as_ref().map(std::path::PathBuf::from),
        sandbox_root: std::env::temp_dir().join("vida-manual-access"),
    };
    let resolved = if std::path::Path::new(path).is_absolute() {
        std::path::PathBuf::from(path)
    } else if let Some(workspace_root) = &context.workspace_root {
        workspace_root.join(path)
    } else {
        std::path::PathBuf::from(path)
    };
    classify_path(&resolved, &context)
}

#[tauri::command]
pub async fn start_mcp_server(
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    permissions: State<'_, PermissionState>,
    name: String,
) -> Result<Vec<McpTool>, String> {
    require_permission(
        &app,
        engine.inner(),
        &permissions,
        PermissionType::ShellExecute,
        "start_mcp_server",
        None,
        format!("Start MCP server '{name}'"),
    )
    .await?;

    let mut e = engine.write().await;
    e.start_mcp_server(&name).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_mcp_server(
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    permissions: State<'_, PermissionState>,
    name: String,
) -> Result<(), String> {
    require_permission(
        &app,
        engine.inner(),
        &permissions,
        PermissionType::ShellExecute,
        "stop_mcp_server",
        None,
        format!("Stop MCP server '{name}'"),
    )
    .await?;

    let mut e = engine.write().await;
    e.stop_mcp_server(&name).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_mcp_servers(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<Vec<McpServerInfo>, String> {
    let e = engine.read().await;
    e.list_mcp_servers().await.map_err(|e| e.to_string())
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
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    permissions: State<'_, PermissionState>,
    tool_name: String,
    arguments: serde_json::Value,
) -> Result<McpToolResult, String> {
    let path = extract_tool_path(&arguments);
    let perm = infer_tool_permission(&tool_name);
    let (action, resource) = {
        let e = engine.read().await;
        (
            infer_tool_action(&tool_name),
            classify_tool_resource(&e, path.as_deref()),
        )
    };

    require_access(
        &app,
        engine.inner(),
        &permissions,
        action,
        resource,
        path.clone(),
        format!("Run MCP tool '{tool_name}'"),
    )
    .await?;

    require_permission(
        &app,
        engine.inner(),
        &permissions,
        perm,
        "call_mcp_tool",
        path,
        format!("Allow MCP tool '{tool_name}' to run"),
    )
    .await?;

    let mut e = engine.write().await;
    e.call_mcp_tool(&tool_name, arguments)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_mcp_server_config(
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    permissions: State<'_, PermissionState>,
    config: McpServerConfigRow,
) -> Result<(), String> {
    require_access(
        &app,
        engine.inner(),
        &permissions,
        AccessAction::Modify,
        AccessResource::IaConfig,
        None,
        format!("Save MCP server config '{}'", config.name),
    )
    .await?;

    let e = engine.read().await;
    e.save_mcp_server_config(&config)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_mcp_server_config(
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    permissions: State<'_, PermissionState>,
    id: String,
) -> Result<(), String> {
    require_access(
        &app,
        engine.inner(),
        &permissions,
        AccessAction::Delete,
        AccessResource::IaConfig,
        None,
        format!("Delete MCP server config '{id}'"),
    )
    .await?;

    let mut e = engine.write().await;
    e.delete_mcp_server_config(&id)
        .await
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::infer_tool_permission;
    use vida_core::PermissionType;

    #[test]
    fn test_infer_tool_permission_read() {
        assert_eq!(infer_tool_permission("read_file"), PermissionType::FileRead);
    }

    #[test]
    fn test_infer_tool_permission_write() {
        assert_eq!(
            infer_tool_permission("write_file"),
            PermissionType::FileWrite
        );
    }

    #[test]
    fn test_infer_tool_permission_network() {
        assert_eq!(
            infer_tool_permission("fetch_url"),
            PermissionType::NetworkAccess
        );
    }
}
