use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};

/// A tool discovered from an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub server_name: String,
}

/// Information about an MCP server (for frontend display).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub command: String,
    pub running: bool,
    pub tool_count: usize,
    pub tools: Vec<McpTool>,
}

/// Result of a tool call.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<McpToolResultContent>,
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResultContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

/// Handle to a running MCP server child process.
struct McpServerHandle {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    tools: Vec<McpTool>,
    command: String,
    next_id: AtomicU64,
}

impl McpServerHandle {
    fn next_request_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }
}

/// Error type for MCP operations.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Server not found: {0}")]
    ServerNotFound(String),
    #[error("Tool not found: {0}")]
    ToolNotFound(String),
    #[error("Server already running: {0}")]
    AlreadyRunning(String),
    #[error("Protocol error: {0}")]
    Protocol(String),
    #[error("Tool call error: {0}")]
    ToolError(String),
}

/// Manages MCP server processes and tool routing.
pub struct McpManager {
    servers: HashMap<String, McpServerHandle>,
    #[cfg(test)]
    test_tools: Vec<McpTool>,
    #[cfg(test)]
    test_tool_results: HashMap<String, McpToolResult>,
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
            #[cfg(test)]
            test_tools: Vec::new(),
            #[cfg(test)]
            test_tool_results: HashMap::new(),
        }
    }

    /// Start an MCP server process and discover its tools.
    pub fn start_server(
        &mut self,
        name: &str,
        command: &str,
        args: &[String],
        env: &HashMap<String, String>,
    ) -> Result<Vec<McpTool>, McpError> {
        if self.servers.contains_key(name) {
            return Err(McpError::AlreadyRunning(name.to_string()));
        }

        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null());

        for (k, v) in env {
            cmd.env(k, v);
        }

        let mut child = cmd.spawn()?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| McpError::Protocol("Failed to capture stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| McpError::Protocol("Failed to capture stdout".to_string()))?;

        let mut handle = McpServerHandle {
            process: child,
            stdin,
            stdout: BufReader::new(stdout),
            tools: Vec::new(),
            command: command.to_string(),
            next_id: AtomicU64::new(1),
        };

        // Send initialize request
        let init_id = handle.next_request_id();
        let init_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": init_id,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "vida-ai",
                    "version": "0.5.0"
                }
            }
        });
        send_jsonrpc(&mut handle.stdin, &init_request)?;
        let _init_response = read_jsonrpc(&mut handle.stdout)?;

        // Send initialized notification
        let initialized = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        });
        send_jsonrpc(&mut handle.stdin, &initialized)?;

        // Discover tools
        let tools_id = handle.next_request_id();
        let tools_request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": tools_id,
            "method": "tools/list",
            "params": {}
        });
        send_jsonrpc(&mut handle.stdin, &tools_request)?;
        let tools_response = read_jsonrpc(&mut handle.stdout)?;

        let tools = parse_tools_response(&tools_response, name)?;
        handle.tools = tools.clone();

        self.servers.insert(name.to_string(), handle);
        Ok(tools)
    }

    /// Stop a running MCP server.
    pub fn stop_server(&mut self, name: &str) -> Result<(), McpError> {
        let mut handle = self
            .servers
            .remove(name)
            .ok_or_else(|| McpError::ServerNotFound(name.to_string()))?;
        let _ = handle.process.kill();
        let _ = handle.process.wait();
        Ok(())
    }

    /// List all configured servers with their running status.
    pub fn list_servers(&self) -> Vec<McpServerInfo> {
        self.servers
            .iter()
            .map(|(name, handle)| McpServerInfo {
                name: name.clone(),
                command: handle.command.clone(),
                running: true,
                tool_count: handle.tools.len(),
                tools: handle.tools.clone(),
            })
            .collect()
    }

    /// List all tools from all running servers.
    pub fn list_tools(&self) -> Vec<McpTool> {
        let tools: Vec<McpTool> = self
            .servers
            .values()
            .flat_map(|h| h.tools.iter().cloned())
            .collect();

        #[cfg(test)]
        let tools = {
            let mut tools = tools;
            tools.extend(self.test_tools.clone());
            tools
        };

        tools
    }

    /// Call a tool by name, routing to the correct server.
    pub fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: serde_json::Value,
    ) -> Result<McpToolResult, McpError> {
        #[cfg(test)]
        if let Some(result) = self.test_tool_results.get(tool_name) {
            return Ok(result.clone());
        }

        // Find which server owns this tool
        let server_name = self
            .servers
            .iter()
            .find(|(_, h)| h.tools.iter().any(|t| t.name == tool_name))
            .map(|(name, _)| name.clone())
            .ok_or_else(|| McpError::ToolNotFound(tool_name.to_string()))?;

        let handle = self
            .servers
            .get_mut(&server_name)
            .ok_or_else(|| McpError::ServerNotFound(server_name.clone()))?;

        let req_id = handle.next_request_id();
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": req_id,
            "method": "tools/call",
            "params": {
                "name": tool_name,
                "arguments": arguments
            }
        });

        send_jsonrpc(&mut handle.stdin, &request)?;
        let response = read_jsonrpc(&mut handle.stdout)?;

        parse_tool_call_response(&response)
    }

    /// Check if a server is currently running.
    pub fn is_running(&self, name: &str) -> bool {
        self.servers.contains_key(name)
    }

    /// Get the number of running servers.
    pub fn running_count(&self) -> usize {
        self.servers.len()
    }

    #[cfg(test)]
    pub fn register_test_tool(&mut self, tool: McpTool, result: McpToolResult) {
        self.test_tool_results.insert(tool.name.clone(), result);
        self.test_tools.push(tool);
    }
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for McpManager {
    fn drop(&mut self) {
        let names: Vec<String> = self.servers.keys().cloned().collect();
        for name in names {
            let _ = self.stop_server(&name);
        }
    }
}

