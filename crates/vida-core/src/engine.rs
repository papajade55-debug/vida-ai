use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::auth::{actor_role_storage, AuthSession, AuthStatus, AuthUser};
use vida_db::{
    Database, McpServerConfigRow, MessageRow, ProviderConfigRow, RecentWorkspaceRow, SessionRow,
    TeamMemberRow, TeamRow, UserRow,
};

use crate::access::{ActorRole, AgentToolContext};
use crate::agent_loop::run_agent_loop;
use crate::mcp::{McpManager, McpServerInfo, McpTool, McpToolResult};
use vida_providers::anthropic::AnthropicProvider;
use vida_providers::google::GoogleProvider;
use vida_providers::ollama::OllamaProvider;
use vida_providers::openai::OpenAIProvider;
use vida_providers::registry::ProviderRegistry;
use vida_providers::traits::*;
use vida_security::keychain::{KeychainManager, MockSecretStore, SecretStore};
use vida_security::pin::PinManager;

use crate::config::AppConfig;
use crate::error::VidaError;
use crate::permissions::{PermissionManager, PermissionMode, PermissionResult, PermissionType};
use crate::workspace::{load_workspace_config, save_workspace_config, WorkspaceConfig};

/// Color palette for auto-assignment to team members.
const TEAM_COLORS: &[&str] = &[
    "#6366f1", "#ec4899", "#14b8a6", "#f59e0b", "#8b5cf6", "#06b6d4", "#f97316", "#10b981",
];

const DEFAULT_OLLAMA_URL: &str = "http://localhost:11434";
const DEFAULT_OLLAMA_MODEL: &str = "qwen3:14b";
const DEFAULT_OPENAI_URL: &str = "https://api.openai.com";
const DEFAULT_OPENAI_MODEL: &str = "gpt-4o-mini";
const DEFAULT_ANTHROPIC_URL: &str = "https://api.anthropic.com";
const DEFAULT_ANTHROPIC_MODEL: &str = "claude-3-5-haiku-20241022";
const DEFAULT_GOOGLE_URL: &str = "https://generativelanguage.googleapis.com";
const DEFAULT_GOOGLE_MODEL: &str = "gemini-2.0-flash";
const DEFAULT_GROQ_URL: &str = "https://api.groq.com/openai";
const DEFAULT_GROQ_MODEL: &str = "llama-3.3-70b-versatile";
const DEFAULT_MISTRAL_URL: &str = "https://api.mistral.ai";
const DEFAULT_MISTRAL_MODEL: &str = "mistral-large-latest";
const DEFAULT_DEEPSEEK_URL: &str = "https://api.deepseek.com";
const DEFAULT_DEEPSEEK_MODEL: &str = "deepseek-chat";
const DEFAULT_CEREBRAS_URL: &str = "https://api.cerebras.ai";
const DEFAULT_CEREBRAS_MODEL: &str = "qwen-3-235b";
const DEFAULT_NVIDIA_URL: &str = "https://integrate.api.nvidia.com";
const DEFAULT_NVIDIA_MODEL: &str = "meta/llama-4-maverick-17b-128e-instruct";
const DEFAULT_SAMBANOVA_URL: &str = "https://api.sambanova.ai";
const DEFAULT_SAMBANOVA_MODEL: &str = "Meta-Llama-3.3-70B-Instruct";
const DEFAULT_OPENROUTER_URL: &str = "https://openrouter.ai/api";
const DEFAULT_OPENROUTER_MODEL: &str = "google/gemini-2.5-flash-preview";
const DEFAULT_ZHIPUAI_URL: &str = "https://open.bigmodel.cn/api/paas";
const DEFAULT_ZHIPUAI_MODEL: &str = "glm-4-plus";
const DEFAULT_DASHSCOPE_URL: &str = "https://dashscope.aliyuncs.com/compatible-mode";
const DEFAULT_DASHSCOPE_MODEL: &str = "qwen-max";
const TEAM_ROLE_OWNER: &str = "owner";
const TEAM_ROLE_ADMIN: &str = "admin";
const TEAM_ROLE_MEMBER: &str = "member";
const TEAM_ROLE_VIEWER: &str = "viewer";

const LOGIN_MAX_ATTEMPTS: u32 = 5;
const LOGIN_WINDOW_SECS: u64 = 300; // 5 minutes
const LOGIN_BLOCK_SECS: u64 = 900; // 15 minutes

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TeamStreamEvent {
    AgentToken {
        agent_id: String,
        agent_name: String,
        agent_color: String,
        content: String,
    },
    AgentDone {
        agent_id: String,
    },
    AgentError {
        agent_id: String,
        error: String,
    },
    AllDone,
}

pub struct VidaEngine {
    pub db: Database,
    pub providers: ProviderRegistry,
    pub secrets: Box<dyn SecretStore>,
    pub config: AppConfig,
    pub workspace_path: Option<String>,
    pub workspace_config: WorkspaceConfig,
    pub permission_manager: PermissionManager,
    pub mcp_manager: McpManager,
    pub current_actor: Option<AuthSession>,
    login_attempts: std::collections::HashMap<String, (u32, std::time::Instant)>,
    login_blocked_until: std::collections::HashMap<String, std::time::Instant>,
}

fn default_team_role(index: usize) -> &'static str {
    if index == 0 {
        TEAM_ROLE_OWNER
    } else {
        TEAM_ROLE_MEMBER
    }
}

fn normalize_team_role(role: &str) -> Result<String, VidaError> {
    let normalized = role.trim().to_lowercase();
    match normalized.as_str() {
        TEAM_ROLE_OWNER | TEAM_ROLE_ADMIN | TEAM_ROLE_MEMBER | TEAM_ROLE_VIEWER => Ok(normalized),
        _ => Err(VidaError::Config(format!(
            "Invalid team role '{role}'. Expected one of: owner, admin, member, viewer"
        ))),
    }
}

fn team_member_role(member: &TeamMemberRow) -> &str {
    member.role.as_deref().unwrap_or(TEAM_ROLE_MEMBER)
}

fn team_role_can_execute(role: &str) -> bool {
    matches!(role, TEAM_ROLE_OWNER | TEAM_ROLE_ADMIN | TEAM_ROLE_MEMBER)
}

fn validate_username(username: &str) -> Result<(), VidaError> {
    let trimmed = username.trim();
    if trimmed.len() < 3 {
        return Err(VidaError::Authentication(
            "Username must be at least 3 characters".to_string(),
        ));
    }
    if !trimmed
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '.'))
    {
        return Err(VidaError::Authentication(
            "Username may contain only letters, numbers, '.', '_' and '-'".to_string(),
        ));
    }
    Ok(())
}

fn validate_password(password: &str) -> Result<(), VidaError> {
    if password.len() < 8 {
        return Err(VidaError::Authentication(
            "Password must be at least 8 characters".to_string(),
        ));
    }
    Ok(())
}

fn hash_auth_password(password: &str) -> Result<String, VidaError> {
    PinManager::hash_password(password).map_err(|e| VidaError::Authentication(e.to_string()))
}

impl VidaEngine {
    /// Initialize with real OS keychain (production).
    pub async fn init(data_dir: &Path) -> Result<Self, VidaError> {
        let keychain = KeychainManager::new("vida-ai");
        Self::init_with_secrets(data_dir, Box::new(keychain)).await
    }

    /// Initialize with custom SecretStore (for testing).
    pub async fn init_with_secrets(
        data_dir: &Path,
        secrets: Box<dyn SecretStore>,
    ) -> Result<Self, VidaError> {
        std::fs::create_dir_all(data_dir).map_err(|e| VidaError::Config(e.to_string()))?;
        let db_path = format!("sqlite:{}/vida.db?mode=rwc", data_dir.display());
        let db = Database::connect(&db_path).await?;
        db.run_migrations().await?;

        // Load config from DB or use defaults
        let config = match db.get_config("app_config").await? {
            Some(json) => serde_json::from_str(&json).unwrap_or_default(),
            None => AppConfig::default(),
        };

        let workspace_config = WorkspaceConfig::default();
        let permission_manager = PermissionManager::new(
            workspace_config.permission_mode.clone(),
            workspace_config.permissions.clone(),
        );

        let mcp_manager = McpManager::new();

        let mut engine = Self {
            db,
            providers: ProviderRegistry::new(),
            secrets,
            config,
            workspace_path: None,
            workspace_config,
            permission_manager,
            mcp_manager,
            current_actor: None,
            login_attempts: std::collections::HashMap::new(),
            login_blocked_until: std::collections::HashMap::new(),
        };

        engine.seed_default_provider_configs().await?;
        engine.refresh_providers().await?;

        Ok(engine)
    }

