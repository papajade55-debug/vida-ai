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
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OpenAIToolDefinition>>,
    stream: bool,
}

#[derive(Serialize)]
struct OpenAIMessage {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OpenAIMessageToolCall>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIMessageToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAIToolFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OpenAIToolFunction {
    name: String,
    arguments: String,
}

#[derive(Serialize)]
struct OpenAIToolDefinition {
    #[serde(rename = "type")]
    tool_type: String,
    function: OpenAIToolSpec,
}

#[derive(Serialize)]
struct OpenAIToolSpec {
    name: String,
    description: String,
    parameters: serde_json::Value,
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
    content: Option<serde_json::Value>,
    #[serde(default)]
    tool_calls: Vec<OpenAIMessageToolCall>,
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
    #[serde(default)]
    tool_calls: Vec<OpenAIStreamToolCallDelta>,
}

#[derive(Deserialize)]
struct OpenAIStreamToolCallDelta {
    index: usize,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    function: Option<OpenAIStreamFunctionDelta>,
}

#[derive(Deserialize)]
struct OpenAIStreamFunctionDelta {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    arguments: Option<String>,
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
        .map(|m| match m.role {
            ChatRole::Assistant if m.tool_call_id.is_some() && m.name.is_some() => {
                let arguments = extract_tool_call_arguments(&m.content)
                    .unwrap_or_else(|| serde_json::json!({}));
                OpenAIMessage {
                    role: "assistant".to_string(),
                    content: None,
                    tool_call_id: None,
                    name: None,
                    tool_calls: Some(vec![OpenAIMessageToolCall {
                        id: m.tool_call_id.clone().unwrap_or_default(),
                        call_type: "function".to_string(),
                        function: OpenAIToolFunction {
                            name: m.name.clone().unwrap_or_default(),
                            arguments: arguments.to_string(),
                        },
                    }]),
                }
            }
            _ => OpenAIMessage {
                role: match m.role {
                    ChatRole::System => "system".to_string(),
                    ChatRole::User => "user".to_string(),
                    ChatRole::Assistant => "assistant".to_string(),
                    ChatRole::Tool => "tool".to_string(),
                },
                content: Some(serde_json::Value::String(m.content.clone())),
                tool_call_id: matches!(m.role, ChatRole::Tool)
                    .then(|| m.tool_call_id.clone())
                    .flatten(),
                name: None,
                tool_calls: None,
            },
        })
        .collect()
}

fn to_openai_tools(options: &Option<CompletionOptions>) -> Option<Vec<OpenAIToolDefinition>> {
    options.as_ref().and_then(|opts| {
        opts.tools.as_ref().map(|tools| {
            tools
                .iter()
                .map(|tool| OpenAIToolDefinition {
                    tool_type: "function".to_string(),
                    function: OpenAIToolSpec {
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        parameters: tool.parameters.clone(),
                    },
                })
                .collect()
        })
    })
}

fn extract_tool_call_arguments(content: &str) -> Option<serde_json::Value> {
    let start = content.find("<tool_call>")?;
    let after = &content[start + "<tool_call>".len()..];
    let end = after.find("</tool_call>")?;
    let payload: serde_json::Value = serde_json::from_str(after[..end].trim()).ok()?;
    payload.get("arguments").cloned()
}

fn openai_content_to_text(content: Option<&serde_json::Value>) -> String {
    match content {
        Some(serde_json::Value::String(text)) => text.clone(),
        Some(serde_json::Value::Array(parts)) => parts
            .iter()
            .filter_map(|part| {
                part.get("text")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            })
            .collect::<Vec<_>>()
            .join(""),
        _ => String::new(),
    }
}

fn parse_openai_tool_calls(
    calls: &[OpenAIMessageToolCall],
) -> Result<Vec<ToolCall>, ProviderError> {
    calls
        .iter()
        .filter(|call| call.call_type == "function")
        .map(|call| {
            let arguments = serde_json::from_str(&call.function.arguments)?;
            Ok(ToolCall {
                id: call.id.clone(),
                name: call.function.name.clone(),
                arguments,
            })
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
            tools: to_openai_tools(&options),
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
        let choice = oai_resp.choices.first().ok_or(ProviderError::Internal(
            "No choices in response".to_string(),
        ))?;
        let usage = oai_resp.usage.unwrap_or(OpenAIUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        });
        let tool_calls = parse_openai_tool_calls(&choice.message.tool_calls)?;

        Ok(CompletionResponse {
            content: openai_content_to_text(choice.message.content.as_ref()),
            model: oai_resp.model,
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
            tool_calls,
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
            tools: to_openai_tools(&options),
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
            let _ = tx
                .send(StreamEvent::Error {
                    error: body.clone(),
                })
                .await;
            let _ = tx.send(StreamEvent::Done).await;
            return Err(ProviderError::Api(body));
        }

        use futures_util::StreamExt;
        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();
        let mut tool_call_buffers: Vec<(String, String, String)> = Vec::new(); // (id, name, arguments_json)

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
                            // Émettre les tool calls accumulés avant de terminer
                            if !tool_call_buffers.is_empty() {
                                for (id, name, args_json) in &tool_call_buffers {
                                    let args: serde_json::Value = serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));
                                    let tag = format!("<tool_call>{{\"id\":\"{}\",\"name\":\"{}\",\"arguments\":{}}}</tool_call>", id, name, args);
                                    let _ = tx.send(StreamEvent::Token { content: tag }).await;
                                }
                            }
                            let _ = tx.send(StreamEvent::Done).await;
                            return Ok(());
                        }
                        match serde_json::from_str::<OpenAIStreamChunk>(data) {
                            Ok(chunk) => {
                                if let Some(choice) = chunk.choices.first() {
                                    if let Some(content) = &choice.delta.content {
                                        if !content.is_empty() {
                                            let _ = tx
                                                .send(StreamEvent::Token {
                                                    content: content.clone(),
                                                })
                                                .await;
                                        }
                                    }
                                    for tc_delta in &choice.delta.tool_calls {
                                        let idx = tc_delta.index;
                                        while tool_call_buffers.len() <= idx {
                                            tool_call_buffers.push((String::new(), String::new(), String::new()));
                                        }
                                        if let Some(id) = &tc_delta.id {
                                            tool_call_buffers[idx].0 = id.clone();
                                        }
                                        if let Some(func) = &tc_delta.function {
                                            if let Some(name) = &func.name {
                                                tool_call_buffers[idx].1 = name.clone();
                                            }
                                            if let Some(args) = &func.arguments {
                                                tool_call_buffers[idx].2.push_str(args);
                                            }
                                        }
                                    }
                                    if choice.finish_reason.is_some() {
                                        // Émettre les tool calls accumulés avant de terminer
                                        if !tool_call_buffers.is_empty() {
                                            for (id, name, args_json) in &tool_call_buffers {
                                                let args: serde_json::Value = serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));
                                                let tag = format!("<tool_call>{{\"id\":\"{}\",\"name\":\"{}\",\"arguments\":{}}}</tool_call>", id, name, args);
                                                let _ = tx.send(StreamEvent::Token { content: tag }).await;
                                            }
                                        }
                                        let _ = tx.send(StreamEvent::Done).await;
                                        return Ok(());
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
        // Émettre les tool calls accumulés avant de terminer
        if !tool_call_buffers.is_empty() {
            for (id, name, args_json) in &tool_call_buffers {
                let args: serde_json::Value = serde_json::from_str(args_json).unwrap_or(serde_json::json!({}));
                let tag = format!("<tool_call>{{\"id\":\"{}\",\"name\":\"{}\",\"arguments\":{}}}</tool_call>", id, name, args);
                let _ = tx.send(StreamEvent::Token { content: tag }).await;
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
        let choice = oai_resp
            .choices
            .first()
            .ok_or(ProviderError::Internal("No choices".to_string()))?;
        let usage = oai_resp.usage.unwrap_or(OpenAIUsage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        });

        Ok(CompletionResponse {
            content: openai_content_to_text(choice.message.content.as_ref()),
            model: oai_resp.model,
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
            tool_calls: parse_openai_tool_calls(&choice.message.tool_calls)?,
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
            id: "openai".to_string(),
            display_name: "OpenAI".to_string(),
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
        let messages = vec![ChatMessage {
            role: ChatRole::User,
            content: "Hi".to_string(),
            tool_call_id: None,
            name: None,
        }];
        let oai_msgs = to_openai_messages(&messages);
        assert_eq!(oai_msgs.len(), 1);
        assert_eq!(oai_msgs[0].role, "user");
        assert_eq!(
            oai_msgs[0].content,
            Some(serde_json::Value::String("Hi".to_string()))
        );
    }

    #[test]
    fn test_to_openai_messages_with_tool_call() {
        let messages = vec![ChatMessage {
            role: ChatRole::Assistant,
            content: r#"<tool_call>{"id":"call_1","name":"read_file","arguments":{"path":"/tmp/demo.txt"}}</tool_call>"#.to_string(),
            tool_call_id: Some("call_1".to_string()),
            name: Some("read_file".to_string()),
        }];

        let oai_msgs = to_openai_messages(&messages);
        assert_eq!(oai_msgs.len(), 1);
        assert_eq!(oai_msgs[0].role, "assistant");
        assert!(oai_msgs[0].content.is_none());
        assert_eq!(oai_msgs[0].tool_calls.as_ref().map(Vec::len), Some(1));
    }

    #[test]
    fn test_parse_openai_tool_calls() {
        let parsed = parse_openai_tool_calls(&[OpenAIMessageToolCall {
            id: "call_1".to_string(),
            call_type: "function".to_string(),
            function: OpenAIToolFunction {
                name: "read_file".to_string(),
                arguments: r#"{"path":"/tmp/demo.txt"}"#.to_string(),
            },
        }])
        .unwrap();

        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].name, "read_file");
        assert_eq!(parsed[0].arguments["path"], "/tmp/demo.txt");
    }

    #[test]
    fn test_openai_provider_info() {
        let provider = OpenAIProvider::new("https://api.openai.com", "sk-test", "gpt-4o");
        let info = provider.info();
        assert_eq!(info.id, "openai");
        assert_eq!(info.display_name, "OpenAI");
        assert_eq!(info.provider_type, ProviderType::Cloud);
    }

    #[test]
    fn test_openai_compatible_base_url() {
        let provider = OpenAIProvider::new("https://api.groq.com/openai", "gsk-test", "llama3-70b");
        assert_eq!(provider.base_url, "https://api.groq.com/openai");
        assert_eq!(provider.default_model, "llama3-70b");
    }
}
