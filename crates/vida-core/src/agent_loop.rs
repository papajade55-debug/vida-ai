use std::sync::Arc;

use serde_json::Value;
use vida_providers::traits::{
    ChatMessage, ChatRole, CompletionOptions, CompletionResponse, LLMProvider, ToolCall,
    ToolDefinition,
};

use crate::access::{authorize_agent_tool_call, AgentToolContext};
use crate::error::VidaError;
use crate::mcp::{McpManager, McpTool, McpToolResult, McpToolResultContent};
use crate::tool_validator::validate_tool_call;

const MAX_TOOL_ITERATIONS: usize = 8;
const TOOL_CALL_TIMEOUT_SECS: u64 = 30;
const TOOL_CALL_START: &str = "<tool_call>";
const TOOL_CALL_END: &str = "</tool_call>";

#[derive(Debug, Clone)]
pub struct ToolExecutionRecord {
    pub call: ToolCall,
    pub result: McpToolResult,
}

#[derive(Debug, Clone)]
pub struct AgentLoopResult {
    pub response: CompletionResponse,
    pub records: Vec<ToolExecutionRecord>,
}

impl AgentLoopResult {
    pub fn rendered_content(&self) -> String {
        let mut out = String::new();

        for record in &self.records {
            out.push_str(&format_tool_call_tag(&record.call));
            out.push('\n');
            out.push_str(&format_tool_result_tag(&record.call.name, &record.result));
            out.push('\n');
        }

        let final_content = strip_tool_markup(&self.response.content).trim().to_string();
        if !final_content.is_empty() {
            out.push_str(&final_content);
        }

        out.trim().to_string()
    }
}

pub async fn run_agent_loop(
    provider: Arc<dyn LLMProvider>,
    mut messages: Vec<ChatMessage>,
    mut options: CompletionOptions,
    available_tools: Vec<McpTool>,
    mcp_manager: &mut McpManager,
    agent_context: Option<&AgentToolContext>,
) -> Result<AgentLoopResult, VidaError> {
    if available_tools.is_empty() {
        let response = provider.chat_completion(&messages, Some(options)).await?;
        return Ok(AgentLoopResult {
            response,
            records: vec![],
        });
    }

    let tool_definitions = to_tool_definitions(&available_tools);
    options.tools = Some(tool_definitions.clone());
    messages.push(ChatMessage {
        role: ChatRole::System,
        content: build_fallback_tool_prompt(&tool_definitions),
        tool_call_id: None,
        name: None,
    });

    let mut records = Vec::new();

    for _ in 0..MAX_TOOL_ITERATIONS {
        let response = provider
            .chat_completion(&messages, Some(options.clone()))
            .await?;

        let tool_calls = if response.tool_calls.is_empty() {
            parse_tagged_tool_calls(&response.content)?
        } else {
            response.tool_calls.clone()
        };

        if tool_calls.is_empty() {
            return Ok(AgentLoopResult { response, records });
        }

        if !response.content.trim().is_empty() {
            messages.push(ChatMessage {
                role: ChatRole::Assistant,
                content: response.content.clone(),
                tool_call_id: None,
                name: None,
            });
        }

        for call in tool_calls {
            validate_tool_call(&call, &tool_definitions)
                .map_err(|e| VidaError::Config(e.to_string()))?;
            if let Some(context) = agent_context {
                authorize_agent_tool_call(&call.name, &call.arguments, context)
                    .map_err(VidaError::Config)?;
            }

            let result = match mcp_manager.call_tool(&call.name, call.arguments.clone()) {
                Ok(r) => r,
                Err(e) => McpToolResult {
                    content: vec![McpToolResultContent {
                        content_type: "text".to_string(),
                        text: format!("Tool execution error: {e}"),
                    }],
                    is_error: true,
                },
            };

            messages.push(ChatMessage {
                role: ChatRole::Assistant,
                content: format_tool_call_tag(&call),
                tool_call_id: Some(call.id.clone()),
                name: Some(call.name.clone()),
            });
            messages.push(ChatMessage {
                role: ChatRole::Tool,
                content: flatten_tool_result(&result),
                tool_call_id: Some(call.id.clone()),
                name: Some(call.name.clone()),
            });

            records.push(ToolExecutionRecord { call, result });
        }
    }

    Err(VidaError::Config(format!(
        "Agent loop exceeded {MAX_TOOL_ITERATIONS} tool iterations"
    )))
}

fn to_tool_definitions(tools: &[McpTool]) -> Vec<ToolDefinition> {
    tools
        .iter()
        .map(|tool| ToolDefinition {
            name: tool.name.clone(),
            description: tool.description.clone(),
            parameters: tool.input_schema.clone(),
        })
        .collect()
}

