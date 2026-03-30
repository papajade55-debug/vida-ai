# Vida AI — Tool Calling Fiable — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add reliable LLM-driven tool calling to Vida AI so agents can use MCP tools (filesystem, shell, web) in an autonomous loop — the feature OpenFang fails at.

**Architecture:** Extend `LLMProvider` trait with `tools` parameter support. Add an `AgentLoop` that orchestrates LLM↔MCP: send messages + tool definitions → detect tool_calls in response → validate with JSON Schema → execute via McpManager → feed results back → repeat until done. Validation happens in Rust before any tool executes.

**Tech Stack:** Rust (jsonschema crate for validation), existing McpManager, existing LLMProvider trait (extended), Tauri events for streaming tool call status to frontend.

---

## File Structure

```
crates/vida-providers/src/
├── traits.rs              # MODIFY: Add ToolDefinition, ToolCall, ToolResult to trait
├── ollama.rs              # MODIFY: Add tools support (Ollama native tool calling)
├── openai.rs              # MODIFY: Add tools support (OpenAI function calling)
├── anthropic.rs           # MODIFY: Add tools support (Anthropic tool_use)
├── google.rs              # MODIFY: Add tools support (Gemini function calling)

crates/vida-core/src/
├── agent_loop.rs          # CREATE: AgentLoop — orchestrator LLM ↔ tools
├── tool_validator.rs      # CREATE: JSON Schema validation of tool arguments
├── engine.rs              # MODIFY: Add agent_chat() that uses AgentLoop
├── lib.rs                 # MODIFY: Export new modules

src-tauri/src/commands/
├── chat.rs                # MODIFY: Add agent_stream_completion command

src/hooks/
├── useAgentStream.ts      # CREATE: Hook for streaming with tool call events
src/components/chat/
├── ToolCallBubble.tsx     # CREATE: UI component showing tool execution
├── MessageBubble.tsx      # MODIFY: Render tool calls inline
```

---

### Task 1: Extend LLMProvider Trait with Tool Types

**Files:**
- Modify: `crates/vida-providers/src/traits.rs`

- [ ] **Step 1: Write failing test for new types**

```rust
// In crates/vida-providers/src/traits.rs, add to #[cfg(test)] mod tests:

#[test]
fn test_tool_definition_serialization() {
    let tool = ToolDefinition {
        name: "read_file".to_string(),
        description: "Read a file".to_string(),
        parameters: serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path" }
            },
            "required": ["path"]
        }),
    };
    let json = serde_json::to_string(&tool).unwrap();
    assert!(json.contains("read_file"));
}

#[test]
fn test_tool_call_serialization() {
    let call = ToolCall {
        id: "call_1".to_string(),
        name: "read_file".to_string(),
        arguments: serde_json::json!({"path": "/tmp/test.txt"}),
    };
    let json = serde_json::to_string(&call).unwrap();
    assert!(json.contains("call_1"));
    assert!(json.contains("read_file"));
}

#[test]
fn test_completion_response_with_tool_calls() {
    let resp = CompletionResponse {
        content: String::new(),
        model: "qwen3:14b".to_string(),
        prompt_tokens: 100,
        completion_tokens: 50,
        total_tokens: 150,
        tool_calls: vec![ToolCall {
            id: "call_1".to_string(),
            name: "write_file".to_string(),
            arguments: serde_json::json!({"path": "/tmp/a.txt", "content": "hello"}),
        }],
    };
    assert_eq!(resp.tool_calls.len(), 1);
    assert!(resp.content.is_empty());
}

#[test]
fn test_chat_message_tool_result() {
    let msg = ChatMessage {
        role: ChatRole::Tool,
        content: "File written successfully".to_string(),
        tool_call_id: Some("call_1".to_string()),
        name: Some("write_file".to_string()),
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("tool"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd "/home/hackos0911/AI/projects/IA/Vida ui" && cargo test -p vida-providers -- test_tool 2>&1 | tail -5`
Expected: FAIL — types don't exist yet

- [ ] **Step 3: Add the tool types and extend existing types**