    /// Initialize with in-memory DB (for testing).
    pub async fn init_in_memory() -> Result<Self, VidaError> {
        let db = Database::connect_in_memory().await?;
        db.run_migrations().await?;
        let secrets = Box::new(MockSecretStore::new());
        let config = AppConfig::default();
        let workspace_config = WorkspaceConfig::default();
        let permission_manager = PermissionManager::new(
            workspace_config.permission_mode.clone(),
            workspace_config.permissions.clone(),
        );
        let mcp_manager = McpManager::new();
        let mut engine = Self {
            db,
            providers: ProviderRegistry::new(),
            secrets,
            config,
            workspace_path: None,
            workspace_config,
            permission_manager,
            mcp_manager,
            current_actor: None,
            login_attempts: std::collections::HashMap::new(),
            login_blocked_until: std::collections::HashMap::new(),
        };
        engine.seed_default_provider_configs().await?;
        engine.refresh_providers().await?;
        Ok(engine)
    }

    // ── Chat ──

    pub async fn send_message(
        &mut self,
        session_id: &str,
        content: &str,
    ) -> Result<CompletionResponse, VidaError> {
        let session = self
            .db
            .get_session(session_id)
            .await?
            .ok_or_else(|| VidaError::SessionNotFound(session_id.to_string()))?;

        let provider = self
            .providers
            .get(&session.provider_id)
            .ok_or_else(|| VidaError::ProviderNotFound(session.provider_id.clone()))?;

        // Load history
        let history = self.db.get_messages(session_id).await?;

        // Insert user message
        let user_msg = MessageRow {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: "user".to_string(),
            content: content.to_string(),
            token_count: None,
            created_at: String::new(),
            agent_id: None,
            agent_name: None,
            agent_color: None,
        };
        self.db.insert_message(&user_msg).await?;

        // Build ChatMessage list
        let mut messages = Vec::new();
        if let Some(ref prompt) = session.system_prompt {
            messages.push(ChatMessage {
                role: ChatRole::System,
                content: prompt.clone(),
                tool_call_id: None,
                name: None,
            });
        }
        for msg in &history {
            let role = match msg.role.as_str() {
                "system" => ChatRole::System,
                "user" => ChatRole::User,
                _ => ChatRole::Assistant,
            };
            messages.push(ChatMessage {
                role,
                content: msg.content.clone(),
                tool_call_id: None,
                name: None,
            });
        }
        messages.push(ChatMessage {
            role: ChatRole::User,
            content: content.to_string(),
            tool_call_id: None,
            name: None,
        });

        let options = CompletionOptions {
            model: Some(session.model.clone()),
            ..Default::default()
        };

        let response = if self.mcp_manager.list_tools().is_empty() {
            provider.chat_completion(&messages, Some(options)).await?
        } else {
            let agent_context =
                self.agent_tool_context(session.team_id.as_deref().unwrap_or(session_id))?;
            messages.push(ChatMessage {
                role: ChatRole::System,
                content: format!(
                    "Agent execution policy: read only within the project workspace and write only inside the sandbox at {}. Never modify team configuration, critical source code, or use shell tools.",
                    agent_context.sandbox_root.display()
                ),
                tool_call_id: None,
                name: None,
            });
            let tools = self.mcp_manager.list_tools();
            let loop_result = run_agent_loop(
                provider.clone(),
                messages,
                options,
                tools,
                &mut self.mcp_manager,
                Some(&agent_context),
            )
            .await?;

            CompletionResponse {
                content: loop_result.rendered_content(),
                ..loop_result.response
            }
        };

        // Insert assistant message
        let assistant_msg = MessageRow {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: "assistant".to_string(),
            content: response.content.clone(),
            token_count: Some(response.total_tokens as i32),
            created_at: String::new(),
            agent_id: None,
            agent_name: None,
            agent_color: None,
        };
        self.db.insert_message(&assistant_msg).await?;

        Ok(response)
    }

    pub async fn send_message_stream(
        &mut self,
        session_id: &str,
        content: &str,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), VidaError> {
        if !self.mcp_manager.list_tools().is_empty() {
            let response = self.send_message(session_id, content).await?;
            if !response.content.is_empty() {
                let _ = tx
                    .send(StreamEvent::Token {
                        content: response.content,
                    })
                    .await;
            }
            let _ = tx.send(StreamEvent::Done).await;
            return Ok(());
        }

        let session = self
            .db
            .get_session(session_id)
            .await?
            .ok_or_else(|| VidaError::SessionNotFound(session_id.to_string()))?;

        let provider = self
            .providers
            .get(&session.provider_id)
            .ok_or_else(|| VidaError::ProviderNotFound(session.provider_id.clone()))?;

        let history = self.db.get_messages(session_id).await?;

        // Insert user message
        let user_msg = MessageRow {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: "user".to_string(),
            content: content.to_string(),
            token_count: None,
            created_at: String::new(),
            agent_id: None,
            agent_name: None,
            agent_color: None,
        };
        self.db.insert_message(&user_msg).await?;

        // Build messages
        let mut messages = Vec::new();
        if let Some(ref prompt) = session.system_prompt {
            messages.push(ChatMessage {
                role: ChatRole::System,
                content: prompt.clone(),
                tool_call_id: None,
                name: None,
            });
        }
        for msg in &history {
            let role = match msg.role.as_str() {
                "system" => ChatRole::System,
                "user" => ChatRole::User,
                _ => ChatRole::Assistant,
            };
            messages.push(ChatMessage {
                role,
                content: msg.content.clone(),
                tool_call_id: None,
                name: None,
            });
        }
        messages.push(ChatMessage {
            role: ChatRole::User,
            content: content.to_string(),
            tool_call_id: None,
            name: None,
        });

        let options = CompletionOptions {
            model: Some(session.model.clone()),
            ..Default::default()
        };

        // Intercept stream to collect full response
        let (inner_tx, mut inner_rx) = mpsc::channel::<StreamEvent>(100);
        let tx_clone = tx.clone();
        let db = &self.db;
        let sid = session_id.to_string();

        // Spawn provider streaming
        let provider_clone = provider.clone();
        let messages_clone = messages.clone();
        let options_clone = options.clone();
        tokio::spawn(async move {
            let _ = provider_clone
                .chat_completion_stream(&messages_clone, Some(options_clone), inner_tx)
                .await;
        });

        // Forward events and collect tokens
        let mut full_content = String::new();
        while let Some(event) = inner_rx.recv().await {
            match &event {
                StreamEvent::Token { content } => {
                    full_content.push_str(content);
                }
                StreamEvent::Done => {
                    let _ = tx_clone.send(event).await;
                    break;
                }
                _ => {}
            }
            let _ = tx_clone.send(event).await;
        }

        // Insert assistant message with full content
        if !full_content.is_empty() {
            let assistant_msg = MessageRow {
                id: Uuid::new_v4().to_string(),
                session_id: sid,
                role: "assistant".to_string(),
                content: full_content,
                token_count: None,
                created_at: String::new(),
                agent_id: None,
                agent_name: None,
                agent_color: None,
            };
            db.insert_message(&assistant_msg).await?;
        }

        Ok(())
    }

    // ── Sessions ──

    pub async fn create_session(
        &self,
        provider_id: &str,
        model: &str,
    ) -> Result<SessionRow, VidaError> {
        let session = SessionRow {
            id: Uuid::new_v4().to_string(),
            title: None,
            provider_id: provider_id.to_string(),
            model: model.to_string(),
            system_prompt: None,
            created_at: String::new(),
            updated_at: String::new(),
            team_id: None,
        };
        self.db.create_session(&session).await?;
        Ok(session)
    }

    pub async fn list_sessions(&self, limit: u32) -> Result<Vec<SessionRow>, VidaError> {
        Ok(self.db.list_sessions(limit).await?)
    }

    pub async fn get_session_messages(
        &self,
        session_id: &str,
    ) -> Result<Vec<MessageRow>, VidaError> {
        Ok(self.db.get_messages(session_id).await?)
    }

    pub async fn delete_session(&self, id: &str) -> Result<(), VidaError> {
        Ok(self.db.delete_session(id).await?)
    }

    // ── Providers ──

    pub async fn list_providers(&self) -> Vec<ProviderInfo> {
        let mut infos = self.providers.list();
        let defaults = self.provider_default_models().await;

        for info in &mut infos {
            if let Some(provider) = self.providers.get(&info.id) {
                info.models = provider
                    .list_models()
                    .await
                    .unwrap_or_else(|_| defaults.get(&info.id).cloned().into_iter().collect());
            }
        }

        infos
    }

    pub async fn list_models(&self, provider_id: &str) -> Result<Vec<String>, VidaError> {
        let provider = self
            .providers
            .get(provider_id)
            .ok_or_else(|| VidaError::ProviderNotFound(provider_id.to_string()))?;
        Ok(provider.list_models().await?)
    }

    pub async fn health_check_all(&self) -> Vec<(String, bool)> {
        self.providers
            .health_check_all()
            .await
            .into_iter()
            .map(|(name, result)| (name, result.is_ok()))
            .collect()
    }

    // ── Auth ──

    pub async fn auth_status(&self) -> Result<AuthStatus, VidaError> {
        Ok(AuthStatus {
            has_users: self.has_users().await?,
            actor: self.current_actor.clone(),
        })
    }

    pub fn current_actor(&self) -> Option<AuthSession> {
        self.current_actor.clone()
    }

    pub async fn has_users(&self) -> Result<bool, VidaError> {
        Ok(self.db.count_users().await? > 0)
    }

