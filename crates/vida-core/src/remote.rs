//! Remote HTTP/WebSocket server for headless access to Vida AI.
//!
//! Feature-gated behind `#[cfg(feature = "remote")]`.
//! Provides REST API + WebSocket streaming, secured with Bearer token auth.

#[cfg(feature = "remote")]
mod server {
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
    use tokio::sync::RwLock;

    use axum::{
        extract::{ws, Query, State as AxumState, WebSocketUpgrade},
        http::{HeaderMap, StatusCode, Uri},
        middleware::{self, Next},
        response::{IntoResponse, Json},
        routing::{get, post},
        Router,
    };
    use serde::{Deserialize, Serialize};
    use tower_http::cors::{Any, CorsLayer};
    use uuid::Uuid;

    use crate::access::ActorRole;
    use crate::auth::{actor_role_storage, AuthSession, AuthStatus};
    use crate::engine::VidaEngine;
    use vida_db::{AuditEventRow, MessageRow, TeamMemberRow, TeamRow};

    const REMOTE_SESSION_TTL: Duration = Duration::from_secs(12 * 60 * 60);
    const LOGIN_WINDOW: Duration = Duration::from_secs(5 * 60);
    const LOGIN_MAX_ATTEMPTS: u32 = 5;
    const LOGIN_BLOCK_DURATION: Duration = Duration::from_secs(15 * 60);

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

    #[derive(Debug, Deserialize)]
    pub struct SessionQuery {
        pub limit: Option<u32>,
    }

