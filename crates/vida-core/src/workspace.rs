use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::VidaError;
use crate::permissions::{PermissionConfig, PermissionMode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    pub name: String,
    #[serde(default)]
    pub default_provider: Option<String>,
    #[serde(default)]
    pub default_model: Option<String>,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default)]
    pub permission_mode: PermissionMode,
    #[serde(default)]
    pub permissions: PermissionConfig,
}

impl Default for WorkspaceConfig {
    fn default() -> Self {
        Self {
            name: "Untitled".to_string(),
            default_provider: None,
            default_model: None,
            system_prompt: None,
            permission_mode: PermissionMode::Ask,
            permissions: PermissionConfig::default(),
        }
    }
}

pub fn load_workspace_config(workspace_path: &Path) -> Result<WorkspaceConfig, VidaError> {
    let config_path = workspace_path.join(".vida").join("config.json");
    if !config_path.exists() {
        return Ok(WorkspaceConfig::default());
    }
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| VidaError::Config(format!("Failed to read workspace config: {}", e)))?;
    serde_json::from_str(&content)
        .map_err(|e| VidaError::Config(format!("Invalid workspace config: {}", e)))
}

pub fn save_workspace_config(
    workspace_path: &Path,
    config: &WorkspaceConfig,
) -> Result<(), VidaError> {
    let vida_dir = workspace_path.join(".vida");
    std::fs::create_dir_all(&vida_dir)
        .map_err(|e| VidaError::Config(format!("Failed to create .vida dir: {}", e)))?;
    let config_path = vida_dir.join("config.json");
    let content = serde_json::to_string_pretty(config)
        .map_err(|e| VidaError::Config(format!("Failed to serialize config: {}", e)))?;
    std::fs::write(&config_path, content)
        .map_err(|e| VidaError::Config(format!("Failed to write config: {}", e)))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_load_default_when_no_config() {
        let tmp = TempDir::new().unwrap();
        let config = load_workspace_config(tmp.path()).unwrap();
        assert_eq!(config.name, "Untitled");
        assert_eq!(config.permission_mode, PermissionMode::Ask);
        assert!(config.default_provider.is_none());
    }

    #[test]
    fn test_save_and_load_config() {
        let tmp = TempDir::new().unwrap();
        let config = WorkspaceConfig {
            name: "My Project".to_string(),
            default_provider: Some("ollama".to_string()),
            default_model: Some("llama3".to_string()),
            system_prompt: Some("You are helpful.".to_string()),
            permission_mode: PermissionMode::Yolo,
            permissions: PermissionConfig {
                file_read: true,
                file_write: true,
                shell_execute: true,
                network_access: true,
            },
        };

        save_workspace_config(tmp.path(), &config).unwrap();
        let loaded = load_workspace_config(tmp.path()).unwrap();

        assert_eq!(loaded.name, "My Project");
        assert_eq!(loaded.default_provider, Some("ollama".to_string()));
        assert_eq!(loaded.default_model, Some("llama3".to_string()));
        assert_eq!(loaded.system_prompt, Some("You are helpful.".to_string()));
        assert_eq!(loaded.permission_mode, PermissionMode::Yolo);
        assert!(loaded.permissions.shell_execute);
    }

    #[test]
    fn test_save_creates_vida_dir() {
        let tmp = TempDir::new().unwrap();
        let config = WorkspaceConfig::default();
        save_workspace_config(tmp.path(), &config).unwrap();

        assert!(tmp.path().join(".vida").exists());
        assert!(tmp.path().join(".vida").join("config.json").exists());
    }

    #[test]
    fn test_default_workspace_config_serde_roundtrip() {
        let config = WorkspaceConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: WorkspaceConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "Untitled");
        assert_eq!(deserialized.permission_mode, PermissionMode::Ask);
    }

    #[test]
    fn test_partial_config_deserialization() {
        // Ensure missing optional fields use defaults
        let json = r#"{"name": "Minimal"}"#;
        let config: WorkspaceConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.name, "Minimal");
        assert!(config.default_provider.is_none());
        assert_eq!(config.permission_mode, PermissionMode::Ask);
        assert!(config.permissions.file_read);
        assert!(!config.permissions.file_write);
    }
}
