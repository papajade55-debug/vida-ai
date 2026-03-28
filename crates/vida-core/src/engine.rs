use std::path::Path;
use tokio::sync::mpsc;
use uuid::Uuid;

use vida_db::{Database, MessageRow, SessionRow};
use vida_providers::traits::*;
use vida_providers::registry::ProviderRegistry;
use vida_security::keychain::{SecretStore, KeychainManager, MockSecretStore};

use crate::config::AppConfig;
use crate::error::VidaError;

pub struct VidaEngine {
    pub db: Database,
    pub providers: ProviderRegistry,
    pub secrets: Box<dyn SecretStore>,
    pub config: AppConfig,
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

        Ok(Self { db, providers, secrets, config })
    }

    /// Initialize with in-memory DB (for testing).
    pub async fn init_in_memory() -> Result<Self, VidaError> {
        let db = Database::connect_in_memory().await?;
        db.run_migrations().await?;
        let secrets = Box::new(MockSecretStore::new());
        let config = AppConfig::default();
        let providers = ProviderRegistry::new();
        Ok(Self { db, providers, secrets, config })
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
}