```rust
// Add to crates/vida-providers/src/traits.rs, after StreamEvent enum:

/// A tool that can be called by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema
}

/// A tool call requested by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Options for completion — add tools field
// MODIFY CompletionOptions: add this field:
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub tools: Option<Vec<ToolDefinition>>,

/// MODIFY CompletionResponse: add this field:
//     #[serde(default)]
//     pub tool_calls: Vec<ToolCall>,

/// MODIFY ChatRole: add Tool variant
// ChatRole { System, User, Assistant, Tool }

/// MODIFY ChatMessage: add optional fields
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub tool_call_id: Option<String>,
//     #[serde(skip_serializing_if = "Option::is_none")]
//     pub name: Option<String>,
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cd "/home/hackos0911/AI/projects/IA/Vida ui" && cargo test -p vida-providers 2>&1 | tail -10`
Expected: ALL PASS (existing + new tests)

- [ ] **Step 5: Commit**

```bash
cd "/home/hackos0911/AI/projects/IA/Vida ui"
git add crates/vida-providers/src/traits.rs
git commit -m "feat(providers): add ToolDefinition, ToolCall types and extend ChatMessage/CompletionResponse for tool calling"
```

---

### Task 2: Tool Argument Validator

**Files:**
- Create: `crates/vida-core/src/tool_validator.rs`
- Modify: `crates/vida-core/src/lib.rs`
- Modify: `crates/vida-core/Cargo.toml`

- [ ] **Step 1: Add jsonschema dependency**

In `crates/vida-core/Cargo.toml`, add to `[dependencies]`:
```toml
jsonschema = "0.18"
```

- [ ] **Step 2: Write failing tests**

Create `crates/vida-core/src/tool_validator.rs`:
```rust
use serde_json::Value;
use vida_providers::traits::ToolCall;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Tool '{0}' not found in available tools")]
    ToolNotFound(String),
    #[error("Invalid arguments for tool '{tool}': {message}")]
    InvalidArguments { tool: String, message: String },
    #[error("Schema compilation failed: {0}")]
    SchemaError(String),
}

/// Validate a tool call's arguments against the tool's JSON Schema.
pub fn validate_tool_call(
    call: &ToolCall,
    schema: &Value,
) -> Result<(), ValidationError> {
    let compiled = jsonschema::validator_for(schema)
        .map_err(|e| ValidationError::SchemaError(e.to_string()))?;

    let result = compiled.validate(&call.arguments);
    if let Err(errors) = result {
        let messages: Vec<String> = errors.map(|e| e.to_string()).collect();
        return Err(ValidationError::InvalidArguments {
            tool: call.name.clone(),
            message: messages.join("; "),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_valid_tool_call() {
        let call = ToolCall {
            id: "1".to_string(),
            name: "write_file".to_string(),
            arguments: json!({"path": "/tmp/test.txt", "content": "hello"}),
        };
        let schema = json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["path", "content"]
        });
        assert!(validate_tool_call(&call, &schema).is_ok());
    }

    #[test]
    fn test_missing_required_field() {
        let call = ToolCall {
            id: "1".to_string(),
            name: "write_file".to_string(),
            arguments: json!({"path": "/tmp/test.txt"}),
        };
        let schema = json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "content": { "type": "string" }
            },
            "required": ["path", "content"]
        });
        let err = validate_tool_call(&call, &schema).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidArguments { .. }));
    }

    #[test]
    fn test_wrong_type() {
        let call = ToolCall {
            id: "1".to_string(),
            name: "read_file".to_string(),
            arguments: json!({"path": 42}),
        };
        let schema = json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" }
            },
            "required": ["path"]
        });
        let err = validate_tool_call(&call, &schema).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidArguments { .. }));
    }

    #[test]
    fn test_empty_schema_accepts_anything() {
        let call = ToolCall {
            id: "1".to_string(),
            name: "no_args".to_string(),
            arguments: json!({}),
        };
        let schema = json!({"type": "object"});
        assert!(validate_tool_call(&call, &schema).is_ok());
    }
}
```

- [ ] **Step 3: Export module in lib.rs**

In `crates/vida-core/src/lib.rs`, add:
```rust
pub mod tool_validator;
```

