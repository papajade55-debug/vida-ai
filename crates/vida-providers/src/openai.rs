use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::traits::*;

pub struct OpenAIProvider {
    client: Client,
    base_url: String,
    api_key: String,
    default_model: String,
}

impl OpenAIProvider {
    /// Create a new OpenAI-compatible provider.
    /// `base_url` can be any OpenAI-compatible endpoint (OpenAI, Groq, Mistral, Together, etc.)
    pub fn new(base_url: &str, api_key: &str, default_model: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            default_model: default_model.to_string(),
        }
    }
}

// ── OpenAI API types ──

#[derive(Serialize)]
struct OpenAIChatRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    stream: bool,
}

#[derive(Serialize)]
struct OpenAIMessage {
    role: String,
    content: serde_json::Value,
}

#[derive(Deserialize)]
struct OpenAIChatResponse {
    choices: Vec<OpenAIChoice>,
    model: String,
    #[serde(default)]
    usage: Option<OpenAIUsage>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIResponseMessage,
}

#[derive(Deserialize)]
struct OpenAIResponseMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize)]
struct OpenAIStreamChunk {
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIStreamDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIStreamDelta {
    content: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModelEntry>,
}

#[derive(Deserialize)]
struct OpenAIModelEntry {
    id: String,
}

#[derive(Deserialize)]
struct OpenAIErrorResponse {
    error: OpenAIErrorBody,
}

#[derive(Deserialize)]
struct OpenAIErrorBody {
    message: String,
}

// ── Helpers ──

fn to_openai_messages(messages: &[ChatMessage]) -> Vec<OpenAIMessage> {
    messages
        .iter()
        .map(|m| OpenAIMessage {
            role: match m.role {
                ChatRole::System => "system".to_string(),
                ChatRole::User => "user".to_string(),
                ChatRole::Assistant => "assistant".to_string(),
            },
            content: serde_json::Value::String(m.content.clone()),
        })
        .collect()
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn chat_completion(
        &self,
        messages: &[ChatMessage],
        options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError> {
        let model = options
            .as_ref()
            .and_then(|o| o.model.clone())
            .unwrap_or_else(|| self.default_model.clone());

        let request = OpenAIChatRequest {
            model,
            messages: to_openai_messages(messages),
            temperature: options.as_ref().and_then(|o| o.temperature),
            max_tokens: options.as_ref().and_then(|o| o.max_tokens),
            top_p: options.as_ref().and_then(|o| o.top_p),
            stream: false,
        };

        let resp = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;

        if resp.status() == 401 {
            return Err(ProviderError::Unauthorized);
        }

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            if let Ok(err_resp) = serde_json::from_str::<OpenAIErrorResponse>(&body) {
                return Err(ProviderError::Api(err_resp.error.message));
            }
            return Err(ProviderError::Api(body));
        }

        let oai_resp: OpenAIChatResponse = resp.json().await?;
        let choice = oai_resp.choices.first().ok_or(ProviderError::Internal("No choices in response".to_string()))?;
        let usage = oai_resp.usage.unwrap_or(OpenAIUsage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 });

        Ok(CompletionResponse {
            content: choice.message.content.clone().unwrap_or_default(),
            model: oai_resp.model,
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        })
    }

    async fn chat_completion_stream(
        &self,
        messages: &[ChatMessage],
        options: Option<CompletionOptions>,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), ProviderError> {
        let model = options
            .as_ref()
            .and_then(|o| o.model.clone())
            .unwrap_or_else(|| self.default_model.clone());

        let request = OpenAIChatRequest {
            model,
            messages: to_openai_messages(messages),
            temperature: options.as_ref().and_then(|o| o.temperature),
            max_tokens: options.as_ref().and_then(|o| o.max_tokens),
            top_p: options.as_ref().and_then(|o| o.top_p),
            stream: true,
        };

        let resp = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            let _ = tx.send(StreamEvent::Error { error: body.clone() }).await;
            let _ = tx.send(StreamEvent::Done).await;
            return Err(ProviderError::Api(body));
        }