// ── JSON-RPC helpers ──

fn send_jsonrpc(stdin: &mut ChildStdin, msg: &serde_json::Value) -> Result<(), McpError> {
    let serialized = serde_json::to_string(msg)?;
    writeln!(stdin, "{}", serialized)?;
    stdin.flush()?;
    Ok(())
}

fn read_jsonrpc(stdout: &mut BufReader<ChildStdout>) -> Result<serde_json::Value, McpError> {
    let mut line = String::new();
    stdout.read_line(&mut line)?;
    if line.is_empty() {
        return Err(McpError::Protocol("Empty response from server".to_string()));
    }
    let value: serde_json::Value = serde_json::from_str(line.trim())?;
    Ok(value)
}

fn parse_tools_response(
    response: &serde_json::Value,
    server_name: &str,
) -> Result<Vec<McpTool>, McpError> {
    let tools_array = response
        .get("result")
        .and_then(|r| r.get("tools"))
        .and_then(|t| t.as_array())
        .ok_or_else(|| McpError::Protocol("No tools array in response".to_string()))?;

    let mut tools = Vec::new();
    for tool_val in tools_array {
        let name = tool_val
            .get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("unknown")
            .to_string();
        let description = tool_val
            .get("description")
            .and_then(|d| d.as_str())
            .unwrap_or("")
            .to_string();
        let input_schema = tool_val
            .get("inputSchema")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        tools.push(McpTool {
            name,
            description,
            input_schema,
            server_name: server_name.to_string(),
        });
    }
    Ok(tools)
}

