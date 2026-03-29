use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::traits::*;

pub struct GoogleProvider {
    client: Client,
    base_url: String,
    api_key: String,
    default_model: String,
}

impl GoogleProvider {
    /// Create a new Google Gemini provider.
    /// `base_url` defaults to `https://generativelanguage.googleapis.com`.
    pub fn new(base_url: &str, api_key: &str, default_model: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            default_model: default_model.to_string(),
        }
    }
}

// ── Google Gemini API types ──

#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
}

#[derive(Serialize, Deserialize, Clone)]
struct GeminiContent {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    parts: Vec<GeminiPart>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(untagged)]
enum GeminiPart {
    Text {
        text: String,
    },
    InlineData {
        inline_data: GeminiInlineData,
    },
}

#[derive(Serialize, Deserialize, Clone)]
struct GeminiInlineData {
    mime_type: String,
    data: String,
}

#[derive(Serialize)]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<i32>,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(default)]
    usage_metadata: Option<GeminiUsageMetadata>,
    #[serde(default)]
    model_version: Option<String>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Deserialize)]
struct GeminiUsageMetadata {
    #[serde(default)]
    prompt_token_count: u32,
    #[serde(default)]
    candidates_token_count: u32,
    #[serde(default)]
    total_token_count: u32,
}

#[derive(Deserialize)]
struct GeminiModelsResponse {
    models: Vec<GeminiModelEntry>,
}

#[derive(Deserialize)]
struct GeminiModelEntry {
    name: String,
    #[serde(default)]
    supported_generation_methods: Vec<String>,
}

#[derive(Deserialize)]
struct GeminiErrorResponse {
    error: GeminiErrorBody,
}

#[derive(Deserialize)]
struct GeminiErrorBody {
    message: String,
}

// ── Helpers ──

/// Convert ChatMessage list to Gemini format.
/// System messages are extracted into system_instruction.
/// Gemini uses "user" and "model" roles (not "assistant").
fn extract_system_and_contents(
    messages: &[ChatMessage],
) -> (Option<GeminiContent>, Vec<GeminiContent>) {
    let mut system_parts = Vec::new();
    let mut contents = Vec::new();

    for m in messages {
        match m.role {
            ChatRole::System => {
                system_parts.push(GeminiPart::Text {
                    text: m.content.clone(),
                });
            }
            ChatRole::User => {
                contents.push(GeminiContent {
                    role: Some("user".to_string()),
                    parts: vec![GeminiPart::Text {
                        text: m.content.clone(),
                    }],
                });
            }
            ChatRole::Assistant => {
                contents.push(GeminiContent {
                    role: Some("model".to_string()),
                    parts: vec![GeminiPart::Text {
                        text: m.content.clone(),
                    }],
                });
            }
        }
    }

    let system_instruction = if system_parts.is_empty() {
        None
    } else {
        Some(GeminiContent {
            role: None,
            parts: system_parts,
        })
    };

    (system_instruction, contents)
}

#[async_trait]
impl LLMProvider for GoogleProvider {
    async fn chat_completion(
        &self,
        messages: &[ChatMessage],
        options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError> {
        let model = options
            .as_ref()
            .and_then(|o| o.model.clone())
            .unwrap_or_else(|| self.default_model.clone());

        let (system_instruction, contents) = extract_system_and_contents(messages);

        let generation_config = options.as_ref().map(|o| GeminiGenerationConfig {
            temperature: o.temperature,
            max_output_tokens: o.max_tokens,
            top_p: o.top_p,
            top_k: o.top_k,
        });

        let request = GeminiRequest {
            contents,
            system_instruction,
            generation_config,
        };

        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.base_url, model, self.api_key
        );

        let resp = self.client.post(&url).json(&request).send().await?;

        if resp.status() == 401 || resp.status() == 403 {
            return Err(ProviderError::Unauthorized);
        }

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            if let Ok(err_resp) = serde_json::from_str::<GeminiErrorResponse>(&body) {
                return Err(ProviderError::Api(err_resp.error.message));
            }
            return Err(ProviderError::Api(body));
        }

        let gemini_resp: GeminiResponse = resp.json().await?;
        let candidate = gemini_resp
            .candidates
            .first()
            .ok_or(ProviderError::Internal(
                "No candidates in response".to_string(),
            ))?;

