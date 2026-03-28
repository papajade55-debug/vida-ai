# Vida AI Phase 2A — UI / Design System / Chat Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Liquid Glass frontend for Vida AI — design system, Zustand store, streaming chat UI, unified sidebar with sessions and agents.

**Architecture:** React 19 + TypeScript frontend communicating with the Rust/Tauri v2 backend (Phase 1). Zustand for state with granular subscriptions to optimize streaming. CSS custom properties for adaptive light/dark theming. Framer Motion for animations.

**Tech Stack:** React 19, TypeScript, Tailwind CSS 4, Framer Motion, Zustand 4, react-markdown, rehype-highlight, remark-gfm, highlight.js, lucide-react, react-i18next, @tauri-apps/api.

**Spec:** `docs/superpowers/specs/2026-03-29-vida-ai-phase2a-ui-design.md`

---

## File Map

### New files (22)
- Create: `src/design-system/tokens.css`
- Create: `src/design-system/GlassPanel.tsx`
- Create: `src/design-system/GlassButton.tsx`
- Create: `src/design-system/GlassInput.tsx`
- Create: `src/design-system/GlassCard.tsx`
- Create: `src/design-system/StatusDot.tsx`
- Create: `src/stores/store.ts`
- Create: `src/hooks/useStreamCompletion.ts`
- Create: `src/hooks/useSessions.ts`
- Create: `src/hooks/useProviders.ts`
- Create: `src/hooks/useTheme.ts`
- Create: `src/components/layout/AppLayout.tsx`
- Create: `src/components/layout/Sidebar.tsx`
- Create: `src/components/sidebar/SessionList.tsx`
- Create: `src/components/sidebar/SessionItem.tsx`
- Create: `src/components/sidebar/AgentList.tsx`
- Create: `src/components/sidebar/AgentItem.tsx`
- Create: `src/components/chat/ChatArea.tsx`
- Create: `src/components/chat/ChatHeader.tsx`
- Create: `src/components/chat/MessageList.tsx`
- Create: `src/components/chat/MessageBubble.tsx`
- Create: `src/components/chat/ChatInput.tsx`

### Modified files (3)
- Modify: `src/index.css`
- Modify: `src/main.tsx`
- Modify: `src/App.tsx`

---

## Task 1: Install dependencies + Design tokens

**Files:**
- Modify: `package.json` (npm install)
- Create: `src/design-system/tokens.css`
- Modify: `src/index.css`

- [ ] **Step 1: Install new NPM packages**

```bash
cd "/home/hackos0911/AI/projects/IA/Vida ui"
npm install zustand react-markdown rehype-highlight remark-gfm highlight.js
```

- [ ] **Step 2: Create design tokens CSS**

`src/design-system/tokens.css`:
```css
:root {
  --glass-bg: rgba(255, 255, 255, 0.7);
  --glass-blur: 16px;
  --glass-border: rgba(0, 0, 0, 0.06);
  --glass-shadow: 0 2px 12px rgba(0, 0, 0, 0.04);
  --bg-primary: #f0f0f5;
  --text-primary: #1a1a2e;
  --text-secondary: #6b7280;
  --accent: #4f46e5;
  --accent-hover: #4338ca;
  --accent-text: #ffffff;
  --msg-user-bg: var(--accent);
  --msg-user-text: #ffffff;
  --msg-assistant-bg: rgba(0, 0, 0, 0.03);
  --msg-assistant-border: rgba(0, 0, 0, 0.06);
  --status-active: #22c55e;
  --status-streaming: #f59e0b;
  --status-error: #ef4444;
  --status-offline: #9ca3af;
  --radius: 12px;
  --radius-lg: 16px;
  --sidebar-width: 280px;
}

[data-theme="dark"] {
  --glass-bg: rgba(255, 255, 255, 0.05);
  --glass-blur: 20px;
  --glass-border: rgba(255, 255, 255, 0.08);
  --glass-shadow: 0 4px 24px rgba(0, 0, 0, 0.2);
  --bg-primary: #0f0c29;
  --text-primary: #e0e0e8;
  --text-secondary: #8b949e;
  --accent: #818cf8;
  --accent-hover: #6366f1;
  --accent-text: #ffffff;
  --msg-user-bg: rgba(99, 102, 241, 0.2);
  --msg-user-text: #c7d2fe;
  --msg-assistant-bg: rgba(255, 255, 255, 0.06);
  --msg-assistant-border: rgba(255, 255, 255, 0.04);
  --status-active: #4ade80;
  --status-streaming: #fbbf24;
  --status-error: #f87171;
  --status-offline: #6b7280;
}

*,
*::before,
*::after {
  transition: background-color 0.3s ease, color 0.3s ease, border-color 0.3s ease;
}
```

- [ ] **Step 3: Update index.css to import tokens**

Replace `src/index.css` with:
```css
@import "tailwindcss";
@import "./design-system/tokens.css";

body {
  margin: 0;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-family: system-ui, -apple-system, sans-serif;
  overflow: hidden;
  height: 100vh;
}

#root {
  height: 100vh;
  display: flex;
}

/* Highlight.js theme override for code blocks */
.hljs {
  background: #1e1e2e !important;
  border-radius: 8px;
  padding: 12px !important;
  font-size: 13px;
}

/* Scrollbar styling */
::-webkit-scrollbar {
  width: 6px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: var(--text-secondary);
  border-radius: 3px;
  opacity: 0.3;
}
```

