pub mod access;
pub mod agent_loop;
pub mod auth;
pub mod config;
pub mod engine;
pub mod error;
pub mod mcp;
pub mod permissions;
pub mod remote;
pub mod telegram;
pub mod tool_validator;
pub mod workspace;

pub use access::{
    authorize_agent_tool_call, classify_path, evaluate_access, infer_tool_action, AccessAction,
    AccessDecision, AccessRequest, AccessResource, ActorRole, AgentToolContext,
};
pub use auth::{actor_role_storage, parse_actor_role, AuthSession, AuthStatus, AuthUser};
pub use config::AppConfig;
pub use engine::TeamStreamEvent;
pub use engine::VidaEngine;
pub use error::VidaError;
pub use mcp::{McpError, McpManager, McpServerInfo, McpTool, McpToolResult, McpToolResultContent};
pub use permissions::{
    PermissionConfig, PermissionManager, PermissionMode, PermissionResult, PermissionType,
};
pub use workspace::WorkspaceConfig;

#[cfg(feature = "remote")]
pub use remote::{generate_token, RemoteServer};

#[cfg(feature = "telegram")]
pub use telegram::{TelegramBot, TelegramConfig};
