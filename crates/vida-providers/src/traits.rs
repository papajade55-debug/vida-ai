use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompletionOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_penalty: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamEvent {
    Token { content: String },
    Error { error: String },
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Local,
    Cloud,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub name: String,
    pub provider_type: ProviderType,
    pub models: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Network: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Serialization: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("Model not supported: {0}")]
    ModelNotSupported(String),
    #[error("Provider unavailable")]
    Unavailable,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Internal: {0}")]
    Internal(String),
}

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat_completion(
        &self,
        messages: &[ChatMessage],
        options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError>;

    async fn chat_completion_stream(
        &self,
        messages: &[ChatMessage],
        options: Option<CompletionOptions>,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), ProviderError>;

    async fn vision_completion(
        &self,
        image_data: Vec<u8>,
        prompt: &str,
        options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError>;

    async fn health_check(&self) -> Result<(), ProviderError>;

    fn info(&self) -> ProviderInfo;

    async fn list_models(&self) -> Result<Vec<String>, ProviderError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_role_serialization() {
        let json = serde_json::to_string(&ChatRole::User).unwrap();
        assert_eq!(json, "\"user\"");
        let role: ChatRole = serde_json::from_str("\"assistant\"").unwrap();
        assert_eq!(role, ChatRole::Assistant);
    }

    #[test]
    fn test_chat_message_serialization() {
        let msg = ChatMessage {
            role: ChatRole::System,
            content: "You are helpful.".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"system\""));
        assert!(json.contains("You are helpful."));
    }

    #[test]
    fn test_completion_options_skip_none() {
        let opts = CompletionOptions {
            temperature: Some(0.7),
            ..Default::default()
        };
        let json = serde_json::to_string(&opts).unwrap();
        assert!(json.contains("\"temperature\":0.7"));
        assert!(!json.contains("\"model\""));
        assert!(!json.contains("\"max_tokens\""));
    }

    #[test]
    fn test_stream_event_variants() {
        let token = StreamEvent::Token { content: "Hello".to_string() };
        let json = serde_json::to_string(&token).unwrap();
        assert!(json.contains("Token"));
        assert!(json.contains("Hello"));

        let done = StreamEvent::Done;
        let json = serde_json::to_string(&done).unwrap();
        assert!(json.contains("Done"));
    }

    #[test]
    fn test_provider_type_serialization() {
        let json = serde_json::to_string(&ProviderType::Local).unwrap();
        assert_eq!(json, "\"local\"");
    }
}
