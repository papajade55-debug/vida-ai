//! Remote HTTP/WebSocket server for headless access to Vida AI.
//!
//! Feature-gated behind `#[cfg(feature = "remote")]`.
//! Provides REST API + WebSocket streaming, secured with Bearer token auth.

#[cfg(feature = "remote")]
mod server {
    use std::net::SocketAddr;
    use std::sync::Arc;
    use tokio::sync::RwLock;

    use axum::{
        extract::{State as AxumState, WebSocketUpgrade, ws},
        http::{HeaderMap, StatusCode},
        middleware::{self, Next},
        response::{IntoResponse, Json},
        routing::{get, post},
        Router,
    };
    use serde::{Deserialize, Serialize};
    use tower_http::cors::{Any, CorsLayer};

    use crate::engine::VidaEngine;

    // ── Request / Response types ──

    #[derive(Debug, Deserialize)]
    pub struct ChatSendRequest {
        pub session_id: String,
        pub content: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct CreateSessionRequest {
        pub provider_id: String,
        pub model: String,
    }

    #[derive(Debug, Serialize)]
    pub struct HealthResponse {
        pub status: String,
        pub version: String,
    }

    #[derive(Debug, Serialize)]
    pub struct ErrorResponse {
        pub error: String,
    }

    // ── Shared state ──

    #[derive(Clone)]
    pub struct AppState {
        pub engine: Arc<RwLock<VidaEngine>>,
        pub token: String,
    }

    // ── Auth middleware ──

    pub async fn auth_middleware(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
        request: axum::extract::Request,
        next: Next,
    ) -> Result<impl IntoResponse, StatusCode> {
        // Health endpoint is public
        if request.uri().path() == "/api/health" {
            return Ok(next.run(request).await);
        }

        let auth_header = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if let Some(token) = auth_header.strip_prefix("Bearer ") {
            if token == state.token {
                return Ok(next.run(request).await);
            }
        }

        Err(StatusCode::UNAUTHORIZED)
    }

    // ── Route handlers ──

    async fn health_handler() -> Json<HealthResponse> {
        Json(HealthResponse {
            status: "ok".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        })
    }

    async fn list_providers_handler(
        AxumState(state): AxumState<AppState>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let engine = state.engine.read().await;
        let providers = engine.list_providers();
        Ok(Json(providers))
    }

