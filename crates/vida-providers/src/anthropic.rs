use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::traits::*;

pub struct AnthropicProvider {
    client: Client,
    base_url: String,
    api_key: String,
    default_model: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider.
    /// `base_url` defaults to `https://api.anthropic.com` but can be overridden.
    pub fn new(base_url: &str, api_key: &str, default_model: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            default_model: default_model.to_string(),
        }
    }
}

// ── Anthropic API types ──

#[derive(Serialize)]
struct AnthropicChatRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<i32>,
    stream: bool,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: serde_json::Value,
}

#[derive(Deserialize)]
struct AnthropicChatResponse {
    content: Vec<AnthropicContentBlock>,
    model: String,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    #[serde(default)]
    text: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicUsage {
    #[serde(default)]
    input_tokens: u32,
    #[serde(default)]
    output_tokens: u32,
}

#[derive(Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    delta: Option<AnthropicStreamDelta>,
}

#[derive(Deserialize)]
struct AnthropicStreamDelta {
    #[serde(rename = "type")]
    #[serde(default)]
    #[allow(dead_code)]
    delta_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicErrorResponse {
    error: AnthropicErrorBody,
}

#[derive(Deserialize)]
struct AnthropicErrorBody {
    message: String,
}

// ── Helpers ──

/// Convert ChatMessage list to Anthropic format.
/// Extracts system messages into a separate string (Anthropic requires system as top-level field).
fn extract_system_and_messages(messages: &[ChatMessage]) -> (Option<String>, Vec<AnthropicMessage>) {
    let mut system_parts = Vec::new();
    let mut anthropic_msgs = Vec::new();

    for m in messages {
        match m.role {
            ChatRole::System => {
                system_parts.push(m.content.clone());
            }
            ChatRole::User => {
                anthropic_msgs.push(AnthropicMessage {
                    role: "user".to_string(),
                    content: serde_json::Value::String(m.content.clone()),
                });
            }
            ChatRole::Assistant => {
                anthropic_msgs.push(AnthropicMessage {
                    role: "assistant".to_string(),
                    content: serde_json::Value::String(m.content.clone()),
                });
            }
        }
    }

    let system = if system_parts.is_empty() {
        None
    } else {
        Some(system_parts.join("\n"))
    };

    (system, anthropic_msgs)
}

#[async_trait]
impl LLMProvider for AnthropicProvider {
    async fn chat_completion(
        &self,
        messages: &[ChatMessage],
        options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError> {
        let model = options
            .as_ref()
            .and_then(|o| o.model.clone())
            .unwrap_or_else(|| self.default_model.clone());

        let (system, anthropic_messages) = extract_system_and_messages(messages);

        let request = AnthropicChatRequest {
            model,
            messages: anthropic_messages,
            system,
            temperature: options.as_ref().and_then(|o| o.temperature),
            max_tokens: Some(options.as_ref().and_then(|o| o.max_tokens).unwrap_or(4096)),
            top_p: options.as_ref().and_then(|o| o.top_p),
            top_k: options.as_ref().and_then(|o| o.top_k),
            stream: false,
        };

        let resp = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if resp.status() == 401 {
            return Err(ProviderError::Unauthorized);
        }

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            if let Ok(err_resp) = serde_json::from_str::<AnthropicErrorResponse>(&body) {
                return Err(ProviderError::Api(err_resp.error.message));
            }
            return Err(ProviderError::Api(body));
        }

        let api_resp: AnthropicChatResponse = resp.json().await?;

        let content = api_resp
            .content
            .iter()
            .filter(|b| b.block_type == "text")
            .filter_map(|b| b.text.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("");

        let usage = api_resp.usage.unwrap_or(AnthropicUsage {
            input_tokens: 0,
            output_tokens: 0,
        });

        Ok(CompletionResponse {
            content,
            model: api_resp.model,
            prompt_tokens: usage.input_tokens,
            completion_tokens: usage.output_tokens,
            total_tokens: usage.input_tokens + usage.output_tokens,
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

        let (system, anthropic_messages) = extract_system_and_messages(messages);

        let request = AnthropicChatRequest {
            model,
            messages: anthropic_messages,
            system,
            temperature: options.as_ref().and_then(|o| o.temperature),
            max_tokens: Some(options.as_ref().and_then(|o| o.max_tokens).unwrap_or(4096)),
            top_p: options.as_ref().and_then(|o| o.top_p),
            top_k: options.as_ref().and_then(|o| o.top_k),
            stream: true,
        };

        let resp = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
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

                        if line.is_empty() {
                            continue;
                        }

                        // Anthropic SSE: lines prefixed with "event: " and "data: "
                        if !line.starts_with("data: ") {
                            continue;
                        }

                        let data = &line[6..];
                        match serde_json::from_str::<AnthropicStreamEvent>(data) {
                            Ok(event) => {
                                match event.event_type.as_str() {
                                    "content_block_delta" => {
                                        if let Some(delta) = &event.delta {
                                            if let Some(text) = &delta.text {
                                                if !text.is_empty() {
                                                    let _ = tx
                                                        .send(StreamEvent::Token {
                                                            content: text.clone(),
                                                        })
                                                        .await;
                                                }
                                            }
                                        }
                                    }
                                    "message_stop" => {
                                        let _ = tx.send(StreamEvent::Done).await;
                                        return Ok(());
                                    }
                                    "message_delta" => {
                                        if let Some(delta) = &event.delta {
                                            if delta.stop_reason.is_some() {
                                                let _ = tx.send(StreamEvent::Done).await;
                                                return Ok(());
                                            }
                                        }
                                    }
                                    _ => {} // message_start, content_block_start, content_block_stop, ping
                                }
                            }
                            Err(_) => {
                                // Non-JSON line or unknown format — skip
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
            {
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": "image/png",
                    "data": b64
                }
            },
            {
                "type": "text",
                "text": prompt
            }
        ]);

        let request = serde_json::json!({
            "model": model,
            "max_tokens": options.as_ref().and_then(|o| o.max_tokens).unwrap_or(1024),
            "messages": [{
                "role": "user",
                "content": content
            }]
        });

        let resp = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api(body));
        }

        let api_resp: AnthropicChatResponse = resp.json().await?;

        let text_content = api_resp
            .content
            .iter()
            .filter(|b| b.block_type == "text")
            .filter_map(|b| b.text.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("");

        let usage = api_resp.usage.unwrap_or(AnthropicUsage {
            input_tokens: 0,
            output_tokens: 0,
        });

        Ok(CompletionResponse {
            content: text_content,
            model: api_resp.model,
            prompt_tokens: usage.input_tokens,
            completion_tokens: usage.output_tokens,
            total_tokens: usage.input_tokens + usage.output_tokens,
        })
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        // Anthropic has no /models endpoint; send a minimal request to check auth.
        // We use a HEAD-like approach: send a tiny messages request.
        // If we get 401, it's unauthorized. If we get any other response, the API is reachable.
        let request = serde_json::json!({
            "model": self.default_model,
            "max_tokens": 1,
            "messages": [{"role": "user", "content": "hi"}]
        });

        let resp = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        if resp.status() == 401 {
            return Err(ProviderError::Unauthorized);
        }
        // Any non-network response means the API is reachable
        Ok(())
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "Anthropic".to_string(),
            provider_type: ProviderType::Cloud,
            models: vec![],
        }
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        // Anthropic has no public models listing endpoint; return hardcoded list.
        Ok(vec![
            "claude-sonnet-4-20250514".to_string(),
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-5-haiku-20241022".to_string(),
            "claude-3-opus-20240229".to_string(),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_anthropic_provider_info() {
        let provider =
            AnthropicProvider::new("https://api.anthropic.com", "sk-ant-test", "claude-sonnet-4-20250514");
        let info = provider.info();
        assert_eq!(info.name, "Anthropic");
        assert_eq!(info.provider_type, ProviderType::Cloud);
    }

    #[test]
    fn test_extract_system_and_messages() {
        let messages = vec![
            ChatMessage {
                role: ChatRole::System,
                content: "You are helpful.".to_string(),
            },
            ChatMessage {
                role: ChatRole::User,
                content: "Hi".to_string(),
            },
            ChatMessage {
                role: ChatRole::Assistant,
                content: "Hello!".to_string(),
            },
        ];
        let (system, msgs) = extract_system_and_messages(&messages);
        assert_eq!(system, Some("You are helpful.".to_string()));
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[1].role, "assistant");
    }

    #[test]
    fn test_anthropic_compatible_base_url() {
        let provider =
            AnthropicProvider::new("https://custom-proxy.example.com/", "key-123", "claude-3-5-haiku-20241022");
        assert_eq!(
            provider.base_url,
            "https://custom-proxy.example.com"
        );
        assert_eq!(provider.default_model, "claude-3-5-haiku-20241022");
    }
}
