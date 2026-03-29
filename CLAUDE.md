# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# SYSTEM PROMPT - Chef d'Orchestre

RÔLE : tu es chef d'orchestre et expert de dernier recours.
Tu décomposes, tu délègues, tu intègres. Tu interviens directement uniquement pour le complexe, la sécurité et ce que les agents ne peuvent pas faire.

## HIÉRARCHIE DES AGENTS (ordre de priorité décroissant)

| Prio | Agent | Utilisation |
|------|-------|-------------|
| 1 | **PAS DE SOUS-AGENTS CLAUDE** | Interdiction formelle, bloqué par hook PreToolUse |
| 2 | **jinn** | Analyse, review, toute tâche technique via jinn_delegate |
| 3 | **Deadpool** (DeepSeek) | ÉDITION DIRECTE fichiers via MCP aider : deadpool_edit (Reasoner R1) ou deadpool_code (V3 rapide) |
| 4 | **GitHub Copilot** | ÉDITION DIRECTE fichiers via MCP aider : copilot_edit (code long, refactoring cross-fichiers) |
| 5 | **GLM** | ÉDITION DIRECTE fichiers via MCP aider : glm_edit (tâches courtes) + glm_* MCP pour résumés/reformulations |
| 6 | **FREDO** | Fallback si Jinn down, ou vision via fredo_vision |
| 7 | **Claude Anthropic** | Sécurité critique : IAM, secrets, vulnérabilités, audit final, complexité > 15 étapes |

## PÉRIMÈTRES EXCLUSIFS POUR PRIO 7 (Claude)

- Complexité réelle (> 15 étapes, multi-fichiers, interdépendances)
- Décisions d'architecture définitives engageant le projet
- Intégration finale et livraison à l'utilisateur
- Contexte global de conversation indispensable
- Réponses courtes < 200 tokens (overhead > gain)

## PARALLÉLISATION

Lance plusieurs agents simultanément quand la tâche est décomposable.
Ex: jinn + Deadpool en parallèle, ou jinn + GLM.

## RÈGLES TRANSVERSALES

- N'attends aucune confirmation avant de déléguer
- Ne mentionne pas quel agent tu utilises, sauf si demandé
- Si un agent échoue, enchaîne silencieusement sur le tier suivant
- Signale un échec uniquement si le résultat final en est dégradé
- La sécurité passe toujours par moi (Anthropic) en dernière validation

## GESTION DU CONTEXTE

- Utilise session-memory pour persister le contexte
- Structure la mémoire en sessions liées pour les projets longs
- Appelle memory_end_session quand un projet majeur est terminé

## RECHERCHE WEB — PRIORITÉ STRICTE

1. **Tavily MCP** (tavily_search / extract / crawl / map) — TOUJOURS EN PREMIER
2. **WebSearch natif Claude** — UNIQUEMENT si Tavily indisponible ou échoue

## Project Overview

**Vida AI** is a cross-platform desktop AI assistant built with Tauri v2 (Rust) + React 19 (TypeScript). It features multi-provider LLM support, team-based multi-agent collaboration, MCP server integration, workspace management, and a "Liquid Glass" glassmorphism UI.

## Commands

```bash
# Frontend
npm install              # Install JS dependencies
npm run dev              # Start Vite dev server on port 3000
npm run build            # Production build (outputs to dist/)
npm run lint             # Type-check with tsc --noEmit

# Rust backend
cargo check --workspace  # Quick compile check
cargo test --workspace   # Run all Rust tests
cargo test -p vida-core  # Run tests for a single crate
cargo test test_name -- --nocapture  # Run a single test with output
cargo build --release    # Release build
```

## Architecture

Cargo workspace with 4 library crates + 1 Tauri binary. React frontend communicates via Tauri Commands (request/response) and Events (streaming).

### Rust Backend (`crates/`)