    pub async fn bootstrap_admin_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<AuthSession, VidaError> {
        if self.has_users().await? {
            return Err(VidaError::Authentication(
                "Local admin bootstrap is only allowed when no users exist".to_string(),
            ));
        }

        self.create_user_internal(username, password, ActorRole::SuperAdmin)
            .await
    }

    pub async fn authenticate_user(
        &self,
        username: &str,
        password: &str,
    ) -> Result<AuthSession, VidaError> {
        let user = self
            .db
            .get_user_by_username(username)
            .await?
            .ok_or_else(|| VidaError::Authentication("Invalid username or password".to_string()))?;

        if user.active == 0 {
            return Err(VidaError::Authentication(
                "User account is disabled".to_string(),
            ));
        }

        let verified = PinManager::verify_password(password, &user.password_hash)
            .map_err(|e| VidaError::Authentication(e.to_string()))?;
        if !verified {
            return Err(VidaError::Authentication(
                "Invalid username or password".to_string(),
            ));
        }

        AuthSession::try_from(&user).map_err(VidaError::Authentication)
    }

    pub async fn bootstrap_local_admin(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<AuthSession, VidaError> {
        let session = self.bootstrap_admin_user(username, password).await?;
        self.current_actor = Some(session.clone());
        Ok(session)
    }

    pub async fn login_local(
        &mut self,
        username: &str,
        password: &str,
    ) -> Result<AuthSession, VidaError> {
        // Rate limiting
        let now = std::time::Instant::now();
        if let Some(blocked_until) = self.login_blocked_until.get(username) {
            if now < *blocked_until {
                return Err(VidaError::Authentication(
                    "Too many login attempts. Try again later.".to_string(),
                ));
            } else {
                self.login_blocked_until.remove(&username.to_string());
            }
        }

        match self.authenticate_user(username, password).await {
            Ok(session) => {
                self.login_attempts.remove(&username.to_string());
                self.current_actor = Some(session.clone());
                Ok(session)
            }
            Err(err) => {
                // Track failed attempt
                let entry = self.login_attempts.entry(username.to_string()).or_insert((0, now));
                if now.duration_since(entry.1).as_secs() > LOGIN_WINDOW_SECS {
                    *entry = (1, now); // Reset window
                } else {
                    entry.0 += 1;
                    if entry.0 >= LOGIN_MAX_ATTEMPTS {
                        self.login_blocked_until.insert(
                            username.to_string(),
                            now + std::time::Duration::from_secs(LOGIN_BLOCK_SECS),
                        );
                        self.login_attempts.remove(&username.to_string());
                    }
                }
                Err(err)
            }
        }
    }

    pub fn logout_local(&mut self) {
        self.current_actor = None;
    }

    pub async fn list_users(&self) -> Result<Vec<AuthUser>, VidaError> {
        self.db
            .list_users()
            .await?
            .into_iter()
            .map(AuthUser::try_from)
            .collect::<Result<Vec<_>, _>>()
            .map_err(VidaError::Authentication)
    }

    pub async fn create_user(
        &self,
        username: &str,
        password: &str,
        role: ActorRole,
    ) -> Result<AuthUser, VidaError> {
        self.create_user_internal(username, password, role).await?;
        let user = self
            .db
            .get_user_by_username(username)
            .await?
            .ok_or_else(|| VidaError::Authentication("User was not created".to_string()))?;
        AuthUser::try_from(user).map_err(VidaError::Authentication)
    }

    pub async fn change_current_password(
        &self,
        current_password: &str,
        new_password: &str,
    ) -> Result<(), VidaError> {
        let actor = self
            .current_actor
            .clone()
            .ok_or_else(|| VidaError::Authentication("Authentication required".to_string()))?;

        self.change_password_for_actor(&actor, current_password, new_password)
            .await
    }

    pub async fn change_password_for_actor(
        &self,
        actor: &AuthSession,
        current_password: &str,
        new_password: &str,
    ) -> Result<(), VidaError> {
        let actor = actor.clone();

        let user = self
            .db
            .get_user(&actor.user_id)
            .await?
            .ok_or_else(|| VidaError::Authentication("User not found".to_string()))?;

        let verified = PinManager::verify_password(current_password, &user.password_hash)
            .map_err(|e| VidaError::Authentication(e.to_string()))?;
        if !verified {
            return Err(VidaError::Authentication(
                "Current password is invalid".to_string(),
            ));
        }

        let new_hash = hash_auth_password(new_password)?;
        self.db
            .update_user_password(&actor.user_id, &new_hash)
            .await?;
        Ok(())
    }

    async fn create_user_internal(
        &self,
        username: &str,
        password: &str,
        role: ActorRole,
    ) -> Result<AuthSession, VidaError> {
        validate_username(username)?;
        validate_password(password)?;

        if self.db.get_user_by_username(username).await?.is_some() {
            return Err(VidaError::Authentication(format!(
                "User '{username}' already exists"
            )));
        }

        let password_hash = hash_auth_password(password)?;
        let user = UserRow {
            id: Uuid::new_v4().to_string(),
            username: username.trim().to_string(),
            password_hash,
            role: actor_role_storage(role).to_string(),
            active: 1,
            created_at: String::new(),
        };

        self.db.create_user(&user).await?;
        AuthSession::try_from(&user).map_err(VidaError::Authentication)
    }

    // ── Security ──

    pub async fn is_pin_configured(&self) -> Result<bool, VidaError> {
        self.has_users().await
    }

    pub async fn store_api_key(&mut self, provider_id: &str, key: &str) -> Result<(), VidaError> {
        self.secrets
            .store(&format!("{}-api-key", provider_id), key)?;
        self.refresh_providers().await?;
        Ok(())
    }

    pub async fn remove_api_key(&mut self, provider_id: &str) -> Result<(), VidaError> {
        self.secrets.delete(&format!("{}-api-key", provider_id))?;
        self.refresh_providers().await?;
        Ok(())
    }

    pub fn get_api_key(&self, provider_id: &str) -> Result<String, VidaError> {
        Ok(self.secrets.get(&format!("{}-api-key", provider_id))?)
    }

    async fn seed_default_provider_configs(&self) -> Result<(), VidaError> {
        self.db
            .ensure_provider_config(
                "ollama",
                "ollama",
                Some(DEFAULT_OLLAMA_URL),
                Some(DEFAULT_OLLAMA_MODEL),
            )
            .await?;
        self.db
            .ensure_provider_config(
                "openai",
                "openai",
                Some(DEFAULT_OPENAI_URL),
                Some(DEFAULT_OPENAI_MODEL),
            )
            .await?;
        self.db
            .ensure_provider_config(
                "anthropic",
                "anthropic",
                Some(DEFAULT_ANTHROPIC_URL),
                Some(DEFAULT_ANTHROPIC_MODEL),
            )
            .await?;
        self.db
            .ensure_provider_config(
                "google",
                "google",
                Some(DEFAULT_GOOGLE_URL),
                Some(DEFAULT_GOOGLE_MODEL),
            )
            .await?;
        // OpenAI-compatible cloud providers
        self.db.ensure_provider_config("groq", "cloud", Some(DEFAULT_GROQ_URL), Some(DEFAULT_GROQ_MODEL)).await?;
        self.db.ensure_provider_config("mistral", "cloud", Some(DEFAULT_MISTRAL_URL), Some(DEFAULT_MISTRAL_MODEL)).await?;
        self.db.ensure_provider_config("deepseek", "cloud", Some(DEFAULT_DEEPSEEK_URL), Some(DEFAULT_DEEPSEEK_MODEL)).await?;
        self.db.ensure_provider_config("cerebras", "cloud", Some(DEFAULT_CEREBRAS_URL), Some(DEFAULT_CEREBRAS_MODEL)).await?;
        self.db.ensure_provider_config("nvidia", "cloud", Some(DEFAULT_NVIDIA_URL), Some(DEFAULT_NVIDIA_MODEL)).await?;
        self.db.ensure_provider_config("sambanova", "cloud", Some(DEFAULT_SAMBANOVA_URL), Some(DEFAULT_SAMBANOVA_MODEL)).await?;
        self.db.ensure_provider_config("openrouter", "cloud", Some(DEFAULT_OPENROUTER_URL), Some(DEFAULT_OPENROUTER_MODEL)).await?;
        self.db.ensure_provider_config("zhipuai", "cloud", Some(DEFAULT_ZHIPUAI_URL), Some(DEFAULT_ZHIPUAI_MODEL)).await?;
        self.db.ensure_provider_config("dashscope", "cloud", Some(DEFAULT_DASHSCOPE_URL), Some(DEFAULT_DASHSCOPE_MODEL)).await?;
        Ok(())
    }

    pub async fn refresh_providers(&mut self) -> Result<(), VidaError> {
        let configs = self.db.list_providers().await?;
        let mut registry = ProviderRegistry::new();

        for config in configs.iter().filter(|config| config.enabled != 0) {
            if let Some(provider) = self.build_provider(config)? {
                let _ = registry.add(config.id.clone(), provider);
            }
        }

        self.providers = registry;
        Ok(())
    }

    fn build_provider(
        &self,
        config: &ProviderConfigRow,
    ) -> Result<Option<Arc<dyn LLMProvider>>, VidaError> {
        let id = config.id.to_ascii_lowercase();
        let kind = config.provider_type.to_ascii_lowercase();
        let base_url = config.base_url.as_deref();
        let default_model = config.default_model.as_deref();

        let provider: Arc<dyn LLMProvider> = match id.as_str() {
            "ollama" => Arc::new(OllamaProvider::new(base_url.unwrap_or(DEFAULT_OLLAMA_URL))),
            "openai" => Arc::new(OpenAIProvider::new(
                base_url.unwrap_or(DEFAULT_OPENAI_URL),
                &self.provider_api_key("openai"),
                default_model.unwrap_or(DEFAULT_OPENAI_MODEL),
            )),
            "anthropic" => Arc::new(AnthropicProvider::new(
                base_url.unwrap_or(DEFAULT_ANTHROPIC_URL),
                &self.provider_api_key("anthropic"),
                default_model.unwrap_or(DEFAULT_ANTHROPIC_MODEL),
            )),
            "google" => Arc::new(GoogleProvider::new(
                base_url.unwrap_or(DEFAULT_GOOGLE_URL),
                &self.provider_api_key("google"),
                default_model.unwrap_or(DEFAULT_GOOGLE_MODEL),
            )),
            "groq" => Arc::new(OpenAIProvider::with_name(
                base_url.unwrap_or(DEFAULT_GROQ_URL),
                &self.provider_api_key("groq"),
                default_model.unwrap_or(DEFAULT_GROQ_MODEL),
                "groq",
                "Groq",
            )),
            "mistral" => Arc::new(OpenAIProvider::with_name(
                base_url.unwrap_or(DEFAULT_MISTRAL_URL),
                &self.provider_api_key("mistral"),
                default_model.unwrap_or(DEFAULT_MISTRAL_MODEL),
                "mistral",
                "Mistral",
            )),
            "deepseek" => Arc::new(OpenAIProvider::with_name(
                base_url.unwrap_or(DEFAULT_DEEPSEEK_URL),
                &self.provider_api_key("deepseek"),
                default_model.unwrap_or(DEFAULT_DEEPSEEK_MODEL),
                "deepseek",
                "DeepSeek",
            )),
            "cerebras" => Arc::new(OpenAIProvider::with_name(
                base_url.unwrap_or(DEFAULT_CEREBRAS_URL),
                &self.provider_api_key("cerebras"),
                default_model.unwrap_or(DEFAULT_CEREBRAS_MODEL),
                "cerebras",
                "Cerebras",
            )),
            "nvidia" => Arc::new(OpenAIProvider::with_name(
                base_url.unwrap_or(DEFAULT_NVIDIA_URL),
                &self.provider_api_key("nvidia"),
                default_model.unwrap_or(DEFAULT_NVIDIA_MODEL),
                "nvidia",
                "NVIDIA NIM",
            )),
            "sambanova" => Arc::new(OpenAIProvider::with_name(
                base_url.unwrap_or(DEFAULT_SAMBANOVA_URL),
                &self.provider_api_key("sambanova"),
                default_model.unwrap_or(DEFAULT_SAMBANOVA_MODEL),
                "sambanova",
                "SambaNova",
            )),
            "openrouter" => Arc::new(OpenAIProvider::with_name(
                base_url.unwrap_or(DEFAULT_OPENROUTER_URL),
                &self.provider_api_key("openrouter"),
                default_model.unwrap_or(DEFAULT_OPENROUTER_MODEL),
                "openrouter",
                "OpenRouter",
            )),
            "zhipuai" => Arc::new(OpenAIProvider::with_name(
                base_url.unwrap_or(DEFAULT_ZHIPUAI_URL),
                &self.provider_api_key("zhipuai"),
                default_model.unwrap_or(DEFAULT_ZHIPUAI_MODEL),
                "zhipuai",
                "ZhipuAI",
            )),
            "dashscope" => Arc::new(OpenAIProvider::with_name(
                base_url.unwrap_or(DEFAULT_DASHSCOPE_URL),
                &self.provider_api_key("dashscope"),
                default_model.unwrap_or(DEFAULT_DASHSCOPE_MODEL),
                "dashscope",
                "DashScope",
            )),
            _ if kind.contains("ollama") => {
                Arc::new(OllamaProvider::new(base_url.unwrap_or(DEFAULT_OLLAMA_URL)))
            }
            _ if kind.contains("openai") || kind.contains("cloud") => {
                Arc::new(OpenAIProvider::with_name(
                    base_url.unwrap_or(DEFAULT_OPENAI_URL),
                    &self.provider_api_key(&config.id),
                    default_model.unwrap_or(DEFAULT_OPENAI_MODEL),
                    &config.id,
                    &config.id,
                ))
            }
            _ if kind.contains("anthropic") => Arc::new(AnthropicProvider::new(
                base_url.unwrap_or(DEFAULT_ANTHROPIC_URL),
                &self.provider_api_key(&config.id),
                default_model.unwrap_or(DEFAULT_ANTHROPIC_MODEL),
            )),
            _ if kind.contains("google") || kind.contains("gemini") => {
                Arc::new(GoogleProvider::new(
                    base_url.unwrap_or(DEFAULT_GOOGLE_URL),
                    &self.provider_api_key(&config.id),
                    default_model.unwrap_or(DEFAULT_GOOGLE_MODEL),
                ))
            }
            _ => return Ok(None),
        };

        Ok(Some(provider))
    }

    fn provider_api_key(&self, provider_id: &str) -> String {
        // 1. Try keychain first
        if let Ok(key) = self.secrets.get(&format!("{}-api-key", provider_id)) {
            if !key.is_empty() {
                return key;
            }
        }
        // 2. Fallback to environment variable: {PROVIDER_ID}_API_KEY
        let env_var = format!("{}_API_KEY", provider_id.to_uppercase());
        if let Ok(key) = std::env::var(&env_var) {
            if !key.is_empty() {
                return key;
            }
        }
        // 3. Special mappings for common env var names
        let alt_env = match provider_id {
            "google" => Some("GEMINI_API_KEY"),
            "nvidia" => Some("NVIDIA_API_KEY"),
            "dashscope" => Some("DASHSCOPE_API_KEY"),
            "zhipuai" => Some("ZHIPUAI_API_KEY"),
            _ => None,
        };
        if let Some(var) = alt_env {
            if let Ok(key) = std::env::var(var) {
                if !key.is_empty() {
                    return key;
                }
            }
        }
        String::new()
    }

    async fn provider_default_models(&self) -> std::collections::HashMap<String, String> {
        self.db
            .list_providers()
            .await
            .unwrap_or_default()
            .into_iter()
            .filter_map(|config| config.default_model.map(|model| (config.id, model)))
            .collect()
    }

    fn agent_tool_context(&self, scope_id: &str) -> Result<AgentToolContext, VidaError> {
        let sandbox_root = if let Some(workspace_path) = &self.workspace_path {
            Path::new(workspace_path)
                .join(".vida")
                .join("sandboxes")
                .join(scope_id)
        } else {
            std::env::temp_dir()
                .join("vida-ai")
                .join("sandboxes")
                .join(scope_id)
        };

        std::fs::create_dir_all(&sandbox_root)
            .map_err(|e| VidaError::Config(format!("Failed to create agent sandbox: {e}")))?;

        Ok(AgentToolContext {
            workspace_root: self.workspace_path.as_ref().map(PathBuf::from),
            sandbox_root,
        })
    }

    // ── Teams ──

    /// Create a new team with members. Each member is a (provider_id, model) tuple.
    /// Colors are auto-assigned from the palette.
    pub async fn create_team(
        &self,
        name: &str,
        members: Vec<(String, String)>,
    ) -> Result<TeamRow, VidaError> {
        if members.is_empty() {
            return Err(VidaError::Config(
                "Team must contain at least one member".to_string(),
            ));
        }

        let team = TeamRow {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            mode: "parallel".to_string(),
            description: None,
            system_prompt: None,
            created_at: String::new(),
        };
        self.db.create_team(&team).await?;

        for (i, (provider_id, model)) in members.iter().enumerate() {
            let color = TEAM_COLORS[i % TEAM_COLORS.len()];
            let provider_label = self
                .providers
                .get(provider_id)
                .map(|provider| provider.info().display_name)
                .unwrap_or_else(|| provider_id.clone());
            let display_name = format!("{} ({})", model, provider_label);
            let member = TeamMemberRow {
                id: Uuid::new_v4().to_string(),
                team_id: team.id.clone(),
                provider_id: provider_id.clone(),
                model: model.clone(),
                display_name: Some(display_name),
                color: color.to_string(),
                role: Some(default_team_role(i).to_string()),
                department: None,
                system_prompt: None,
                created_at: String::new(),
            };
            self.db.add_team_member(&member).await?;
        }

        Ok(team)
    }

    /// List all teams.
    pub async fn list_teams(&self) -> Result<Vec<TeamRow>, VidaError> {
        Ok(self.db.list_teams().await?)
    }

    /// Get a team with all its members.
    pub async fn get_team_with_members(
        &self,
        team_id: &str,
    ) -> Result<(TeamRow, Vec<TeamMemberRow>), VidaError> {
        let team = self
            .db
            .get_team(team_id)
            .await?
            .ok_or_else(|| VidaError::Config(format!("Team not found: {}", team_id)))?;
        let members = self.db.get_team_members(team_id).await?;
        Ok((team, members))
    }

    pub async fn set_team_member_role(
        &self,
        team_id: &str,
        member_id: &str,
        role: &str,
    ) -> Result<TeamMemberRow, VidaError> {
        let normalized_role = normalize_team_role(role)?;
        let (_, members) = self.get_team_with_members(team_id).await?;
        let target = members
            .iter()
            .find(|member| member.id == member_id)
            .ok_or_else(|| VidaError::Config(format!("Team member not found: {}", member_id)))?;

        let owner_count = members
            .iter()
            .filter(|member| team_member_role(member) == TEAM_ROLE_OWNER)
            .count();

        if team_member_role(target) == TEAM_ROLE_OWNER
            && normalized_role != TEAM_ROLE_OWNER
            && owner_count <= 1
        {
            return Err(VidaError::Config(
                "Team must keep at least one owner".to_string(),
            ));
        }

        self.db
            .update_team_member_role(member_id, &normalized_role)
            .await?;

        let (_, updated_members) = self.get_team_with_members(team_id).await?;
        updated_members
            .into_iter()
            .find(|member| member.id == member_id)
            .ok_or_else(|| {
                VidaError::Config(format!("Team member not found after update: {}", member_id))
            })
    }

    /// Delete a team and its members (cascade).
    pub async fn delete_team(&self, id: &str) -> Result<(), VidaError> {
        Ok(self.db.delete_team(id).await?)
    }

    /// Create a session associated with a team.
    /// Uses the first member's provider_id/model as the session's provider (required by schema),
    /// and stores team_id on the session.
    pub async fn create_team_session(&self, team_id: &str) -> Result<SessionRow, VidaError> {
        let (team, members) = self.get_team_with_members(team_id).await?;
        let runnable_members: Vec<_> = members
            .iter()
            .filter(|member| team_role_can_execute(team_member_role(member)))
            .collect();
        if runnable_members.is_empty() {
            return Err(VidaError::Config(format!(
                "Team '{}' has no member with execution access",
                team.name
            )));
        }

        let first = runnable_members[0];
        let session = SessionRow {
            id: Uuid::new_v4().to_string(),
            title: Some(format!("Team: {}", team.name)),
            provider_id: first.provider_id.clone(),
            model: first.model.clone(),
            system_prompt: None,
            created_at: String::new(),
            updated_at: String::new(),
            team_id: Some(team_id.to_string()),
        };
        self.db.create_session(&session).await?;
        Ok(session)
    }

    // ── Workspaces ──

    /// Open a workspace directory, load its .vida/config.json, update recent list.
    pub async fn open_workspace(&mut self, path: &str) -> Result<WorkspaceConfig, VidaError> {
        let workspace_path = std::path::Path::new(path);
        if !workspace_path.exists() {
            return Err(VidaError::Config(format!(
                "Workspace path does not exist: {}",
                path
            )));
        }

        let config = load_workspace_config(workspace_path)?;
        self.permission_manager =
            PermissionManager::new(config.permission_mode.clone(), config.permissions.clone());
        self.workspace_config = config.clone();
        self.workspace_path = Some(path.to_string());

        // Update recent workspaces in DB
        self.db.add_recent_workspace(path, &config.name).await?;

        Ok(config)
    }

    /// Create a new workspace with .vida/config.json defaults.
    pub async fn create_workspace(
        &mut self,
        path: &str,
        name: &str,
    ) -> Result<WorkspaceConfig, VidaError> {
        let workspace_path = std::path::Path::new(path);
        let mut config = WorkspaceConfig::default();
        config.name = name.to_string();

        save_workspace_config(workspace_path, &config)?;

        self.permission_manager =
            PermissionManager::new(config.permission_mode.clone(), config.permissions.clone());
        self.workspace_config = config.clone();
        self.workspace_path = Some(path.to_string());

        // Update recent workspaces in DB
        self.db.add_recent_workspace(path, name).await?;

        Ok(config)
    }

    /// List recent workspaces from DB.
    pub async fn list_recent_workspaces(&self) -> Result<Vec<RecentWorkspaceRow>, VidaError> {
        Ok(self.db.list_recent_workspaces(20).await?)
    }

    /// Get the current workspace config.
    pub fn get_workspace_config(&self) -> &WorkspaceConfig {
        &self.workspace_config
    }

    /// Update the workspace config. If a workspace is open, saves to disk.
    pub fn set_workspace_config(&mut self, config: WorkspaceConfig) -> Result<(), VidaError> {
        self.permission_manager =
            PermissionManager::new(config.permission_mode.clone(), config.permissions.clone());

        if let Some(ref path) = self.workspace_path {
            save_workspace_config(std::path::Path::new(path), &config)?;
        }

        self.workspace_config = config;
        Ok(())
    }

    /// Get current permission mode.
    pub fn get_permission_mode(&self) -> &PermissionMode {
        self.permission_manager.mode()
    }

    /// Set permission mode.
    pub fn set_permission_mode(&mut self, mode: PermissionMode) -> Result<(), VidaError> {
        self.permission_manager.set_mode(mode.clone());
        self.workspace_config.permission_mode = mode;

        if let Some(ref path) = self.workspace_path {
            save_workspace_config(std::path::Path::new(path), &self.workspace_config)?;
        }

        Ok(())
    }

    /// Check a permission against the current permission manager.
    pub fn check_permission(&self, perm: PermissionType) -> PermissionResult {
        self.permission_manager.check(perm)
    }

    // ── MCP ──

    /// Start an MCP server by config name. Looks up the config in DB, spawns the process.
    pub async fn start_mcp_server(&mut self, name: &str) -> Result<Vec<McpTool>, VidaError> {
        // Find config in DB
        let configs = self
            .db
            .list_mcp_servers(self.workspace_path.as_deref())
            .await?;
        let config = configs
            .iter()
            .find(|c| c.name == name)
            .ok_or_else(|| VidaError::Config(format!("MCP server config not found: {}", name)))?;

        let args: Vec<String> = config
            .args_json
            .as_ref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default();
        let env: std::collections::HashMap<String, String> = config
            .env_json
            .as_ref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default();

        self.mcp_manager
            .start_server(name, &config.command, &args, &env)
            .map_err(|e| VidaError::Config(e.to_string()))
    }

    /// Stop a running MCP server.
    pub fn stop_mcp_server(&mut self, name: &str) -> Result<(), VidaError> {
        self.mcp_manager
            .stop_server(name)
            .map_err(|e| VidaError::Config(e.to_string()))
    }

    /// List all MCP servers (from DB configs + running status).
    pub async fn list_mcp_servers(&self) -> Result<Vec<McpServerInfo>, VidaError> {
        let configs = self
            .db
            .list_mcp_servers(self.workspace_path.as_deref())
            .await?;
        let running = self.mcp_manager.list_servers();
        let running_map: std::collections::HashMap<String, &McpServerInfo> =
            running.iter().map(|s| (s.name.clone(), s)).collect();

        let mut servers = Vec::new();
        for config in &configs {
            if let Some(running_info) = running_map.get(&config.name) {
                servers.push((*running_info).clone());
            } else {
                servers.push(McpServerInfo {
                    name: config.name.clone(),
                    command: config.command.clone(),
                    running: false,
                    tool_count: 0,
                    tools: vec![],
                });
            }
        }
        Ok(servers)
    }

    /// List all tools from all running MCP servers.
    pub fn list_mcp_tools(&self) -> Vec<McpTool> {
        self.mcp_manager.list_tools()
    }

    /// Call an MCP tool by name.
    pub fn call_mcp_tool(
        &mut self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, VidaError> {
        self.mcp_manager
            .call_tool(tool_name, arguments)
            .map_err(|e| VidaError::Config(e.to_string()))
    }

    /// Save an MCP server configuration to DB.
    pub async fn save_mcp_server_config(
        &self,
        config: &McpServerConfigRow,
    ) -> Result<(), VidaError> {
        self.db.upsert_mcp_server(config).await?;
        Ok(())
    }

    // ── Remote Server ──

    /// Generate a new remote API token and store it in the secret store.
    pub fn generate_remote_token(&self) -> Result<String, VidaError> {
        let token = crate::remote::generate_token();
        self.secrets.store("remote-api-token", &token)?;
        Ok(token)
    }

    /// Get the current remote API token from the secret store.
    pub fn get_remote_token(&self) -> Result<String, VidaError> {
        Ok(self.secrets.get("remote-api-token")?)
    }

    /// Delete an MCP server configuration from DB.
    pub async fn delete_mcp_server_config(&mut self, id: &str) -> Result<(), VidaError> {
        // Also stop if running
        if let Some(config) = self.db.get_mcp_server(id).await? {
            if self.mcp_manager.is_running(&config.name) {
                let _ = self.mcp_manager.stop_server(&config.name);
            }
        }
        self.db.delete_mcp_server(id).await?;
        Ok(())
    }

    /// Send a message to ALL team members in parallel.
    /// Each agent streams its response independently via TeamStreamEvent.
    pub async fn send_team_message_stream(
        &self,
        session_id: &str,
        content: &str,
        tx: mpsc::Sender<TeamStreamEvent>,
    ) -> Result<(), VidaError> {
        let session = self
            .db
            .get_session(session_id)
            .await?
            .ok_or_else(|| VidaError::SessionNotFound(session_id.to_string()))?;

        let team_id = session
            .team_id
            .as_ref()
            .ok_or_else(|| VidaError::Config("Session has no team_id".to_string()))?;

        let team = self
            .db
            .get_team(team_id)
            .await?
            .ok_or_else(|| VidaError::Config(format!("Team not found: {team_id}")))?;

        let members: Vec<_> = self
            .db
            .get_team_members(team_id)
            .await?
            .into_iter()
            .filter(|member| team_role_can_execute(team_member_role(member)))
            .collect();
        if members.is_empty() {
            return Err(VidaError::Config(
                "Team has no member with execution access".to_string(),
            ));
        }

        // Load history
        let history = self.db.get_messages(session_id).await?;

        // Insert user message
        let user_msg = MessageRow {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: "user".to_string(),
            content: content.to_string(),
            token_count: None,
            created_at: String::new(),
            agent_id: None,
            agent_name: None,
            agent_color: None,
        };
        self.db.insert_message(&user_msg).await?;

        // Build chat messages from history (shared across agents)
        let mut chat_messages = Vec::new();
        // 1. Session-level system prompt
        if let Some(ref prompt) = session.system_prompt {
            chat_messages.push(ChatMessage {
                role: ChatRole::System,
                content: prompt.clone(),
                tool_call_id: None,
                name: None,
            });
        }
        // 2. Team-level system prompt (shared context for all agents)
        if let Some(ref team_prompt) = team.system_prompt {
            if !team_prompt.is_empty() {
                chat_messages.push(ChatMessage {
                    role: ChatRole::System,
                    content: team_prompt.clone(),
                    tool_call_id: None,
                    name: None,
                });
            }
        }
        for msg in &history {
            let role = match msg.role.as_str() {
                "system" => ChatRole::System,
                "user" => ChatRole::User,
                _ => ChatRole::Assistant,
            };
            chat_messages.push(ChatMessage {
                role,
                content: msg.content.clone(),
                tool_call_id: None,
                name: None,
            });
        }
        chat_messages.push(ChatMessage {
            role: ChatRole::User,
            content: content.to_string(),
            tool_call_id: None,
            name: None,
        });

        let done_count = Arc::new(AtomicUsize::new(0));
        let total_members = members.len();
        let sid = session_id.to_string();

        // Spawn a task for each team member
        for member in &members {
            let provider = match self.providers.get(&member.provider_id) {
                Some(p) => p,
                None => {
                    let _ = tx
                        .send(TeamStreamEvent::AgentError {
                            agent_id: member.id.clone(),
                            error: format!("Provider not found: {}", member.provider_id),
                        })
                        .await;
                    let count = done_count.fetch_add(1, Ordering::SeqCst) + 1;
                    if count == total_members {
                        let _ = tx.send(TeamStreamEvent::AllDone).await;
                    }
                    continue;
                }
            };

            let agent_id = member.id.clone();
            let agent_name = member
                .display_name
                .clone()
                .unwrap_or_else(|| format!("{}/{}", member.provider_id, member.model));
            let agent_color = member.color.clone();
            let model = member.model.clone();
            // 3. Per-agent system prompt (personality/role)
            let mut messages_clone = chat_messages.clone();
            if let Some(ref member_prompt) = member.system_prompt {
                if !member_prompt.is_empty() {
                    messages_clone.push(ChatMessage {
                        role: ChatRole::System,
                        content: member_prompt.clone(),
                        tool_call_id: None,
                        name: None,
                    });
                }
            }
            let tx_clone = tx.clone();
            let done_count_clone = done_count.clone();
            let total = total_members;
            let db_pool = self.db.pool().clone();
            let session_id_clone = sid.clone();

            tokio::spawn(async move {
                let options = CompletionOptions {
                    model: Some(model),
                    ..Default::default()
                };

                let (inner_tx, mut inner_rx) = mpsc::channel::<StreamEvent>(100);

                // Spawn the provider stream
                let provider_clone = provider.clone();
                tokio::spawn(async move {
                    let _ = provider_clone
                        .chat_completion_stream(&messages_clone, Some(options), inner_tx)
                        .await;
                });

                // Forward tokens as TeamStreamEvent
                let mut full_content = String::new();
                while let Some(event) = inner_rx.recv().await {
                    match event {
                        StreamEvent::Token { content } => {
                            full_content.push_str(&content);
                            let _ = tx_clone
                                .send(TeamStreamEvent::AgentToken {
                                    agent_id: agent_id.clone(),
                                    agent_name: agent_name.clone(),
                                    agent_color: agent_color.clone(),
                                    content,
                                })
                                .await;
                        }
                        StreamEvent::Error { error } => {
                            let _ = tx_clone
                                .send(TeamStreamEvent::AgentError {
                                    agent_id: agent_id.clone(),
                                    error,
                                })
                                .await;
                        }
                        StreamEvent::Done => {
                            break;
                        }
                    }
                }

                // Save agent's response to DB
                if !full_content.is_empty() {
                    let msg = MessageRow {
                        id: Uuid::new_v4().to_string(),
                        session_id: session_id_clone,
                        role: "assistant".to_string(),
                        content: full_content,
                        token_count: None,
                        created_at: String::new(),
                        agent_id: Some(agent_id.clone()),
                        agent_name: Some(agent_name.clone()),
                        agent_color: Some(agent_color.clone()),
                    };
                    // Use the pool directly to insert
                    let _ = sqlx::query(
                        "INSERT INTO messages (id, session_id, role, content, token_count, agent_id, agent_name, agent_color, created_at)
                         VALUES (?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))"
                    )
                    .bind(&msg.id)
                    .bind(&msg.session_id)
                    .bind(&msg.role)
                    .bind(&msg.content)
                    .bind(msg.token_count)
                    .bind(&msg.agent_id)
                    .bind(&msg.agent_name)
                    .bind(&msg.agent_color)
                    .execute(&db_pool)
                    .await;
                }

                let _ = tx_clone
                    .send(TeamStreamEvent::AgentDone {
                        agent_id: agent_id.clone(),
                    })
                    .await;

                let count = done_count_clone.fetch_add(1, Ordering::SeqCst) + 1;
                if count == total {
                    let _ = tx_clone.send(TeamStreamEvent::AllDone).await;
                }
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicUsize, Ordering as TestOrdering};
    use std::sync::Arc;
    use vida_db::ProviderConfigRow;

    struct MockProvider;
    struct MockToolProvider {
        calls: AtomicUsize,
    }

    impl MockToolProvider {
        fn new() -> Self {
            Self {
                calls: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl LLMProvider for MockProvider {
        async fn chat_completion(
            &self,
            _: &[ChatMessage],
            _: Option<CompletionOptions>,
        ) -> Result<CompletionResponse, ProviderError> {
            Ok(CompletionResponse {
                content: "Hello from mock!".to_string(),
                model: "mock-model".to_string(),
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                tool_calls: vec![],
            })
        }
        async fn chat_completion_stream(
            &self,
            _: &[ChatMessage],
            _: Option<CompletionOptions>,
            tx: mpsc::Sender<StreamEvent>,
        ) -> Result<(), ProviderError> {
            let _ = tx
                .send(StreamEvent::Token {
                    content: "Hello ".to_string(),
                })
                .await;
            let _ = tx
                .send(StreamEvent::Token {
                    content: "world!".to_string(),
                })
                .await;
            let _ = tx.send(StreamEvent::Done).await;
            Ok(())
        }
        async fn vision_completion(
            &self,
            _: Vec<u8>,
            _: &str,
            _: Option<CompletionOptions>,
        ) -> Result<CompletionResponse, ProviderError> {
            Err(ProviderError::Internal("not supported".to_string()))
        }
        async fn health_check(&self) -> Result<(), ProviderError> {
            Ok(())
        }
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                id: "mock".to_string(),
                display_name: "Mock".to_string(),
                provider_type: ProviderType::Local,
                models: vec!["mock-model".to_string()],
            }
        }
        async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
            Ok(vec!["mock-model".to_string()])
        }
    }

    #[async_trait]
    impl LLMProvider for MockToolProvider {
        async fn chat_completion(
            &self,
            messages: &[ChatMessage],
            _: Option<CompletionOptions>,
        ) -> Result<CompletionResponse, ProviderError> {
            let call_no = self.calls.fetch_add(1, TestOrdering::SeqCst);

            if call_no == 0 {
                assert!(messages
                    .iter()
                    .any(|msg| matches!(msg.role, ChatRole::System)
                        && msg.content.contains("You can use tools")));
                Ok(CompletionResponse {
                    content: r#"<tool_call>{"id":"call_1","name":"read_file","arguments":{"path":"docs/demo.txt"}}</tool_call>"#.to_string(),
                    model: "mock-tool".to_string(),
                    prompt_tokens: 10,
                    completion_tokens: 5,
                    total_tokens: 15,
                    tool_calls: vec![],
                })
            } else {
                assert!(messages.iter().any(|msg| matches!(msg.role, ChatRole::Tool)
                    && msg.content.contains("demo file content")));
                Ok(CompletionResponse {
                    content: "I read the file successfully.".to_string(),
                    model: "mock-tool".to_string(),
                    prompt_tokens: 12,
                    completion_tokens: 6,
                    total_tokens: 18,
                    tool_calls: vec![],
                })
            }
        }

        async fn chat_completion_stream(
            &self,
            _: &[ChatMessage],
            _: Option<CompletionOptions>,
            tx: mpsc::Sender<StreamEvent>,
        ) -> Result<(), ProviderError> {
            let _ = tx.send(StreamEvent::Done).await;
            Ok(())
        }

        async fn vision_completion(
            &self,
            _: Vec<u8>,
            _: &str,
            _: Option<CompletionOptions>,
        ) -> Result<CompletionResponse, ProviderError> {
            Err(ProviderError::Internal("not supported".to_string()))
        }

        async fn health_check(&self) -> Result<(), ProviderError> {
            Ok(())
        }

        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                id: "mock-tool".to_string(),
                display_name: "MockTool".to_string(),
                provider_type: ProviderType::Local,
                models: vec!["mock-tool".to_string()],
            }
        }

        async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
            Ok(vec!["mock-tool".to_string()])
        }
    }

    async fn setup_engine() -> VidaEngine {
        let mut engine = VidaEngine::init_in_memory().await.unwrap();

        // Register mock provider in DB and registry
        let config = ProviderConfigRow {
            id: "mock".to_string(),
            provider_type: "local".to_string(),
            base_url: None,
            default_model: Some("mock-model".to_string()),
            enabled: 1,
            config_json: None,
            created_at: String::new(),
        };
        engine.db.upsert_provider(&config).await.unwrap();
        engine
            .providers
            .add("mock".to_string(), Arc::new(MockProvider))
            .unwrap();
        engine
    }

    #[tokio::test]
    async fn test_engine_init_in_memory() {
        let engine = VidaEngine::init_in_memory().await;
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_create_session() {
        let engine = setup_engine().await;
        let session = engine.create_session("mock", "mock-model").await.unwrap();
        assert_eq!(session.provider_id, "mock");
        assert_eq!(session.model, "mock-model");
        assert!(session.team_id.is_none());
    }

    #[tokio::test]
    async fn test_send_message() {
        let mut engine = setup_engine().await;
        let session = engine.create_session("mock", "mock-model").await.unwrap();
        let response = engine.send_message(&session.id, "Hi").await.unwrap();
        assert_eq!(response.content, "Hello from mock!");

        // Check messages in DB
        let messages = engine.get_session_messages(&session.id).await.unwrap();
        assert_eq!(messages.len(), 2); // user + assistant
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
    }

    #[tokio::test]
    async fn test_send_message_session_not_found() {
        let mut engine = VidaEngine::init_in_memory().await.unwrap();
        let result = engine.send_message("nonexistent", "Hi").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_and_get_api_key() {
        let mut engine = VidaEngine::init_in_memory().await.unwrap();
        engine.store_api_key("openai", "sk-test123").await.unwrap();
        let key = engine.get_api_key("openai").unwrap();
        assert_eq!(key, "sk-test123");
    }

    #[tokio::test]
    async fn test_bootstrap_and_login_local() {
        let mut engine = VidaEngine::init_in_memory().await.unwrap();
        let actor = engine
            .bootstrap_local_admin("admin.local", "supersecret")
            .await
            .unwrap();
        assert_eq!(actor.username, "admin.local");
        assert!(matches!(actor.role, ActorRole::SuperAdmin));
        assert!(engine.auth_status().await.unwrap().has_users);

        engine.logout_local();
        let actor = engine
            .login_local("admin.local", "supersecret")
            .await
            .unwrap();
        assert_eq!(actor.username, "admin.local");
    }

    #[tokio::test]
    async fn test_create_user_and_change_password() {
        let mut engine = VidaEngine::init_in_memory().await.unwrap();
        engine
            .bootstrap_local_admin("admin.local", "supersecret")
            .await
            .unwrap();

        let user = engine
            .create_user("arch.local", "architect1", ActorRole::Architect)
            .await
            .unwrap();
        assert!(matches!(user.role, ActorRole::Architect));

        engine
            .login_local("arch.local", "architect1")
            .await
            .unwrap();
        engine
            .change_current_password("architect1", "architect2")
            .await
            .unwrap();
        engine.logout_local();
        assert!(engine
            .login_local("arch.local", "architect1")
            .await
            .is_err());
        assert!(engine.login_local("arch.local", "architect2").await.is_ok());
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let engine = setup_engine().await;
        engine.create_session("mock", "mock-model").await.unwrap();
        engine.create_session("mock", "mock-model").await.unwrap();
        let sessions = engine.list_sessions(10).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_session_cascades() {
        let mut engine = setup_engine().await;
        let session = engine.create_session("mock", "mock-model").await.unwrap();
        engine.send_message(&session.id, "test").await.unwrap();
        engine.delete_session(&session.id).await.unwrap();
        let messages = engine.get_session_messages(&session.id).await.unwrap();
        assert!(messages.is_empty());
    }

    // ── Team Tests ──

    #[tokio::test]
    async fn test_create_team() {
        let engine = setup_engine().await;
        let team = engine
            .create_team(
                "My Team",
                vec![("mock".to_string(), "mock-model".to_string())],
            )
            .await
            .unwrap();

        assert_eq!(team.name, "My Team");
        assert_eq!(team.mode, "parallel");

        let (fetched, members) = engine.get_team_with_members(&team.id).await.unwrap();
        assert_eq!(fetched.name, "My Team");
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].color, "#6366f1");
        assert_eq!(members[0].provider_id, "mock");
        assert_eq!(members[0].role.as_deref(), Some("owner"));
    }

    #[tokio::test]
    async fn test_create_team_multiple_members_colors() {
        let engine = setup_engine().await;

        // Add a second provider
        let config2 = ProviderConfigRow {
            id: "mock2".to_string(),
            provider_type: "local".to_string(),
            base_url: None,
            default_model: Some("mock-model-2".to_string()),
            enabled: 1,
            config_json: None,
            created_at: String::new(),
        };
        engine.db.upsert_provider(&config2).await.unwrap();

        let team = engine
            .create_team(
                "Multi Team",
                vec![
                    ("mock".to_string(), "mock-model".to_string()),
                    ("mock2".to_string(), "mock-model-2".to_string()),
                ],
            )
            .await
            .unwrap();

        let (_, members) = engine.get_team_with_members(&team.id).await.unwrap();
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].color, "#6366f1");
        assert_eq!(members[1].color, "#ec4899");
        assert_eq!(members[0].role.as_deref(), Some("owner"));
        assert_eq!(members[1].role.as_deref(), Some("member"));
    }

