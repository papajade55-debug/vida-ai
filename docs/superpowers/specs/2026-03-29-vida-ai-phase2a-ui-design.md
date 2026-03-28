# Vida AI — Phase 2A Design Spec: UI / Design System / Chat

**Date:** 2026-03-29
**Status:** Approved
**Scope:** Phase 2A — Design System (Liquid Glass) + Chat Interface + Streaming UI
**Depends on:** Phase 1 (Rust Core — complete)

## 1. Overview

Phase 2A builds the frontend for Vida AI: a Liquid Glass design system with adaptive light/dark theming, a chat interface with real-time token streaming from the Tauri backend, and a unified sidebar showing sessions and agent status.

### Design Decisions

| Decision | Choice | Rationale |
|---|---|---|
| State management | Zustand | Granular subscriptions — streaming re-renders only the active bubble |
| Layout | Unified sidebar (sessions + agents) | Compact, prepares for Phase 3 teams, no icon rail overhead |
| Visual style | Adaptive Glass (light + dark) | Pro in daylight, immersive at night, auto-detects OS preference |
| Markdown | react-markdown + rehype-highlight | Extensible remark/rehype pipeline, standard React ecosystem |
| Animations | Framer Motion (already installed) | Spring physics for sidebar, pulsation for agent status |

## 2. Component Architecture

### 2.1 File Structure

```
src/
├── main.tsx                    # Entry point, i18n init, theme init
├── App.tsx                     # Auth gate → AppLayout
├── index.css                   # Tailwind + imports tokens.css
│
├── design-system/              # Liquid Glass primitives
│   ├── GlassPanel.tsx          # Container: backdrop-blur + border + shadow
│   ├── GlassButton.tsx         # Button: primary | secondary | ghost
│   ├── GlassInput.tsx          # Input/textarea: auto-resize, onSubmit
│   ├── GlassCard.tsx           # Clickable card: active state, hover
│   ├── StatusDot.tsx           # Status indicator: idle/streaming/error + pulse
│   └── tokens.css              # CSS custom properties (light + dark)
│
├── components/
│   ├── layout/
│   │   ├── AppLayout.tsx       # CSS Grid: sidebar + chat area
│   │   └── Sidebar.tsx         # Unified: sessions top + agents bottom
│   │
│   ├── sidebar/
│   │   ├── SessionList.tsx     # Scrollable session list
│   │   ├── SessionItem.tsx     # One session: title, provider, date
│   │   ├── AgentList.tsx       # Provider/agent list with status
│   │   └── AgentItem.tsx       # One agent: name, model, StatusDot
│   │
│   ├── chat/
│   │   ├── ChatArea.tsx        # Container: header + messages + input
│   │   ├── ChatHeader.tsx      # Session name, provider, model info
│   │   ├── MessageList.tsx     # Scroll container, auto-scroll logic
│   │   ├── MessageBubble.tsx   # One message: user/assistant/streaming/error
│   │   ├── ChatInput.tsx       # Textarea + send/stop button
│   │   └── StreamingDot.tsx    # Animated "typing..." indicator
│   │
│   ├── settings/
│   │   ├── SettingsModal.tsx   # Modal: language, theme, password
│   │   └── ApiKeyForm.tsx      # Add/edit provider API key
│   │
│   └── ThemeToggle.tsx         # Light/dark toggle button
│
├── hooks/
│   ├── useStreamCompletion.ts  # Listen Tauri Events → Zustand
│   ├── useSessions.ts          # CRUD sessions via Tauri Commands
│   ├── useProviders.ts         # List/health providers
│   └── useTheme.ts             # OS detection + toggle + persist
│
├── stores/
│   └── store.ts                # Single Zustand store, 4 slices
│
├── lib/
│   └── tauri.ts                # Typed invoke/listen wrappers (Phase 1)
│
└── locales/                    # i18n (Phase 1)
    ├── en/common.json
    ├── zh-CN/common.json
    └── fr/common.json
```

### 2.2 Component Dependency Graph

```
App
└── AppLayout
    ├── Sidebar (GlassPanel)
    │   ├── SessionList
    │   │   └── SessionItem (GlassCard)
    │   ├── AgentList
    │   │   └── AgentItem (GlassCard + StatusDot)
    │   └── ThemeToggle (GlassButton)
    │
    └── ChatArea (GlassPanel)
        ├── ChatHeader
        ├── MessageList
        │   └── MessageBubble (react-markdown + rehype-highlight)
        │       └── StreamingDot (when streaming)
        ├── ChatInput (GlassInput + GlassButton)
        └── SettingsModal (on demand)
            └── ApiKeyForm (GlassInput + GlassButton)
```

## 3. Design System — Liquid Glass

### 3.1 CSS Tokens

Defined in `design-system/tokens.css`, switched via `data-theme` attribute on `<html>`.

**Light theme (`:root`):**

