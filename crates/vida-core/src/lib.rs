pub mod error;
pub mod config;
pub mod engine;
pub mod mcp;
pub mod permissions;
pub mod remote;
pub mod telegram;
pub mod workspace;

pub use error::VidaError;
pub use config::AppConfig;
pub use engine::VidaEngine;
pub use engine::TeamStreamEvent;
pub use mcp::{McpManager, McpTool, McpServerInfo, McpToolResult, McpToolResultContent, McpError};
pub use permissions::{PermissionMode, PermissionType, PermissionResult, PermissionManager, PermissionConfig};
pub use workspace::WorkspaceConfig;

#[cfg(feature = "remote")]
pub use remote::{RemoteServer, generate_token};

#[cfg(feature = "telegram")]
pub use telegram::{TelegramBot, TelegramConfig};