    #[tokio::test]
    async fn test_set_team_member_role() {
        let engine = setup_engine().await;
        let team = engine
            .create_team(
                "RBAC Team",
                vec![
                    ("mock".to_string(), "mock-model".to_string()),
                    ("mock".to_string(), "mock-model-2".to_string()),
                ],
            )
            .await
            .unwrap();

        let (_, members) = engine.get_team_with_members(&team.id).await.unwrap();
        let updated = engine
            .set_team_member_role(&team.id, &members[1].id, "viewer")
            .await
            .unwrap();
        assert_eq!(updated.role.as_deref(), Some("viewer"));
    }

    #[tokio::test]
    async fn test_set_team_member_role_rejects_last_owner_demotion() {
        let engine = setup_engine().await;
        let team = engine
            .create_team(
                "Owner Team",
                vec![("mock".to_string(), "mock-model".to_string())],
            )
            .await
            .unwrap();

        let (_, members) = engine.get_team_with_members(&team.id).await.unwrap();
        let err = engine
            .set_team_member_role(&team.id, &members[0].id, "viewer")
            .await
            .unwrap_err();
        assert!(err.to_string().contains("at least one owner"));
    }

    #[tokio::test]
    async fn test_list_teams() {
        let engine = setup_engine().await;
        engine
            .create_team(
                "Team A",
                vec![("mock".to_string(), "mock-model".to_string())],
            )
            .await
            .unwrap();
        engine
            .create_team(
                "Team B",
                vec![("mock".to_string(), "mock-model".to_string())],
            )
            .await
            .unwrap();

        let teams = engine.list_teams().await.unwrap();
        assert_eq!(teams.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_team() {
        let engine = setup_engine().await;
        let team = engine
            .create_team(
                "Delete Me",
                vec![("mock".to_string(), "mock-model".to_string())],
            )
            .await
            .unwrap();

        engine.delete_team(&team.id).await.unwrap();
        let teams = engine.list_teams().await.unwrap();
        assert!(teams.is_empty());
    }

    #[tokio::test]
    async fn test_create_team_session() {
        let engine = setup_engine().await;
        let team = engine
            .create_team(
                "Session Team",
                vec![("mock".to_string(), "mock-model".to_string())],
            )
            .await
            .unwrap();

        let session = engine.create_team_session(&team.id).await.unwrap();
        assert_eq!(session.team_id, Some(team.id.clone()));
        assert_eq!(session.title, Some("Team: Session Team".to_string()));
        assert_eq!(session.provider_id, "mock");
    }

    #[tokio::test]
    async fn test_send_team_message_stream() {
        let mut engine = setup_engine().await;

        // Register a second mock provider
        let config2 = ProviderConfigRow {
            id: "mock2".to_string(),
            provider_type: "local".to_string(),
            base_url: None,
            default_model: Some("mock-model".to_string()),
            enabled: 1,
            config_json: None,
            created_at: String::new(),
        };
        engine.db.upsert_provider(&config2).await.unwrap();
        engine
            .providers
            .add("mock2".to_string(), Arc::new(MockProvider))
            .unwrap();

        let team = engine
            .create_team(
                "Stream Team",
                vec![
                    ("mock".to_string(), "mock-model".to_string()),
                    ("mock2".to_string(), "mock-model".to_string()),
                ],
            )
            .await
            .unwrap();

        let session = engine.create_team_session(&team.id).await.unwrap();

        let (tx, mut rx) = mpsc::channel::<TeamStreamEvent>(100);
        engine
            .send_team_message_stream(&session.id, "Hello team", tx)
            .await
            .unwrap();

        // Collect all events
        let mut agent_tokens: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        let mut agent_dones = 0;
        let mut all_done = false;

        while let Some(event) = rx.recv().await {
            match event {
                TeamStreamEvent::AgentToken {
                    agent_id, content, ..
                } => {
                    agent_tokens.entry(agent_id).or_default().push_str(&content);
                }
                TeamStreamEvent::AgentDone { .. } => {
                    agent_dones += 1;
                }
                TeamStreamEvent::AllDone => {
                    all_done = true;
                    break;
                }
                TeamStreamEvent::AgentError { .. } => {
                    panic!("Unexpected agent error");
                }
            }
        }

        assert_eq!(agent_tokens.len(), 2);
        assert_eq!(agent_dones, 2);
        assert!(all_done);

        // Each mock sends "Hello world!"
        for (_id, content) in &agent_tokens {
            assert_eq!(content, "Hello world!");
        }

        // Verify messages saved to DB: 1 user + 2 assistant (one per agent)
        // Give spawned tasks a moment to finish DB writes
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let messages = engine.get_session_messages(&session.id).await.unwrap();
        assert_eq!(messages.len(), 3); // 1 user + 2 agents
        assert_eq!(messages[0].role, "user");

        let agent_msgs: Vec<_> = messages.iter().filter(|m| m.role == "assistant").collect();
        assert_eq!(agent_msgs.len(), 2);
        for msg in &agent_msgs {
            assert!(msg.agent_id.is_some());
            assert!(msg.agent_name.is_some());
            assert!(msg.agent_color.is_some());
            assert_eq!(msg.content, "Hello world!");
        }
    }

    // ── Workspace Tests ──

    #[tokio::test]
    async fn test_create_workspace() {
        let mut engine = VidaEngine::init_in_memory().await.unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().to_str().unwrap();

        let config = engine.create_workspace(path, "Test Project").await.unwrap();
        assert_eq!(config.name, "Test Project");
        assert_eq!(config.permission_mode, PermissionMode::Ask);
        assert_eq!(engine.workspace_path, Some(path.to_string()));

        // Check .vida/config.json was created
        assert!(tmp.path().join(".vida").join("config.json").exists());

        // Check recent workspaces
        let recent = engine.list_recent_workspaces().await.unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].name, "Test Project");
    }