| Token | Value | Purpose |
|---|---|---|
| `--glass-bg` | `rgba(255,255,255,0.7)` | Panel background |
| `--glass-blur` | `16px` | Backdrop blur amount |
| `--glass-border` | `rgba(0,0,0,0.06)` | Panel border color |
| `--glass-shadow` | `0 2px 12px rgba(0,0,0,0.04)` | Panel shadow |
| `--bg-primary` | `#f0f0f5` | App background |
| `--text-primary` | `#1a1a2e` | Primary text |
| `--text-secondary` | `#6b7280` | Secondary text |
| `--accent` | `#4f46e5` | Accent / interactive |
| `--accent-hover` | `#4338ca` | Accent hover |
| `--msg-user-bg` | `var(--accent)` | User message bubble |
| `--msg-assistant-bg` | `rgba(0,0,0,0.03)` | Assistant message bubble |
| `--status-active` | `#22c55e` | Agent idle/healthy |
| `--status-streaming` | `#f59e0b` | Agent streaming |
| `--status-error` | `#ef4444` | Agent error |
| `--radius` | `12px` | Default border-radius |
| `--radius-lg` | `16px` | Large border-radius |

**Dark theme (`[data-theme="dark"]`):**

| Token | Value | Purpose |
|---|---|---|
| `--glass-bg` | `rgba(255,255,255,0.05)` | Panel background |
| `--glass-blur` | `20px` | Backdrop blur (stronger in dark) |
| `--glass-border` | `rgba(255,255,255,0.08)` | Panel border color |
| `--glass-shadow` | `0 4px 24px rgba(0,0,0,0.2)` | Panel shadow |
| `--bg-primary` | `#0f0c29` | App background |
| `--text-primary` | `#e0e0e8` | Primary text |
| `--text-secondary` | `#8b949e` | Secondary text |
| `--accent` | `#818cf8` | Accent (lighter for dark bg) |
| `--accent-hover` | `#6366f1` | Accent hover |
| `--msg-user-bg` | `rgba(99,102,241,0.2)` | User message bubble |
| `--msg-assistant-bg` | `rgba(255,255,255,0.06)` | Assistant message bubble |
| `--status-active` | `#4ade80` | Agent idle/healthy |
| `--status-streaming` | `#fbbf24` | Agent streaming |
| `--status-error` | `#f87171` | Agent error |

### 3.2 Primitive Components

**GlassPanel** — Structural container. Applies `background: var(--glass-bg)`, `backdrop-filter: blur(var(--glass-blur))`, `border: 1px solid var(--glass-border)`, `box-shadow: var(--glass-shadow)`, `border-radius: var(--radius-lg)`. Props: `children`, `className?`, `padding?` (default `16px`).

**GlassButton** — Three variants:
- `primary`: `background: var(--accent)`, white text, hover darken
- `secondary`: `background: var(--glass-bg)`, text color, glass border
- `ghost`: transparent, text color, hover shows glass-bg
Props: `children`, `variant`, `onClick`, `disabled?`, `icon?` (lucide-react icon component).

**GlassInput** — Input/textarea hybrid. `multiline` prop switches between `<input>` and `<textarea>` with auto-resize (1 to 8 lines, then scroll). Glass background, border on focus = accent. Props: `value`, `onChange`, `placeholder?`, `multiline?`, `onSubmit?` (Enter handler).

**GlassCard** — Clickable card for sidebar items. Default: glass-bg background. Active state: accent border + subtle accent background. Hover: slight brightness increase. Props: `children`, `active?`, `onClick?`, `className?`.

**StatusDot** — 6-8px circle. Colors from tokens (`--status-active`, `--status-streaming`, `--status-error`). Streaming state triggers a Framer Motion pulse animation (`scale: [1, 1.4, 1]`, infinite, 1.5s duration) + glow via `box-shadow: 0 0 8px var(--status-streaming)`. Props: `status: "idle" | "streaming" | "error" | "offline"`.

### 3.3 Theme Switching

1. Boot: detect OS with `window.matchMedia("(prefers-color-scheme: dark)")`
2. Check Zustand persisted preference (overrides OS if set)
3. Apply: `document.documentElement.setAttribute("data-theme", theme)`
4. Toggle: `ThemeToggle` calls `store.setTheme()` which updates attribute + persists
5. Transition: `* { transition: background 0.3s, color 0.3s, border-color 0.3s }` for smooth switch

## 4. Zustand Store

### 4.1 State Shape

```typescript
interface VidaStore {
  // Sessions
  sessions: SessionRow[];
  currentSessionId: string | null;

  // Messages
  messages: Record<string, MessageRow[]>;  // keyed by session_id
  streamingMessageId: string | null;
  streamingContent: string;

  // Providers
  providers: ProviderInfo[];
  providerHealth: Record<string, boolean>;

  // UI
  theme: "light" | "dark";
  sidebarOpen: boolean;
  settingsOpen: boolean;
}
```

### 4.2 Actions

**Sessions:** `setSessions`, `setCurrentSession`, `addSession`, `removeSession`

**Messages:** `setMessages(sessionId, msgs)`, `addMessage(sessionId, msg)`

**Streaming (hot path):**
- `startStreaming(messageId)` — sets `streamingMessageId`, clears `streamingContent`
- `appendToken(token)` — concatenates to `streamingContent` (simple string append)
- `finishStreaming()` — flushes `streamingContent` into `messages[sessionId]`, clears streaming state

