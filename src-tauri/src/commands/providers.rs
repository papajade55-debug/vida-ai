use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use vida_core::{VidaEngine, VidaError};
use vida_providers::traits::ProviderInfo;

#[tauri::command]
pub async fn list_providers(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<Vec<ProviderInfo>, String> {
    let e = engine.read().await;
    Ok(e.list_providers())
}

#[tauri::command]
pub async fn list_models(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    provider_id: String,
) -> Result<Vec<String>, String> {
    let e = engine.read().await;
    e.list_models(&provider_id)
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn health_check(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<Vec<(String, bool)>, String> {
    let e = engine.read().await;
    Ok(e.health_check_all().await)
}