    #[tokio::test]
    async fn test_open_workspace() {
        let mut engine = VidaEngine::init_in_memory().await.unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().to_str().unwrap();

        // Create workspace first
        engine.create_workspace(path, "My Workspace").await.unwrap();

        // Re-open it
        let mut engine2 = VidaEngine::init_in_memory().await.unwrap();
        let config = engine2.open_workspace(path).await.unwrap();
        assert_eq!(config.name, "My Workspace");
        assert_eq!(engine2.workspace_path, Some(path.to_string()));
    }

    #[tokio::test]
    async fn test_open_nonexistent_workspace() {
        let mut engine = VidaEngine::init_in_memory().await.unwrap();
        let result = engine.open_workspace("/nonexistent/path").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_default_providers_seeded() {
        let engine = VidaEngine::init_in_memory().await.unwrap();
        let providers = engine.list_providers().await;
        let ids: std::collections::HashSet<_> = providers.into_iter().map(|p| p.id).collect();
        assert!(ids.contains("ollama"));
        assert!(ids.contains("openai"));
        assert!(ids.contains("anthropic"));
        assert!(ids.contains("google"));
    }

    #[tokio::test]
    async fn test_send_message_with_tool_loop() {
        let mut engine = VidaEngine::init_in_memory().await.unwrap();
        let workspace = tempfile::TempDir::new().unwrap();
        engine.workspace_path = Some(workspace.path().display().to_string());

        let config = ProviderConfigRow {
            id: "mock-tool".to_string(),
            provider_type: "local".to_string(),
            base_url: None,
            default_model: Some("mock-tool".to_string()),
            enabled: 1,
            config_json: None,
            created_at: String::new(),
        };
        engine.db.upsert_provider(&config).await.unwrap();
        engine
            .providers
            .add("mock-tool".to_string(), Arc::new(MockToolProvider::new()))
            .unwrap();

        engine.mcp_manager.register_test_tool(
            crate::mcp::McpTool {
                name: "read_file".to_string(),
                description: "Read a file".to_string(),
                input_schema: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"]
                }),
                server_name: "test".to_string(),
            },
            crate::mcp::McpToolResult {
                content: vec![crate::mcp::McpToolResultContent {
                    content_type: "text".to_string(),
                    text: "demo file content".to_string(),
                }],
                is_error: false,
            },
        );