- [ ] **Step 4: Run tests**

Run: `cd "/home/hackos0911/AI/projects/IA/Vida ui" && cargo test -p vida-core -- test_valid_tool_call test_missing_required test_wrong_type test_empty_schema -v 2>&1 | tail -10`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```bash
cd "/home/hackos0911/AI/projects/IA/Vida ui"
git add crates/vida-core/Cargo.toml crates/vida-core/src/tool_validator.rs crates/vida-core/src/lib.rs
git commit -m "feat(core): add tool_validator with JSON Schema validation for tool call arguments"
```

---

### Task 3: Agent Loop — The Core Orchestrator

**Files:**
- Create: `crates/vida-core/src/agent_loop.rs`
- Modify: `crates/vida-core/src/lib.rs`

- [ ] **Step 1: Write the AgentLoop tests**

Create `crates/vida-core/src/agent_loop.rs`:
```rust
use std::collections::HashMap;
use tokio::sync::mpsc;
use vida_providers::traits::*;
use crate::mcp::{McpManager, McpTool, McpToolResult, McpToolResultContent};
use crate::tool_validator::{validate_tool_call, ValidationError};

/// Events emitted by the agent loop for UI streaming.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AgentEvent {
    /// LLM is generating text
    Token { content: String },
    /// LLM requested a tool call
    ToolCallStart { id: String, name: String, arguments: serde_json::Value },
    /// Tool execution completed
    ToolCallResult { id: String, name: String, result: String, is_error: bool },
    /// Tool call was rejected by validation
    ToolCallRejected { id: String, name: String, reason: String },
    /// Agent finished (no more tool calls)
    Done { total_iterations: u32 },
    /// Error
    Error { error: String },
}

/// Configuration for the agent loop.
pub struct AgentLoopConfig {
    pub max_iterations: u32,
    pub max_tool_calls_per_iteration: u32,
}

impl Default for AgentLoopConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            max_tool_calls_per_iteration: 5,
        }
    }
}

/// Run the agent loop: LLM ↔ tools until the LLM returns a text-only response.
///
/// Flow:
/// 1. Send messages + tool definitions to LLM
/// 2. If LLM returns tool_calls → validate → execute → add results to messages → goto 1
/// 3. If LLM returns text only → done
pub async fn run_agent_loop(
    provider: &dyn LLMProvider,
    messages: &mut Vec<ChatMessage>,
    mcp_manager: &mut McpManager,
    options: Option<CompletionOptions>,
    config: AgentLoopConfig,
    tx: mpsc::Sender<AgentEvent>,
) -> Result<CompletionResponse, String> {
    // Build tool definitions from MCP
    let mcp_tools = mcp_manager.list_tools();
    let tool_defs: Vec<ToolDefinition> = mcp_tools.iter().map(|t| ToolDefinition {
        name: t.name.clone(),
        description: t.description.clone(),
        parameters: t.input_schema.clone(),
    }).collect();

    // Build tool schema lookup for validation
    let schema_map: HashMap<String, serde_json::Value> = mcp_tools.iter()
        .map(|t| (t.name.clone(), t.input_schema.clone()))
        .collect();

    let mut opts = options.unwrap_or_default();
    if !tool_defs.is_empty() {
        opts.tools = Some(tool_defs);
    }

    let mut iteration = 0u32;
    let mut last_response = None;

    loop {
        iteration += 1;
        if iteration > config.max_iterations {
            let _ = tx.send(AgentEvent::Error {
                error: format!("Max iterations ({}) reached", config.max_iterations),
            }).await;
            break;
        }

        // Call LLM
        let response = provider
            .chat_completion(messages, Some(opts.clone()))
            .await
            .map_err(|e| e.to_string())?;

        // If no tool calls → we're done
        if response.tool_calls.is_empty() {
            // Emit final text tokens
            if !response.content.is_empty() {
                let _ = tx.send(AgentEvent::Token {
                    content: response.content.clone(),
                }).await;
            }
            let _ = tx.send(AgentEvent::Done { total_iterations: iteration }).await;
            last_response = Some(response);
            break;
        }

        // Process tool calls
        // Add assistant message with tool calls to history
        messages.push(ChatMessage {
            role: ChatRole::Assistant,
            content: response.content.clone(),
            tool_call_id: None,
            name: None,
        });

        for call in &response.tool_calls {
            let _ = tx.send(AgentEvent::ToolCallStart {
                id: call.id.clone(),
                name: call.name.clone(),
                arguments: call.arguments.clone(),
            }).await;

            // Validate against schema
            if let Some(schema) = schema_map.get(&call.name) {
                if let Err(e) = validate_tool_call(call, schema) {
                    let reason = e.to_string();
                    let _ = tx.send(AgentEvent::ToolCallRejected {
                        id: call.id.clone(),
                        name: call.name.clone(),
                        reason: reason.clone(),
                    }).await;
                    // Add error as tool result
                    messages.push(ChatMessage {
                        role: ChatRole::Tool,
                        content: format!("Validation error: {}", reason),
                        tool_call_id: Some(call.id.clone()),
                        name: Some(call.name.clone()),
                    });
                    continue;
                }
            }

            // Execute tool
            let result = mcp_manager.call_tool(&call.name, call.arguments.clone());
            match result {
                Ok(tool_result) => {
                    let text = tool_result.content.iter()
                        .map(|c| c.text.as_str())
                        .collect::<Vec<_>>()
                        .join("\n");
                    let _ = tx.send(AgentEvent::ToolCallResult {
                        id: call.id.clone(),
                        name: call.name.clone(),
                        result: text.clone(),
                        is_error: tool_result.is_error,
                    }).await;
                    messages.push(ChatMessage {
                        role: ChatRole::Tool,
                        content: text,
                        tool_call_id: Some(call.id.clone()),
                        name: Some(call.name.clone()),
                    });
                }
                Err(e) => {
                    let err_msg = e.to_string();
                    let _ = tx.send(AgentEvent::ToolCallResult {
                        id: call.id.clone(),
                        name: call.name.clone(),
                        result: err_msg.clone(),
                        is_error: true,
                    }).await;
                    messages.push(ChatMessage {
                        role: ChatRole::Tool,
                        content: format!("Error: {}", err_msg),
                        tool_call_id: Some(call.id.clone()),
                        name: Some(call.name.clone()),
                    });
                }
            }
        }

        last_response = Some(response);
    }

    last_response.ok_or_else(|| "No response from LLM".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_loop_config_defaults() {
        let config = AgentLoopConfig::default();
        assert_eq!(config.max_iterations, 10);
        assert_eq!(config.max_tool_calls_per_iteration, 5);
    }

    #[test]
    fn test_agent_event_serialization() {
        let event = AgentEvent::ToolCallStart {
            id: "call_1".to_string(),
            name: "read_file".to_string(),
            arguments: serde_json::json!({"path": "/tmp/test"}),
        };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("ToolCallStart"));
        assert!(json.contains("read_file"));
    }

    #[test]
    fn test_agent_event_done() {
        let event = AgentEvent::Done { total_iterations: 3 };
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("3"));
    }

    #[test]
    fn test_tool_def_from_mcp_tool() {
        let mcp_tool = McpTool {
            name: "write_file".to_string(),
            description: "Write a file".to_string(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": {"type": "string"},
                    "content": {"type": "string"}
                },
                "required": ["path", "content"]
            }),
            server_name: "filesystem".to_string(),
        };
        let tool_def = ToolDefinition {
            name: mcp_tool.name.clone(),
            description: mcp_tool.description.clone(),
            parameters: mcp_tool.input_schema.clone(),
        };
        assert_eq!(tool_def.name, "write_file");
        assert!(tool_def.parameters["required"].as_array().unwrap().len() == 2);
    }
}
```

