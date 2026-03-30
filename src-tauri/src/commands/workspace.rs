use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::RwLock;
use vida_core::{
    AccessAction, AccessResource, PermissionMode, VidaEngine, VidaError, WorkspaceConfig,
};
use vida_db::RecentWorkspaceRow;

use super::access::require_access;
use super::permissions::PermissionState;

#[tauri::command]
pub async fn open_workspace(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    path: String,
) -> Result<WorkspaceConfig, String> {
    let mut e = engine.write().await;
    e.open_workspace(&path)
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn create_workspace(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    path: String,
    name: String,
) -> Result<WorkspaceConfig, String> {
    let mut e = engine.write().await;
    e.create_workspace(&path, &name)
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn list_recent_workspaces(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<Vec<RecentWorkspaceRow>, String> {
    let e = engine.read().await;
    e.list_recent_workspaces()
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn get_workspace_config(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<WorkspaceConfig, String> {
    let e = engine.read().await;
    Ok(e.get_workspace_config().clone())
}

#[tauri::command]
pub async fn set_workspace_config(
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    permissions: State<'_, PermissionState>,
    config: WorkspaceConfig,
) -> Result<(), String> {
    require_access(
        &app,
        engine.inner(),
        &permissions,
        AccessAction::Modify,
        AccessResource::IaConfig,
        None,
        "Modify workspace configuration".to_string(),
    )
    .await?;

    let mut e = engine.write().await;
    e.set_workspace_config(config)
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn get_permission_mode(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<String, String> {
    let e = engine.read().await;
    let mode = e.get_permission_mode();
    serde_json::to_string(mode).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn set_permission_mode(
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    permissions: State<'_, PermissionState>,
    mode: PermissionMode,
) -> Result<(), String> {
    require_access(
        &app,
        engine.inner(),
        &permissions,
        AccessAction::Modify,
        AccessResource::IaConfig,
        None,
        format!("Change workspace permission mode to '{mode:?}'"),
    )
    .await?;

    let mut e = engine.write().await;
    e.set_permission_mode(mode)
        .map_err(|e: VidaError| e.to_string())
}
