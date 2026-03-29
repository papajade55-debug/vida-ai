# Vida AI — Phase 3 Design Spec: Team/Multi-Agent

**Date:** 2026-03-29
**Status:** Approved
**Scope:** Phase 3 — Team creation, parallel multi-agent streaming, sidebar agent animations
**Depends on:** Phase 1 (Rust Core), Phase 2A (UI/Chat)

## 1. Overview

Phase 3 adds multi-agent "Teams" to Vida AI. Users create teams by selecting models via checkboxes, then chat sessions can use a team instead of a single model. When a message is sent to a team, it's dispatched to ALL agents in parallel — each agent streams its response independently, displayed in the chat with agent name and color.

### Design Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Orchestration mode | Parallel (all respond) | Simplest, most useful (compare responses), no orchestrator needed |
| Future modes | Configurable (prepared) | DB schema supports role field for future orchestrator/round-robin |
| Stream identification | Single event name + agent_id in payload | Simpler than N event channels, avoids listener proliferation |
| UI display | Stacked bubbles with agent header | More readable than split view, scales to N agents |
| New crate | No — extend vida-core | Team logic is orchestration, belongs in vida-core |

## 2. Database Changes

### 2.1 New Tables

```sql
-- Migration 002_teams.sql

CREATE TABLE IF NOT EXISTS teams (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    mode       TEXT NOT NULL DEFAULT 'parallel',  -- 'parallel' | 'orchestrator' | 'roundtable'
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS team_members (
    id          TEXT PRIMARY KEY,
    team_id     TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    model       TEXT NOT NULL,
    display_name TEXT,                             -- optional custom name
    color       TEXT NOT NULL DEFAULT '#6366f1',   -- hex color for chat bubble
    role        TEXT,                              -- future: 'lead', 'reviewer', etc.
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
    FOREIGN KEY (provider_id) REFERENCES provider_configs(id)
);

CREATE INDEX IF NOT EXISTS idx_team_members_team ON team_members(team_id);
```

### 2.2 Modified Tables

```sql
-- Add to sessions table
ALTER TABLE sessions ADD COLUMN team_id TEXT NULL REFERENCES teams(id);

-- Add to messages table
ALTER TABLE messages ADD COLUMN agent_id TEXT NULL;
ALTER TABLE messages ADD COLUMN agent_name TEXT NULL;
ALTER TABLE messages ADD COLUMN agent_color TEXT NULL;
```

## 3. Backend Changes (vida-core + vida-db)

### 3.1 New Models (vida-db)

```rust
pub struct TeamRow {
    pub id: String,
    pub name: String,
    pub mode: String,          // "parallel"
    pub created_at: String,
}

pub struct TeamMemberRow {
    pub id: String,
    pub team_id: String,
    pub provider_id: String,
    pub model: String,
    pub display_name: Option<String>,
    pub color: String,
    pub role: Option<String>,
    pub created_at: String,
}
```

### 3.2 New Repository Methods (vida-db)

```rust
// Teams
pub async fn create_team(&self, team: &TeamRow) -> Result<(), DbError>;
pub async fn list_teams(&self) -> Result<Vec<TeamRow>, DbError>;
pub async fn get_team(&self, id: &str) -> Result<Option<TeamRow>, DbError>;
pub async fn delete_team(&self, id: &str) -> Result<(), DbError>;

// Team Members
pub async fn add_team_member(&self, member: &TeamMemberRow) -> Result<(), DbError>;
pub async fn get_team_members(&self, team_id: &str) -> Result<Vec<TeamMemberRow>, DbError>;
pub async fn remove_team_member(&self, id: &str) -> Result<(), DbError>;
```

### 3.3 New MessageRow Fields

The existing `MessageRow` gains 3 optional fields: `agent_id`, `agent_name`, `agent_color`. These are `NULL` for solo sessions and populated for team sessions.

### 3.4 VidaEngine — Team Methods

```rust
// Team management
pub async fn create_team(&self, name: &str, members: Vec<(String, String)>) -> Result<TeamRow, VidaError>;
pub async fn list_teams(&self) -> Result<Vec<TeamRow>, VidaError>;
pub async fn get_team_with_members(&self, team_id: &str) -> Result<(TeamRow, Vec<TeamMemberRow>), VidaError>;
pub async fn delete_team(&self, id: &str) -> Result<(), VidaError>;

// Team session
pub async fn create_team_session(&self, team_id: &str) -> Result<SessionRow, VidaError>;

// Parallel streaming — sends message to ALL team members
pub async fn send_team_message_stream(
    &self,
    session_id: &str,
    content: &str,
    tx: mpsc::Sender<TeamStreamEvent>,
) -> Result<(), VidaError>;
```