- [ ] **Step 2: Export in lib.rs**

Add to `crates/vida-core/src/lib.rs`:
```rust
pub mod agent_loop;
```

- [ ] **Step 3: Run tests**

Run: `cd "/home/hackos0911/AI/projects/IA/Vida ui" && cargo test -p vida-core -- agent_loop -v 2>&1 | tail -10`
Expected: ALL PASS (unit tests, no integration yet)

- [ ] **Step 4: Commit**

```bash
cd "/home/hackos0911/AI/projects/IA/Vida ui"
git add crates/vida-core/src/agent_loop.rs crates/vida-core/src/lib.rs
git commit -m "feat(core): add AgentLoop orchestrator — LLM ↔ MCP tool calling with validation"
```

---

### Task 4: OpenAI Provider Tool Calling Support

**Files:**
- Modify: `crates/vida-providers/src/openai.rs`

This is the reference implementation. Ollama, Anthropic, and Google follow the same pattern with format adaptations.

- [ ] **Step 1: Write failing test**

Add to the test module in `openai.rs`:
```rust
#[test]
fn test_build_tools_payload() {
    let tools = vec![ToolDefinition {
        name: "read_file".to_string(),
        description: "Read a file".to_string(),
        parameters: serde_json::json!({"type": "object", "properties": {"path": {"type": "string"}}}),
    }];
    let payload = build_openai_tools(&tools);
    assert_eq!(payload.as_array().unwrap().len(), 1);
    assert_eq!(payload[0]["type"], "function");
    assert_eq!(payload[0]["function"]["name"], "read_file");
}

#[test]
fn test_parse_tool_calls_from_response() {
    let response_json = serde_json::json!({
        "choices": [{
            "message": {
                "role": "assistant",
                "content": null,
                "tool_calls": [{
                    "id": "call_abc",
                    "type": "function",
                    "function": {
                        "name": "read_file",
                        "arguments": "{\"path\":\"/tmp/test.txt\"}"
                    }
                }]
            }
        }],
        "usage": {"prompt_tokens": 10, "completion_tokens": 5, "total_tokens": 15}
    });
    let tool_calls = parse_openai_tool_calls(&response_json);
    assert_eq!(tool_calls.len(), 1);
    assert_eq!(tool_calls[0].id, "call_abc");
    assert_eq!(tool_calls[0].name, "read_file");
}
```