- [ ] **Step 4: Verify build**

```bash
npm run lint
```

Expected: passes (no type errors on CSS files).

- [ ] **Step 5: Commit**

```bash
git add src/design-system/tokens.css src/index.css package.json package-lock.json
git commit -m "feat(ui): add Liquid Glass design tokens (light+dark) and install UI deps"
```

---

## Task 2: Design System Primitives (5 components)

**Files:**
- Create: `src/design-system/GlassPanel.tsx`
- Create: `src/design-system/GlassButton.tsx`
- Create: `src/design-system/GlassInput.tsx`
- Create: `src/design-system/GlassCard.tsx`
- Create: `src/design-system/StatusDot.tsx`

- [ ] **Step 1: Create GlassPanel**

`src/design-system/GlassPanel.tsx`:
```tsx
import { ReactNode } from "react";

interface GlassPanelProps {
  children: ReactNode;
  className?: string;
  padding?: string;
}

export function GlassPanel({ children, className = "", padding = "p-4" }: GlassPanelProps) {
  return (
    <div
      className={`${padding} ${className}`}
      style={{
        background: "var(--glass-bg)",
        backdropFilter: `blur(var(--glass-blur))`,
        WebkitBackdropFilter: `blur(var(--glass-blur))`,
        border: "1px solid var(--glass-border)",
        boxShadow: "var(--glass-shadow)",
        borderRadius: "var(--radius-lg)",
      }}
    >
      {children}
    </div>
  );
}
```

- [ ] **Step 2: Create GlassButton**

`src/design-system/GlassButton.tsx`:
```tsx
import { ReactNode, ButtonHTMLAttributes } from "react";

interface GlassButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: "primary" | "secondary" | "ghost";
  icon?: ReactNode;
  children?: ReactNode;
}

const variantStyles = {
  primary: {
    background: "var(--accent)",
    color: "var(--accent-text)",
    border: "1px solid transparent",
  },
  secondary: {
    background: "var(--glass-bg)",
    color: "var(--text-primary)",
    border: "1px solid var(--glass-border)",
  },
  ghost: {
    background: "transparent",
    color: "var(--text-secondary)",
    border: "1px solid transparent",
  },
};

export function GlassButton({
  variant = "secondary",
  icon,
  children,
  className = "",
  disabled,
  ...props
}: GlassButtonProps) {
  return (
    <button
      className={`flex items-center gap-2 px-3 py-2 rounded-[var(--radius)] cursor-pointer
        hover:opacity-80 active:scale-95 transition-all duration-150
        disabled:opacity-40 disabled:cursor-not-allowed disabled:active:scale-100
        ${className}`}
      style={variantStyles[variant]}
      disabled={disabled}
      {...props}
    >
      {icon}
      {children}
    </button>
  );
}
```

- [ ] **Step 3: Create GlassInput**

`src/design-system/GlassInput.tsx`:
```tsx
import { useRef, useEffect, KeyboardEvent, ChangeEvent } from "react";

interface GlassInputProps {
  value: string;
  onChange: (value: string) => void;
  placeholder?: string;
  multiline?: boolean;
  onSubmit?: () => void;
  disabled?: boolean;
  className?: string;
}

export function GlassInput({
  value,
  onChange,
  placeholder,
  multiline = false,
  onSubmit,
  disabled = false,
  className = "",
}: GlassInputProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (multiline && textareaRef.current) {
      textareaRef.current.style.height = "auto";
      const scrollHeight = textareaRef.current.scrollHeight;
      const maxHeight = 8 * 24; // 8 lines * ~24px line-height
      textareaRef.current.style.height = `${Math.min(scrollHeight, maxHeight)}px`;
    }
  }, [value, multiline]);

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey && onSubmit) {
      e.preventDefault();
      onSubmit();
    }
  };

  const handleChange = (e: ChangeEvent<HTMLInputElement | HTMLTextAreaElement>) => {
    onChange(e.target.value);
  };

  const baseStyle = {
    background: "var(--glass-bg)",
    color: "var(--text-primary)",
    border: "1px solid var(--glass-border)",
    borderRadius: "var(--radius)",
  };

  const baseClass = `w-full px-3 py-2 outline-none placeholder:text-[var(--text-secondary)]
    focus:border-[var(--accent)] transition-colors ${className}`;

  if (multiline) {
    return (
      <textarea
        ref={textareaRef}
        value={value}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        placeholder={placeholder}
        disabled={disabled}
        rows={1}
        className={`${baseClass} resize-none overflow-y-auto`}
        style={{ ...baseStyle, maxHeight: "192px" }}
      />
    );
  }

  return (
    <input
      type="text"
      value={value}
      onChange={handleChange}
      onKeyDown={handleKeyDown}
      placeholder={placeholder}
      disabled={disabled}
      className={baseClass}
      style={baseStyle}
    />
  );
}
```

- [ ] **Step 4: Create GlassCard**

