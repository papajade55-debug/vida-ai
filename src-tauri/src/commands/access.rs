use std::sync::Arc;

use tauri::AppHandle;
use tokio::sync::RwLock;
use vida_core::{
    evaluate_access, AccessAction, AccessDecision, AccessRequest, AccessResource, VidaEngine,
};

use super::permissions::{require_human_approval, PermissionState};

pub async fn require_access(
    app: &AppHandle,
    engine: &Arc<RwLock<VidaEngine>>,
    permissions: &PermissionState,
    action: AccessAction,
    resource: AccessResource,
    path: Option<String>,
    description: String,
) -> Result<(), String> {
    let actor = {
        let e = engine.read().await;
        e.current_actor()
            .ok_or_else(|| "Authentication required".to_string())?
            .role
    };

    match evaluate_access(&AccessRequest {
        actor,
        action,
        resource,
    }) {
        AccessDecision::Allow => Ok(()),
        AccessDecision::Deny => Err(format!(
            "Access denied for {:?} {:?} by {:?}",
            action, resource, actor
        )),
        AccessDecision::RequireHumanApproval => {
            require_human_approval(app, permissions, "human_approval", path, description).await
        }
    }
}
