use std::sync::Arc;

use async_trait::async_trait;
use serde_json::json;
use tokio::sync::mpsc;

use vida_core::agent_loop::run_agent_loop;
use vida_core::mcp::McpManager;
use vida_providers::traits::*;

// ── Mock provider: always returns text (no tool calls) ──

struct DirectTextProvider;

#[async_trait]
impl LLMProvider for DirectTextProvider {
    async fn chat_completion(
        &self,
        _messages: &[ChatMessage],
        _options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError> {
        Ok(CompletionResponse {
            content: "Direct answer".to_string(),
            model: "mock".to_string(),
            prompt_tokens: 5,
            completion_tokens: 2,
            total_tokens: 7,
            tool_calls: vec![],
        })
    }

    async fn chat_completion_stream(
        &self,
        _messages: &[ChatMessage],
        _options: Option<CompletionOptions>,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), ProviderError> {
        let _ = tx
            .send(StreamEvent::Token {
                content: "direct".to_string(),
            })
            .await;
        let _ = tx.send(StreamEvent::Done).await;
        Ok(())
    }

    async fn vision_completion(
        &self,
        _image_data: Vec<u8>,
        _prompt: &str,
        _options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError> {
        Err(ProviderError::Unavailable)
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        Ok(())
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: "direct".to_string(),
            display_name: "Direct".to_string(),
            provider_type: ProviderType::Local,
            models: vec!["direct-model".to_string()],
        }
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(vec!["direct-model".to_string()])
    }
}

// ── Mock provider: returns tool call via XML tags in content ──

struct XmlToolCallProvider {
    call_count: std::sync::atomic::AtomicUsize,
}

impl XmlToolCallProvider {
    fn new() -> Self {
        Self {
            call_count: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl LLMProvider for XmlToolCallProvider {
    async fn chat_completion(
        &self,
        _messages: &[ChatMessage],
        _options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError> {
        let count = self
            .call_count
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        if count == 0 {
            // First call: native tool_calls
            Ok(CompletionResponse {
                content: String::new(),
                model: "mock".to_string(),
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
                tool_calls: vec![ToolCall {
                    id: "call_1".to_string(),
                    name: "unknown_tool".to_string(),
                    arguments: json!({"key": "value"}),
                }],
            })
        } else {
            // Second call: text only
            Ok(CompletionResponse {
                content: "Done processing".to_string(),
                model: "mock".to_string(),
                prompt_tokens: 20,
                completion_tokens: 10,
                total_tokens: 30,
                tool_calls: vec![],
            })
        }
    }

    async fn chat_completion_stream(
        &self,
        _messages: &[ChatMessage],
        _options: Option<CompletionOptions>,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), ProviderError> {
        let _ = tx.send(StreamEvent::Done).await;
        Ok(())
    }

    async fn vision_completion(
        &self,
        _image_data: Vec<u8>,
        _prompt: &str,
        _options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError> {
        Err(ProviderError::Unavailable)
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        Ok(())
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            id: "xml-tool".to_string(),
            display_name: "XML Tool".to_string(),
            provider_type: ProviderType::Local,
            models: vec![],
        }
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        Ok(vec![])
    }
}

// ── Tests ──

/// When no tools are available, the agent loop should pass through directly
/// to the LLM and return the response without any tool execution.
#[tokio::test]
async fn test_agent_loop_no_tools_passthrough() {
    let provider: Arc<dyn LLMProvider> = Arc::new(DirectTextProvider);

    let messages = vec![ChatMessage {
        role: ChatRole::User,
        content: "Hello".to_string(),
        tool_call_id: None,
        name: None,
    }];

    let options = CompletionOptions::default();
    let mut mcp_manager = McpManager::new();

    let result = run_agent_loop(
        provider,
        messages,
        options,
        vec![], // empty tools → passthrough
        &mut mcp_manager,
        None,
    )
    .await
    .unwrap();

    assert!(result.records.is_empty());
    assert_eq!(result.response.content, "Direct answer");
    assert_eq!(result.rendered_content(), "Direct answer");
}

/// When the LLM returns a tool call for a tool that doesn't exist in MCP,
/// the agent loop should fail with a validation error.
#[tokio::test]
async fn test_agent_loop_unknown_tool_fails_validation() {
    let provider: Arc<dyn LLMProvider> = Arc::new(XmlToolCallProvider::new());

    let messages = vec![ChatMessage {
        role: ChatRole::User,
        content: "Do something".to_string(),
        tool_call_id: None,
        name: None,
    }];

    let options = CompletionOptions::default();
    let mut mcp_manager = McpManager::new();

    // Provide a different tool than what the LLM will request
    let available_tools = vec![vida_core::McpTool {
        name: "list_files".to_string(),
        description: "List files".to_string(),
        input_schema: json!({"type": "object"}),
        server_name: "fs".to_string(),
    }];

    let result = run_agent_loop(
        provider,
        messages,
        options,
        available_tools,
        &mut mcp_manager,
        None,
    )
    .await;

    // Should fail because "unknown_tool" is not in available_tools
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("not found") || err.contains("unknown_tool"),
        "Expected tool not found error, got: {}",
        err
    );
}

/// Verify that the passthrough path returns correct token counts.
#[tokio::test]
async fn test_agent_loop_passthrough_preserves_metadata() {
    let provider: Arc<dyn LLMProvider> = Arc::new(DirectTextProvider);

    let messages = vec![ChatMessage {
        role: ChatRole::User,
        content: "Hello".to_string(),
        tool_call_id: None,
        name: None,
    }];

    let options = CompletionOptions::default();
    let mut mcp_manager = McpManager::new();

    let result = run_agent_loop(provider, messages, options, vec![], &mut mcp_manager, None)
        .await
        .unwrap();

    assert_eq!(result.response.model, "mock");
    assert_eq!(result.response.prompt_tokens, 5);
    assert_eq!(result.response.completion_tokens, 2);
    assert_eq!(result.response.total_tokens, 7);
}
