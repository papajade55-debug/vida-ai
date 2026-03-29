use vida_db::DbError;
use vida_providers::traits::ProviderError;
use vida_security::SecurityError;
use crate::mcp::McpError;

#[derive(Debug, thiserror::Error)]
pub enum VidaError {
    #[error("Provider: {0}")]
    Provider(#[from] ProviderError),
    #[error("Security: {0}")]
    Security(#[from] SecurityError),
    #[error("Database: {0}")]
    Database(#[from] DbError),
    #[error("MCP: {0}")]
    Mcp(#[from] McpError),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),
    #[error("Config: {0}")]
    Config(String),
}

// Serialize for Tauri commands
impl serde::Serialize for VidaError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
