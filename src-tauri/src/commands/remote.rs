use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use vida_core::VidaEngine;

#[cfg(feature = "remote")]
use vida_core::RemoteServer;

#[cfg(feature = "remote")]
use tokio::sync::Mutex;

/// Shared remote server state managed by Tauri.
#[cfg(feature = "remote")]
pub struct RemoteState {
    pub server: Mutex<RemoteServer>,
}

// ── Remote status response ──

#[derive(serde::Serialize)]
pub struct RemoteStatus {
    pub enabled: bool,
    pub port: u16,
}

// ── Commands ──

#[tauri::command]
pub async fn enable_remote(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    #[cfg(feature = "remote")] remote: State<'_, RemoteState>,
    port: u16,
) -> Result<(), String> {
    #[cfg(feature = "remote")]
    {
        let e = engine.read().await;
        let token = match e.get_remote_token() {
            Ok(t) => t,
            Err(_) => e.generate_remote_token().map_err(|e| e.to_string())?,
        };
        drop(e);

        let engine_arc = engine.inner().clone();
        let mut server = remote.server.lock().await;
        server.stop(); // Stop if already running

        // Create a new server with the requested port
        *server = RemoteServer::new(port);
        server
            .start(engine_arc, token)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    #[cfg(not(feature = "remote"))]
    {
        let _ = (engine, port);
        Err("Remote feature is not enabled in this build".to_string())
    }
}

#[tauri::command]
pub async fn disable_remote(
    #[cfg(feature = "remote")] remote: State<'_, RemoteState>,
) -> Result<(), String> {
    #[cfg(feature = "remote")]
    {
        let mut server = remote.server.lock().await;
        server.stop();
        Ok(())
    }

    #[cfg(not(feature = "remote"))]
    {
        Err("Remote feature is not enabled in this build".to_string())
    }
}

#[tauri::command]
pub async fn get_remote_status(
    #[cfg(feature = "remote")] remote: State<'_, RemoteState>,
) -> Result<RemoteStatus, String> {
    #[cfg(feature = "remote")]
    {
        let server = remote.server.lock().await;
        Ok(RemoteStatus {
            enabled: server.is_running(),
            port: server.port(),
        })
    }

    #[cfg(not(feature = "remote"))]
    {
        Ok(RemoteStatus {
            enabled: false,
            port: 3690,
        })
    }
}

#[tauri::command]
pub async fn get_remote_token(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<String, String> {
    let e = engine.read().await;
    e.get_remote_token().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn regenerate_remote_token(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<String, String> {
    let e = engine.read().await;
    e.generate_remote_token().map_err(|e| e.to_string())
}
