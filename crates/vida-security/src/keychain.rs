use crate::SecurityError;

pub trait SecretStore: Send + Sync {
    fn store(&self, key: &str, value: &str) -> Result<(), SecurityError>;
    fn get(&self, key: &str) -> Result<String, SecurityError>;
    fn delete(&self, key: &str) -> Result<(), SecurityError>;
    fn list(&self) -> Result<Vec<String>, SecurityError>;
}

/// Production implementation using OS keychain via `keyring` crate.
pub struct KeychainManager {
    service: String,
    /// Track stored keys (keyring doesn't support listing)
    stored_keys: std::sync::Mutex<Vec<String>>,
}

impl KeychainManager {
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_string(),
            stored_keys: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl SecretStore for KeychainManager {
    fn store(&self, key: &str, value: &str) -> Result<(), SecurityError> {
        let entry = keyring::Entry::new(&self.service, key)
            .map_err(|e| SecurityError::KeychainAccess(e.to_string()))?;
        entry
            .set_password(value)
            .map_err(|e| SecurityError::KeychainAccess(e.to_string()))?;
        let mut keys = self.stored_keys.lock().unwrap();
        if !keys.contains(&key.to_string()) {
            keys.push(key.to_string());
        }
        Ok(())
    }

    fn get(&self, key: &str) -> Result<String, SecurityError> {
        let entry = keyring::Entry::new(&self.service, key)
            .map_err(|e| SecurityError::KeychainAccess(e.to_string()))?;
        entry
            .get_password()
            .map_err(|e| SecurityError::SecretNotFound(format!("{}: {}", key, e)))
    }

    fn delete(&self, key: &str) -> Result<(), SecurityError> {
        let entry = keyring::Entry::new(&self.service, key)
            .map_err(|e| SecurityError::KeychainAccess(e.to_string()))?;
        entry
            .delete_credential()
            .map_err(|e| SecurityError::KeychainAccess(e.to_string()))?;
        let mut keys = self.stored_keys.lock().unwrap();
        keys.retain(|k| k != key);
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>, SecurityError> {
        let keys = self.stored_keys.lock().unwrap();
        Ok(keys.clone())
    }
}

/// In-memory mock for testing — no OS keychain needed.
pub struct MockSecretStore {
    secrets: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl MockSecretStore {
    pub fn new() -> Self {
        Self {
            secrets: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for MockSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for MockSecretStore {
    fn store(&self, key: &str, value: &str) -> Result<(), SecurityError> {
        self.secrets
            .lock()
            .unwrap()
            .insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn get(&self, key: &str) -> Result<String, SecurityError> {
        self.secrets
            .lock()
            .unwrap()
            .get(key)
            .cloned()
            .ok_or_else(|| SecurityError::SecretNotFound(key.to_string()))
    }

    fn delete(&self, key: &str) -> Result<(), SecurityError> {
        self.secrets.lock().unwrap().remove(key);
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>, SecurityError> {
        Ok(self.secrets.lock().unwrap().keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_store_roundtrip() {
        let store = MockSecretStore::new();
        store.store("api-key", "sk-123").unwrap();
        assert_eq!(store.get("api-key").unwrap(), "sk-123");
    }

    #[test]
    fn test_mock_store_not_found() {
        let store = MockSecretStore::new();
        assert!(store.get("nonexistent").is_err());
    }

    #[test]
    fn test_mock_store_delete() {
        let store = MockSecretStore::new();
        store.store("key", "value").unwrap();
        store.delete("key").unwrap();
        assert!(store.get("key").is_err());
    }

    #[test]
    fn test_mock_store_list() {
        let store = MockSecretStore::new();
        store.store("a", "1").unwrap();
        store.store("b", "2").unwrap();
        let list = store.list().unwrap();
        assert_eq!(list.len(), 2);
    }
}