**Why `streamingContent` is separate:** If each token updated `messages[sessionId][last].content`, Zustand would create a new array reference → `MessageList` would re-render ALL messages at 50+ tokens/sec. With a separate `streamingContent` string, only the streaming `MessageBubble` subscribes to it. Other bubbles and the sidebar don't re-render.

**Providers:** `setProviders`, `setProviderHealth`

**UI:** `setTheme`, `toggleSidebar`, `setSettingsOpen`

### 4.3 Persist

Zustand `persist` middleware with `name: "vida-store"`. Persists: `theme`, `sidebarOpen`. Does NOT persist `sessions` or `messages` (loaded from Tauri backend on boot).

## 5. Hooks

### 5.1 useStreamCompletion

Manages the full streaming lifecycle:

1. User calls `sendMessage(content)` from ChatInput
2. Hook creates a placeholder assistant MessageRow with empty content
3. Calls `store.startStreaming(msgId)`
4. Calls `store.addMessage(sessionId, userMsg)` (optimistic)
5. Invokes `api.streamCompletion(sessionId, content)`
6. Listens to `llm-stream-{sessionId}` Tauri Events
7. On `Token`: calls `store.appendToken(token)`
8. On `Done`: calls `store.finishStreaming()`
9. On `Error`: calls `store.finishStreaming()` + shows error
10. Cleanup: unlistens on unmount or session change

Returns: `{ sendMessage: (content: string) => void, isStreaming: boolean }`

### 5.2 useSessions

Boot: calls `api.listSessions(50)` → `store.setSessions()`.
On session select: calls `api.getMessages(id)` → `store.setMessages(id, msgs)`.
Create: calls `api.createSession(providerId, model)` → `store.addSession()`.
Delete: calls `api.deleteSession(id)` → `store.removeSession()`.

Returns: `{ sessions, currentSessionId, createSession, deleteSession, selectSession }`

### 5.3 useProviders

Boot: calls `api.listProviders()` → `store.setProviders()`, then `api.healthCheck()` → `store.setProviderHealth()`.
Periodic: re-check health every 60 seconds.

Returns: `{ providers, health, refreshProviders }`

### 5.4 useTheme

Boot: detect OS preference, check persisted value.
Toggle: switch and persist.

Returns: `{ theme, toggleTheme }`

## 6. Chat UI Behavior

### 6.1 MessageBubble Rendering

| Type | Alignment | Background | Content Rendering |
|---|---|---|---|
| User | Right | `var(--msg-user-bg)` | Plain text (no markdown) |
| Assistant | Left | `var(--msg-assistant-bg)` | react-markdown + rehype-highlight + remark-gfm |
| Streaming | Left | `var(--msg-assistant-bg)` | Same as assistant, source = `store.streamingContent`, `StreamingDot` appended |
| Error | Left | Transparent red | Error icon + message + Retry button |

Code blocks: dark background (always), syntax highlighting, "Copy" button on hover.

### 6.2 ChatInput

- Textarea with auto-resize: 1 line default, grows to 8 lines max, then scrolls
- Enter = send, Shift+Enter = newline
- Send button (lucide `Send` icon) — disabled when empty or streaming
- During streaming: Send button becomes Stop button (lucide `Square` icon)
- Placeholder: `t("chat.placeholder")` (i18n)

### 6.3 Auto-scroll

- `MessageList` auto-scrolls to bottom on new message or during streaming
- If user scrolls up manually: auto-scroll disables, "↓ Scroll to bottom" button appears
- User clicks "↓" or sends new message → re-enables auto-scroll

### 6.4 New Chat Flow

1. User clicks "+" button in sidebar header
2. Inline dropdown appears: provider selector → model selector
3. Calls `useSessions.createSession(providerId, model)`
4. New session added to sidebar, becomes active, chat area shows empty state

### 6.5 Empty State

No sessions: centered "Welcome to Vida AI" + Vida logo + "Start a new chat" button.
No messages in session: centered "Send a message to begin" hint.

### 6.6 Loading State

On boot: skeleton shimmer animation on sidebar (sessions) and chat area while data loads from backend.

## 7. NPM Dependencies (new for Phase 2A)

| Package | Version | Purpose |
|---|---|---|
| zustand | ^4.5 | State management |
| react-markdown | ^9 | Markdown rendering |
| rehype-highlight | ^7 | Syntax highlighting |
| remark-gfm | ^4 | GitHub Flavored Markdown |
| highlight.js | ^11 | Language grammars |

Already installed (Phase 1): react-i18next, i18next, i18next-browser-languagedetector, @tauri-apps/api, lucide-react, motion (Framer Motion), tailwindcss, @tailwindcss/vite.

## 8. Out of Scope (Phase 2B+)

- File drag & drop (Phase 2B)
- Vision UI — image upload + preview (Phase 2B)
- Voice chat / Whisper.cpp (Phase 2B)
- Additional providers: Anthropic, Google (Phase 2B)
- Virtual scrolling for very long conversations (optimization, later)
- Keyboard shortcuts beyond Enter/Shift+Enter (later)
