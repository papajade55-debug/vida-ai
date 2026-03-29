use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PermissionMode {
    Yolo,
    Ask,
    Sandbox,
}

impl Default for PermissionMode {
    fn default() -> Self {
        Self::Ask
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PermissionType {
    FileRead,
    FileWrite,
    ShellExecute,
    NetworkAccess,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PermissionResult {
    Allowed,
    Denied,
    NeedsApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionConfig {
    pub file_read: bool,
    pub file_write: bool,
    pub shell_execute: bool,
    pub network_access: bool,
}

impl Default for PermissionConfig {
    fn default() -> Self {
        Self {
            file_read: true,
            file_write: false,
            shell_execute: false,
            network_access: true,
        }
    }
}

pub struct PermissionManager {
    mode: PermissionMode,
    config: PermissionConfig,
}

impl PermissionManager {
    pub fn new(mode: PermissionMode, config: PermissionConfig) -> Self {
        Self { mode, config }
    }

    pub fn check(&self, perm: PermissionType) -> PermissionResult {
        match self.mode {
            PermissionMode::Yolo => PermissionResult::Allowed,
            PermissionMode::Sandbox => {
                if self.is_granted(&perm) {
                    PermissionResult::Allowed
                } else {
                    PermissionResult::Denied
                }
            }
            PermissionMode::Ask => {
                if self.is_granted(&perm) {
                    PermissionResult::Allowed
                } else {
                    PermissionResult::NeedsApproval
                }
            }
        }
    }

    fn is_granted(&self, perm: &PermissionType) -> bool {
        match perm {
            PermissionType::FileRead => self.config.file_read,
            PermissionType::FileWrite => self.config.file_write,
            PermissionType::ShellExecute => self.config.shell_execute,
            PermissionType::NetworkAccess => self.config.network_access,
        }
    }

    pub fn mode(&self) -> &PermissionMode {
        &self.mode
    }

    pub fn set_mode(&mut self, mode: PermissionMode) {
        self.mode = mode;
    }

    pub fn config(&self) -> &PermissionConfig {
        &self.config
    }

    pub fn set_config(&mut self, config: PermissionConfig) {
        self.config = config;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yolo_mode_always_allows() {
        let manager = PermissionManager::new(PermissionMode::Yolo, PermissionConfig::default());

        assert_eq!(manager.check(PermissionType::FileRead), PermissionResult::Allowed);
        assert_eq!(manager.check(PermissionType::FileWrite), PermissionResult::Allowed);
        assert_eq!(manager.check(PermissionType::ShellExecute), PermissionResult::Allowed);
        assert_eq!(manager.check(PermissionType::NetworkAccess), PermissionResult::Allowed);
    }

    #[test]
    fn test_ask_mode_with_defaults() {
        let manager = PermissionManager::new(PermissionMode::Ask, PermissionConfig::default());

        // file_read and network_access are true by default
        assert_eq!(manager.check(PermissionType::FileRead), PermissionResult::Allowed);
        assert_eq!(manager.check(PermissionType::NetworkAccess), PermissionResult::Allowed);
        // file_write and shell_execute are false by default
        assert_eq!(manager.check(PermissionType::FileWrite), PermissionResult::NeedsApproval);
        assert_eq!(manager.check(PermissionType::ShellExecute), PermissionResult::NeedsApproval);
    }

    #[test]
    fn test_sandbox_mode_denies_unganted() {
        let manager = PermissionManager::new(PermissionMode::Sandbox, PermissionConfig::default());

        // file_read and network_access are true by default
        assert_eq!(manager.check(PermissionType::FileRead), PermissionResult::Allowed);
        assert_eq!(manager.check(PermissionType::NetworkAccess), PermissionResult::Allowed);
        // file_write and shell_execute are denied (not NeedsApproval)
        assert_eq!(manager.check(PermissionType::FileWrite), PermissionResult::Denied);
        assert_eq!(manager.check(PermissionType::ShellExecute), PermissionResult::Denied);
    }

    #[test]
    fn test_sandbox_all_granted() {
        let config = PermissionConfig {
            file_read: true,
            file_write: true,
            shell_execute: true,
            network_access: true,
        };
        let manager = PermissionManager::new(PermissionMode::Sandbox, config);

        assert_eq!(manager.check(PermissionType::FileRead), PermissionResult::Allowed);
        assert_eq!(manager.check(PermissionType::FileWrite), PermissionResult::Allowed);
        assert_eq!(manager.check(PermissionType::ShellExecute), PermissionResult::Allowed);
        assert_eq!(manager.check(PermissionType::NetworkAccess), PermissionResult::Allowed);
    }

    #[test]
    fn test_ask_all_granted() {
        let config = PermissionConfig {
            file_read: true,
            file_write: true,
            shell_execute: true,
            network_access: true,
        };
        let manager = PermissionManager::new(PermissionMode::Ask, config);

        assert_eq!(manager.check(PermissionType::FileWrite), PermissionResult::Allowed);
        assert_eq!(manager.check(PermissionType::ShellExecute), PermissionResult::Allowed);
    }

    #[test]
    fn test_mode_getter() {
        let manager = PermissionManager::new(PermissionMode::Yolo, PermissionConfig::default());
        assert_eq!(manager.mode(), &PermissionMode::Yolo);
    }

    #[test]
    fn test_permission_mode_serde() {
        let mode = PermissionMode::Ask;
        let json = serde_json::to_string(&mode).unwrap();
        assert_eq!(json, "\"ask\"");

        let deserialized: PermissionMode = serde_json::from_str("\"sandbox\"").unwrap();
        assert_eq!(deserialized, PermissionMode::Sandbox);
    }

    #[test]
    fn test_permission_config_serde() {
        let config = PermissionConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: PermissionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.file_read, true);
        assert_eq!(deserialized.file_write, false);
    }
}