### 3.5 TeamStreamEvent

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TeamStreamEvent {
    AgentToken {
        agent_id: String,
        agent_name: String,
        agent_color: String,
        content: String,
    },
    AgentDone {
        agent_id: String,
    },
    AgentError {
        agent_id: String,
        error: String,
    },
    AllDone,
}
```

### 3.6 Parallel Dispatch (send_team_message_stream)

1. Get session → get team_id → get team members
2. Insert user message into DB
3. For EACH team member, spawn a tokio task:
   - Create a per-agent mpsc channel
   - Call `provider.chat_completion_stream(messages, options, agent_tx)`
   - Forward agent tokens as `TeamStreamEvent::AgentToken` to the main tx
   - On done: send `TeamStreamEvent::AgentDone`
4. Track completion count — when all agents done, send `TeamStreamEvent::AllDone`
5. Save each agent's complete response as a separate message in DB (with agent_id, agent_name, agent_color)

## 4. Tauri Commands (new)

| Command | Type | Delegates to |
|---|---|---|
| `create_team` | Command | engine.create_team() |
| `list_teams` | Command | engine.list_teams() |
| `get_team` | Command | engine.get_team_with_members() |
| `delete_team` | Command | engine.delete_team() |
| `create_team_session` | Command | engine.create_team_session() |
| `stream_team_completion` | Command + Event | engine.send_team_message_stream() → emit("team-stream-{session_id}") |

## 5. Frontend Changes

### 5.1 New Zustand Slices

```typescript
// Add to store.ts
teams: TeamRow[];
setTeams: (teams: TeamRow[]) => void;
addTeam: (team: TeamRow) => void;
removeTeam: (id: string) => void;

// Streaming per agent
agentStreaming: Record<string, string>;  // agent_id → accumulated content
startTeamStreaming: (agentIds: string[]) => void;
appendAgentToken: (agentId: string, token: string) => void;
finishAgentStreaming: (agentId: string) => void;
finishAllStreaming: () => void;
```

### 5.2 New Components

```
src/components/teams/
├── TeamCreator.tsx      # Modal: provider/model checkboxes, team name, Create button
├── TeamList.tsx         # List of teams in sidebar (below sessions, above agents)
├── TeamItem.tsx         # One team card with member avatars
└── TeamMemberBadge.tsx  # Colored dot + model name
```

### 5.3 Modified Components

**Sidebar.tsx** — Add "Teams" section between Sessions and Agents. "+" button opens TeamCreator modal.

**ChatArea.tsx** — Detect if current session has team_id. If yes, use `useTeamStreamCompletion` instead of `useStreamCompletion`.

**MessageBubble.tsx** — If message has `agent_name`, show an agent header bar above the content: colored dot + agent name + model. Different subtle background per agent (using agent_color with low opacity).

**New Chat modal** — Two tabs: "Single Model" (existing) and "Team" (lists teams, click to create team session).

### 5.4 New Hook: useTeamStreamCompletion

Similar to `useStreamCompletion` but:
- Listens to `team-stream-{session_id}` events
- Dispatches `TeamStreamEvent` to per-agent streaming state in store
- Creates N placeholder assistant messages (one per agent)
- Each agent's tokens update its own bubble independently

### 5.5 Agent Color Palette

Pre-defined palette for auto-assignment (max 8 distinct colors):
```
#6366f1 (indigo), #ec4899 (pink), #14b8a6 (teal), #f59e0b (amber),
#8b5cf6 (violet), #06b6d4 (cyan), #f97316 (orange), #10b981 (emerald)
```

## 6. UI Flows

### 6.1 Create Team

1. User clicks "+" in Teams section of sidebar
2. `TeamCreator` modal opens
3. Left column: list of available providers/models (from `useProviders`)
4. Each model has a **checkbox** — user checks the ones they want
5. Right column: shows selected members with color preview
6. User enters a team name
7. Click "Create Team" → `api.createTeam(name, selectedMembers)`

### 6.2 Start Team Chat

1. User clicks a team in sidebar → New Chat modal (Team tab)
2. Or: "+" New Chat → Team tab → select team → Create
3. Session created with `team_id` → chat area shows team header (team name + member badges)

### 6.3 Team Message Flow

1. User types message, presses Send
2. `useTeamStreamCompletion.sendMessage(content)` called
3. N placeholder assistant bubbles appear (one per agent), each with agent name/color header
4. Each bubble streams independently (parallel)
5. StatusDot in sidebar shows "streaming" for active agents
6. When all agents finish → AllDone

## 7. Out of Scope

- Orchestrator mode (agent decides who responds) — future
- Round-table mode (agents take turns) — future
- Agent-to-agent communication — future
- Team editing (add/remove members after creation) — future (delete + recreate for now)