- [ ] **Step 2: Implement tool payload building and response parsing**

Add helper functions in `openai.rs`:
```rust
fn build_openai_tools(tools: &[ToolDefinition]) -> serde_json::Value {
    serde_json::json!(tools.iter().map(|t| {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": t.name,
                "description": t.description,
                "parameters": t.parameters,
            }
        })
    }).collect::<Vec<_>>())
}

fn parse_openai_tool_calls(response: &serde_json::Value) -> Vec<ToolCall> {
    response["choices"][0]["message"]["tool_calls"]
        .as_array()
        .map(|calls| {
            calls.iter().filter_map(|c| {
                let id = c["id"].as_str()?.to_string();
                let name = c["function"]["name"].as_str()?.to_string();
                let args_str = c["function"]["arguments"].as_str()?;
                let arguments = serde_json::from_str(args_str).ok()?;
                Some(ToolCall { id, name, arguments })
            }).collect()
        })
        .unwrap_or_default()
}
```

Then modify `chat_completion` to include tools in the request body when `options.tools` is Some, and parse tool_calls from the response.

- [ ] **Step 3: Run all provider tests**

Run: `cd "/home/hackos0911/AI/projects/IA/Vida ui" && cargo test -p vida-providers -v 2>&1 | tail -15`
Expected: ALL PASS

- [ ] **Step 4: Commit**

```bash
cd "/home/hackos0911/AI/projects/IA/Vida ui"
git add crates/vida-providers/src/openai.rs
git commit -m "feat(providers): add OpenAI tool calling support — build tools payload + parse tool_calls response"
```

---

### Task 5: Ollama Provider Tool Calling Support

**Files:**
- Modify: `crates/vida-providers/src/ollama.rs`

- [ ] **Step 1: Implement Ollama tool calling**

Ollama uses the same OpenAI-compatible format for tool calling via `/api/chat`:
```json
{
  "model": "qwen3:14b",
  "messages": [...],
  "tools": [{"type": "function", "function": {...}}]
}
```

Response includes `message.tool_calls` in the same format. Reuse the same `build_openai_tools` and `parse_openai_tool_calls` helpers (extract to a shared module or duplicate).

