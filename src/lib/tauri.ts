import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

// ── Types mirroring Rust structs ──

export interface ProviderInfo {
  name: string;
  provider_type: "local" | "cloud";
  models: string[];
}

export interface SessionRow {
  id: string;
  title: string | null;
  provider_id: string;
  model: string;
  system_prompt: string | null;
  created_at: string;
  updated_at: string;
  team_id: string | null;
}

export interface MessageRow {
  id: string;
  session_id: string;
  role: "system" | "user" | "assistant";
  content: string;
  token_count: number | null;
  created_at: string;
  agent_id: string | null;
  agent_name: string | null;
  agent_color: string | null;
}

export interface CompletionResponse {
  content: string;
  model: string;
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
}

export type StreamEvent =
  | { Token: { content: string } }
  | { Error: { error: string } }
  | "Done";

export interface TeamRow {
  id: string;
  name: string;
  mode: string;
  created_at: string;
}

export interface TeamMemberRow {
  id: string;
  team_id: string;
  provider_id: string;
  model: string;
  display_name: string | null;
  color: string;
  role: string | null;
  created_at: string;
}

export type TeamStreamEvent =
  | { AgentToken: { agent_id: string; agent_name: string; agent_color: string; content: string } }
  | { AgentDone: { agent_id: string } }
  | { AgentError: { agent_id: string; error: string } }
  | "AllDone";

export interface AppConfig {
  language: string;
  theme: string;
}

export type PermissionMode = "yolo" | "ask" | "sandbox";

export interface PermissionConfig {
  file_read: boolean;
  file_write: boolean;
  shell_execute: boolean;
  network_access: boolean;
}

export interface WorkspaceConfig {
  name: string;
  default_provider: string | null;
  default_model: string | null;
  system_prompt: string | null;
  permission_mode: PermissionMode;
  permissions: PermissionConfig;
}

export interface RecentWorkspaceRow {
  path: string;
  name: string;
  last_used: string;
}

// ── MCP types ──

export interface McpServerInfo {
  name: string;
  command: string;
  running: boolean;
  tool_count: number;
  tools: McpTool[];
}

export interface McpTool {
  name: string;
  description: string;
  input_schema: Record<string, unknown>;
  server_name: string;
}

export interface McpToolResultContent {
  type: string;
  text: string;
}

export interface McpToolResult {
  content: McpToolResultContent[];
  is_error: boolean;
}

export interface McpServerConfigRow {
  id: string;
  workspace_path: string | null;
  name: string;
  command: string;
  args_json: string | null;
  env_json: string | null;
  enabled: number;
  created_at: string;
}

// ── Typed invoke wrappers ──

export const api = {
  // Auth
  isPinConfigured: () => invoke<boolean>("is_pin_configured"),
  storeApiKey: (providerId: string, key: string) =>
    invoke<void>("store_api_key", { providerId, key }),
  removeApiKey: (providerId: string) =>
    invoke<void>("remove_api_key", { providerId }),

  // Providers
  listProviders: () => invoke<ProviderInfo[]>("list_providers"),
  listModels: (providerId: string) =>
    invoke<string[]>("list_models", { providerId }),
  healthCheck: () => invoke<[string, boolean][]>("health_check"),

  // Chat
  sendMessage: (sessionId: string, content: string) =>
    invoke<CompletionResponse>("send_message", { sessionId, content }),
  streamCompletion: (sessionId: string, content: string) =>
    invoke<void>("stream_completion", { sessionId, content }),
  createSession: (providerId: string, model: string) =>
    invoke<SessionRow>("create_session", { providerId, model }),
  listSessions: (limit: number) =>
    invoke<SessionRow[]>("list_sessions", { limit }),
  getMessages: (sessionId: string) =>
    invoke<MessageRow[]>("get_messages", { sessionId }),
  deleteSession: (sessionId: string) =>
    invoke<void>("delete_session", { sessionId }),
  sendVisionMessage: (sessionId: string, imageBase64: string, prompt: string) =>
    invoke<CompletionResponse>("send_vision_message", { sessionId, imageBase64, prompt }),

  // Teams
  createTeam: (name: string, members: [string, string][]) =>
    invoke<TeamRow>("create_team", { name, members }),
  listTeams: () => invoke<TeamRow[]>("list_teams"),
  getTeam: (teamId: string) =>
    invoke<[TeamRow, TeamMemberRow[]]>("get_team", { teamId }),
  deleteTeam: (teamId: string) =>
    invoke<void>("delete_team", { teamId }),
  createTeamSession: (teamId: string) =>
    invoke<SessionRow>("create_team_session", { teamId }),
  streamTeamCompletion: (sessionId: string, content: string) =>
    invoke<void>("stream_team_completion", { sessionId, content }),

  // MCP
  startMcpServer: (name: string) =>
    invoke<McpTool[]>("start_mcp_server", { name }),
  stopMcpServer: (name: string) =>
    invoke<void>("stop_mcp_server", { name }),
  listMcpServers: () =>
    invoke<McpServerInfo[]>("list_mcp_servers"),
  listMcpTools: () =>
    invoke<McpTool[]>("list_mcp_tools"),
  callMcpTool: (toolName: string, args: Record<string, unknown>) =>
    invoke<McpToolResult>("call_mcp_tool", { toolName, arguments: args }),
  saveMcpServerConfig: (config: McpServerConfigRow) =>
    invoke<void>("save_mcp_server_config", { config }),
  deleteMcpServerConfig: (id: string) =>
    invoke<void>("delete_mcp_server_config", { id }),

  // Config
  getConfig: () => invoke<AppConfig>("get_config"),

  // Workspaces
  openWorkspace: (path: string) =>
    invoke<WorkspaceConfig>("open_workspace", { path }),
  createWorkspace: (path: string, name: string) =>
    invoke<WorkspaceConfig>("create_workspace", { path, name }),
  listRecentWorkspaces: () =>
    invoke<RecentWorkspaceRow[]>("list_recent_workspaces"),
  getWorkspaceConfig: () =>
    invoke<WorkspaceConfig>("get_workspace_config"),
  setWorkspaceConfig: (config: WorkspaceConfig) =>
    invoke<void>("set_workspace_config", { config }),
  getPermissionMode: () =>
    invoke<string>("get_permission_mode"),
  setPermissionMode: (mode: PermissionMode) =>
    invoke<void>("set_permission_mode", { mode }),
};

// ── Stream listener ──

export function onStreamEvent(
  sessionId: string,
  callback: (event: StreamEvent) => void
) {
  return listen<StreamEvent>(`llm-stream-${sessionId}`, (e) => {
    callback(e.payload);
  });
}

export function onTeamStreamEvent(
  sessionId: string,
  callback: (event: TeamStreamEvent) => void
) {
  return listen<TeamStreamEvent>(`team-stream-${sessionId}`, (e) => {
    callback(e.payload);
  });
}