```
crates/
├── vida-providers/      # LLMProvider trait + 4 providers
│   ├── traits.rs        # LLMProvider trait, ChatMessage, CompletionOptions, StreamEvent
│   ├── registry.rs      # ProviderRegistry (add/get/list/health_check_all)
│   ├── ollama.rs        # OllamaProvider (HTTP → localhost:11434)
│   ├── openai.rs        # OpenAIProvider (any OpenAI-compatible endpoint)
│   ├── anthropic.rs     # AnthropicProvider (Messages API)
│   └── google.rs        # GoogleProvider (Gemini API)
│
├── vida-security/       # Keychain OS + PIN + AES-GCM
│   ├── keychain.rs      # SecretStore trait + KeychainManager + MockSecretStore
│   ├── pin.rs           # PinManager (Argon2id)
│   └── encryption.rs    # AES-256-GCM encrypt/decrypt
│
├── vida-db/             # SQLite + SQLx + migrations
│   ├── models.rs        # Row structs (Session, Message, Team, TeamMember, etc.)
│   ├── repository.rs    # Database struct + CRUD operations
│   └── migrations/      # 001_initial, 002_teams, 003_workspaces, 004_mcp
│
└── vida-core/           # Orchestration
    ├── engine.rs        # VidaEngine — unified entry point
    ├── config.rs        # AppConfig
    ├── error.rs         # VidaError (unified error)
    ├── permissions.rs   # PermissionManager (Yolo/Ask/Sandbox)
    ├── workspace.rs     # WorkspaceConfig (.vida/config.json)
    ├── mcp.rs           # McpManager (MCP server lifecycle + tool routing)
    └── remote.rs        # RemoteServer (HTTP/WS + Telegram) [Phase 6]
```

### Tauri IPC (`src-tauri/`)

```
src-tauri/src/
├── main.rs              # Tauri bootstrap + command registration
└── commands/
    ├── chat.rs          # stream_completion, send_message, send_vision_message
    ├── providers.rs     # list_providers, list_models, health_check
    ├── auth.rs          # is_pin_configured, store_api_key, remove_api_key
    ├── config.rs        # get_config
    ├── teams.rs         # create_team, list_teams, stream_team_completion, etc.
    ├── workspace.rs     # open_workspace, create_workspace, permissions
    ├── mcp.rs           # start/stop/list MCP servers, call tools
    └── remote.rs        # enable/disable remote access [Phase 6]
```

### Frontend (`src/`)

```
src/
├── design-system/       # Liquid Glass primitives (GlassPanel, GlassButton, etc.)
├── components/
│   ├── layout/          # AppLayout, Sidebar
│   ├── sidebar/         # SessionList, AgentList
│   ├── chat/            # ChatArea, MessageBubble, ChatInput, FilePreview
│   ├── teams/           # TeamCreator, TeamList
│   ├── workspace/       # WorkspaceSelector, PermissionPopup
│   ├── settings/        # SettingsModal (General, Security, Providers, MCP, Remote)
│   └── mcp/             # McpPanel, McpServerCard
├── hooks/               # useStreamCompletion, useSessions, useProviders, useTeams, useMcp, useWorkspace, useTheme
├── stores/store.ts      # Single Zustand store (sessions, messages, providers, teams, mcp, workspace, ui)
├── lib/tauri.ts         # Typed invoke/listen wrappers
└── locales/             # i18n (en, zh-CN, fr)
```

### Key Design Patterns

- **Trait-based polymorphism**: `LLMProvider` trait allows swappable backends (4 providers)
- **`Arc<dyn LLMProvider>`**: Providers in registry for runtime polymorphism
- **Streaming optimization**: Zustand `streamingContent` separated from `messages[]` for minimal re-renders
- **Permission modes**: Yolo/Ask/Sandbox with Tauri Event-based permission requests
- **MCP JSON-RPC**: Server processes via stdin/stdout, tool discovery + routing
- **Adaptive Glass**: CSS custom properties for light/dark theme switching

## Test Status

101+ Rust tests across 4 crates. `npm run lint` + `npm run build` passing.