- [ ] **Step 2: Run tests**

Run: `cd "/home/hackos0911/AI/projects/IA/Vida ui" && cargo test -p vida-providers -v 2>&1 | tail -15`
Expected: ALL PASS

- [ ] **Step 3: Commit**

```bash
cd "/home/hackos0911/AI/projects/IA/Vida ui"
git add crates/vida-providers/src/ollama.rs
git commit -m "feat(providers): add Ollama tool calling support (OpenAI-compatible format)"
```

---

### Task 6: Anthropic + Google Provider Tool Calling

**Files:**
- Modify: `crates/vida-providers/src/anthropic.rs`
- Modify: `crates/vida-providers/src/google.rs`

- [ ] **Step 1: Anthropic tool_use format**

Anthropic uses a different format:
```json
{
  "tools": [{"name": "...", "description": "...", "input_schema": {...}}],
  "messages": [...]
}
```
Response has `content` blocks with `type: "tool_use"`:
```json
{"type": "tool_use", "id": "toolu_xxx", "name": "read_file", "input": {"path": "..."}}
```

Implement the conversion in `anthropic.rs`.

- [ ] **Step 2: Google Gemini function calling format**

Gemini uses:
```json
{
  "tools": [{"function_declarations": [{"name": "...", "description": "...", "parameters": {...}}]}]
}
```
Response has `functionCall` in parts.

Implement the conversion in `google.rs`.

- [ ] **Step 3: Run all tests**

Run: `cd "/home/hackos0911/AI/projects/IA/Vida ui" && cargo test --workspace -v 2>&1 | tail -15`
Expected: ALL PASS

- [ ] **Step 4: Commit**

```bash
cd "/home/hackos0911/AI/projects/IA/Vida ui"
git add crates/vida-providers/src/anthropic.rs crates/vida-providers/src/google.rs
git commit -m "feat(providers): add Anthropic tool_use + Google Gemini function calling support"
```

---

### Task 7: Wire AgentLoop into VidaEngine

**Files:**
- Modify: `crates/vida-core/src/engine.rs`

- [ ] **Step 1: Add `agent_chat` method to VidaEngine**

```rust
/// Send a message with agent loop (tool calling enabled).
pub async fn agent_chat(
    &mut self,
    session_id: &str,
    content: &str,
    tx: mpsc::Sender<AgentEvent>,
) -> Result<CompletionResponse, VidaError> {
    let session = self.db.get_session(session_id).await?
        .ok_or_else(|| VidaError::NotFound("Session".to_string()))?;

    let provider = self.providers.get(&session.provider_id)
        .ok_or_else(|| VidaError::NotFound("Provider".to_string()))?;

    // Save user message
    self.db.insert_message(session_id, "user", content).await?;

    // Build message history
    let db_messages = self.db.get_messages(session_id).await?;
    let mut messages: Vec<ChatMessage> = db_messages.iter().map(|m| ChatMessage {
        role: match m.role.as_str() {
            "user" => ChatRole::User,
            "assistant" => ChatRole::Assistant,
            "system" => ChatRole::System,
            "tool" => ChatRole::Tool,
            _ => ChatRole::User,
        },
        content: m.content.clone(),
        tool_call_id: None,
        name: None,
    }).collect();

    let options = CompletionOptions {
        model: Some(session.model.clone()),
        ..Default::default()
    };

    let config = AgentLoopConfig::default();

    let response = run_agent_loop(
        provider.as_ref(),
        &mut messages,
        &mut self.mcp_manager,
        Some(options),
        config,
        tx,
    ).await.map_err(|e| VidaError::Internal(e))?;

    // Save assistant response
    self.db.insert_message(session_id, "assistant", &response.content).await?;

    Ok(response)
}
```

- [ ] **Step 2: Run workspace tests**

Run: `cd "/home/hackos0911/AI/projects/IA/Vida ui" && cargo test --workspace 2>&1 | tail -10`
Expected: ALL PASS

- [ ] **Step 3: Commit**

