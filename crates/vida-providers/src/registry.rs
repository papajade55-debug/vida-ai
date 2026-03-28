use std::collections::HashMap;
use std::sync::Arc;

use crate::traits::{LLMProvider, ProviderError, ProviderInfo};

pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn LLMProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub fn add(&mut self, name: String, provider: Arc<dyn LLMProvider>) -> Result<(), String> {
        if self.providers.contains_key(&name) {
            return Err(format!("Provider '{}' already registered", name));
        }
        self.providers.insert(name, provider);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn LLMProvider>> {
        self.providers.get(name).cloned()
    }

    pub fn list(&self) -> Vec<ProviderInfo> {
        self.providers.values().map(|p| p.info()).collect()
    }

    pub async fn health_check_all(&self) -> Vec<(String, Result<(), ProviderError>)> {
        let mut results = Vec::new();
        for (name, provider) in &self.providers {
            let result = provider.health_check().await;
            results.push((name.clone(), result));
        }
        results
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::*;
    use async_trait::async_trait;
    use tokio::sync::mpsc;

    struct FakeProvider {
        name: String,
        healthy: bool,
    }

    #[async_trait]
    impl LLMProvider for FakeProvider {
        async fn chat_completion(&self, _: &[ChatMessage], _: Option<CompletionOptions>) -> Result<CompletionResponse, ProviderError> {
            Ok(CompletionResponse {
                content: "fake".to_string(),
                model: "fake-model".to_string(),
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            })
        }
        async fn chat_completion_stream(&self, _: &[ChatMessage], _: Option<CompletionOptions>, _: mpsc::Sender<StreamEvent>) -> Result<(), ProviderError> {
            Ok(())
        }
        async fn vision_completion(&self, _: Vec<u8>, _: &str, _: Option<CompletionOptions>) -> Result<CompletionResponse, ProviderError> {
            Err(ProviderError::Internal("not supported".to_string()))
        }
        async fn health_check(&self) -> Result<(), ProviderError> {
            if self.healthy { Ok(()) } else { Err(ProviderError::Unavailable) }
        }
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: self.name.clone(),
                provider_type: ProviderType::Local,
                models: vec!["fake-model".to_string()],
            }
        }
        async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
            Ok(vec!["fake-model".to_string()])
        }
    }

    #[test]
    fn test_registry_add_and_get() {
        let mut reg = ProviderRegistry::new();
        let provider = Arc::new(FakeProvider { name: "test".to_string(), healthy: true });
        assert!(reg.add("test".to_string(), provider).is_ok());
        assert!(reg.get("test").is_some());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_duplicate_add_fails() {
        let mut reg = ProviderRegistry::new();
        let p1 = Arc::new(FakeProvider { name: "test".to_string(), healthy: true });
        let p2 = Arc::new(FakeProvider { name: "test".to_string(), healthy: true });
        assert!(reg.add("test".to_string(), p1).is_ok());
        assert!(reg.add("test".to_string(), p2).is_err());
    }

    #[test]
    fn test_registry_list() {
        let mut reg = ProviderRegistry::new();
        reg.add("a".to_string(), Arc::new(FakeProvider { name: "A".to_string(), healthy: true })).unwrap();
        reg.add("b".to_string(), Arc::new(FakeProvider { name: "B".to_string(), healthy: true })).unwrap();
        let list = reg.list();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_registry_health_check_all() {
        let mut reg = ProviderRegistry::new();
        reg.add("healthy".to_string(), Arc::new(FakeProvider { name: "H".to_string(), healthy: true })).unwrap();
        reg.add("sick".to_string(), Arc::new(FakeProvider { name: "S".to_string(), healthy: false })).unwrap();
        let results = reg.health_check_all().await;
        assert_eq!(results.len(), 2);
        let healthy_result = results.iter().find(|(n, _)| n == "healthy").unwrap();
        assert!(healthy_result.1.is_ok());
        let sick_result = results.iter().find(|(n, _)| n == "sick").unwrap();
        assert!(sick_result.1.is_err());
    }
}