    #[derive(Debug, Deserialize)]
    pub struct SessionMessagesQuery {
        pub session_id: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct TeamQuery {
        pub team_id: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct TeamCreateRequest {
        pub name: String,
        pub members: Vec<(String, String)>,
        #[serde(default)]
        pub description: Option<String>,
        #[serde(default)]
        pub system_prompt: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    pub struct AuthRequest {
        pub username: String,
        pub password: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct ChangePasswordRequest {
        pub current_password: String,
        pub new_password: String,
    }

    #[derive(Debug, Deserialize)]
    pub struct RemoteCreateUserRequest {
        pub username: String,
        pub password: String,
        pub role: ActorRole,
    }

    #[derive(Debug, Serialize)]
    pub struct HealthResponse {
        pub status: String,
        pub version: String,
        pub uptime_seconds: u64,
        pub remote_session_ttl_seconds: u64,
    }

    #[derive(Debug, Serialize)]
    pub struct ErrorResponse {
        pub error: String,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct RemoteAuthResponse {
        pub session_token: String,
        pub actor: AuthSession,
    }

    #[derive(Debug, Deserialize, Default)]
    pub struct AuthQuery {
        pub service_token: Option<String>,
        pub session_token: Option<String>,
    }

    #[derive(Debug, Deserialize, Default)]
    pub struct AuditQuery {
        pub limit: Option<u32>,
        pub actor_username: Option<String>,
        pub event_type: Option<String>,
        pub created_after: Option<String>,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct AdminHealthResponse {
        pub status: String,
        pub version: String,
        pub uptime_seconds: u64,
        pub has_users: bool,
        pub active_sessions: usize,
        pub rate_limited_users: usize,
        pub audit_event_count: i64,
        pub latest_audit_at: Option<String>,
        pub provider_count: usize,
        pub mcp_tool_count: usize,
    }

    #[derive(Debug, Serialize, Deserialize)]
    pub struct TeamDetailResponse {
        pub team: TeamRow,
        pub members: Vec<TeamMemberRow>,
    }

    #[derive(Debug, Clone)]
    pub(crate) struct RemoteSessionState {
        pub actor: AuthSession,
        pub expires_at: Instant,
    }

    #[derive(Debug, Clone)]
    pub(crate) struct LoginRateLimitState {
        window_started_at: Instant,
        attempts: u32,
        blocked_until: Option<Instant>,
    }

    // ── Shared state ──

    #[derive(Clone)]
    pub struct AppState {
        pub(crate) engine: Arc<RwLock<VidaEngine>>,
        pub(crate) token: String,
        pub(crate) sessions: Arc<RwLock<HashMap<String, RemoteSessionState>>>,
        pub(crate) login_limits: Arc<RwLock<HashMap<String, LoginRateLimitState>>>,
        pub(crate) started_at: Instant,
    }

    // ── Auth middleware ──

    fn parse_query_param(query: Option<&str>, key: &str) -> Option<String> {
        let query = query?;
        for pair in query.split('&') {
            let (name, value) = pair.split_once('=')?;
            if name == key {
                return Some(value.to_string());
            }
        }
        None
    }

    pub(crate) fn extract_service_token(headers: &HeaderMap, uri: &Uri) -> Option<String> {
        if let Some(auth_header) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
            if let Some(token) = auth_header.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }

        if let Some(token) = headers
            .get("x-vida-service-token")
            .and_then(|v| v.to_str().ok())
        {
            return Some(token.to_string());
        }

        parse_query_param(uri.query(), "service_token")
    }

    fn extract_session_token(
        headers: &HeaderMap,
        query: Option<&AuthQuery>,
        uri: Option<&Uri>,
    ) -> Option<String> {
        if let Some(token) = headers.get("x-vida-session").and_then(|v| v.to_str().ok()) {
            return Some(token.to_string());
        }

        if let Some(token) = query.and_then(|q| q.session_token.clone()) {
            return Some(token);
        }

        uri.and_then(|current| parse_query_param(current.query(), "session_token"))
    }

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

        if extract_service_token(&headers, request.uri()).as_deref() == Some(state.token.as_str()) {
            return Ok(next.run(request).await);
        }

        Err(StatusCode::UNAUTHORIZED)
    }

    async fn lookup_remote_actor(
        state: &AppState,
        headers: &HeaderMap,
        query: Option<&AuthQuery>,
        uri: Option<&Uri>,
    ) -> Option<AuthSession> {
        cleanup_expired_sessions(state).await;
        let session_token = extract_session_token(headers, query, uri)?;
        state
            .sessions
            .read()
            .await
            .get(&session_token)
            .map(|session| session.actor.clone())
    }

    async fn require_remote_actor(
        state: &AppState,
        headers: &HeaderMap,
        query: Option<&AuthQuery>,
        uri: Option<&Uri>,
    ) -> Result<AuthSession, StatusCode> {
        lookup_remote_actor(state, headers, query, uri)
            .await
            .ok_or(StatusCode::UNAUTHORIZED)
    }

    fn authorize_remote_role(actor: &AuthSession, allowed: &[ActorRole]) -> Result<(), StatusCode> {
        if allowed.contains(&actor.role) {
            Ok(())
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    }

    fn uptime_seconds(state: &AppState) -> u64 {
        state.started_at.elapsed().as_secs()
    }

    fn unix_timestamp_seconds() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    fn log_remote_event(level: &str, event: &str, fields: serde_json::Value) {
        eprintln!(
            "{}",
            serde_json::json!({
                "ts": unix_timestamp_seconds(),
                "level": level,
                "component": "vida-remote",
                "event": event,
                "fields": fields,
            })
        );
    }

    async fn write_audit_event(
        state: &AppState,
        actor: Option<&AuthSession>,
        event_type: &str,
        resource: Option<&str>,
        details: serde_json::Value,
    ) {
        let event = AuditEventRow {
            id: Uuid::new_v4().to_string(),
            actor_username: actor.map(|current| current.username.clone()),
            actor_role: actor.map(|current| actor_role_storage(current.role).to_string()),
            event_type: event_type.to_string(),
            resource: resource.map(|value| value.to_string()),
            details_json: Some(details.to_string()),
            created_at: String::new(),
        };

        let engine = state.engine.read().await;
        let _ = engine.db.insert_audit_event(&event).await;
    }

    async fn cleanup_expired_sessions(state: &AppState) {
        let now = Instant::now();
        state
            .sessions
            .write()
            .await
            .retain(|_, session| session.expires_at > now);
    }

    async fn active_session_count(state: &AppState) -> usize {
        cleanup_expired_sessions(state).await;
        state.sessions.read().await.len()
    }

    async fn rate_limited_user_count(state: &AppState) -> usize {
        let now = Instant::now();
        state
            .login_limits
            .read()
            .await
            .values()
            .filter(|entry| {
                entry
                    .blocked_until
                    .is_some_and(|blocked_until| blocked_until > now)
            })
            .count()
    }

    async fn check_login_rate_limit(state: &AppState, username: &str) -> Result<(), StatusCode> {
        let now = Instant::now();
        let key = username.trim().to_ascii_lowercase();
        let limits = state.login_limits.read().await;
        let Some(entry) = limits.get(&key) else {
            return Ok(());
        };

        if let Some(blocked_until) = entry.blocked_until {
            if blocked_until > now {
                return Err(StatusCode::TOO_MANY_REQUESTS);
            }
        }

        Ok(())
    }

    async fn record_login_failure(state: &AppState, username: &str) {
        let now = Instant::now();
        let key = username.trim().to_ascii_lowercase();
        let mut limits = state.login_limits.write().await;
        let entry = limits.entry(key).or_insert(LoginRateLimitState {
            window_started_at: now,
            attempts: 0,
            blocked_until: None,
        });

        if now.duration_since(entry.window_started_at) > LOGIN_WINDOW {
            entry.window_started_at = now;
            entry.attempts = 0;
            entry.blocked_until = None;
        }

        if let Some(blocked_until) = entry.blocked_until {
            if blocked_until > now {
                return;
            }
            entry.blocked_until = None;
            entry.attempts = 0;
            entry.window_started_at = now;
        }

        entry.attempts += 1;
        if entry.attempts >= LOGIN_MAX_ATTEMPTS {
            entry.blocked_until = Some(now + LOGIN_BLOCK_DURATION);
        }
    }

    async fn clear_login_failures(state: &AppState, username: &str) {
        state
            .login_limits
            .write()
            .await
            .remove(&username.trim().to_ascii_lowercase());
    }

    async fn issue_remote_session(state: &AppState, actor: AuthSession) -> RemoteAuthResponse {
        cleanup_expired_sessions(state).await;
        let session_token = generate_token();
        state.sessions.write().await.insert(
            session_token.clone(),
            RemoteSessionState {
                actor: actor.clone(),
                expires_at: Instant::now() + REMOTE_SESSION_TTL,
            },
        );
        RemoteAuthResponse {
            session_token,
            actor,
        }
    }

    // ── Route handlers ──

    async fn health_handler(AxumState(state): AxumState<AppState>) -> Json<HealthResponse> {
        Json(HealthResponse {
            status: "ok".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: uptime_seconds(&state),
            remote_session_ttl_seconds: REMOTE_SESSION_TTL.as_secs(),
        })
    }

    async fn admin_health_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
    ) -> Result<impl IntoResponse, StatusCode> {
        let actor = require_remote_actor(&state, &headers, None, None).await?;
        authorize_remote_role(&actor, &[ActorRole::SuperAdmin])?;

        let active_sessions = active_session_count(&state).await;
        let rate_limited_users = rate_limited_user_count(&state).await;

        let engine = state.engine.read().await;
        let has_users = engine
            .has_users()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let provider_count = engine.list_providers().await.len();
        let mcp_tool_count = engine.mcp_manager.list_tools().len();
        let audit_event_count = engine
            .db
            .count_audit_events()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let latest_audit_at = engine
            .db
            .list_audit_events(1)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .into_iter()
            .next()
            .map(|event| event.created_at);

        Ok(Json(AdminHealthResponse {
            status: "ok".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            uptime_seconds: uptime_seconds(&state),
            has_users,
            active_sessions,
            rate_limited_users,
            audit_event_count,
            latest_audit_at,
            provider_count,
            mcp_tool_count,
        }))
    }

    async fn auth_status_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
        query: Query<AuthQuery>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let engine = state.engine.read().await;
        Ok(Json(AuthStatus {
            has_users: engine
                .has_users()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
            actor: lookup_remote_actor(&state, &headers, Some(&query), None).await,
        }))
    }

    async fn auth_bootstrap_handler(
        AxumState(state): AxumState<AppState>,
        Json(req): Json<AuthRequest>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let engine = state.engine.read().await;
        let actor = match engine
            .bootstrap_admin_user(&req.username, &req.password)
            .await
        {
            Ok(actor) => actor,
            Err(_) => {
                drop(engine);
                write_audit_event(
                    &state,
                    None,
                    "remote.auth.bootstrap_failed",
                    Some("remote_auth"),
                    serde_json::json!({ "username": req.username }),
                )
                .await;
                log_remote_event(
                    "warn",
                    "remote.auth.bootstrap_failed",
                    serde_json::json!({ "username": req.username }),
                );
                return Err(StatusCode::BAD_REQUEST);
            }
        };
        drop(engine);
        write_audit_event(
            &state,
            Some(&actor),
            "remote.auth.bootstrap",
            Some("remote_auth"),
            serde_json::json!({ "username": actor.username }),
        )
        .await;
        log_remote_event(
            "info",
            "remote.auth.bootstrap",
            serde_json::json!({
                "username": actor.username,
                "role": actor_role_storage(actor.role),
            }),
        );
        Ok(Json(issue_remote_session(&state, actor).await))
    }

    async fn auth_login_handler(
        AxumState(state): AxumState<AppState>,
        Json(req): Json<AuthRequest>,
    ) -> Result<impl IntoResponse, StatusCode> {
        if let Err(status) = check_login_rate_limit(&state, &req.username).await {
            write_audit_event(
                &state,
                None,
                "remote.auth.rate_limited",
                Some("remote_auth"),
                serde_json::json!({ "username": req.username }),
            )
            .await;
            log_remote_event(
                "warn",
                "remote.auth.rate_limited",
                serde_json::json!({ "username": req.username }),
            );
            return Err(status);
        }
        let engine = state.engine.read().await;
        let actor = match engine.authenticate_user(&req.username, &req.password).await {
            Ok(actor) => actor,
            Err(_) => {
                drop(engine);
                record_login_failure(&state, &req.username).await;
                write_audit_event(
                    &state,
                    None,
                    "remote.auth.login_failed",
                    Some("remote_auth"),
                    serde_json::json!({ "username": req.username }),
                )
                .await;
                log_remote_event(
                    "warn",
                    "remote.auth.login_failed",
                    serde_json::json!({ "username": req.username }),
                );
                return Err(StatusCode::UNAUTHORIZED);
            }
        };
        drop(engine);
        clear_login_failures(&state, &req.username).await;
        write_audit_event(
            &state,
            Some(&actor),
            "remote.auth.login",
            Some("remote_auth"),
            serde_json::json!({ "username": actor.username }),
        )
        .await;
        log_remote_event(
            "info",
            "remote.auth.login",
            serde_json::json!({
                "username": actor.username,
                "role": actor_role_storage(actor.role),
            }),
        );
        Ok(Json(issue_remote_session(&state, actor).await))
    }

    async fn auth_logout_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
        query: Query<AuthQuery>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let session_token =
            extract_session_token(&headers, Some(&query), None).ok_or(StatusCode::UNAUTHORIZED)?;
        let removed = state.sessions.write().await.remove(&session_token);
        if let Some(session) = removed {
            write_audit_event(
                &state,
                Some(&session.actor),
                "remote.auth.logout",
                Some("remote_auth"),
                serde_json::json!({ "username": session.actor.username }),
            )
            .await;
            log_remote_event(
                "info",
                "remote.auth.logout",
                serde_json::json!({ "username": session.actor.username }),
            );
            Ok(StatusCode::NO_CONTENT)
        } else {
            Err(StatusCode::UNAUTHORIZED)
        }
    }

    async fn auth_change_password_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
        Json(req): Json<ChangePasswordRequest>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let actor = require_remote_actor(&state, &headers, None, None).await?;
        let engine = state.engine.read().await;
        engine
            .change_password_for_actor(&actor, &req.current_password, &req.new_password)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        write_audit_event(
            &state,
            Some(&actor),
            "remote.auth.change_password",
            Some("remote_auth"),
            serde_json::json!({ "username": actor.username }),
        )
        .await;
        log_remote_event(
            "info",
            "remote.auth.change_password",
            serde_json::json!({ "username": actor.username }),
        );
        Ok(StatusCode::NO_CONTENT)
    }