```bash
cd "/home/hackos0911/AI/projects/IA/Vida ui"
git add crates/vida-core/src/engine.rs
git commit -m "feat(engine): add agent_chat() method with AgentLoop for tool-calling sessions"
```

---

### Task 8: Tauri IPC — Agent Stream Command

**Files:**
- Modify: `src-tauri/src/commands/chat.rs`

- [ ] **Step 1: Add agent_stream_completion command**

```rust
#[tauri::command]
pub async fn agent_stream_completion(
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    session_id: String,
    content: String,
) -> Result<(), String> {
    let (tx, mut rx) = mpsc::channel::<vida_core::agent_loop::AgentEvent>(100);
    let event_name = format!("agent-stream-{}", session_id);

    let engine_ref = engine.inner().clone();
    let sid = session_id.clone();

    tokio::spawn(async move {
        let mut e = engine_ref.write().await;
        let _ = e.agent_chat(&sid, &content, tx).await;
    });

    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let is_done = matches!(event, vida_core::agent_loop::AgentEvent::Done { .. });
            let _ = app.emit(&event_name, &event);
            if is_done { break; }
        }
    });

    Ok(())
}
```

Note: needs `engine.write()` (not read) because `agent_chat` takes `&mut self` for MCP tool execution.

- [ ] **Step 2: Register command in main.rs**

Add `agent_stream_completion` to the Tauri command registration.

- [ ] **Step 3: Build check**

Run: `cd "/home/hackos0911/AI/projects/IA/Vida ui" && cargo check --workspace 2>&1 | tail -5`
Expected: No errors

- [ ] **Step 4: Commit**

```bash
cd "/home/hackos0911/AI/projects/IA/Vida ui"
git add src-tauri/src/commands/chat.rs src-tauri/src/main.rs
git commit -m "feat(tauri): add agent_stream_completion command for tool-calling chat sessions"
```

---

### Task 9: Frontend — ToolCallBubble Component

**Files:**
- Create: `src/components/chat/ToolCallBubble.tsx`
- Modify: `src/components/chat/MessageBubble.tsx`

- [ ] **Step 1: Create ToolCallBubble**

```tsx
// src/components/chat/ToolCallBubble.tsx
import { useState } from 'react';

interface ToolCallProps {
  id: string;
  name: string;
  arguments: Record<string, unknown>;
  result?: string;
  isError?: boolean;
  isRejected?: boolean;
  rejectedReason?: string;
}

export function ToolCallBubble({ name, arguments: args, result, isError, isRejected, rejectedReason }: ToolCallProps) {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="my-1 rounded-lg border border-white/10 bg-white/5 p-2 text-sm">
      <div
        className="flex items-center gap-2 cursor-pointer"
        onClick={() => setExpanded(!expanded)}
      >
        <span className={`text-xs px-1.5 py-0.5 rounded ${
          isRejected ? 'bg-yellow-500/20 text-yellow-300' :
          isError ? 'bg-red-500/20 text-red-300' :
          result ? 'bg-green-500/20 text-green-300' :
          'bg-blue-500/20 text-blue-300 animate-pulse'
        }`}>
          {isRejected ? '⚠ rejected' : isError ? '✗ error' : result ? '✓ done' : '⟳ running'}
        </span>
        <code className="text-white/80">{name}</code>
        <span className="text-white/40 text-xs ml-auto">{expanded ? '▲' : '▼'}</span>
      </div>
      {expanded && (
        <div className="mt-2 space-y-1">
          <pre className="text-xs text-white/50 overflow-auto max-h-32 p-1 bg-black/20 rounded">
            {JSON.stringify(args, null, 2)}
          </pre>
          {result && (
            <pre className={`text-xs overflow-auto max-h-48 p-1 rounded ${
              isError ? 'text-red-300 bg-red-500/10' : 'text-green-300 bg-green-500/10'
            }`}>
              {result}
            </pre>
          )}
          {rejectedReason && (
            <p className="text-xs text-yellow-300">{rejectedReason}</p>
          )}
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Build check**

Run: `cd "/home/hackos0911/AI/projects/IA/Vida ui" && npm run build 2>&1 | tail -5`
Expected: Build OK

- [ ] **Step 3: Commit**

```bash
cd "/home/hackos0911/AI/projects/IA/Vida ui"
git add src/components/chat/ToolCallBubble.tsx
git commit -m "feat(ui): add ToolCallBubble component for inline tool call display"
```

---

### Task 10: Frontend — useAgentStream Hook

**Files:**
- Create: `src/hooks/useAgentStream.ts`

- [ ] **Step 1: Create the hook**

```typescript
// src/hooks/useAgentStream.ts
import { useCallback, useRef } from 'react';
import { invoke } from '../lib/tauri';
import { listen, UnlistenFn } from '@tauri-apps/api/event';
import { useStore } from '../stores/store';

