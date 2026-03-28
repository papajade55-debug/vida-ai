use std::sync::Arc;
use tauri::State;
use tokio::sync::{RwLock, RwLockReadGuard};
use vida_core::{AppConfig, VidaEngine};

#[tauri::command]
pub async fn get_config(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<AppConfig, String> {
    let e: RwLockReadGuard<'_, VidaEngine> = engine.read().await;
    Ok(e.config.clone())
}
