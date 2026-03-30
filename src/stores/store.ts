import { create } from "zustand";
import { persist } from "zustand/middleware";
import type {
  AuthSession,
  AuthUser,
  SessionRow,
  MessageRow,
  ProviderInfo,
  TeamRow,
  WorkspaceConfig,
  RecentWorkspaceRow,
  McpServerInfo,
} from "@/src/lib/tauri";

// ── State types ──

interface SessionsSlice {
  sessions: SessionRow[];
  currentSessionId: string | null;
}

interface MessagesSlice {
  messages: Record<string, MessageRow[]>;
  streamingMessageId: string | null;
  streamingContent: string;
}

interface TeamsSlice {
  teams: TeamRow[];
  agentStreaming: Record<string, string>; // agent_id → accumulated content
}

interface WorkspaceSlice {
  workspacePath: string | null;
  workspaceConfig: WorkspaceConfig | null;
  recentWorkspaces: RecentWorkspaceRow[];
}

interface McpSlice {
  mcpServers: McpServerInfo[];
}

interface ProvidersSlice {
  providers: ProviderInfo[];
  providerHealth: Record<string, boolean>;
}

interface AuthSlice {
  authActor: AuthSession | null;
  authUsers: AuthUser[];
}

type Theme = "light" | "dark";

interface UiSlice {
  theme: Theme;
  sidebarOpen: boolean;
  settingsOpen: boolean;
}

// ── Actions ──

interface SessionsActions {
  setSessions: (sessions: SessionRow[]) => void;
  setCurrentSession: (id: string | null) => void;
  addSession: (session: SessionRow) => void;
  removeSession: (id: string) => void;
}

interface MessagesActions {
  setMessages: (sessionId: string, messages: MessageRow[]) => void;
  addMessage: (sessionId: string, message: MessageRow) => void;
  startStreaming: (messageId: string) => void;
  appendToken: (token: string) => void;
  finishStreaming: () => void;
}

interface TeamsActions {
  setTeams: (teams: TeamRow[]) => void;
  addTeam: (team: TeamRow) => void;
  removeTeam: (id: string) => void;
  startTeamStreaming: (agentIds: string[]) => void;
  appendAgentToken: (agentId: string, token: string) => void;
  finishAgentStreaming: (agentId: string) => void;
  finishAllStreaming: () => void;
}

interface WorkspaceActions {
  setWorkspacePath: (path: string | null) => void;
  setWorkspaceConfig: (config: WorkspaceConfig | null) => void;
  setRecentWorkspaces: (workspaces: RecentWorkspaceRow[]) => void;
}

interface McpActions {
  setMcpServers: (servers: McpServerInfo[]) => void;
}

interface ProvidersActions {
  setProviders: (providers: ProviderInfo[]) => void;
  setProviderHealth: (health: Record<string, boolean>) => void;
}

interface AuthActions {
  setAuthActor: (actor: AuthSession | null) => void;
  setAuthUsers: (users: AuthUser[]) => void;
}

interface UiActions {
  setTheme: (theme: Theme) => void;
  toggleSidebar: () => void;
  setSettingsOpen: (open: boolean) => void;
}

// ── Combined store type ──

type StoreState = SessionsSlice &
  MessagesSlice &
  TeamsSlice &
  McpSlice &
  WorkspaceSlice &
  ProvidersSlice &
  AuthSlice &
  UiSlice &
  SessionsActions &
  MessagesActions &
  TeamsActions &
  McpActions &
  WorkspaceActions &
  ProvidersActions &
  AuthActions &
  UiActions;

// ── Store ──