    async fn list_sessions_handler(
        AxumState(state): AxumState<AppState>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let engine = state.engine.read().await;
        match engine.list_sessions(50).await {
            Ok(sessions) => Ok(Json(sessions)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    async fn create_session_handler(
        AxumState(state): AxumState<AppState>,
        Json(req): Json<CreateSessionRequest>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let engine = state.engine.read().await;
        match engine.create_session(&req.provider_id, &req.model).await {
            Ok(session) => Ok(Json(session)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    async fn chat_send_handler(
        AxumState(state): AxumState<AppState>,
        Json(req): Json<ChatSendRequest>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let engine = state.engine.read().await;
        match engine.send_message(&req.session_id, &req.content).await {
            Ok(response) => Ok(Json(response)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    async fn chat_stream_handler(
        AxumState(state): AxumState<AppState>,
        ws_upgrade: WebSocketUpgrade,
    ) -> impl IntoResponse {
        ws_upgrade.on_upgrade(move |mut socket| async move {
            use tokio::sync::mpsc;
            use vida_providers::traits::StreamEvent;

            // Wait for the first message containing session_id + content as JSON
            let initial_msg = match socket.recv().await {
                Some(Ok(ws::Message::Text(text))) => text,
                _ => return,
            };

            let req: ChatSendRequest = match serde_json::from_str(&initial_msg) {
                Ok(r) => r,
                Err(e) => {
                    let _ = socket
                        .send(ws::Message::Text(
                            serde_json::json!({"error": e.to_string()}).to_string().into(),
                        ))
                        .await;
                    return;
                }
            };

            let (tx, mut rx) = mpsc::channel::<StreamEvent>(100);

            let engine = state.engine.read().await;
            let sid = req.session_id.clone();
            let content = req.content.clone();

            // Spawn the streaming task
            // We need to drop the read lock before spawning to avoid deadlock
            // Clone what we need
            let engine_arc = state.engine.clone();
            drop(engine);

            tokio::spawn(async move {
                let e = engine_arc.read().await;
                let _ = e.send_message_stream(&sid, &content, tx).await;
            });

            // Forward stream events as JSON over WebSocket
            while let Some(event) = rx.recv().await {
                let json = match &event {
                    StreamEvent::Token { content } => {
                        serde_json::json!({"type": "token", "content": content})
                    }
                    StreamEvent::Error { error } => {
                        serde_json::json!({"type": "error", "error": error})
                    }
                    StreamEvent::Done => {
                        serde_json::json!({"type": "done"})
                    }
                };
                let is_done = matches!(event, StreamEvent::Done);
                if socket
                    .send(ws::Message::Text(json.to_string().into()))
                    .await
                    .is_err()
                {
                    break;
                }
                if is_done {
                    break;
                }
            }
        })
    }

    // ── Router builder ──

    pub fn build_router(state: AppState) -> Router {
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        Router::new()
            .route("/api/health", get(health_handler))
            .route("/api/providers", get(list_providers_handler))
            .route("/api/sessions", get(list_sessions_handler))
            .route("/api/sessions/create", post(create_session_handler))
            .route("/api/chat/send", post(chat_send_handler))
            .route("/api/chat/stream", get(chat_stream_handler))
            .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
            .layer(cors)
            .with_state(state)
    }

    // ── RemoteServer ──

    /// Manages the lifecycle of the embedded HTTP/WS server.
    pub struct RemoteServer {
        shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
        port: u16,
    }

    impl RemoteServer {
        pub fn new(port: u16) -> Self {
            Self {
                shutdown_tx: None,
                port,
            }
        }

        pub fn port(&self) -> u16 {
            self.port
        }

        pub fn is_running(&self) -> bool {
            self.shutdown_tx.is_some()
        }

        /// Start the HTTP server. Returns immediately; server runs in background.
        pub async fn start(
            &mut self,
            engine: Arc<RwLock<VidaEngine>>,
            token: String,
        ) -> Result<(), String> {
            if self.is_running() {
                return Err("Remote server is already running".to_string());
            }

            let state = AppState {
                engine,
                token,
            };

            let app = build_router(state);
            let addr = SocketAddr::from(([0, 0, 0, 0], self.port));

            let listener = tokio::net::TcpListener::bind(addr)
                .await
                .map_err(|e| format!("Failed to bind to port {}: {}", self.port, e))?;

            let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

            tokio::spawn(async move {
                axum::serve(listener, app)
                    .with_graceful_shutdown(async {
                        let _ = shutdown_rx.await;
                    })
                    .await
                    .ok();
            });

            self.shutdown_tx = Some(shutdown_tx);
            Ok(())
        }

        /// Stop the running server.
        pub fn stop(&mut self) {
            if let Some(tx) = self.shutdown_tx.take() {
                let _ = tx.send(());
            }
        }
    }

    /// Generate a cryptographically random API token.
    pub fn generate_token() -> String {
        use std::fmt::Write;
        let mut rng_bytes = [0u8; 32];
        // Use getrandom for secure random bytes
        getrandom::fill(&mut rng_bytes).expect("Failed to generate random bytes");
        let mut token = String::with_capacity(68);
        token.push_str("vida_");
        for byte in &rng_bytes {
            write!(token, "{:02x}", byte).unwrap();
        }
        token
    }
}

// ── Public re-exports (feature-gated) ──

#[cfg(feature = "remote")]
pub use server::{
    AppState, RemoteServer, build_router, generate_token,
    ChatSendRequest, CreateSessionRequest, HealthResponse, ErrorResponse,
};

// ── No-op stubs when feature is disabled ──

#[cfg(not(feature = "remote"))]
pub fn generate_token() -> String {
    String::from("remote-feature-disabled")
}

// ── Tests ──

#[cfg(test)]
#[cfg(feature = "remote")]
mod tests {
    use super::*;

    #[test]
    fn test_generate_token_format() {
        let token = generate_token();
        assert!(token.starts_with("vida_"));
        // vida_ (5) + 64 hex chars = 69 total
        assert_eq!(token.len(), 69);
    }

    #[test]
    fn test_generate_token_uniqueness() {
        let t1 = generate_token();
        let t2 = generate_token();
        assert_ne!(t1, t2);
    }

    #[test]
    fn test_remote_server_new() {
        let server = RemoteServer::new(3690);
        assert_eq!(server.port(), 3690);
        assert!(!server.is_running());
    }

    #[tokio::test]
    async fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: "0.1.0".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
    }

    #[test]
    fn test_chat_send_request_deserialization() {
        let json = r#"{"session_id": "abc", "content": "hello"}"#;
        let req: ChatSendRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.session_id, "abc");
        assert_eq!(req.content, "hello");
    }

    #[test]
    fn test_create_session_request_deserialization() {
        let json = r#"{"provider_id": "ollama", "model": "llama3"}"#;
        let req: CreateSessionRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.provider_id, "ollama");
        assert_eq!(req.model, "llama3");
    }
}