`src/design-system/GlassCard.tsx`:
```tsx
import { ReactNode } from "react";

interface GlassCardProps {
  children: ReactNode;
  active?: boolean;
  onClick?: () => void;
  className?: string;
}

export function GlassCard({ children, active = false, onClick, className = "" }: GlassCardProps) {
  return (
    <div
      onClick={onClick}
      className={`px-3 py-2 cursor-pointer transition-all duration-150
        hover:brightness-110 rounded-[var(--radius)] ${className}`}
      style={{
        background: active ? "var(--glass-bg)" : "transparent",
        border: active ? "1px solid var(--accent)" : "1px solid transparent",
      }}
    >
      {children}
    </div>
  );
}
```

- [ ] **Step 5: Create StatusDot**

`src/design-system/StatusDot.tsx`:
```tsx
import { motion } from "motion/react";

interface StatusDotProps {
  status: "idle" | "streaming" | "error" | "offline";
}

const colorMap = {
  idle: "var(--status-active)",
  streaming: "var(--status-streaming)",
  error: "var(--status-error)",
  offline: "var(--status-offline)",
};

export function StatusDot({ status }: StatusDotProps) {
  const color = colorMap[status];
  const baseStyle = {
    width: 8,
    height: 8,
    borderRadius: "50%",
    backgroundColor: color,
    boxShadow: status !== "offline" ? `0 0 6px ${color}` : "none",
    flexShrink: 0,
  };

  if (status === "streaming") {
    return (
      <motion.div
        style={baseStyle}
        animate={{ scale: [1, 1.4, 1] }}
        transition={{ duration: 1.5, repeat: Infinity, ease: "easeInOut" }}
      />
    );
  }

  return <div style={baseStyle} />;
}
```

- [ ] **Step 6: Verify build**

```bash
npm run lint
```

- [ ] **Step 7: Commit**

```bash
git add src/design-system/
git commit -m "feat(ui): add 5 Liquid Glass primitives (GlassPanel, GlassButton, GlassInput, GlassCard, StatusDot)"
```

---

## Task 3: Zustand Store

**Files:**
- Create: `src/stores/store.ts`

- [ ] **Step 1: Create the Zustand store**

`src/stores/store.ts`:
```tsx
import { create } from "zustand";
import { persist } from "zustand/middleware";
import type { SessionRow, MessageRow, ProviderInfo } from "@/lib/tauri";

interface VidaStore {
  // Sessions
  sessions: SessionRow[];
  currentSessionId: string | null;
  setSessions: (sessions: SessionRow[]) => void;
  setCurrentSession: (id: string | null) => void;
  addSession: (session: SessionRow) => void;
  removeSession: (id: string) => void;

  // Messages
  messages: Record<string, MessageRow[]>;
  streamingMessageId: string | null;
  streamingContent: string;
  setMessages: (sessionId: string, msgs: MessageRow[]) => void;
  addMessage: (sessionId: string, msg: MessageRow) => void;
  startStreaming: (messageId: string) => void;
  appendToken: (token: string) => void;
  finishStreaming: () => void;

  // Providers
  providers: ProviderInfo[];
  providerHealth: Record<string, boolean>;
  setProviders: (providers: ProviderInfo[]) => void;
  setProviderHealth: (health: Record<string, boolean>) => void;

  // UI
  theme: "light" | "dark";
  sidebarOpen: boolean;
  settingsOpen: boolean;
  setTheme: (theme: "light" | "dark") => void;
  toggleSidebar: () => void;
  setSettingsOpen: (open: boolean) => void;
}

export const useStore = create<VidaStore>()(
  persist(
    (set) => ({
      // Sessions
      sessions: [],
      currentSessionId: null,
      setSessions: (sessions) => set({ sessions }),
      setCurrentSession: (id) => set({ currentSessionId: id }),
      addSession: (session) =>
        set((s) => ({ sessions: [session, ...s.sessions], currentSessionId: session.id })),
      removeSession: (id) =>
        set((s) => ({
          sessions: s.sessions.filter((sess) => sess.id !== id),
          currentSessionId: s.currentSessionId === id ? null : s.currentSessionId,
          messages: (() => { const m = { ...s.messages }; delete m[id]; return m; })(),
        })),

      // Messages
      messages: {},
      streamingMessageId: null,
      streamingContent: "",
      setMessages: (sessionId, msgs) =>
        set((s) => ({ messages: { ...s.messages, [sessionId]: msgs } })),
      addMessage: (sessionId, msg) =>
        set((s) => ({
          messages: {
            ...s.messages,
            [sessionId]: [...(s.messages[sessionId] || []), msg],
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

      // Providers
      providers: [],
      providerHealth: {},
      setProviders: (providers) => set({ providers }),
      setProviderHealth: (health) => set({ providerHealth: health }),

      // UI
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
      partialize: (state) => ({ theme: state.theme, sidebarOpen: state.sidebarOpen }),
      onRehydrateStorage: () => (state) => {
        if (state?.theme) {
          document.documentElement.setAttribute("data-theme", state.theme);
        }
      },
    }
  )
);
```

- [ ] **Step 2: Verify build**

```bash
npm run lint
```

- [ ] **Step 3: Commit**

```bash
git add src/stores/store.ts
git commit -m "feat(ui): add Zustand store with 4 slices (sessions, messages+streaming, providers, ui)"
```

---

## Task 4: Hooks (useTheme, useProviders, useSessions, useStreamCompletion)