export const useStore = create<StoreState>()(
  persist(
    (set) => ({
      // ── Sessions slice ──
      sessions: [],
      currentSessionId: null,

      setSessions: (sessions) => set({ sessions }),

      setCurrentSession: (id) => set({ currentSessionId: id }),

      addSession: (session) =>
        set((s) => ({ sessions: [session, ...s.sessions] })),

      removeSession: (id) =>
        set((s) => ({
          sessions: s.sessions.filter((sess) => sess.id !== id),
          currentSessionId:
            s.currentSessionId === id ? null : s.currentSessionId,
          messages: (() => { const m = { ...s.messages }; delete m[id]; return m; })(),
        })),

      // ── Messages slice ──
      messages: {},
      streamingMessageId: null,
      streamingContent: "",

      setMessages: (sessionId, messages) =>
        set((s) => ({
          messages: { ...s.messages, [sessionId]: messages },
        })),

      addMessage: (sessionId, message) =>
        set((s) => ({
          messages: {
            ...s.messages,
            [sessionId]: [...(s.messages[sessionId] ?? []), message],
          },
        })),

      startStreaming: (messageId) =>
        set({ streamingMessageId: messageId, streamingContent: "" }),

      appendToken: (token) =>
        set((s) => ({ streamingContent: s.streamingContent + token })),

      finishStreaming: () =>
        set((s) => {
          if (!s.streamingMessageId || !s.currentSessionId) {
            return { streamingMessageId: null, streamingContent: "" };
          }
          const sessionMsgs = s.messages[s.currentSessionId] || [];
          const updated = sessionMsgs.map((msg) =>
            msg.id === s.streamingMessageId
              ? { ...msg, content: s.streamingContent }
              : msg
          );
          return {
            messages: { ...s.messages, [s.currentSessionId]: updated },
            streamingMessageId: null,
            streamingContent: "",
          };
        }),

      // ── Teams slice ──
      teams: [],
      agentStreaming: {},

      setTeams: (teams) => set({ teams }),

      addTeam: (team) =>
        set((s) => ({ teams: [team, ...s.teams] })),

      removeTeam: (id) =>
        set((s) => ({ teams: s.teams.filter((t) => t.id !== id) })),

      startTeamStreaming: (agentIds) =>
        set(() => {
          const agentStreaming: Record<string, string> = {};
          for (const id of agentIds) {
            agentStreaming[id] = "";
          }
          return { agentStreaming };
        }),

      appendAgentToken: (agentId, token) =>
        set((s) => ({
          agentStreaming: {
            ...s.agentStreaming,
            [agentId]: (s.agentStreaming[agentId] ?? "") + token,
          },
        })),

      finishAgentStreaming: (agentId) =>
        set((s) => {
          const updated = { ...s.agentStreaming };
          delete updated[agentId];
          return { agentStreaming: updated };
        }),

      finishAllStreaming: () => set({ agentStreaming: {} }),

      // ── MCP slice ──
      mcpServers: [],

      setMcpServers: (servers) => set({ mcpServers: servers }),

      // ── Workspace slice ──
      workspacePath: null,
      workspaceConfig: null,
      recentWorkspaces: [],

      setWorkspacePath: (path) => set({ workspacePath: path }),
      setWorkspaceConfig: (config) => set({ workspaceConfig: config }),
      setRecentWorkspaces: (workspaces) => set({ recentWorkspaces: workspaces }),

      // ── Providers slice ──
      providers: [],
      providerHealth: {},

      setProviders: (providers) => set({ providers }),

      setProviderHealth: (health) => set({ providerHealth: health }),

      // ── Auth slice ──
      authActor: null,
      authUsers: [],

      setAuthActor: (actor) => set({ authActor: actor }),
      setAuthUsers: (users) => set({ authUsers: users }),

      // ── UI slice ──
      theme: "dark",
      sidebarOpen: true,
      settingsOpen: false,

      setTheme: (theme) => {
        document.documentElement.setAttribute("data-theme", theme);
        set({ theme });
      },

      toggleSidebar: () => set((s) => ({ sidebarOpen: !s.sidebarOpen })),

      setSettingsOpen: (open) => set({ settingsOpen: open }),
    }),
    {
      name: "vida-store",
      partialize: (state) => ({
        theme: state.theme,
        sidebarOpen: state.sidebarOpen,
        currentSessionId: state.currentSessionId,
      }),
      onRehydrateStorage: () => (state) => {
        if (state?.theme) {
          document.documentElement.setAttribute("data-theme", state.theme);
        }
      },
    }
  )
);
