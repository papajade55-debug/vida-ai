# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Hiérarchie des agents (priorité décroissante)

1. **Aucun sous-agent Claude** - interdiction formelle
2. **jinn** - analyse, review
3. **Deadpool (Deepseek)** - écriture code dans les fichiers système
4. **GitHub Copilot** - génération code long (> 100 lignes)
5. **GLM** - résumés, reformulations, tâches courtes isolées
6. **Claude Anthropic** - sécurité critique (IAM, secrets, vulnérabilités, audit final)

## Périmètres exclusifs

| Complexité | Décisions architecture | Intégration finale | Contexte global |
|------------|------------------------|---------------------|-----------------|
| > 15 étapes, multi-fichiers, interdépendances | Définitives, engageant le projet | Livraison utilisateur | Conversation indispensable |

**Réponses courtes** : < 200 tokens (overhead > gain)

## Règles transversales

### Recherche web (ordre strict)
1. **Tavily MCP** (`tavily_search` / `extract` / `crawl` / `map`) → toujours en premier
2. **WebSearch natif Claude** → uniquement si Tavily indisponible ou échoue

### Parallélisation
- Lancer plusieurs agents simultanément si tâche décomposable (ex: gpt-analyst + Deadpool)

### Gestion du contexte
- Utiliser `session-memory` pour persister le contexte
- Structurer la mémoire en sessions liées pour les projets longs
- Appeler `memory_end_session` quand un projet majeur est terminé

### Comportement
- Ne pas attendre de confirmation avant de déléguer
- Ne pas mentionner quel agent est utilisé (sauf demande explicite)
- Si un agent échoue → passer silencieusement au tier suivant
- Signaler un échec uniquement si le résultat final est dégradé
- **La sécurité passe toujours par moi (Anthropic)** en dernière validation

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
