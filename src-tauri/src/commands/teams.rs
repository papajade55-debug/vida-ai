use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, RwLock};
use vida_core::{AccessAction, AccessResource, TeamStreamEvent, VidaEngine, VidaError};
use vida_db::{SessionRow, TeamMemberRow, TeamRow};

use super::access::require_access;
use super::permissions::PermissionState;

#[tauri::command]
pub async fn create_team(
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    permissions: State<'_, PermissionState>,
    name: String,
    members: Vec<(String, String)>,
) -> Result<TeamRow, String> {
    require_access(
        &app,
        engine.inner(),
        &permissions,
        AccessAction::Create,
        AccessResource::TeamConfig,
        None,
        format!("Create team '{name}'"),
    )
    .await?;

    let e = engine.read().await;
    e.create_team(&name, members)
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn list_teams(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<Vec<TeamRow>, String> {
    let e = engine.read().await;
    e.list_teams().await.map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn get_team(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    team_id: String,
) -> Result<(TeamRow, Vec<TeamMemberRow>), String> {
    let e = engine.read().await;
    e.get_team_with_members(&team_id)
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn set_team_member_role(
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    permissions: State<'_, PermissionState>,
    team_id: String,
    member_id: String,
    role: String,
) -> Result<TeamMemberRow, String> {
    require_access(
        &app,
        engine.inner(),
        &permissions,
        AccessAction::Modify,
        AccessResource::TeamConfig,
        None,
        format!("Change team member role to '{role}'"),
    )
    .await?;

    let e = engine.read().await;
    e.set_team_member_role(&team_id, &member_id, &role)
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn delete_team(
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    permissions: State<'_, PermissionState>,
    team_id: String,
) -> Result<(), String> {
    require_access(
        &app,
        engine.inner(),
        &permissions,
        AccessAction::Delete,
        AccessResource::TeamConfig,
        None,
        format!("Delete team '{team_id}'"),
    )
    .await?;

    let e = engine.read().await;
    e.delete_team(&team_id)
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn create_team_session(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    team_id: String,
) -> Result<SessionRow, String> {
    let e = engine.read().await;
    e.create_team_session(&team_id)
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn stream_team_completion(
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    session_id: String,
    content: String,
) -> Result<(), String> {
    let (tx, mut rx) = mpsc::channel::<TeamStreamEvent>(100);
    let event_name = format!("team-stream-{}", session_id);

    let engine_ref = engine.inner().clone();
    let sid = session_id.clone();
    let content_clone = content.clone();

    // Spawn the team streaming in background
    tokio::spawn(async move {
        let e = engine_ref.read().await;
        let _ = e.send_team_message_stream(&sid, &content_clone, tx).await;
    });

    // Forward events to frontend
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let is_all_done = matches!(event, TeamStreamEvent::AllDone);
            let _ = app.emit(&event_name, &event);
            if is_all_done {
                break;
            }
        }
    });

    Ok(())
}
