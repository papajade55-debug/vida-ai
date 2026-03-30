use std::sync::Arc;
use tauri::{AppHandle, State};
use tokio::sync::RwLock;
use vida_core::{
    AccessAction, AccessResource, ActorRole, AuthSession, AuthStatus, AuthUser, VidaEngine,
    VidaError,
};

use super::access::require_access;
use super::permissions::PermissionState;

#[tauri::command]
pub async fn is_pin_configured(engine: State<'_, Arc<RwLock<VidaEngine>>>) -> Result<bool, String> {
    let e = engine.read().await;
    e.is_pin_configured()
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn get_auth_status(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<AuthStatus, String> {
    let e = engine.read().await;
    e.auth_status().await.map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn bootstrap_local_admin(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    username: String,
    password: String,
) -> Result<AuthSession, String> {
    let mut e = engine.write().await;
    e.bootstrap_local_admin(&username, &password)
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn login_local(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    username: String,
    password: String,
) -> Result<AuthSession, String> {
    let mut e = engine.write().await;
    e.login_local(&username, &password)
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn logout_local(engine: State<'_, Arc<RwLock<VidaEngine>>>) -> Result<(), String> {
    let mut e = engine.write().await;
    e.logout_local();
    Ok(())
}

#[tauri::command]
pub async fn list_users(
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    permissions: State<'_, PermissionState>,
) -> Result<Vec<AuthUser>, String> {
    require_access(
        &app,
        engine.inner(),
        &permissions,
        AccessAction::Read,
        AccessResource::IaConfig,
        None,
        "List local users".to_string(),
    )
    .await?;

    let e = engine.read().await;
    e.list_users().await.map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn create_user(
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    permissions: State<'_, PermissionState>,
    username: String,
    password: String,
    role: ActorRole,
) -> Result<AuthUser, String> {
    require_access(
        &app,
        engine.inner(),
        &permissions,
        AccessAction::Create,
        AccessResource::IaConfig,
        None,
        format!("Create local user '{username}'"),
    )
    .await?;

    let e = engine.read().await;
    e.create_user(&username, &password, role)
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn change_password(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    current_password: String,
    new_password: String,
) -> Result<(), String> {
    let e = engine.read().await;
    e.change_current_password(&current_password, &new_password)
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn store_api_key(
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    permissions: State<'_, PermissionState>,
    provider_id: String,
    key: String,
) -> Result<(), String> {
    require_access(
        &app,
        engine.inner(),
        &permissions,
        AccessAction::Modify,
        AccessResource::IaConfig,
        None,
        format!("Store API key for provider '{provider_id}'"),
    )
    .await?;

    let mut e = engine.write().await;
    e.store_api_key(&provider_id, &key)
        .await
        .map_err(|e: VidaError| e.to_string())
}

#[tauri::command]
pub async fn remove_api_key(
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    permissions: State<'_, PermissionState>,
    provider_id: String,
) -> Result<(), String> {
    require_access(
        &app,
        engine.inner(),
        &permissions,
        AccessAction::Delete,
        AccessResource::IaConfig,
        None,
        format!("Remove API key for provider '{provider_id}'"),
    )
    .await?;

    let mut e = engine.write().await;
    e.remove_api_key(&provider_id)
        .await
        .map_err(|e: VidaError| e.to_string())
}