        use futures_util::StreamExt;
        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));
                    while let Some(pos) = buffer.find('\n') {
                        let line = buffer[..pos].trim().to_string();
                        buffer = buffer[pos + 1..].to_string();
                        if line.is_empty() || !line.starts_with("data: ") {
                            continue;
                        }
                        let data = &line[6..];
                        if data == "[DONE]" {
                            let _ = tx.send(StreamEvent::Done).await;
                            return Ok(());
                        }
                        match serde_json::from_str::<OpenAIStreamChunk>(data) {
                            Ok(chunk) => {
                                if let Some(choice) = chunk.choices.first() {
                                    if let Some(content) = &choice.delta.content {
                                        if !content.is_empty() {
                                            let _ = tx.send(StreamEvent::Token {
                                                content: content.clone(),
                                            }).await;
                                        }
                                    }
                                    if choice.finish_reason.is_some() {
                                        let _ = tx.send(StreamEvent::Done).await;
                                        return Ok(());
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(StreamEvent::Error { error: e.to_string() }).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(StreamEvent::Error { error: e.to_string() }).await;
                    let _ = tx.send(StreamEvent::Done).await;
                    return Err(ProviderError::Network(e));
                }
            }
        }
        let _ = tx.send(StreamEvent::Done).await;
        Ok(())
    }

    async fn vision_completion(
        &self,
        image_data: Vec<u8>,
        prompt: &str,
        options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError> {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&image_data);

        let model = options
            .as_ref()
            .and_then(|o| o.model.clone())
            .unwrap_or_else(|| self.default_model.clone());

        let content = serde_json::json!([
            { "type": "text", "text": prompt },
            { "type": "image_url", "image_url": { "url": format!("data:image/png;base64,{}", b64) } }
        ]);

        let request = serde_json::json!({
            "model": model,
            "messages": [{ "role": "user", "content": content }],
            "max_tokens": options.as_ref().and_then(|o| o.max_tokens).unwrap_or(1024),
        });

        let resp = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api(body));
        }

        let oai_resp: OpenAIChatResponse = resp.json().await?;
        let choice = oai_resp.choices.first().ok_or(ProviderError::Internal("No choices".to_string()))?;
        let usage = oai_resp.usage.unwrap_or(OpenAIUsage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 });

        Ok(CompletionResponse {
            content: choice.message.content.clone().unwrap_or_default(),
            model: oai_resp.model,
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        })
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        let resp = self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await?;

        if resp.status() == 401 {
            return Err(ProviderError::Unauthorized);
        }
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(ProviderError::Unavailable)
        }
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "OpenAI".to_string(),
            provider_type: ProviderType::Cloud,
            models: vec![],
        }
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let resp = self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ProviderError::Unavailable);
        }

        let models: OpenAIModelsResponse = resp.json().await?;
        Ok(models.data.into_iter().map(|m| m.id).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_openai_messages() {
        let messages = vec![
            ChatMessage { role: ChatRole::User, content: "Hi".to_string() },
        ];
        let oai_msgs = to_openai_messages(&messages);
        assert_eq!(oai_msgs.len(), 1);
        assert_eq!(oai_msgs[0].role, "user");
    }

    #[test]
    fn test_openai_provider_info() {
        let provider = OpenAIProvider::new("https://api.openai.com", "sk-test", "gpt-4o");
        let info = provider.info();
        assert_eq!(info.name, "OpenAI");
        assert_eq!(info.provider_type, ProviderType::Cloud);
    }

    #[test]
    fn test_openai_compatible_base_url() {
        let provider = OpenAIProvider::new("https://api.groq.com/openai", "gsk-test", "llama3-70b");
        assert_eq!(provider.base_url, "https://api.groq.com/openai");
        assert_eq!(provider.default_model, "llama3-70b");
    }
}
