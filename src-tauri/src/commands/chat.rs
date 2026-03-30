use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, RwLock};
use vida_core::{VidaEngine, VidaError};
use vida_db::{MessageRow, SessionRow};
use vida_providers::traits::{CompletionOptions, CompletionResponse, StreamEvent};

#[tauri::command]
pub async fn send_message(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    session_id: String,
    content: String,
) -> Result<CompletionResponse, String> {
    let mut e = engine.write().await;
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
        let mut e = engine_ref.write().await;
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

#[tauri::command]
pub async fn send_vision_message(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    session_id: String,
    image_base64: String,
    prompt: String,
) -> Result<CompletionResponse, String> {
    use base64::Engine;

    let image_data = base64::engine::general_purpose::STANDARD
        .decode(&image_base64)
        .map_err(|e| format!("Invalid base64: {}", e))?;

    let e = engine.read().await;

    let session =
        e.db.get_session(&session_id)
            .await
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("Session not found: {}", session_id))?;

    let provider = e
        .providers
        .get(&session.provider_id)
        .ok_or_else(|| format!("Provider not found: {}", session.provider_id))?;

    let options = CompletionOptions {
        model: Some(session.model.clone()),
        ..Default::default()
    };

    provider
        .vision_completion(image_data, &prompt, Some(options))
        .await
        .map_err(|e| e.to_string())
}