**Files:**
- Create: `src/hooks/useTheme.ts`
- Create: `src/hooks/useProviders.ts`
- Create: `src/hooks/useSessions.ts`
- Create: `src/hooks/useStreamCompletion.ts`

- [ ] **Step 1: Create useTheme**

`src/hooks/useTheme.ts`:
```tsx
import { useEffect } from "react";
import { useStore } from "@/stores/store";

export function useTheme() {
  const theme = useStore((s) => s.theme);
  const setTheme = useStore((s) => s.setTheme);

  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    // Only set from OS if no persisted preference
    const persisted = localStorage.getItem("vida-store");
    if (!persisted) {
      setTheme(mq.matches ? "dark" : "light");
    }
    const handler = (e: MediaQueryListEvent) => {
      if (!localStorage.getItem("vida-store")) {
        setTheme(e.matches ? "dark" : "light");
      }
    };
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, [setTheme]);

  const toggleTheme = () => setTheme(theme === "dark" ? "light" : "dark");

  return { theme, toggleTheme };
}
```

- [ ] **Step 2: Create useProviders**

`src/hooks/useProviders.ts`:
```tsx
import { useEffect, useCallback } from "react";
import { useStore } from "@/stores/store";
import { api } from "@/lib/tauri";

export function useProviders() {
  const providers = useStore((s) => s.providers);
  const health = useStore((s) => s.providerHealth);
  const setProviders = useStore((s) => s.setProviders);
  const setProviderHealth = useStore((s) => s.setProviderHealth);

  const refresh = useCallback(async () => {
    try {
      const providerList = await api.listProviders();
      setProviders(providerList);
      const healthResults = await api.healthCheck();
      const healthMap: Record<string, boolean> = {};
      for (const [name, ok] of healthResults) {
        healthMap[name] = ok;
      }
      setProviderHealth(healthMap);
    } catch (e) {
      console.error("Failed to load providers:", e);
    }
  }, [setProviders, setProviderHealth]);

  useEffect(() => {
    refresh();
    const interval = setInterval(refresh, 60_000);
    return () => clearInterval(interval);
  }, [refresh]);

  return { providers, health, refresh };
}
```

- [ ] **Step 3: Create useSessions**

`src/hooks/useSessions.ts`:
```tsx
import { useEffect, useCallback } from "react";
import { useStore } from "@/stores/store";
import { api } from "@/lib/tauri";

export function useSessions() {
  const sessions = useStore((s) => s.sessions);
  const currentSessionId = useStore((s) => s.currentSessionId);
  const setSessions = useStore((s) => s.setSessions);
  const setCurrentSession = useStore((s) => s.setCurrentSession);
  const addSession = useStore((s) => s.addSession);
  const removeSession = useStore((s) => s.removeSession);
  const setMessages = useStore((s) => s.setMessages);

  useEffect(() => {
    api.listSessions(50).then(setSessions).catch(console.error);
  }, [setSessions]);

  const selectSession = useCallback(
    async (id: string) => {
      setCurrentSession(id);
      try {
        const msgs = await api.getMessages(id);
        setMessages(id, msgs);
      } catch (e) {
        console.error("Failed to load messages:", e);
      }
    },
    [setCurrentSession, setMessages]
  );

  const createSession = useCallback(
    async (providerId: string, model: string) => {
      try {
        const session = await api.createSession(providerId, model);
        addSession(session);
        setMessages(session.id, []);
      } catch (e) {
        console.error("Failed to create session:", e);
      }
    },
    [addSession, setMessages]
  );

  const deleteSession = useCallback(
    async (id: string) => {
      try {
        await api.deleteSession(id);
        removeSession(id);
      } catch (e) {
        console.error("Failed to delete session:", e);
      }
    },
    [removeSession]
  );

  return { sessions, currentSessionId, selectSession, createSession, deleteSession };
}
```

- [ ] **Step 4: Create useStreamCompletion**

