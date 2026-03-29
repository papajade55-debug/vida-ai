use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, RwLock};
use vida_core::{TeamStreamEvent, VidaEngine, VidaError};
use vida_db::{SessionRow, TeamMemberRow, TeamRow};

#[tauri::command]
pub async fn create_team(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    name: String,
    members: Vec<(String, String)>,
) -> Result<TeamRow, String> {
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
    e.list_teams()
        .await
        .map_err(|e: VidaError| e.to_string())
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
pub async fn delete_team(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    team_id: String,
) -> Result<(), String> {
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