    async fn list_audit_events_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
        query: Query<AuditQuery>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let actor = require_remote_actor(&state, &headers, None, None).await?;
        authorize_remote_role(&actor, &[ActorRole::SuperAdmin])?;

        let limit = query.limit.unwrap_or(50).clamp(1, 200);
        let engine = state.engine.read().await;
        let events = engine
            .db
            .list_audit_events_filtered(
                limit,
                query.actor_username.as_deref(),
                query.event_type.as_deref(),
                query.created_after.as_deref(),
            )
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(events))
    }

    async fn list_users_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
    ) -> Result<impl IntoResponse, StatusCode> {
        let actor = require_remote_actor(&state, &headers, None, None).await?;
        authorize_remote_role(&actor, &[ActorRole::SuperAdmin])?;

        let engine = state.engine.read().await;
        let users = engine
            .list_users()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(users))
    }

    async fn create_user_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
        Json(req): Json<RemoteCreateUserRequest>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let actor = require_remote_actor(&state, &headers, None, None).await?;
        authorize_remote_role(&actor, &[ActorRole::SuperAdmin])?;

        if !matches!(req.role, ActorRole::Architect | ActorRole::Operator) {
            return Err(StatusCode::BAD_REQUEST);
        }

        let engine = state.engine.read().await;
        let user = engine
            .create_user(&req.username, &req.password, req.role)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        write_audit_event(
            &state,
            Some(&actor),
            "remote.admin.user_create",
            Some("users"),
            serde_json::json!({
                "username": user.username,
                "role": actor_role_storage(user.role),
            }),
        )
        .await;
        log_remote_event(
            "info",
            "remote.admin.user_create",
            serde_json::json!({
                "actor_username": actor.username,
                "username": user.username,
                "role": actor_role_storage(user.role),
            }),
        );
        Ok(Json(user))
    }

    async fn list_providers_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
    ) -> Result<impl IntoResponse, StatusCode> {
        let actor = require_remote_actor(&state, &headers, None, None).await?;
        authorize_remote_role(
            &actor,
            &[
                ActorRole::SuperAdmin,
                ActorRole::Architect,
                ActorRole::Operator,
            ],
        )?;
        let engine = state.engine.read().await;
        let providers = engine.list_providers().await;
        Ok(Json(providers))
    }

    async fn list_sessions_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
        query: Query<SessionQuery>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let actor = require_remote_actor(&state, &headers, None, None).await?;
        authorize_remote_role(
            &actor,
            &[
                ActorRole::SuperAdmin,
                ActorRole::Architect,
                ActorRole::Operator,
            ],
        )?;
        let engine = state.engine.read().await;
        let limit = query.limit.unwrap_or(50).clamp(1, 200);
        match engine.list_sessions(limit).await {
            Ok(sessions) => Ok(Json(sessions)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    async fn get_messages_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
        query: Query<SessionMessagesQuery>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let actor = require_remote_actor(&state, &headers, None, None).await?;
        authorize_remote_role(
            &actor,
            &[
                ActorRole::SuperAdmin,
                ActorRole::Architect,
                ActorRole::Operator,
            ],
        )?;
        let engine = state.engine.read().await;
        let messages: Vec<MessageRow> = engine
            .get_session_messages(&query.session_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(messages))
    }

    async fn delete_session_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
        Json(req): Json<SessionMessagesQuery>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let actor = require_remote_actor(&state, &headers, None, None).await?;
        authorize_remote_role(
            &actor,
            &[
                ActorRole::SuperAdmin,
                ActorRole::Architect,
                ActorRole::Operator,
            ],
        )?;
        let engine = state.engine.read().await;
        engine
            .delete_session(&req.session_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(StatusCode::NO_CONTENT)
    }

    async fn create_session_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
        Json(req): Json<CreateSessionRequest>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let actor = require_remote_actor(&state, &headers, None, None).await?;
        authorize_remote_role(
            &actor,
            &[
                ActorRole::SuperAdmin,
                ActorRole::Architect,
                ActorRole::Operator,
            ],
        )?;
        let engine = state.engine.read().await;
        match engine.create_session(&req.provider_id, &req.model).await {
            Ok(session) => Ok(Json(session)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    async fn chat_send_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
        Json(req): Json<ChatSendRequest>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let actor = require_remote_actor(&state, &headers, None, None).await?;
        authorize_remote_role(
            &actor,
            &[
                ActorRole::SuperAdmin,
                ActorRole::Architect,
                ActorRole::Operator,
            ],
        )?;
        let mut engine = state.engine.write().await;
        match engine.send_message(&req.session_id, &req.content).await {
            Ok(response) => Ok(Json(response)),
            Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    }

    async fn list_teams_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
    ) -> Result<impl IntoResponse, StatusCode> {
        let actor = require_remote_actor(&state, &headers, None, None).await?;
        authorize_remote_role(
            &actor,
            &[
                ActorRole::SuperAdmin,
                ActorRole::Architect,
                ActorRole::Operator,
            ],
        )?;
        let engine = state.engine.read().await;
        let teams = engine
            .list_teams()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(teams))
    }

    async fn get_team_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
        query: Query<TeamQuery>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let actor = require_remote_actor(&state, &headers, None, None).await?;
        authorize_remote_role(
            &actor,
            &[
                ActorRole::SuperAdmin,
                ActorRole::Architect,
                ActorRole::Operator,
            ],
        )?;
        let engine = state.engine.read().await;
        let (team, members) = engine
            .get_team_with_members(&query.team_id)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        Ok(Json(TeamDetailResponse { team, members }))
    }

    async fn create_team_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
        Json(req): Json<TeamCreateRequest>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let actor = require_remote_actor(&state, &headers, None, None).await?;
        authorize_remote_role(&actor, &[ActorRole::SuperAdmin, ActorRole::Architect])?;
        let engine = state.engine.read().await;
        let team = engine
            .create_team(&req.name, req.members, req.description, req.system_prompt)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        Ok(Json(team))
    }

    async fn create_team_session_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
        Json(req): Json<TeamQuery>,
    ) -> Result<impl IntoResponse, StatusCode> {
        let actor = require_remote_actor(&state, &headers, None, None).await?;
        authorize_remote_role(
            &actor,
            &[
                ActorRole::SuperAdmin,
                ActorRole::Architect,
                ActorRole::Operator,
            ],
        )?;
        let engine = state.engine.read().await;
        let session = engine
            .create_team_session(&req.team_id)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        Ok(Json(session))
    }

    async fn chat_stream_handler(
        AxumState(state): AxumState<AppState>,
        headers: HeaderMap,
        query: Query<AuthQuery>,
        ws_upgrade: WebSocketUpgrade,
    ) -> impl IntoResponse {
        let actor = match require_remote_actor(&state, &headers, Some(&query), None).await {
            Ok(actor) => actor,
            Err(status) => return status.into_response(),
        };

        if let Err(status) = authorize_remote_role(
            &actor,
            &[
                ActorRole::SuperAdmin,
                ActorRole::Architect,
                ActorRole::Operator,
            ],
        ) {
            return status.into_response();
        }

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
                            serde_json::json!({"error": e.to_string()})
                                .to_string()
                                .into(),
                        ))
                        .await;
                    return;
                }
            };

            let (tx, mut rx) = mpsc::channel::<StreamEvent>(100);

            let sid = req.session_id.clone();
            let content = req.content.clone();

            // Spawn the streaming task
            let engine_arc = state.engine.clone();

            tokio::spawn(async move {
                let mut e = engine_arc.write().await;
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
            .route("/api/admin/health", get(admin_health_handler))
            .route("/api/admin/audit", get(list_audit_events_handler))
            .route("/api/auth/status", get(auth_status_handler))
            .route("/api/auth/bootstrap", post(auth_bootstrap_handler))
            .route("/api/auth/login", post(auth_login_handler))
            .route("/api/auth/logout", post(auth_logout_handler))
            .route(
                "/api/auth/change-password",
                post(auth_change_password_handler),
            )
            .route("/api/admin/users", get(list_users_handler))
            .route("/api/admin/users", post(create_user_handler))
            .route("/api/providers", get(list_providers_handler))
            .route("/api/sessions", get(list_sessions_handler))
            .route("/api/sessions/create", post(create_session_handler))
            .route("/api/sessions/messages", get(get_messages_handler))
            .route("/api/sessions/delete", post(delete_session_handler))
            .route("/api/chat/send", post(chat_send_handler))
            .route("/api/chat/stream", get(chat_stream_handler))
            .route("/api/teams", get(list_teams_handler))
            .route("/api/teams/detail", get(get_team_handler))
            .route("/api/teams/create", post(create_team_handler))
            .route("/api/teams/session", post(create_team_session_handler))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                auth_middleware,
            ))
            .layer(cors)
            .with_state(state)
    }

    // ── RemoteServer ──

    /// Manages the lifecycle of the embedded HTTP/WS server.
    pub struct RemoteServer {
        shutdown_tx: Option<tokio::sync::oneshot::Sender<()>>,
        port: u16,
        bind_addr: String,
    }

    impl RemoteServer {
        pub fn new(port: u16) -> Self {
            Self {
                shutdown_tx: None,
                port,
                bind_addr: "0.0.0.0".to_string(),
            }
        }

        pub fn with_bind_addr(port: u16, bind_addr: impl Into<String>) -> Self {
            Self {
                shutdown_tx: None,
                port,
                bind_addr: bind_addr.into(),
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
                sessions: Arc::new(RwLock::new(HashMap::new())),
                login_limits: Arc::new(RwLock::new(HashMap::new())),
                started_at: Instant::now(),
            };

            let app = build_router(state);
            let bind_target = format!("{}:{}", self.bind_addr, self.port);
            let listener = tokio::net::TcpListener::bind(&bind_target)
                .await
                .map_err(|e| format!("Failed to bind to {}: {}", bind_target, e))?;

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
    build_router, generate_token, AdminHealthResponse, AppState, AuditQuery, AuthQuery,
    AuthRequest, ChangePasswordRequest, ChatSendRequest, CreateSessionRequest, ErrorResponse,
    HealthResponse, RemoteAuthResponse, RemoteCreateUserRequest, RemoteServer,
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
    use axum::{
        body::{to_bytes, Body},
        http::{HeaderMap, Request, StatusCode, Uri},
    };
    use std::time::{Duration, Instant};
    use std::{collections::HashMap, sync::Arc};
    use tokio::sync::RwLock;
    use tower::ServiceExt;

    use crate::{
        remote::server::{extract_service_token, RemoteSessionState},
        ActorRole, AuthSession, AuthStatus, AuthUser, VidaEngine,
    };

    async fn test_app_state() -> AppState {
        let engine = VidaEngine::init_in_memory().await.unwrap();
        AppState {
            engine: Arc::new(RwLock::new(engine)),
            token: "vida_test_service".to_string(),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            login_limits: Arc::new(RwLock::new(HashMap::new())),
            started_at: Instant::now(),
        }
    }

    fn auth_request(username: &str, password: &str) -> Body {
        Body::from(
            serde_json::json!({
                "username": username,
                "password": password,
            })
            .to_string(),
        )
    }

    fn change_password_request(current_password: &str, new_password: &str) -> Body {
        Body::from(
            serde_json::json!({
                "current_password": current_password,
                "new_password": new_password,
            })
            .to_string(),
        )
    }

    fn create_user_request(username: &str, password: &str, role: &str) -> Body {
        Body::from(
            serde_json::json!({
                "username": username,
                "password": password,
                "role": role,
            })
            .to_string(),
        )
    }

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
            uptime_seconds: 1,
            remote_session_ttl_seconds: 60,
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

    #[test]
    fn test_extract_service_token_prefers_headers() {
        let mut headers = HeaderMap::new();
        headers.insert("x-vida-service-token", "via-header".parse().unwrap());
        let uri: Uri = "/api/providers?service_token=via-query".parse().unwrap();
        assert_eq!(
            extract_service_token(&headers, &uri).as_deref(),
            Some("via-header")
        );
    }

    #[tokio::test]
    async fn test_remote_auth_flow_requires_service_and_session_tokens() {
        let state = test_app_state().await;
        let app = build_router(state);

        let health = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::OK);

        let unauthorized = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/auth/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

        let bootstrap = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/bootstrap")
                    .header("authorization", "Bearer vida_test_service")
                    .header("content-type", "application/json")
                    .body(auth_request("remote.admin", "supersecret"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bootstrap.status(), StatusCode::OK);
        let bootstrap_body = to_bytes(bootstrap.into_body(), usize::MAX).await.unwrap();
        let auth: RemoteAuthResponse = serde_json::from_slice(&bootstrap_body).unwrap();
        assert_eq!(auth.actor.role, ActorRole::SuperAdmin);

        let missing_session = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/providers")
                    .header("authorization", "Bearer vida_test_service")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(missing_session.status(), StatusCode::UNAUTHORIZED);

        let providers = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/providers")
                    .header("authorization", "Bearer vida_test_service")
                    .header("x-vida-session", &auth.session_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(providers.status(), StatusCode::OK);

        let logout = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/logout")
                    .header("authorization", "Bearer vida_test_service")
                    .header("x-vida-session", &auth.session_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(logout.status(), StatusCode::NO_CONTENT);

        let after_logout = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/providers")
                    .header("authorization", "Bearer vida_test_service")
                    .header("x-vida-session", &auth.session_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(after_logout.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_remote_auth_status_returns_actor_for_session() {
        let state = test_app_state().await;
        let app = build_router(state);

        let bootstrap = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/bootstrap")
                    .header("authorization", "Bearer vida_test_service")
                    .header("content-type", "application/json")
                    .body(auth_request("remote.arch", "supersecret"))
                    .unwrap(),
            )
            .await
            .unwrap();
        let bootstrap_body = to_bytes(bootstrap.into_body(), usize::MAX).await.unwrap();
        let auth: RemoteAuthResponse = serde_json::from_slice(&bootstrap_body).unwrap();

        let status = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/auth/status")
                    .header("authorization", "Bearer vida_test_service")
                    .header("x-vida-session", &auth.session_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(status.status(), StatusCode::OK);
        let status_body = to_bytes(status.into_body(), usize::MAX).await.unwrap();
        let payload: AuthStatus = serde_json::from_slice(&status_body).unwrap();
        assert!(payload.has_users);
        assert_eq!(
            payload.actor.as_ref().map(|a| a.username.as_str()),
            Some("remote.arch")
        );
    }

    #[tokio::test]
    async fn test_remote_admin_user_management_and_role_restrictions() {
        let state = test_app_state().await;
        let app = build_router(state);

        let bootstrap = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/bootstrap")
                    .header("authorization", "Bearer vida_test_service")
                    .header("content-type", "application/json")
                    .body(auth_request("remote.admin", "supersecret"))
                    .unwrap(),
            )
            .await
            .unwrap();
        let bootstrap_body = to_bytes(bootstrap.into_body(), usize::MAX).await.unwrap();
        let admin_auth: RemoteAuthResponse = serde_json::from_slice(&bootstrap_body).unwrap();

        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/admin/users")
                    .header("authorization", "Bearer vida_test_service")
                    .header("x-vida-session", &admin_auth.session_token)
                    .header("content-type", "application/json")
                    .body(create_user_request(
                        "remote.operator",
                        "operator1",
                        "operator",
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::OK);

        let users = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/users")
                    .header("authorization", "Bearer vida_test_service")
                    .header("x-vida-session", &admin_auth.session_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(users.status(), StatusCode::OK);
        let users_body = to_bytes(users.into_body(), usize::MAX).await.unwrap();
        let payload: Vec<AuthUser> = serde_json::from_slice(&users_body).unwrap();
        assert_eq!(payload.len(), 2);

        let operator_login = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("authorization", "Bearer vida_test_service")
                    .header("content-type", "application/json")
                    .body(auth_request("remote.operator", "operator1"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(operator_login.status(), StatusCode::OK);
        let operator_body = to_bytes(operator_login.into_body(), usize::MAX)
            .await
            .unwrap();
        let operator_auth: RemoteAuthResponse = serde_json::from_slice(&operator_body).unwrap();

        let forbidden = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/users")
                    .header("authorization", "Bearer vida_test_service")
                    .header("x-vida-session", &operator_auth.session_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(forbidden.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn test_remote_change_password_updates_login_credentials() {
        let state = test_app_state().await;
        let app = build_router(state);

        let bootstrap = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/bootstrap")
                    .header("authorization", "Bearer vida_test_service")
                    .header("content-type", "application/json")
                    .body(auth_request("remote.admin", "supersecret"))
                    .unwrap(),
            )
            .await
            .unwrap();
        let bootstrap_body = to_bytes(bootstrap.into_body(), usize::MAX).await.unwrap();
        let admin_auth: RemoteAuthResponse = serde_json::from_slice(&bootstrap_body).unwrap();

        let change_password = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/change-password")
                    .header("authorization", "Bearer vida_test_service")
                    .header("x-vida-session", &admin_auth.session_token)
                    .header("content-type", "application/json")
                    .body(change_password_request("supersecret", "supersafe2"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(change_password.status(), StatusCode::NO_CONTENT);

        let old_login = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("authorization", "Bearer vida_test_service")
                    .header("content-type", "application/json")
                    .body(auth_request("remote.admin", "supersecret"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(old_login.status(), StatusCode::UNAUTHORIZED);

        let new_login = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("authorization", "Bearer vida_test_service")
                    .header("content-type", "application/json")
                    .body(auth_request("remote.admin", "supersafe2"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(new_login.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_expired_remote_session_is_rejected() {
        let state = test_app_state().await;
        let app = build_router(state.clone());

        state.sessions.write().await.insert(
            "expired-session".to_string(),
            RemoteSessionState {
                actor: AuthSession {
                    user_id: "user-1".to_string(),
                    username: "expired.user".to_string(),
                    role: ActorRole::Operator,
                },
                expires_at: Instant::now() - Duration::from_secs(1),
            },
        );

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/providers")
                    .header("authorization", "Bearer vida_test_service")
                    .header("x-vida-session", "expired-session")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(state.sessions.read().await.get("expired-session").is_none());
    }

    #[tokio::test]
    async fn test_remote_login_rate_limit_blocks_after_failures() {
        let state = test_app_state().await;
        let app = build_router(state);

        let bootstrap = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/bootstrap")
                    .header("authorization", "Bearer vida_test_service")
                    .header("content-type", "application/json")
                    .body(auth_request("remote.admin", "supersecret"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bootstrap.status(), StatusCode::OK);

        for _ in 0..5 {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/auth/login")
                        .header("authorization", "Bearer vida_test_service")
                        .header("content-type", "application/json")
                        .body(auth_request("remote.admin", "wrong-password"))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        }

        let blocked = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("authorization", "Bearer vida_test_service")
                    .header("content-type", "application/json")
                    .body(auth_request("remote.admin", "supersecret"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(blocked.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[tokio::test]
    async fn test_remote_auth_events_are_audited() {
        let state = test_app_state().await;
        let app = build_router(state.clone());

        let bootstrap = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/bootstrap")
                    .header("authorization", "Bearer vida_test_service")
                    .header("content-type", "application/json")
                    .body(auth_request("remote.admin", "supersecret"))
                    .unwrap(),
            )
            .await
            .unwrap();
        let bootstrap_body = to_bytes(bootstrap.into_body(), usize::MAX).await.unwrap();
        let admin_auth: RemoteAuthResponse = serde_json::from_slice(&bootstrap_body).unwrap();

        let create_user = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/admin/users")
                    .header("authorization", "Bearer vida_test_service")
                    .header("x-vida-session", &admin_auth.session_token)
                    .header("content-type", "application/json")
                    .body(create_user_request(
                        "remote.arch",
                        "architect1",
                        "architect",
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(create_user.status(), StatusCode::OK);

        let logout = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/logout")
                    .header("authorization", "Bearer vida_test_service")
                    .header("x-vida-session", &admin_auth.session_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(logout.status(), StatusCode::NO_CONTENT);

        let engine = state.engine.read().await;
        let events = engine.db.list_audit_events(10).await.unwrap();
        let event_types = events
            .into_iter()
            .map(|event| event.event_type)
            .collect::<Vec<_>>();
        assert!(event_types
            .iter()
            .any(|value| value == "remote.auth.bootstrap"));
        assert!(event_types
            .iter()
            .any(|value| value == "remote.admin.user_create"));
        assert!(event_types
            .iter()
            .any(|value| value == "remote.auth.logout"));
    }

    #[tokio::test]
    async fn test_admin_audit_endpoint_filters_events() {
        let state = test_app_state().await;
        let app = build_router(state.clone());

        let bootstrap = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/bootstrap")
                    .header("authorization", "Bearer vida_test_service")
                    .header("content-type", "application/json")
                    .body(auth_request("remote.admin", "supersecret"))
                    .unwrap(),
            )
            .await
            .unwrap();
        let bootstrap_body = to_bytes(bootstrap.into_body(), usize::MAX).await.unwrap();
        let admin_auth: RemoteAuthResponse = serde_json::from_slice(&bootstrap_body).unwrap();

        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/admin/users")
                    .header("authorization", "Bearer vida_test_service")
                    .header("x-vida-session", &admin_auth.session_token)
                    .header("content-type", "application/json")
                    .body(create_user_request(
                        "remote.operator",
                        "operator1",
                        "operator",
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::OK);

        let filtered = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/audit?event_type=remote.admin.user_create&limit=10")
                    .header("authorization", "Bearer vida_test_service")
                    .header("x-vida-session", &admin_auth.session_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(filtered.status(), StatusCode::OK);
        let filtered_body = to_bytes(filtered.into_body(), usize::MAX).await.unwrap();
        let events: Vec<vida_db::AuditEventRow> = serde_json::from_slice(&filtered_body).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "remote.admin.user_create");
    }

    #[tokio::test]
    async fn test_admin_health_reports_runtime_counts() {
        let state = test_app_state().await;
        let app = build_router(state);

        let bootstrap = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/bootstrap")
                    .header("authorization", "Bearer vida_test_service")
                    .header("content-type", "application/json")
                    .body(auth_request("remote.admin", "supersecret"))
                    .unwrap(),
            )
            .await
            .unwrap();
        let bootstrap_body = to_bytes(bootstrap.into_body(), usize::MAX).await.unwrap();
        let admin_auth: RemoteAuthResponse = serde_json::from_slice(&bootstrap_body).unwrap();

        let health = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/admin/health")
                    .header("authorization", "Bearer vida_test_service")
                    .header("x-vida-session", &admin_auth.session_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(health.status(), StatusCode::OK);
        let health_body = to_bytes(health.into_body(), usize::MAX).await.unwrap();
        let payload: AdminHealthResponse = serde_json::from_slice(&health_body).unwrap();
        assert!(payload.has_users);
        assert!(payload.active_sessions >= 1);
        assert!(payload.audit_event_count >= 1);
        assert!(payload.provider_count >= 1);
        assert_eq!(payload.status, "ok");
    }
}
