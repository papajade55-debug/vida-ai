use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, RwLock};
use vida_core::{VidaEngine, VidaError};
use vida_db::{MessageRow, SessionRow};
use vida_providers::traits::{CompletionResponse, StreamEvent};

#[tauri::command]
pub async fn send_message(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    session_id: String,
    content: String,
) -> Result<CompletionResponse, String> {
    let e = engine.read().await;
    e.send_message(&session_id, &content)
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn stream_completion(
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    session_id: String,
    content: String,
) -> Result<(), String> {
    let (tx, mut rx) = mpsc::channel::<StreamEvent>(100);
    let event_name = format!("llm-stream-{}", session_id);

    let engine_ref = engine.inner().clone();
    let sid = session_id.clone();
    let content_clone = content.clone();

    // Spawn the streaming in background
    tokio::spawn(async move {
        let e = engine_ref.read().await;
        let _ = e.send_message_stream(&sid, &content_clone, tx).await;
    });

    // Forward events to frontend
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let is_done = matches!(event, StreamEvent::Done);
            let _ = app.emit(&event_name, &event);
            if is_done {
                break;
            }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn create_session(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    provider_id: String,
    model: String,
) -> Result<SessionRow, String> {
    let e = engine.read().await;
    e.create_session(&provider_id, &model)
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn list_sessions(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    limit: u32,
) -> Result<Vec<SessionRow>, String> {
    let e = engine.read().await;
    e.list_sessions(limit)
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn get_messages(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    session_id: String,
) -> Result<Vec<MessageRow>, String> {
    let e = engine.read().await;
    e.get_session_messages(&session_id)
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn delete_session(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    session_id: String,
) -> Result<(), String> {
    let e = engine.read().await;
    e.delete_session(&session_id)
        .await
        .map_err(|e: VidaError| e.to_string())
}
