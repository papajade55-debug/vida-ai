# Vida AI — Phase 5 Design Spec: MCP Integration + Tool Routing

**Date:** 2026-03-29
**Status:** Approved
**Scope:** Phase 5 — MCP server management, tool routing, prompt rewriting for non-tool-calling models
**Depends on:** Phases 1-4

## 1. Overview

Phase 5 integrates the Model Context Protocol (MCP) into Vida AI. The app can launch MCP server processes, route tool calls from LLMs, and make tools accessible even to models that don't natively support tool calling (via prompt rewriting).

## 2. MCP Manager (vida-core)

### 2.1 Architecture
- Extend the existing `crates/vida-core/src/` — no new crate needed
- `McpManager` struct manages MCP server lifecycle (start/stop/list)
- Each MCP server is a child process communicating via stdio (JSON-RPC)
- Configuration stored per-workspace in `.vida/mcp.json`

### 2.2 MCP Config (.vida/mcp.json)
```json
{
  "servers": {
    "filesystem": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-filesystem", "/path/to/workspace"],
      "enabled": true
    },
    "github": {
      "command": "npx",
      "args": ["-y", "@modelcontextprotocol/server-github"],
      "env": { "GITHUB_TOKEN": "..." },
      "enabled": true
    }
  }
}
```

### 2.3 McpManager
```rust
pub struct McpManager {
    servers: HashMap<String, McpServerHandle>,
}

pub struct McpServerHandle {
    process: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    tools: Vec<McpTool>,
}

pub struct McpTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
    pub server_name: String,
}
```

Methods:
- `start_server(name, command, args, env)` — spawn process, initialize JSON-RPC, discover tools
- `stop_server(name)` — kill process
- `list_tools()` — aggregate tools from all running servers
- `call_tool(tool_name, args)` — route to correct server, send JSON-RPC request, return result
- `restart_server(name)` — stop + start

### 2.4 Tool Discovery
On server start: send `initialize` JSON-RPC → receive capabilities → send `tools/list` → cache tools.

### 2.5 Tool Routing
When an LLM requests a tool call:
1. Parse tool_call from LLM response
2. Find which MCP server owns the tool (via McpManager.list_tools())
3. Call the tool via JSON-RPC to the owning server
4. Return result to LLM as tool_result message
5. LLM continues with the result

### 2.6 Prompt Rewriting (non-tool-calling models)
For models without native tool use (e.g., older Ollama models):
1. Inject available tools into system prompt as a structured description
2. Instruct the model to output tool calls in a parseable format (e.g., `<tool_call>{"name":"...","args":{...}}</tool_call>`)
3. Parse the model output for tool_call tags
4. Execute the tool, inject result, let model continue

## 3. DB Changes

```sql
-- Migration 004_mcp.sql
CREATE TABLE IF NOT EXISTS mcp_server_configs (
    id          TEXT PRIMARY KEY,
    workspace_path TEXT,
    name        TEXT NOT NULL,
    command     TEXT NOT NULL,
    args_json   TEXT,
    env_json    TEXT,
    enabled     INTEGER NOT NULL DEFAULT 1,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
```

## 4. Tauri Commands
- `start_mcp_server(name)`, `stop_mcp_server(name)`, `restart_mcp_server(name)`
- `list_mcp_servers()` — returns server name + status + tool count
- `list_mcp_tools()` — all available tools across servers
- `call_mcp_tool(tool_name, args_json)` — execute a tool
- `get_mcp_config()`, `set_mcp_config(config)` — workspace MCP config
- Events: `mcp-server-status` (started/stopped/error)

## 5. Frontend
- Create `src/components/mcp/McpPanel.tsx` — list MCP servers with status (running/stopped), start/stop buttons, tool count
- Create `src/components/mcp/McpServerCard.tsx` — one server card
- Create `src/components/mcp/McpConfigModal.tsx` — add/edit/remove MCP server config
- Create `src/hooks/useMcp.ts` — manage MCP state
- Add MCP section to Settings modal (or sidebar)
- In chat: show tool calls as collapsible blocks (tool name, args, result)

## 6. Out of Scope
- MCP Resources (read-only data) — future
- MCP Prompts (prompt templates) — future
- MCP Sampling (server-initiated LLM calls) — future
- Custom MCP server development — user responsibility
