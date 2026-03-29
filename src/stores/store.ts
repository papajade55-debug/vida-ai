import { create } from "zustand";
import { persist } from "zustand/middleware";
import type {
  SessionRow,
  MessageRow,
  ProviderInfo,
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

interface ProvidersSlice {
  providers: ProviderInfo[];
  providerHealth: Record<string, boolean>;
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

interface ProvidersActions {
  setProviders: (providers: ProviderInfo[]) => void;
  setProviderHealth: (health: Record<string, boolean>) => void;
}

interface UiActions {
  setTheme: (theme: Theme) => void;
  toggleSidebar: () => void;
  setSettingsOpen: (open: boolean) => void;
}

// ── Combined store type ──

type StoreState = SessionsSlice &
  MessagesSlice &
  ProvidersSlice &
  UiSlice &
  SessionsActions &
  MessagesActions &
  ProvidersActions &
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

      // ── Providers slice ──
      providers: [],
      providerHealth: {},

      setProviders: (providers) => set({ providers }),

      setProviderHealth: (health) => set({ providerHealth: health }),

      // ── UI slice ──
      theme: "light",
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