interface AgentEvent {
  Token?: { content: string };
  ToolCallStart?: { id: string; name: string; arguments: Record<string, unknown> };
  ToolCallResult?: { id: string; name: string; result: string; is_error: boolean };
  ToolCallRejected?: { id: string; name: string; reason: string };
  Done?: { total_iterations: number };
  Error?: { error: string };
}

export function useAgentStream() {
  const unlistenRef = useRef<UnlistenFn | null>(null);

  const sendAgentMessage = useCallback(async (sessionId: string, content: string) => {
    const store = useStore.getState();
    const eventName = `agent-stream-${sessionId}`;

    // Clean up previous listener
    if (unlistenRef.current) {
      unlistenRef.current();
    }

    let fullContent = '';

    unlistenRef.current = await listen<AgentEvent>(eventName, (event) => {
      const data = event.payload;

      if (data.Token) {
        fullContent += data.Token.content;
        store.setStreamingContent(sessionId, fullContent);
      }

      if (data.ToolCallStart) {
        store.addToolCall(sessionId, {
          id: data.ToolCallStart.id,
          name: data.ToolCallStart.name,
          arguments: data.ToolCallStart.arguments,
        });
      }

      if (data.ToolCallResult) {
        store.updateToolCall(sessionId, data.ToolCallResult.id, {
          result: data.ToolCallResult.result,
          isError: data.ToolCallResult.is_error,
        });
      }

      if (data.ToolCallRejected) {
        store.updateToolCall(sessionId, data.ToolCallRejected.id, {
          isRejected: true,
          rejectedReason: data.ToolCallRejected.reason,
        });
      }

      if (data.Done) {
        store.finalizeAgentMessage(sessionId, fullContent);
        if (unlistenRef.current) {
          unlistenRef.current();
          unlistenRef.current = null;
        }
      }

      if (data.Error) {
        store.setStreamError(sessionId, data.Error.error);
        if (unlistenRef.current) {
          unlistenRef.current();
          unlistenRef.current = null;
        }
      }
    });

    await invoke('agent_stream_completion', {
      sessionId,
      content,
    });
  }, []);

  return { sendAgentMessage };
}
```

- [ ] **Step 2: Build check**

Run: `cd "/home/hackos0911/AI/projects/IA/Vida ui" && npm run lint 2>&1 | tail -5`
Expected: No type errors

- [ ] **Step 3: Commit**

```bash
cd "/home/hackos0911/AI/projects/IA/Vida ui"
git add src/hooks/useAgentStream.ts
git commit -m "feat(ui): add useAgentStream hook for streaming agent events with tool calls"
```

---

## Summary

| Task | What | Priority |
|------|------|----------|
| 1 | Extend LLMProvider trait with tool types | 🔴 Critical |
| 2 | Tool argument validator (JSON Schema) | 🔴 Critical |
| 3 | AgentLoop orchestrator | 🔴 Critical |
| 4 | OpenAI provider tool calling | 🔴 Critical |
| 5 | Ollama provider tool calling | 🔴 Critical |
| 6 | Anthropic + Google tool calling | 🟡 Important |
| 7 | Wire AgentLoop into VidaEngine | 🔴 Critical |
| 8 | Tauri IPC agent_stream command | 🔴 Critical |
| 9 | ToolCallBubble UI component | 🟡 Important |
| 10 | useAgentStream hook | 🟡 Important |

Tasks 1-5 + 7-8 = MVP fonctionnel. Tasks 6, 9, 10 = polish.
