use std::collections::HashMap;
use std::time::Duration;

use tauri::{AppHandle, Emitter, State};
use tokio::sync::{oneshot, Mutex, RwLock};
use uuid::Uuid;
use vida_core::{PermissionResult, PermissionType, VidaEngine};

use std::sync::Arc;

const PERMISSION_TIMEOUT_SECS: u64 = 120;

#[derive(Default)]
pub struct PermissionState {
    pending: Mutex<HashMap<String, oneshot::Sender<bool>>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PermissionRequest {
    pub request_id: String,
    pub action: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub description: String,
}

async fn emit_approval_request(
    app: &AppHandle,
    permissions: &PermissionState,
    action: &str,
    path: Option<String>,
    description: String,
) -> Result<(), String> {
    let request_id = Uuid::new_v4().to_string();
    let (tx, rx) = oneshot::channel();

    permissions
        .pending
        .lock()
        .await
        .insert(request_id.clone(), tx);

    let payload = PermissionRequest {
        request_id: request_id.clone(),
        action: action.to_string(),
        path,
        description,
    };

    app.emit("permission-request", &payload)
        .map_err(|e| format!("Failed to emit permission request: {e}"))?;

    match tokio::time::timeout(Duration::from_secs(PERMISSION_TIMEOUT_SECS), rx).await {
        Ok(Ok(true)) => Ok(()),
        Ok(Ok(false)) => Err(format!("Permission denied by user for action '{action}'")),
        Ok(Err(_)) => Err(format!("Permission request '{request_id}' was cancelled")),
        Err(_) => {
            permissions.pending.lock().await.remove(&request_id);
            Err(format!(
                "Permission request '{request_id}' timed out after {PERMISSION_TIMEOUT_SECS}s"
            ))
        }
    }
}

pub async fn require_human_approval(
    app: &AppHandle,
    permissions: &PermissionState,
    action: &str,
    path: Option<String>,
    description: String,
) -> Result<(), String> {
    emit_approval_request(app, permissions, action, path, description).await
}

pub async fn require_permission(
    app: &AppHandle,
    engine: &Arc<RwLock<VidaEngine>>,
    permissions: &PermissionState,
    perm: PermissionType,
    action: &str,
    path: Option<String>,
    description: String,
) -> Result<(), String> {
    let check = {
        let e = engine.read().await;
        e.check_permission(perm)
    };

    match check {
        PermissionResult::Allowed => Ok(()),
        PermissionResult::Denied => Err(format!("Permission denied for action '{action}'")),
        PermissionResult::NeedsApproval => {
            emit_approval_request(app, permissions, action, path, description).await
        }
    }
}

#[tauri::command]
pub async fn respond_permission(
    permissions: State<'_, PermissionState>,
    request_id: String,
    allowed: bool,
) -> Result<(), String> {
    let tx = permissions
        .pending
        .lock()
        .await
        .remove(&request_id)
        .ok_or_else(|| format!("Unknown permission request: {request_id}"))?;

    tx.send(allowed)
        .map_err(|_| format!("Permission request {request_id} is no longer waiting"))?;

    Ok(())
}
