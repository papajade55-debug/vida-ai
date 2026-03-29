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

// ── Typed invoke wrappers ──

export const api = {
  // Auth
  isPinConfigured: () => invoke<boolean>("is_pin_configured"),
  storeApiKey: (providerId: string, key: string) =>
    invoke<void>("store_api_key", { providerId, key }),

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

  // Config
  getConfig: () => invoke<AppConfig>("get_config"),
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
