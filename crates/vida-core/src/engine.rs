use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

use vida_db::{Database, MessageRow, SessionRow, TeamRow, TeamMemberRow, RecentWorkspaceRow, McpServerConfigRow};

use crate::mcp::{McpManager, McpTool, McpServerInfo, McpToolResult};
use vida_providers::traits::*;
use vida_providers::registry::ProviderRegistry;
use vida_security::keychain::{SecretStore, KeychainManager, MockSecretStore};

use crate::config::AppConfig;
use crate::error::VidaError;
use crate::permissions::{PermissionManager, PermissionMode, PermissionType, PermissionResult};
use crate::workspace::{WorkspaceConfig, load_workspace_config, save_workspace_config};

/// Color palette for auto-assignment to team members.
const TEAM_COLORS: &[&str] = &[
    "#6366f1", "#ec4899", "#14b8a6", "#f59e0b",
    "#8b5cf6", "#06b6d4", "#f97316", "#10b981",
];

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

        let providers = ProviderRegistry::new();
        let workspace_config = WorkspaceConfig::default();
        let permission_manager = PermissionManager::new(
            workspace_config.permission_mode.clone(),
            workspace_config.permissions.clone(),
        );

        let mcp_manager = McpManager::new();

        Ok(Self {
            db, providers, secrets, config,
            workspace_path: None,
            workspace_config,
            permission_manager,
            mcp_manager,
        })
    }

    /// Initialize with in-memory DB (for testing).
    pub async fn init_in_memory() -> Result<Self, VidaError> {
        let db = Database::connect_in_memory().await?;
        db.run_migrations().await?;
        let secrets = Box::new(MockSecretStore::new());
        let config = AppConfig::default();
        let providers = ProviderRegistry::new();
        let workspace_config = WorkspaceConfig::default();
        let permission_manager = PermissionManager::new(
            workspace_config.permission_mode.clone(),
            workspace_config.permissions.clone(),
        );
        let mcp_manager = McpManager::new();
        Ok(Self {
            db, providers, secrets, config,
            workspace_path: None,
            workspace_config,
            permission_manager,
            mcp_manager,
        })
    }

    // ── Chat ──

    pub async fn send_message(
        &self,
        session_id: &str,
        content: &str,
    ) -> Result<CompletionResponse, VidaError> {
        let session = self.db.get_session(session_id).await?
            .ok_or_else(|| VidaError::SessionNotFound(session_id.to_string()))?;

        let provider = self.providers.get(&session.provider_id)
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
            messages.push(ChatMessage { role: ChatRole::System, content: prompt.clone() });
        }
        for msg in &history {
            let role = match msg.role.as_str() {
                "system" => ChatRole::System,
                "user" => ChatRole::User,
                _ => ChatRole::Assistant,
            };
            messages.push(ChatMessage { role, content: msg.content.clone() });
        }
        messages.push(ChatMessage { role: ChatRole::User, content: content.to_string() });

        let options = CompletionOptions {
            model: Some(session.model.clone()),
            ..Default::default()
        };

        let response = provider.chat_completion(&messages, Some(options)).await?;

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
        &self,
        session_id: &str,
        content: &str,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), VidaError> {
        let session = self.db.get_session(session_id).await?
            .ok_or_else(|| VidaError::SessionNotFound(session_id.to_string()))?;

        let provider = self.providers.get(&session.provider_id)
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
            messages.push(ChatMessage { role: ChatRole::System, content: prompt.clone() });
        }
        for msg in &history {
            let role = match msg.role.as_str() {
                "system" => ChatRole::System,
                "user" => ChatRole::User,
                _ => ChatRole::Assistant,
            };
            messages.push(ChatMessage { role, content: msg.content.clone() });
        }
        messages.push(ChatMessage { role: ChatRole::User, content: content.to_string() });

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
            let _ = provider_clone.chat_completion_stream(&messages_clone, Some(options_clone), inner_tx).await;
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

    pub async fn get_session_messages(&self, session_id: &str) -> Result<Vec<MessageRow>, VidaError> {
        Ok(self.db.get_messages(session_id).await?)
    }

    pub async fn delete_session(&self, id: &str) -> Result<(), VidaError> {
        Ok(self.db.delete_session(id).await?)
    }

    // ── Providers ──

    pub fn list_providers(&self) -> Vec<ProviderInfo> {
        self.providers.list()
    }

    pub async fn list_models(&self, provider_id: &str) -> Result<Vec<String>, VidaError> {
        let provider = self.providers.get(provider_id)
            .ok_or_else(|| VidaError::ProviderNotFound(provider_id.to_string()))?;
        Ok(provider.list_models().await?)
    }

    pub async fn health_check_all(&self) -> Vec<(String, bool)> {
        self.providers.health_check_all().await
            .into_iter()
            .map(|(name, result)| (name, result.is_ok()))
            .collect()
    }

    // ── Security ──

    pub fn is_pin_configured(&self) -> Result<bool, VidaError> {
        // Check if pin_config table has a row
        // For Phase 1, delegate to DB query
        Ok(false) // Placeholder until we wire DB query
    }

    pub async fn store_api_key(&self, provider_id: &str, key: &str) -> Result<(), VidaError> {
        self.secrets.store(&format!("{}-api-key", provider_id), key)?;
        Ok(())
    }

    pub async fn remove_api_key(&self, provider_id: &str) -> Result<(), VidaError> {
        self.secrets.delete(&format!("{}-api-key", provider_id))?;
        Ok(())
    }

    pub fn get_api_key(&self, provider_id: &str) -> Result<String, VidaError> {
        Ok(self.secrets.get(&format!("{}-api-key", provider_id))?)
    }

    // ── Teams ──

    /// Create a new team with members. Each member is a (provider_id, model) tuple.
    /// Colors are auto-assigned from the palette.
    pub async fn create_team(
        &self,
        name: &str,
        members: Vec<(String, String)>,
    ) -> Result<TeamRow, VidaError> {
        let team = TeamRow {
            id: Uuid::new_v4().to_string(),
            name: name.to_string(),
            mode: "parallel".to_string(),
            created_at: String::new(),
        };
        self.db.create_team(&team).await?;

        for (i, (provider_id, model)) in members.iter().enumerate() {
            let color = TEAM_COLORS[i % TEAM_COLORS.len()];
            let display_name = format!("{} ({})", model, provider_id);
            let member = TeamMemberRow {
                id: Uuid::new_v4().to_string(),
                team_id: team.id.clone(),
                provider_id: provider_id.clone(),
                model: model.clone(),
                display_name: Some(display_name),
                color: color.to_string(),
                role: None,
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
        let team = self.db.get_team(team_id).await?
            .ok_or_else(|| VidaError::Config(format!("Team not found: {}", team_id)))?;
        let members = self.db.get_team_members(team_id).await?;
        Ok((team, members))
    }

    /// Delete a team and its members (cascade).
    pub async fn delete_team(&self, id: &str) -> Result<(), VidaError> {
        Ok(self.db.delete_team(id).await?)
    }

    /// Create a session associated with a team.
    /// Uses the first member's provider_id/model as the session's provider (required by schema),
    /// and stores team_id on the session.
    pub async fn create_team_session(
        &self,
        team_id: &str,
    ) -> Result<SessionRow, VidaError> {
        let (team, members) = self.get_team_with_members(team_id).await?;
        if members.is_empty() {
            return Err(VidaError::Config(format!("Team '{}' has no members", team.name)));
        }

        let first = &members[0];
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
            return Err(VidaError::Config(format!("Workspace path does not exist: {}", path)));
        }

        let config = load_workspace_config(workspace_path)?;
        self.permission_manager = PermissionManager::new(
            config.permission_mode.clone(),
            config.permissions.clone(),
        );
        self.workspace_config = config.clone();
        self.workspace_path = Some(path.to_string());

        // Update recent workspaces in DB
        self.db.add_recent_workspace(path, &config.name).await?;

        Ok(config)
    }

    /// Create a new workspace with .vida/config.json defaults.
    pub async fn create_workspace(&mut self, path: &str, name: &str) -> Result<WorkspaceConfig, VidaError> {
        let workspace_path = std::path::Path::new(path);
        let mut config = WorkspaceConfig::default();
        config.name = name.to_string();

        save_workspace_config(workspace_path, &config)?;

        self.permission_manager = PermissionManager::new(
            config.permission_mode.clone(),
            config.permissions.clone(),
        );
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
        self.permission_manager = PermissionManager::new(
            config.permission_mode.clone(),
            config.permissions.clone(),
        );

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
        let configs = self.db.list_mcp_servers(self.workspace_path.as_deref()).await?;
        let config = configs.iter()
            .find(|c| c.name == name)
            .ok_or_else(|| VidaError::Config(format!("MCP server config not found: {}", name)))?;

        let args: Vec<String> = config.args_json.as_ref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default();
        let env: std::collections::HashMap<String, String> = config.env_json.as_ref()
            .and_then(|j| serde_json::from_str(j).ok())
            .unwrap_or_default();

        self.mcp_manager.start_server(name, &config.command, &args, &env)
            .map_err(|e| VidaError::Config(e.to_string()))
    }

    /// Stop a running MCP server.
    pub fn stop_mcp_server(&mut self, name: &str) -> Result<(), VidaError> {
        self.mcp_manager.stop_server(name)
            .map_err(|e| VidaError::Config(e.to_string()))
    }

    /// List all MCP servers (from DB configs + running status).
    pub async fn list_mcp_servers(&self) -> Result<Vec<McpServerInfo>, VidaError> {
        let configs = self.db.list_mcp_servers(self.workspace_path.as_deref()).await?;
        let running = self.mcp_manager.list_servers();
        let running_map: std::collections::HashMap<String, &McpServerInfo> = running.iter()
            .map(|s| (s.name.clone(), s))
            .collect();

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
        self.mcp_manager.call_tool(tool_name, arguments)
            .map_err(|e| VidaError::Config(e.to_string()))
    }

    /// Save an MCP server configuration to DB.
    pub async fn save_mcp_server_config(&self, config: &McpServerConfigRow) -> Result<(), VidaError> {
        self.db.upsert_mcp_server(config).await?;
        Ok(())
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
        let session = self.db.get_session(session_id).await?
            .ok_or_else(|| VidaError::SessionNotFound(session_id.to_string()))?;

        let team_id = session.team_id.as_ref()
            .ok_or_else(|| VidaError::Config("Session has no team_id".to_string()))?;

        let members = self.db.get_team_members(team_id).await?;
        if members.is_empty() {
            return Err(VidaError::Config("Team has no members".to_string()));
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
        if let Some(ref prompt) = session.system_prompt {
            chat_messages.push(ChatMessage { role: ChatRole::System, content: prompt.clone() });
        }
        for msg in &history {
            let role = match msg.role.as_str() {
                "system" => ChatRole::System,
                "user" => ChatRole::User,
                _ => ChatRole::Assistant,
            };
            chat_messages.push(ChatMessage { role, content: msg.content.clone() });
        }
        chat_messages.push(ChatMessage { role: ChatRole::User, content: content.to_string() });

        let done_count = Arc::new(AtomicUsize::new(0));
        let total_members = members.len();
        let sid = session_id.to_string();

        // Spawn a task for each team member
        for member in &members {
            let provider = match self.providers.get(&member.provider_id) {
                Some(p) => p,
                None => {
                    let _ = tx.send(TeamStreamEvent::AgentError {
                        agent_id: member.id.clone(),
                        error: format!("Provider not found: {}", member.provider_id),
                    }).await;
                    let count = done_count.fetch_add(1, Ordering::SeqCst) + 1;
                    if count == total_members {
                        let _ = tx.send(TeamStreamEvent::AllDone).await;
                    }
                    continue;
                }
            };

            let agent_id = member.id.clone();
            let agent_name = member.display_name.clone()
                .unwrap_or_else(|| format!("{}/{}", member.provider_id, member.model));
            let agent_color = member.color.clone();
            let model = member.model.clone();
            let messages_clone = chat_messages.clone();
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
                    let _ = provider_clone.chat_completion_stream(
                        &messages_clone,
                        Some(options),
                        inner_tx,
                    ).await;
                });

                // Forward tokens as TeamStreamEvent
                let mut full_content = String::new();
                while let Some(event) = inner_rx.recv().await {
                    match event {
                        StreamEvent::Token { content } => {
                            full_content.push_str(&content);
                            let _ = tx_clone.send(TeamStreamEvent::AgentToken {
                                agent_id: agent_id.clone(),
                                agent_name: agent_name.clone(),
                                agent_color: agent_color.clone(),
                                content,
                            }).await;
                        }
                        StreamEvent::Error { error } => {
                            let _ = tx_clone.send(TeamStreamEvent::AgentError {
                                agent_id: agent_id.clone(),
                                error,
                            }).await;
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

                let _ = tx_clone.send(TeamStreamEvent::AgentDone {
                    agent_id: agent_id.clone(),
                }).await;

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
    use std::sync::Arc;
    use async_trait::async_trait;
    use vida_db::ProviderConfigRow;

    struct MockProvider;

    #[async_trait]
    impl LLMProvider for MockProvider {
        async fn chat_completion(&self, _: &[ChatMessage], _: Option<CompletionOptions>) -> Result<CompletionResponse, ProviderError> {
            Ok(CompletionResponse {
                content: "Hello from mock!".to_string(),
                model: "mock-model".to_string(),
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            })
        }
        async fn chat_completion_stream(&self, _: &[ChatMessage], _: Option<CompletionOptions>, tx: mpsc::Sender<StreamEvent>) -> Result<(), ProviderError> {
            let _ = tx.send(StreamEvent::Token { content: "Hello ".to_string() }).await;
            let _ = tx.send(StreamEvent::Token { content: "world!".to_string() }).await;
            let _ = tx.send(StreamEvent::Done).await;
            Ok(())
        }
        async fn vision_completion(&self, _: Vec<u8>, _: &str, _: Option<CompletionOptions>) -> Result<CompletionResponse, ProviderError> {
            Err(ProviderError::Internal("not supported".to_string()))
        }
        async fn health_check(&self) -> Result<(), ProviderError> { Ok(()) }
        fn info(&self) -> ProviderInfo {
            ProviderInfo { name: "Mock".to_string(), provider_type: ProviderType::Local, models: vec!["mock-model".to_string()] }
        }
        async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
            Ok(vec!["mock-model".to_string()])
        }
    }

    async fn setup_engine() -> VidaEngine {
        let mut engine = VidaEngine::init_in_memory().await.unwrap();

        // Register mock provider in DB and registry
        let config = ProviderConfigRow {
            id: "mock".to_string(),
            provider_type: "local".to_string(),
            base_url: None, default_model: Some("mock-model".to_string()),
            enabled: 1, config_json: None, created_at: String::new(),
        };
        engine.db.upsert_provider(&config).await.unwrap();
        engine.providers.add("mock".to_string(), Arc::new(MockProvider)).unwrap();
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
        let engine = setup_engine().await;
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
        let engine = VidaEngine::init_in_memory().await.unwrap();
        let result = engine.send_message("nonexistent", "Hi").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_and_get_api_key() {
        let engine = VidaEngine::init_in_memory().await.unwrap();
        engine.store_api_key("openai", "sk-test123").await.unwrap();
        let key = engine.get_api_key("openai").unwrap();
        assert_eq!(key, "sk-test123");
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
        let engine = setup_engine().await;
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
        let team = engine.create_team("My Team", vec![
            ("mock".to_string(), "mock-model".to_string()),
        ]).await.unwrap();

        assert_eq!(team.name, "My Team");
        assert_eq!(team.mode, "parallel");

        let (fetched, members) = engine.get_team_with_members(&team.id).await.unwrap();
        assert_eq!(fetched.name, "My Team");
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].color, "#6366f1");
        assert_eq!(members[0].provider_id, "mock");
    }

    #[tokio::test]
    async fn test_create_team_multiple_members_colors() {
        let engine = setup_engine().await;

        // Add a second provider
        let config2 = ProviderConfigRow {
            id: "mock2".to_string(),
            provider_type: "local".to_string(),
            base_url: None, default_model: Some("mock-model-2".to_string()),
            enabled: 1, config_json: None, created_at: String::new(),
        };
        engine.db.upsert_provider(&config2).await.unwrap();

        let team = engine.create_team("Multi Team", vec![
            ("mock".to_string(), "mock-model".to_string()),
            ("mock2".to_string(), "mock-model-2".to_string()),
        ]).await.unwrap();

        let (_, members) = engine.get_team_with_members(&team.id).await.unwrap();
        assert_eq!(members.len(), 2);
        assert_eq!(members[0].color, "#6366f1");
        assert_eq!(members[1].color, "#ec4899");
    }

    #[tokio::test]
    async fn test_list_teams() {
        let engine = setup_engine().await;
        engine.create_team("Team A", vec![("mock".to_string(), "mock-model".to_string())]).await.unwrap();
        engine.create_team("Team B", vec![("mock".to_string(), "mock-model".to_string())]).await.unwrap();

        let teams = engine.list_teams().await.unwrap();
        assert_eq!(teams.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_team() {
        let engine = setup_engine().await;
        let team = engine.create_team("Delete Me", vec![
            ("mock".to_string(), "mock-model".to_string()),
        ]).await.unwrap();

        engine.delete_team(&team.id).await.unwrap();
        let teams = engine.list_teams().await.unwrap();
        assert!(teams.is_empty());
    }

    #[tokio::test]
    async fn test_create_team_session() {
        let engine = setup_engine().await;
        let team = engine.create_team("Session Team", vec![
            ("mock".to_string(), "mock-model".to_string()),
        ]).await.unwrap();

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
            base_url: None, default_model: Some("mock-model".to_string()),
            enabled: 1, config_json: None, created_at: String::new(),
        };
        engine.db.upsert_provider(&config2).await.unwrap();
        engine.providers.add("mock2".to_string(), Arc::new(MockProvider)).unwrap();

        let team = engine.create_team("Stream Team", vec![
            ("mock".to_string(), "mock-model".to_string()),
            ("mock2".to_string(), "mock-model".to_string()),
        ]).await.unwrap();

        let session = engine.create_team_session(&team.id).await.unwrap();

        let (tx, mut rx) = mpsc::channel::<TeamStreamEvent>(100);
        engine.send_team_message_stream(&session.id, "Hello team", tx).await.unwrap();

        // Collect all events
        let mut agent_tokens: std::collections::HashMap<String, String> = std::collections::HashMap::new();
        let mut agent_dones = 0;
        let mut all_done = false;

        while let Some(event) = rx.recv().await {
            match event {
                TeamStreamEvent::AgentToken { agent_id, content, .. } => {
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