`src/hooks/useStreamCompletion.ts`:
```tsx
import { useCallback, useEffect, useRef } from "react";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { useStore } from "@/stores/store";
import { api, StreamEvent } from "@/lib/tauri";

export function useStreamCompletion() {
  const currentSessionId = useStore((s) => s.currentSessionId);
  const streamingMessageId = useStore((s) => s.streamingMessageId);
  const isStreaming = streamingMessageId !== null;

  const addMessage = useStore((s) => s.addMessage);
  const startStreaming = useStore((s) => s.startStreaming);
  const appendToken = useStore((s) => s.appendToken);
  const finishStreaming = useStore((s) => s.finishStreaming);

  const unlistenRef = useRef<UnlistenFn | null>(null);

  // Cleanup listener on unmount
  useEffect(() => {
    return () => {
      if (unlistenRef.current) {
        unlistenRef.current();
        unlistenRef.current = null;
      }
    };
  }, []);

  const sendMessage = useCallback(
    async (content: string) => {
      if (!currentSessionId || isStreaming) return;

      // Add user message optimistically
      const userMsgId = `user-${Date.now()}`;
      addMessage(currentSessionId, {
        id: userMsgId,
        session_id: currentSessionId,
        role: "user",
        content,
        token_count: null,
        created_at: new Date().toISOString(),
      });

      // Create placeholder assistant message
      const assistantMsgId = `assistant-${Date.now()}`;
      addMessage(currentSessionId, {
        id: assistantMsgId,
        session_id: currentSessionId,
        role: "assistant",
        content: "",
        token_count: null,
        created_at: new Date().toISOString(),
      });

      startStreaming(assistantMsgId);

      // Listen for stream events
      const eventName = `llm-stream-${currentSessionId}`;
      unlistenRef.current = await listen<StreamEvent>(eventName, (event) => {
        const payload = event.payload;
        if (typeof payload === "object" && "Token" in payload) {
          appendToken(payload.Token.content);
        } else if (typeof payload === "object" && "Error" in payload) {
          console.error("Stream error:", payload.Error.error);
          finishStreaming();
          if (unlistenRef.current) {
            unlistenRef.current();
            unlistenRef.current = null;
          }
        } else if (payload === "Done") {
          finishStreaming();
          if (unlistenRef.current) {
            unlistenRef.current();
            unlistenRef.current = null;
          }
        }
      });

      // Invoke the backend streaming command
      try {
        await api.streamCompletion(currentSessionId, content);
      } catch (e) {
        console.error("Failed to start streaming:", e);
        finishStreaming();
        if (unlistenRef.current) {
          unlistenRef.current();
          unlistenRef.current = null;
        }
      }
    },
    [currentSessionId, isStreaming, addMessage, startStreaming, appendToken, finishStreaming]
  );

  return { sendMessage, isStreaming };
}
```

- [ ] **Step 5: Verify build**

```bash
npm run lint
```

- [ ] **Step 6: Commit**

```bash
git add src/hooks/
git commit -m "feat(ui): add 4 hooks (useTheme, useProviders, useSessions, useStreamCompletion)"
```

---

## Task 5: Layout (AppLayout + Sidebar)

**Files:**
- Create: `src/components/layout/AppLayout.tsx`
- Create: `src/components/layout/Sidebar.tsx`

- [ ] **Step 1: Create AppLayout**

`src/components/layout/AppLayout.tsx`:
```tsx
import { ReactNode } from "react";
import { motion, AnimatePresence } from "motion/react";
import { useStore } from "@/stores/store";
import { Sidebar } from "./Sidebar";

interface AppLayoutProps {
  children: ReactNode;
}

export function AppLayout({ children }: AppLayoutProps) {
  const sidebarOpen = useStore((s) => s.sidebarOpen);

  return (
    <div className="flex h-screen w-screen overflow-hidden" style={{ background: "var(--bg-primary)" }}>
      <AnimatePresence>
        {sidebarOpen && (
          <motion.div
            initial={{ width: 0, opacity: 0 }}
            animate={{ width: "var(--sidebar-width)", opacity: 1 }}
            exit={{ width: 0, opacity: 0 }}
            transition={{ type: "spring", stiffness: 300, damping: 30 }}
            className="h-full overflow-hidden flex-shrink-0"
          >
            <Sidebar />
          </motion.div>
        )}
      </AnimatePresence>
      <main className="flex-1 h-full overflow-hidden">
        {children}
      </main>
    </div>
  );
}
```

- [ ] **Step 2: Create Sidebar**

`src/components/layout/Sidebar.tsx`:
```tsx
import { Plus, Settings } from "lucide-react";
import { GlassPanel } from "@/design-system/GlassPanel";
import { GlassButton } from "@/design-system/GlassButton";
import { SessionList } from "@/components/sidebar/SessionList";
import { AgentList } from "@/components/sidebar/AgentList";
import { useStore } from "@/stores/store";

export function Sidebar() {
  const setSettingsOpen = useStore((s) => s.setSettingsOpen);

  return (
    <GlassPanel className="h-full flex flex-col gap-2 overflow-hidden" padding="p-3">
      {/* Header */}
      <div className="flex items-center justify-between px-1">
        <span className="text-sm font-semibold" style={{ color: "var(--text-primary)" }}>
          Vida AI
        </span>
        <div className="flex gap-1">
          <GlassButton variant="ghost" icon={<Plus size={16} />} title="New Chat" />
          <GlassButton
            variant="ghost"
            icon={<Settings size={16} />}
            title="Settings"
            onClick={() => setSettingsOpen(true)}
          />
        </div>
      </div>

      {/* Sessions */}
      <div className="flex-1 overflow-y-auto min-h-0">
        <div className="px-1 py-1">
          <span className="text-xs font-medium uppercase tracking-wider" style={{ color: "var(--text-secondary)" }}>
            Sessions
          </span>
        </div>
        <SessionList />
      </div>

      {/* Agents */}
      <div className="border-t pt-2" style={{ borderColor: "var(--glass-border)" }}>
        <div className="px-1 py-1">
          <span className="text-xs font-medium uppercase tracking-wider" style={{ color: "var(--text-secondary)" }}>
            Agents
          </span>
        </div>
        <AgentList />
      </div>
    </GlassPanel>
  );
}
```

- [ ] **Step 3: Verify build**

```bash
npm run lint
```

- [ ] **Step 4: Commit**

```bash
git add src/components/layout/
git commit -m "feat(ui): add AppLayout (animated sidebar) and Sidebar (sessions+agents)"
```

---