fn build_fallback_tool_prompt(tools: &[ToolDefinition]) -> String {
    let tool_json = serde_json::to_string_pretty(tools).unwrap_or_else(|_| "[]".to_string());
    format!(
        "You can use tools. Available tools are:\n{tool_json}\n\
If you need a tool, answer with only one tag in this exact format:\n\
<tool_call>{{\"id\":\"call_1\",\"name\":\"tool_name\",\"arguments\":{{...}}}}</tool_call>\n\
Do not add explanation around the tag. After receiving tool results, continue normally."
    )
}

fn parse_tagged_tool_calls(content: &str) -> Result<Vec<ToolCall>, VidaError> {
    let mut calls = Vec::new();
    let mut cursor = content;
    let mut fallback_idx = 1usize;

    while let Some(start) = cursor.find(TOOL_CALL_START) {
        let after_start = &cursor[start + TOOL_CALL_START.len()..];
        let Some(end) = after_start.find(TOOL_CALL_END) else {
            break;
        };

        let json_blob = after_start[..end].trim();
        let value: Value = serde_json::from_str(json_blob)
            .map_err(|e| VidaError::Config(format!("Invalid <tool_call> payload: {e}")))?;

        let id = value
            .get("id")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| {
                let id = format!("call_{fallback_idx}");
                fallback_idx += 1;
                id
            });

        let name = value
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| VidaError::Config("Tool call missing 'name'".to_string()))?
            .to_string();

        let arguments = value
            .get("arguments")
            .or_else(|| value.get("args"))
            .cloned()
            .unwrap_or(Value::Object(Default::default()));

        calls.push(ToolCall {
            id,
            name,
            arguments,
        });
        cursor = &after_start[end + TOOL_CALL_END.len()..];
    }

    Ok(calls)
}

fn strip_tool_markup(content: &str) -> String {
    let mut out = String::new();
    let mut cursor = content;

    loop {
        let Some(start) = cursor.find(TOOL_CALL_START) else {
            out.push_str(cursor);
            break;
        };
        out.push_str(&cursor[..start]);
        let after_start = &cursor[start + TOOL_CALL_START.len()..];
        let Some(end) = after_start.find(TOOL_CALL_END) else {
            break;
        };
        cursor = &after_start[end + TOOL_CALL_END.len()..];
    }

    out
}

fn format_tool_call_tag(call: &ToolCall) -> String {
    format!(
        "<tool_call>{}</tool_call>",
        serde_json::json!({
            "id": call.id,
            "name": call.name,
            "arguments": call.arguments,
        })
    )
}

fn format_tool_result_tag(name: &str, result: &McpToolResult) -> String {
    format!(
        "<tool_result name=\"{}\" error=\"{}\">{}</tool_result>",
        name,
        result.is_error,
        flatten_tool_result(result)
    )
}

fn flatten_tool_result(result: &McpToolResult) -> String {
    if result.content.is_empty() {
        return String::new();
    }

    result
        .content
        .iter()
        .map(|item| item.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_tagged_tool_calls() {
        let content = r#"<tool_call>{"id":"call_1","name":"read_file","arguments":{"path":"/tmp/a"}}</tool_call>"#;
        let calls = parse_tagged_tool_calls(content).unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "read_file");
        assert_eq!(calls[0].arguments["path"], "/tmp/a");
    }

    #[test]
    fn test_tool_call_timeout_constant() {
        assert_eq!(TOOL_CALL_TIMEOUT_SECS, 30);
    }

    #[test]
    fn test_rendered_content_contains_tool_blocks() {
        let result = AgentLoopResult {
            response: CompletionResponse {
                content: "Final answer".to_string(),
                model: "mock".to_string(),
                prompt_tokens: 1,
                completion_tokens: 1,
                total_tokens: 2,
                tool_calls: vec![],
            },
            records: vec![ToolExecutionRecord {
                call: ToolCall {
                    id: "call_1".to_string(),
                    name: "read_file".to_string(),
                    arguments: json!({"path":"/tmp/a"}),
                },
                result: McpToolResult {
                    content: vec![crate::mcp::McpToolResultContent {
                        content_type: "text".to_string(),
                        text: "hello".to_string(),
                    }],
                    is_error: false,
                },
            }],
        };

        let rendered = result.rendered_content();
        assert!(rendered.contains("<tool_call>"));
        assert!(rendered.contains("<tool_result"));
        assert!(rendered.contains("Final answer"));
    }
}