        let content = candidate
            .content
            .parts
            .iter()
            .filter_map(|p| match p {
                GeminiPart::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        let usage = gemini_resp.usage_metadata.unwrap_or(GeminiUsageMetadata {
            prompt_token_count: 0,
            candidates_token_count: 0,
            total_token_count: 0,
        });

        let model_name = gemini_resp
            .model_version
            .unwrap_or_else(|| model.clone());

        Ok(CompletionResponse {
            content,
            model: model_name,
            prompt_tokens: usage.prompt_token_count,
            completion_tokens: usage.candidates_token_count,
            total_tokens: usage.total_token_count,
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

        let (system_instruction, contents) = extract_system_and_contents(messages);

        let generation_config = options.as_ref().map(|o| GeminiGenerationConfig {
            temperature: o.temperature,
            max_output_tokens: o.max_tokens,
            top_p: o.top_p,
            top_k: o.top_k,
        });

        let request = GeminiRequest {
            contents,
            system_instruction,
            generation_config,
        };

        let url = format!(
            "{}/v1beta/models/{}:streamGenerateContent?alt=sse&key={}",
            self.base_url, model, self.api_key
        );

        let resp = self.client.post(&url).json(&request).send().await?;

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
                        match serde_json::from_str::<GeminiResponse>(data) {
                            Ok(resp) => {
                                if let Some(candidate) = resp.candidates.first() {
                                    for part in &candidate.content.parts {
                                        if let GeminiPart::Text { text } = part {
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
                            }
                            Err(e) => {
                                let _ = tx
                                    .send(StreamEvent::Error {
                                        error: e.to_string(),
                                    })
                                    .await;
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

        let contents = vec![GeminiContent {
            role: Some("user".to_string()),
            parts: vec![
                GeminiPart::Text {
                    text: prompt.to_string(),
                },
                GeminiPart::InlineData {
                    inline_data: GeminiInlineData {
                        mime_type: "image/png".to_string(),
                        data: b64,
                    },
                },
            ],
        }];

        let generation_config = Some(GeminiGenerationConfig {
            temperature: options.as_ref().and_then(|o| o.temperature),
            max_output_tokens: Some(
                options.as_ref().and_then(|o| o.max_tokens).unwrap_or(1024),
            ),
            top_p: options.as_ref().and_then(|o| o.top_p),
            top_k: options.as_ref().and_then(|o| o.top_k),
        });

        let request = GeminiRequest {
            contents,
            system_instruction: None,
            generation_config,
        };

        let url = format!(
            "{}/v1beta/models/{}:generateContent?key={}",
            self.base_url, model, self.api_key
        );

        let resp = self.client.post(&url).json(&request).send().await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api(body));
        }

        let gemini_resp: GeminiResponse = resp.json().await?;
        let candidate = gemini_resp
            .candidates
            .first()
            .ok_or(ProviderError::Internal("No candidates".to_string()))?;

        let content = candidate
            .content
            .parts
            .iter()
            .filter_map(|p| match p {
                GeminiPart::Text { text } => Some(text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");

        let usage = gemini_resp.usage_metadata.unwrap_or(GeminiUsageMetadata {
            prompt_token_count: 0,
            candidates_token_count: 0,
            total_token_count: 0,
        });

        Ok(CompletionResponse {
            content,
            model: gemini_resp.model_version.unwrap_or(model),
            prompt_tokens: usage.prompt_token_count,
            completion_tokens: usage.candidates_token_count,
            total_tokens: usage.total_token_count,
        })
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        let url = format!(
            "{}/v1beta/models?key={}",
            self.base_url, self.api_key
        );

        let resp = self.client.get(&url).send().await?;

        if resp.status() == 401 || resp.status() == 403 {
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
            name: "Google".to_string(),
            provider_type: ProviderType::Cloud,
            models: vec![],
        }
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let url = format!(
            "{}/v1beta/models?key={}",
            self.base_url, self.api_key
        );

        let resp = self.client.get(&url).send().await?;

        if !resp.status().is_success() {
            return Err(ProviderError::Unavailable);
        }

        let models_resp: GeminiModelsResponse = resp.json().await?;

        Ok(models_resp
            .models
            .into_iter()
            .filter(|m| {
                m.supported_generation_methods
                    .iter()
                    .any(|method| method == "generateContent")
            })
            .map(|m| {
                // Strip "models/" prefix if present
                m.name
                    .strip_prefix("models/")
                    .unwrap_or(&m.name)
                    .to_string()
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_google_provider_info() {
        let provider = GoogleProvider::new(
            "https://generativelanguage.googleapis.com",
            "test-key",
            "gemini-2.0-flash",
        );
        let info = provider.info();
        assert_eq!(info.name, "Google");
        assert_eq!(info.provider_type, ProviderType::Cloud);
    }

    #[test]
    fn test_extract_system_and_contents() {
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
        let (system, contents) = extract_system_and_contents(&messages);
        assert!(system.is_some());
        assert_eq!(contents.len(), 2);
        assert_eq!(contents[0].role, Some("user".to_string()));
        assert_eq!(contents[1].role, Some("model".to_string()));
    }

    #[test]
    fn test_google_compatible_base_url() {
        let provider = GoogleProvider::new(
            "https://custom-proxy.example.com/",
            "key-123",
            "gemini-2.0-flash",
        );
        assert_eq!(provider.base_url, "https://custom-proxy.example.com");
        assert_eq!(provider.default_model, "gemini-2.0-flash");
    }
}
