use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use vida_core::{VidaEngine, VidaError};

#[tauri::command]
pub async fn is_pin_configured(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<bool, String> {
    let e = engine.read().await;
    e.is_pin_configured()
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn store_api_key(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    provider_id: String,
    key: String,
) -> Result<(), String> {
    let e = engine.read().await;
    e.store_api_key(&provider_id, &key)
        .await
        .map_err(|e: VidaError| e.to_string())
}