## Task 6: Sidebar Items (SessionList, SessionItem, AgentList, AgentItem)

**Files:**
- Create: `src/components/sidebar/SessionList.tsx`
- Create: `src/components/sidebar/SessionItem.tsx`
- Create: `src/components/sidebar/AgentList.tsx`
- Create: `src/components/sidebar/AgentItem.tsx`

- [ ] **Step 1: Create SessionItem**

`src/components/sidebar/SessionItem.tsx`:
```tsx
import { Trash2 } from "lucide-react";
import { GlassCard } from "@/design-system/GlassCard";
import type { SessionRow } from "@/lib/tauri";

interface SessionItemProps {
  session: SessionRow;
  active: boolean;
  onSelect: () => void;
  onDelete: () => void;
}

export function SessionItem({ session, active, onSelect, onDelete }: SessionItemProps) {
  return (
    <GlassCard active={active} onClick={onSelect} className="group flex items-center justify-between">
      <div className="min-w-0 flex-1">
        <div className="text-sm truncate" style={{ color: "var(--text-primary)" }}>
          {session.title || `Chat ${session.model}`}
        </div>
        <div className="text-xs truncate" style={{ color: "var(--text-secondary)" }}>
          {session.model}
        </div>
      </div>
      <button
        onClick={(e) => { e.stopPropagation(); onDelete(); }}
        className="opacity-0 group-hover:opacity-60 hover:opacity-100 transition-opacity p-1"
        style={{ color: "var(--text-secondary)" }}
      >
        <Trash2 size={14} />
      </button>
    </GlassCard>
  );
}
```

- [ ] **Step 2: Create SessionList**

`src/components/sidebar/SessionList.tsx`:
```tsx
import { useSessions } from "@/hooks/useSessions";
import { SessionItem } from "./SessionItem";

export function SessionList() {
  const { sessions, currentSessionId, selectSession, deleteSession } = useSessions();

  if (sessions.length === 0) {
    return (
      <div className="px-3 py-4 text-center text-xs" style={{ color: "var(--text-secondary)" }}>
        No sessions yet
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-1">
      {sessions.map((session) => (
        <SessionItem
          key={session.id}
          session={session}
          active={session.id === currentSessionId}
          onSelect={() => selectSession(session.id)}
          onDelete={() => deleteSession(session.id)}
        />
      ))}
    </div>
  );
}
```

- [ ] **Step 3: Create AgentItem**

`src/components/sidebar/AgentItem.tsx`:
```tsx
import { StatusDot } from "@/design-system/StatusDot";
import type { ProviderInfo } from "@/lib/tauri";

interface AgentItemProps {
  provider: ProviderInfo;
  healthy: boolean;
  streaming: boolean;
}

export function AgentItem({ provider, healthy, streaming }: AgentItemProps) {
  const status = streaming ? "streaming" : healthy ? "idle" : "offline";

  return (
    <div className="flex items-center gap-2 px-3 py-1.5">
      <StatusDot status={status} />
      <span className="text-sm truncate" style={{ color: "var(--text-primary)" }}>
        {provider.name}
      </span>
      <span className="text-xs ml-auto" style={{ color: "var(--text-secondary)" }}>
        {provider.provider_type}
      </span>
    </div>
  );
}
```

- [ ] **Step 4: Create AgentList**

`src/components/sidebar/AgentList.tsx`:
```tsx
import { useProviders } from "@/hooks/useProviders";
import { useStore } from "@/stores/store";
import { AgentItem } from "./AgentItem";

export function AgentList() {
  const { providers, health } = useProviders();
  const streamingMessageId = useStore((s) => s.streamingMessageId);

  if (providers.length === 0) {
    return (
      <div className="px-3 py-2 text-xs" style={{ color: "var(--text-secondary)" }}>
        No providers configured
      </div>
    );
  }

  return (
    <div className="flex flex-col">
      {providers.map((provider) => (
        <AgentItem
          key={provider.name}
          provider={provider}
          healthy={health[provider.name] ?? false}
          streaming={streamingMessageId !== null}
        />
      ))}
    </div>
  );
}
```

- [ ] **Step 5: Verify build**

```bash
npm run lint
```

- [ ] **Step 6: Commit**

```bash
git add src/components/sidebar/
git commit -m "feat(ui): add sidebar components (SessionList, SessionItem, AgentList, AgentItem)"
```

---

## Task 7: Chat UI (ChatArea, ChatHeader, MessageList, MessageBubble, ChatInput)

**Files:**
- Create: `src/components/chat/ChatArea.tsx`
- Create: `src/components/chat/ChatHeader.tsx`
- Create: `src/components/chat/MessageList.tsx`
- Create: `src/components/chat/MessageBubble.tsx`
- Create: `src/components/chat/ChatInput.tsx`

- [ ] **Step 1: Create ChatHeader**