fn parse_tool_call_response(response: &serde_json::Value) -> Result<McpToolResult, McpError> {
    if let Some(error) = response.get("error") {
        let msg = error
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown error");
        return Err(McpError::ToolError(msg.to_string()));
    }

    let result = response
        .get("result")
        .ok_or_else(|| McpError::Protocol("No result in tool call response".to_string()))?;

    let is_error = result
        .get("isError")
        .and_then(|e| e.as_bool())
        .unwrap_or(false);

    let content = result
        .get("content")
        .and_then(|c| c.as_array())
        .map(|arr| {
            arr.iter()
                .map(|item| McpToolResultContent {
                    content_type: item
                        .get("type")
                        .and_then(|t| t.as_str())
                        .unwrap_or("text")
                        .to_string(),
                    text: item
                        .get("text")
                        .and_then(|t| t.as_str())
                        .unwrap_or("")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(McpToolResult { content, is_error })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_manager_new() {
        let manager = McpManager::new();
        assert_eq!(manager.running_count(), 0);
        assert!(manager.list_servers().is_empty());
        assert!(manager.list_tools().is_empty());
    }

    #[test]
    fn test_mcp_manager_default() {
        let manager = McpManager::default();
        assert_eq!(manager.running_count(), 0);
    }

    #[test]
    fn test_stop_nonexistent_server() {
        let mut manager = McpManager::new();
        let result = manager.stop_server("nonexistent");
        assert!(result.is_err());
        match result.unwrap_err() {
            McpError::ServerNotFound(name) => assert_eq!(name, "nonexistent"),
            other => panic!("Expected ServerNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_call_tool_no_servers() {
        let mut manager = McpManager::new();
        let result = manager.call_tool("some_tool", serde_json::json!({}));
        assert!(result.is_err());
        match result.unwrap_err() {
            McpError::ToolNotFound(name) => assert_eq!(name, "some_tool"),
            other => panic!("Expected ToolNotFound, got {:?}", other),
        }
    }

    #[test]
    fn test_is_running() {
        let manager = McpManager::new();
        assert!(!manager.is_running("test"));
    }

    #[test]
    fn test_parse_tools_response_valid() {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "tools": [
                    {
                        "name": "read_file",
                        "description": "Read a file from the filesystem",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" }
                            },
                            "required": ["path"]
                        }
                    },
                    {
                        "name": "write_file",
                        "description": "Write content to a file",
                        "inputSchema": {
                            "type": "object",
                            "properties": {
                                "path": { "type": "string" },
                                "content": { "type": "string" }
                            },
                            "required": ["path", "content"]
                        }
                    }
                ]
            }
        });

        let tools = parse_tools_response(&response, "filesystem").unwrap();
        assert_eq!(tools.len(), 2);
        assert_eq!(tools[0].name, "read_file");
        assert_eq!(tools[0].description, "Read a file from the filesystem");
        assert_eq!(tools[0].server_name, "filesystem");
        assert_eq!(tools[1].name, "write_file");
        assert_eq!(tools[1].server_name, "filesystem");
    }

    #[test]
    fn test_parse_tools_response_empty() {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {
                "tools": []
            }
        });

        let tools = parse_tools_response(&response, "test").unwrap();
        assert!(tools.is_empty());
    }

    #[test]
    fn test_parse_tools_response_invalid() {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "result": {}
        });

        let result = parse_tools_response(&response, "test");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_tool_call_response_success() {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": "Hello, world!"
                    }
                ],
                "isError": false
            }
        });

        let result = parse_tool_call_response(&response).unwrap();
        assert!(!result.is_error);
        assert_eq!(result.content.len(), 1);
        assert_eq!(result.content[0].text, "Hello, world!");
        assert_eq!(result.content[0].content_type, "text");
    }

    #[test]
    fn test_parse_tool_call_response_error() {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "error": {
                "code": -32600,
                "message": "Invalid request"
            }
        });

        let result = parse_tool_call_response(&response);
        assert!(result.is_err());
        match result.unwrap_err() {
            McpError::ToolError(msg) => assert_eq!(msg, "Invalid request"),
            other => panic!("Expected ToolError, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_tool_call_response_is_error_flag() {
        let response = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "result": {
                "content": [
                    {
                        "type": "text",
                        "text": "File not found"
                    }
                ],
                "isError": true
            }
        });

        let result = parse_tool_call_response(&response).unwrap();
        assert!(result.is_error);
        assert_eq!(result.content[0].text, "File not found");
    }

    #[test]
    fn test_mcp_tool_serde() {
        let tool = McpTool {
            name: "read_file".to_string(),
            description: "Read a file".to_string(),
            input_schema: serde_json::json!({"type": "object"}),
            server_name: "fs".to_string(),
        };

        let json = serde_json::to_string(&tool).unwrap();
        let deserialized: McpTool = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "read_file");
        assert_eq!(deserialized.server_name, "fs");
    }

    #[test]
    fn test_mcp_server_info_serde() {
        let info = McpServerInfo {
            name: "test".to_string(),
            command: "npx".to_string(),
            running: true,
            tool_count: 3,
            tools: vec![],
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: McpServerInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "test");
        assert!(deserialized.running);
    }

    #[test]
    fn test_mcp_tool_result_serde() {
        let result = McpToolResult {
            content: vec![McpToolResultContent {
                content_type: "text".to_string(),
                text: "output".to_string(),
            }],
            is_error: false,
        };

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: McpToolResult = serde_json::from_str(&json).unwrap();
        assert!(!deserialized.is_error);
        assert_eq!(deserialized.content[0].text, "output");
    }
}