        let session = engine
            .create_session("mock-tool", "mock-tool")
            .await
            .unwrap();
        let response = engine
            .send_message(&session.id, "Read the demo file")
            .await
            .unwrap();

        assert!(response.content.contains("<tool_call>"));
        assert!(response.content.contains("<tool_result"));
        assert!(response.content.contains("I read the file successfully."));

        let messages = engine.get_session_messages(&session.id).await.unwrap();
        assert_eq!(messages.len(), 2);
        assert!(messages[1].content.contains("<tool_call>"));
    }

    #[tokio::test]
    async fn test_set_workspace_config() {
        let mut engine = VidaEngine::init_in_memory().await.unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().to_str().unwrap();

        engine.create_workspace(path, "Original").await.unwrap();

        let mut new_config = engine.get_workspace_config().clone();
        new_config.name = "Updated".to_string();
        new_config.permission_mode = PermissionMode::Yolo;
        engine.set_workspace_config(new_config).unwrap();

        assert_eq!(engine.get_workspace_config().name, "Updated");
        assert_eq!(engine.get_permission_mode(), &PermissionMode::Yolo);

        // Verify persisted to disk
        let loaded = load_workspace_config(tmp.path()).unwrap();
        assert_eq!(loaded.name, "Updated");
        assert_eq!(loaded.permission_mode, PermissionMode::Yolo);
    }

    #[tokio::test]
    async fn test_set_permission_mode() {
        let mut engine = VidaEngine::init_in_memory().await.unwrap();
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().to_str().unwrap();

        engine.create_workspace(path, "Perms Test").await.unwrap();
        assert_eq!(engine.get_permission_mode(), &PermissionMode::Ask);

        engine.set_permission_mode(PermissionMode::Sandbox).unwrap();
        assert_eq!(engine.get_permission_mode(), &PermissionMode::Sandbox);

        // Check permission: file_write is false by default
        let result = engine.check_permission(PermissionType::FileWrite);
        assert_eq!(result, PermissionResult::Denied);

        // In Yolo mode, everything is allowed
        engine.set_permission_mode(PermissionMode::Yolo).unwrap();
        let result = engine.check_permission(PermissionType::FileWrite);
        assert_eq!(result, PermissionResult::Allowed);
    }

    #[tokio::test]
    async fn test_team_stream_session_not_team() {
        let engine = setup_engine().await;
        let session = engine.create_session("mock", "mock-model").await.unwrap();

        let (tx, _rx) = mpsc::channel::<TeamStreamEvent>(100);
        let result = engine.send_team_message_stream(&session.id, "Hi", tx).await;
        assert!(result.is_err()); // Session has no team_id
    }
}