`src/components/chat/ChatHeader.tsx`:
```tsx
import { Menu } from "lucide-react";
import { GlassButton } from "@/design-system/GlassButton";
import { useStore } from "@/stores/store";

export function ChatHeader() {
  const currentSessionId = useStore((s) => s.currentSessionId);
  const sessions = useStore((s) => s.sessions);
  const toggleSidebar = useStore((s) => s.toggleSidebar);

  const session = sessions.find((s) => s.id === currentSessionId);

  return (
    <div className="flex items-center gap-3 px-4 py-3 border-b" style={{ borderColor: "var(--glass-border)" }}>
      <GlassButton variant="ghost" icon={<Menu size={18} />} onClick={toggleSidebar} />
      {session ? (
        <div>
          <div className="text-sm font-medium" style={{ color: "var(--text-primary)" }}>
            {session.title || `Chat with ${session.model}`}
          </div>
          <div className="text-xs" style={{ color: "var(--text-secondary)" }}>
            {session.provider_id} · {session.model}
          </div>
        </div>
      ) : (
        <div className="text-sm" style={{ color: "var(--text-secondary)" }}>
          Select or create a session
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 2: Create MessageBubble**

`src/components/chat/MessageBubble.tsx`:
```tsx
import { memo } from "react";
import ReactMarkdown from "react-markdown";
import rehypeHighlight from "rehype-highlight";
import remarkGfm from "remark-gfm";
import { Copy } from "lucide-react";
import type { MessageRow } from "@/lib/tauri";
import { useStore } from "@/stores/store";

interface MessageBubbleProps {
  message: MessageRow;
}

function MessageBubbleInner({ message }: MessageBubbleProps) {
  const streamingMessageId = useStore((s) => s.streamingMessageId);
  const streamingContent = useStore((s) => s.streamingContent);
  const isThisStreaming = message.id === streamingMessageId;
  const content = isThisStreaming ? streamingContent : message.content;

  if (message.role === "user") {
    return (
      <div className="flex justify-end mb-3">
        <div
          className="max-w-[75%] px-4 py-3 text-sm"
          style={{
            background: "var(--msg-user-bg)",
            color: "var(--msg-user-text, #fff)",
            borderRadius: "var(--radius-lg) var(--radius-lg) 4px var(--radius-lg)",
          }}
        >
          {content}
        </div>
      </div>
    );
  }

  return (
    <div className="flex justify-start mb-3">
      <div
        className="max-w-[85%] px-4 py-3 text-sm prose prose-sm max-w-none"
        style={{
          background: "var(--msg-assistant-bg)",
          border: "1px solid var(--msg-assistant-border)",
          color: "var(--text-primary)",
          borderRadius: "var(--radius-lg) var(--radius-lg) var(--radius-lg) 4px",
        }}
      >
        <ReactMarkdown
          rehypePlugins={[rehypeHighlight]}
          remarkPlugins={[remarkGfm]}
          components={{
            pre: ({ children }) => (
              <div className="relative group">
                <button
                  className="absolute top-2 right-2 opacity-0 group-hover:opacity-70 hover:opacity-100 transition-opacity p-1 rounded"
                  style={{ background: "var(--glass-bg)" }}
                  onClick={() => {
                    const code = (children as any)?.props?.children;
                    if (typeof code === "string") navigator.clipboard.writeText(code);
                  }}
                >
                  <Copy size={14} />
                </button>
                <pre className="overflow-x-auto">{children}</pre>
              </div>
            ),
          }}
        >
          {content || " "}
        </ReactMarkdown>
        {isThisStreaming && (
          <span className="inline-block w-2 h-4 ml-1 animate-pulse" style={{ background: "var(--accent)" }} />
        )}
      </div>
    </div>
  );
}

export const MessageBubble = memo(MessageBubbleInner, (prev, next) => {
  // Re-render if this is or was the streaming message, or if content changed
  const store = useStore.getState();
  if (prev.message.id === store.streamingMessageId || next.message.id === store.streamingMessageId) {
    return false; // always re-render streaming bubble
  }
  return prev.message.content === next.message.content;
});
```

- [ ] **Step 3: Create MessageList**

`src/components/chat/MessageList.tsx`:
```tsx
import { useRef, useEffect, useState } from "react";
import { useStore } from "@/stores/store";
import { MessageBubble } from "./MessageBubble";
import { GlassButton } from "@/design-system/GlassButton";
import { ArrowDown } from "lucide-react";

