use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::traits::*;

pub struct OllamaProvider {
    client: Client,
    base_url: String,
}

impl OllamaProvider {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

// ── Ollama API types ──

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OllamaToolDefinition>>,
}

#[derive(Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Vec<String>>,
}

#[derive(Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repeat_penalty: Option<f32>,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaResponseMessage,
    model: String,
    #[serde(default)]
    prompt_eval_count: u32,
    #[serde(default)]
    eval_count: u32,
    #[serde(default)]
    done: bool,
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    #[serde(default)]
    content: String,
    #[serde(default)]
    tool_calls: Vec<OllamaResponseToolCall>,
}

#[derive(Deserialize)]
struct OllamaResponseToolCall {
    function: OllamaResponseFunction,
}

#[derive(Deserialize)]
struct OllamaResponseFunction {
    name: String,
    arguments: serde_json::Value,
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
}

// ── Ollama tool types ──

#[derive(Serialize)]
struct OllamaToolDefinition {
    #[serde(rename = "type")]
    tool_type: String,
    function: OllamaToolFunction,
}

#[derive(Serialize)]
struct OllamaToolFunction {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

// ── Helpers ──

fn to_ollama_messages(messages: &[ChatMessage]) -> Vec<OllamaMessage> {
    messages
        .iter()
        .map(|m| OllamaMessage {
            role: match m.role {
                ChatRole::System => "system".to_string(),
                ChatRole::User => "user".to_string(),
                ChatRole::Assistant => "assistant".to_string(),
                ChatRole::Tool => "tool".to_string(),
            },
            content: m.content.clone(),
            images: None,
        })
        .collect()
}

fn to_ollama_options(options: &Option<CompletionOptions>) -> Option<OllamaOptions> {
    options.as_ref().map(|o| OllamaOptions {
        temperature: o.temperature,
        num_predict: o.max_tokens,
        top_p: o.top_p,
        top_k: o.top_k,
        repeat_penalty: o.repeat_penalty,
    })
}

fn to_ollama_tools(options: &Option<CompletionOptions>) -> Option<Vec<OllamaToolDefinition>> {
    options.as_ref().and_then(|opts| {
        opts.tools.as_ref().map(|tools| {
            tools
                .iter()
                .map(|tool| OllamaToolDefinition {
                    tool_type: "function".to_string(),
                    function: OllamaToolFunction {
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        parameters: tool.parameters.clone(),
                    },
                })
                .collect()
        })
    })
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn chat_completion(
        &self,
        messages: &[ChatMessage],
        options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError> {
        let model = options
            .as_ref()
            .and_then(|o| o.model.clone())
            .unwrap_or_else(|| "llama3".to_string());

        let request = OllamaChatRequest {
            model: model.clone(),
            messages: to_ollama_messages(messages),
            stream: false,
            options: to_ollama_options(&options),
            tools: to_ollama_tools(&options),
        };

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api(format!("Ollama {}: {}", status, body)));
        }

        let ollama_resp: OllamaChatResponse = resp.json().await?;

        Ok(CompletionResponse {
            content: ollama_resp.message.content,
            model: ollama_resp.model,
            prompt_tokens: ollama_resp.prompt_eval_count,
            completion_tokens: ollama_resp.eval_count,
            total_tokens: ollama_resp.prompt_eval_count + ollama_resp.eval_count,
            tool_calls: ollama_resp
                .message
                .tool_calls
                .iter()
                .enumerate()
                .map(|(i, tc)| ToolCall {
                    id: format!("call_{}", i + 1),
                    name: tc.function.name.clone(),
                    arguments: tc.function.arguments.clone(),
                })
                .collect(),
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
            .unwrap_or_else(|| "llama3".to_string());

        let request = OllamaChatRequest {
            model,
            messages: to_ollama_messages(messages),
            stream: true,
            options: to_ollama_options(&options),
            tools: None, // tool calling not supported in streaming mode yet
        };

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let _ = tx
                .send(StreamEvent::Error {
                    error: format!("Ollama {}: {}", status, body),
                })
                .await;
            let _ = tx.send(StreamEvent::Done).await;
            return Err(ProviderError::Api(format!("Ollama {}", status)));
        }

        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));
                    // Ollama sends newline-delimited JSON
                    while let Some(pos) = buffer.find('\n') {
                        let line = buffer[..pos].to_string();
                        buffer = buffer[pos + 1..].to_string();
                        if line.trim().is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<OllamaChatResponse>(&line) {
                            Ok(resp) => {
                                if resp.done {
                                    let _ = tx.send(StreamEvent::Done).await;
                                    return Ok(());
                                }
                                if !resp.message.content.is_empty() {
                                    let _ = tx
                                        .send(StreamEvent::Token {
                                            content: resp.message.content,
                                        })
                                        .await;
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
                    let _ = tx
                        .send(StreamEvent::Error {
                            error: e.to_string(),
                        })
                        .await;
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
            .unwrap_or_else(|| "llava".to_string());

        let request = OllamaChatRequest {
            model: model.clone(),
            messages: vec![OllamaMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
                images: Some(vec![b64]),
            }],
            stream: false,
            options: to_ollama_options(&options),
            tools: None, // vision requests don't use tool calling
        };

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api(format!("Ollama {}: {}", status, body)));
        }

        let ollama_resp: OllamaChatResponse = resp.json().await?;

        Ok(CompletionResponse {
            content: ollama_resp.message.content,
            model: ollama_resp.model,
            prompt_tokens: ollama_resp.prompt_eval_count,
            completion_tokens: ollama_resp.eval_count,
            total_tokens: ollama_resp.prompt_eval_count + ollama_resp.eval_count,
            tool_calls: vec![],
        })
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        let resp = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(ProviderError::Unavailable)
        }
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: "ollama".to_string(),
            display_name: "Ollama".to_string(),
            provider_type: ProviderType::Local,
            models: vec![], // populated by list_models()
        }
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let resp = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ProviderError::Unavailable);
        }

        let tags: OllamaTagsResponse = resp.json().await?;
        Ok(tags.models.into_iter().map(|m| m.name).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_ollama_messages() {
        let messages = vec![
            ChatMessage {
                role: ChatRole::System,
                content: "Be helpful.".to_string(),
                tool_call_id: None,
                name: None,
            },
            ChatMessage {
                role: ChatRole::User,
                content: "Hello".to_string(),
                tool_call_id: None,
                name: None,
            },
        ];
        let ollama_msgs = to_ollama_messages(&messages);
        assert_eq!(ollama_msgs.len(), 2);
        assert_eq!(ollama_msgs[0].role, "system");
        assert_eq!(ollama_msgs[1].role, "user");
    }

    #[test]
    fn test_to_ollama_options() {
        let opts = Some(CompletionOptions {
            temperature: Some(0.5),
            max_tokens: Some(100),
            ..Default::default()
        });
        let ollama_opts = to_ollama_options(&opts).unwrap();
        assert_eq!(ollama_opts.temperature, Some(0.5));
        assert_eq!(ollama_opts.num_predict, Some(100));
    }

    #[test]
    fn test_ollama_provider_info() {
        let provider = OllamaProvider::new("http://localhost:11434");
        let info = provider.info();
        assert_eq!(info.id, "ollama");
        assert_eq!(info.display_name, "Ollama");
        assert_eq!(info.provider_type, ProviderType::Local);
    }

    #[test]
    fn test_to_ollama_tools() {
        let opts = Some(CompletionOptions {
            tools: Some(vec![ToolDefinition {
                name: "read_file".to_string(),
                description: "Read a file".to_string(),
                parameters: serde_json::json!({"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}),
            }]),
            ..Default::default()
        });
        let tools = to_ollama_tools(&opts).unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].function.name, "read_file");
    }
}
