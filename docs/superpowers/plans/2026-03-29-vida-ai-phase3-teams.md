# Vida AI Phase 3 — Teams/Multi-Agent Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add multi-agent "Teams" — create teams via checkboxes, parallel streaming from all agents, agent-identified chat bubbles.

**Architecture:** New DB migration (teams + team_members), extended vida-db models + repository, extended vida-core engine with parallel team streaming via tokio::spawn, new Tauri commands, new frontend components (TeamCreator, TeamList, useTeamStreamCompletion).

**Tech Stack:** Rust (sqlx, tokio), React 19, TypeScript, Zustand, Framer Motion, Tauri v2.

**Spec:** `docs/superpowers/specs/2026-03-29-vida-ai-phase3-teams-design.md`

---

## Task 1: DB Migration + Models

**Files:**
- Create: `crates/vida-db/migrations/002_teams.sql`
- Modify: `crates/vida-db/src/models.rs`
- Modify: `crates/vida-db/src/repository.rs`

Read the spec §2 for the exact schema. Implement:
1. Migration: CREATE TABLE teams, team_members. ALTER sessions ADD team_id. ALTER messages ADD agent_id, agent_name, agent_color.
2. Models: TeamRow, TeamMemberRow structs (String UUIDs, not i64).
3. Repository: CRUD for teams and team_members.
4. Tests: create_team, add_members, get_team_with_members, delete_team_cascades.
5. Commit.

## Task 2: TeamStreamEvent + VidaEngine team methods

**Files:**
- Modify: `crates/vida-core/src/engine.rs`
- Modify: `crates/vida-core/src/lib.rs`

Implement:
1. TeamStreamEvent enum (AgentToken, AgentDone, AgentError, AllDone).
2. VidaEngine methods: create_team, list_teams, get_team_with_members, delete_team, create_team_session, send_team_message_stream.
3. send_team_message_stream: for each team member, spawn a tokio task that calls provider.chat_completion_stream and forwards tokens as TeamStreamEvent.
4. Tests with MockProvider.
5. Commit.

## Task 3: Tauri Commands for Teams

**Files:**
- Create: `src-tauri/src/commands/teams.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/main.rs`

Implement 6 new commands: create_team, list_teams, get_team, delete_team, create_team_session, stream_team_completion.
Commit.

## Task 4: Frontend — Store + Hook + Types

**Files:**
- Modify: `src/stores/store.ts` (add teams slice + agentStreaming)
- Modify: `src/lib/tauri.ts` (add team types + API wrappers)
- Create: `src/hooks/useTeams.ts`
- Create: `src/hooks/useTeamStreamCompletion.ts`

Commit.

## Task 5: Frontend — Team Components

**Files:**
- Create: `src/components/teams/TeamCreator.tsx` (modal with provider/model checkboxes)
- Create: `src/components/teams/TeamList.tsx`
- Create: `src/components/teams/TeamItem.tsx`
- Create: `src/components/teams/TeamMemberBadge.tsx`

Commit.

## Task 6: Frontend — Wire Teams into existing UI

**Files:**
- Modify: `src/components/layout/Sidebar.tsx` (add Teams section)
- Modify: `src/components/chat/ChatArea.tsx` (detect team session)
- Modify: `src/components/chat/MessageBubble.tsx` (show agent header)
- Modify: `src/components/chat/ChatInput.tsx` (use team stream when team session)

Commit.

## Task 7: Final verification

Run: cargo test --workspace, npm run lint, npm run build, cargo check --workspace.