export function MessageList() {
  const currentSessionId = useStore((s) => s.currentSessionId);
  const messages = useStore((s) => (currentSessionId ? s.messages[currentSessionId] || [] : []));
  const streamingContent = useStore((s) => s.streamingContent);

  const containerRef = useRef<HTMLDivElement>(null);
  const [autoScroll, setAutoScroll] = useState(true);
  const [showScrollBtn, setShowScrollBtn] = useState(false);

  // Auto-scroll on new messages or streaming
  useEffect(() => {
    if (autoScroll && containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [messages.length, streamingContent, autoScroll]);

  const handleScroll = () => {
    if (!containerRef.current) return;
    const { scrollTop, scrollHeight, clientHeight } = containerRef.current;
    const atBottom = scrollHeight - scrollTop - clientHeight < 50;
    setAutoScroll(atBottom);
    setShowScrollBtn(!atBottom);
  };

  const scrollToBottom = () => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
      setAutoScroll(true);
      setShowScrollBtn(false);
    }
  };

  if (messages.length === 0) {
    return (
      <div className="flex-1 flex items-center justify-center">
        <div className="text-center">
          <div className="text-2xl mb-2">💬</div>
          <div className="text-sm" style={{ color: "var(--text-secondary)" }}>
            Send a message to begin
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="flex-1 relative overflow-hidden">
      <div
        ref={containerRef}
        onScroll={handleScroll}
        className="h-full overflow-y-auto px-4 py-4"
      >
        {messages.map((msg) => (
          <MessageBubble key={msg.id} message={msg} />
        ))}
      </div>
      {showScrollBtn && (
        <div className="absolute bottom-2 left-1/2 -translate-x-1/2">
          <GlassButton variant="secondary" icon={<ArrowDown size={16} />} onClick={scrollToBottom}>
            ↓
          </GlassButton>
        </div>
      )}
    </div>
  );
}
```

- [ ] **Step 4: Create ChatInput**

`src/components/chat/ChatInput.tsx`:
```tsx
import { useState } from "react";
import { Send, Square } from "lucide-react";
import { GlassInput } from "@/design-system/GlassInput";
import { GlassButton } from "@/design-system/GlassButton";
import { useStreamCompletion } from "@/hooks/useStreamCompletion";
import { useTranslation } from "react-i18next";

export function ChatInput() {
  const [input, setInput] = useState("");
  const { sendMessage, isStreaming } = useStreamCompletion();
  const { t } = useTranslation();

  const handleSend = () => {
    const trimmed = input.trim();
    if (!trimmed || isStreaming) return;
    sendMessage(trimmed);
    setInput("");
  };

  return (
    <div className="flex items-end gap-2 px-4 py-3 border-t" style={{ borderColor: "var(--glass-border)" }}>
      <div className="flex-1">
        <GlassInput
          value={input}
          onChange={setInput}
          placeholder={t("chat.placeholder")}
          multiline
          onSubmit={handleSend}
          disabled={isStreaming}
        />
      </div>
      {isStreaming ? (
        <GlassButton variant="secondary" icon={<Square size={18} />} title="Stop" />
      ) : (
        <GlassButton
          variant="primary"
          icon={<Send size={18} />}
          onClick={handleSend}
          disabled={!input.trim()}
          title={t("chat.send")}
        />
      )}
    </div>
  );
}
```

- [ ] **Step 5: Create ChatArea**

`src/components/chat/ChatArea.tsx`:
```tsx
import { GlassPanel } from "@/design-system/GlassPanel";
import { useStore } from "@/stores/store";
import { ChatHeader } from "./ChatHeader";
import { MessageList } from "./MessageList";
import { ChatInput } from "./ChatInput";

export function ChatArea() {
  const currentSessionId = useStore((s) => s.currentSessionId);

  if (!currentSessionId) {
    return (
      <div className="h-full flex items-center justify-center">
        <div className="text-center">
          <div className="text-4xl mb-4">🤖</div>
          <div className="text-xl font-semibold mb-2" style={{ color: "var(--text-primary)" }}>
            Welcome to Vida AI
          </div>
          <div className="text-sm" style={{ color: "var(--text-secondary)" }}>
            Create a new chat to get started
          </div>
        </div>
      </div>
    );
  }

  return (
    <GlassPanel className="h-full flex flex-col overflow-hidden" padding="p-0">
      <ChatHeader />
      <MessageList />
      <ChatInput />
    </GlassPanel>
  );
}
```

- [ ] **Step 6: Verify build**

```bash
npm run lint
```

- [ ] **Step 7: Commit**

```bash
git add src/components/chat/
git commit -m "feat(ui): add chat components (ChatArea, ChatHeader, MessageList, MessageBubble, ChatInput)"
```

---

## Task 8: Wire App + i18n init + Final verification

**Files:**
- Modify: `src/main.tsx`
- Modify: `src/App.tsx`

- [ ] **Step 1: Update main.tsx with i18n initialization**

Replace `src/main.tsx` with:
```tsx
import React from "react";
import ReactDOM from "react-dom/client";
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import App from "./App";
import "./index.css";

import enCommon from "./locales/en/common.json";
import zhCommon from "./locales/zh-CN/common.json";
import frCommon from "./locales/fr/common.json";

i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources: {
      en: { translation: enCommon },
      "zh-CN": { translation: zhCommon },
      fr: { translation: frCommon },
    },
    fallbackLng: "en",
    interpolation: { escapeValue: false },
  });

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
```

- [ ] **Step 2: Update App.tsx**

Replace `src/App.tsx` with:
```tsx
import { AppLayout } from "@/components/layout/AppLayout";
import { ChatArea } from "@/components/chat/ChatArea";
import { useTheme } from "@/hooks/useTheme";

export default function App() {
  // Initialize theme detection on mount
  useTheme();

  return (
    <AppLayout>
      <ChatArea />
    </AppLayout>
  );
}
```

- [ ] **Step 3: Verify TypeScript compiles**

```bash
npm run lint
```

Expected: passes with no errors.

- [ ] **Step 4: Verify Vite builds**

```bash
npm run build
```

Expected: builds successfully to `dist/`.

- [ ] **Step 5: Commit**

```bash
git add src/main.tsx src/App.tsx
git commit -m "feat(ui): wire App with i18n, theme, AppLayout + ChatArea — Phase 2A complete"
```

- [ ] **Step 6: Verify full Rust workspace still compiles**

```bash
cargo check --workspace
```

Expected: compiles OK.
