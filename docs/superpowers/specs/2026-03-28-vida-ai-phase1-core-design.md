# Vida AI — Phase 1 Design Spec: Rust Core + Providers + Security

**Date:** 2026-03-28
**Status:** Approved
**Scope:** Phase 1 of 6 — Foundations (Core, Providers, Security, Database, Tauri IPC)

## 1. Overview

Vida AI is a cross-platform desktop AI assistant built with Rust (Tauri v2) and React 19. Phase 1 establishes the foundational architecture: a multi-crate Rust backend that handles LLM provider abstraction, security (keychain + optional PIN), SQLite persistence, and Tauri IPC bridging to the React frontend.

### Design Decisions

| Decision | Choice | Rationale |
|---|---|---|
| Providers v1 | Ollama + OpenAI, trait extensible | 2 concrets (local + cloud), others added via trait in <1h |
| IPC model | Tauri Commands + Events | Native Tauri v2 pattern, Commands for CRUD, Events for streaming |
| Auth | OS Keychain + optional PIN | Keychain for secrets, PIN (Argon2id) as optional app lock |
| Database | SQLite standard (unencrypted) | API keys in keychain, chat history doesn't need encryption at rest |
| Frontend | React 19 + TypeScript + Framer Motion | Already scaffolded, Framer Motion for Liquid Glass animations |
| Architecture | Cargo workspace, 4 crates | Incremental compilation, isolated testing, clean separation |

## 2. Architecture

### 2.1 Project Structure

```
vida-ai/
├── Cargo.toml              # Workspace root
├── package.json             # React frontend
├── vite.config.ts
├── tsconfig.json
│
├── crates/
│   ├── vida-providers/      # Trait LLMProvider + Ollama + OpenAI
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── traits.rs       # LLMProvider, ChatMessage, CompletionOptions
│   │   │   ├── ollama.rs       # OllamaProvider (HTTP → localhost:11434)
│   │   │   ├── openai.rs       # OpenAIProvider (HTTP → api.openai.com)
│   │   │   └── registry.rs     # ProviderRegistry (discovery + factory)
│   │   └── Cargo.toml
│   │
│   ├── vida-security/       # Keychain OS + PIN + AES-GCM
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── keychain.rs     # OS keychain (Secret Service / Keychain / CredMgr)
│   │   │   ├── pin.rs          # Optional PIN + Argon2id
│   │   │   └── encryption.rs   # AES-256-GCM for generic encryption
│   │   └── Cargo.toml
│   │
│   ├── vida-db/             # SQLite + SQLx + migrations
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── models.rs       # Session, Message, ProviderConfig
│   │   │   └── repository.rs   # CRUD operations
│   │   ├── migrations/         # SQLx migrations
│   │   └── Cargo.toml
│   │
│   └── vida-core/           # Orchestration (depends on the 3 above)
│       ├── src/
│       │   ├── lib.rs
│       │   ├── engine.rs       # VidaEngine — unified entry point
│       │   ├── config.rs       # AppConfig (serde, .vida/ file)
│       │   └── error.rs        # VidaError (thiserror)
│       └── Cargo.toml
│
├── src-tauri/               # Tauri v2 bridge
│   ├── src/
│   │   ├── main.rs             # Tauri bootstrap + plugin setup
│   │   └── commands/
│   │       ├── mod.rs
│   │       ├── chat.rs         # stream_completion, send_message
│   │       ├── providers.rs    # list_providers, health_check
│   │       ├── auth.rs         # unlock, set_pin, store_api_key
│   │       └── config.rs       # get/set app config
│   ├── Cargo.toml              # depends on vida-core
│   └── tauri.conf.json
│
└── src/                     # React 19 frontend
    ├── main.tsx
    ├── App.tsx
    ├── index.css
    ├── components/
    ├── hooks/
    ├── stores/
    └── lib/
        └── tauri.ts            # invoke() + listen() wrappers
```

### 2.2 Crate Dependency Graph

```
vida-providers ──┐
vida-security ───┼──→ vida-core ──→ src-tauri ⟷ React Frontend
vida-db ─────────┘                     (IPC: Commands + Events)
```

No leaf crate depends on another — only `vida-core` assembles them. `src-tauri` is a thin binary containing only Tauri commands that delegate to `vida-core`. Zero business logic in the Tauri layer.

## 3. Provider Abstraction Layer (`vida-providers`)

### 3.1 Trait LLMProvider

```rust
#[async_trait]
pub trait LLMProvider: Send + Sync {
    /// Non-streaming chat completion
    async fn chat_completion(
        &self,
        messages: &[ChatMessage],
        options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError>;

    /// Streaming via mpsc channel (dyn-safe, no generic Stream)
    async fn chat_completion_stream(
        &self,
        messages: &[ChatMessage],
        options: Option<CompletionOptions>,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), ProviderError>;

    /// Vision completion (image bytes + text prompt)
    async fn vision_completion(
        &self,
        image_data: Vec<u8>,
        prompt: &str,
        options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError>;

    /// Health check — is the provider reachable and authenticated?
    async fn health_check(&self) -> Result<(), ProviderError>;

    /// Provider metadata (name, type, models)
    fn info(&self) -> ProviderInfo;

    /// List available models dynamically
    async fn list_models(&self) -> Result<Vec<String>, ProviderError>;
}
```

### 3.2 Data Types

```rust
pub enum ChatRole { System, User, Assistant }

pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

pub struct CompletionOptions {
    pub model: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub top_k: Option<i32>,
    pub repeat_penalty: Option<f32>,
}

pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

pub enum StreamEvent {
    Token { content: String },
    Error { error: String },
    Done,
}

pub struct ProviderInfo {
    pub name: String,
    pub provider_type: ProviderType,  // Local | Cloud
    pub models: Vec<String>,
}
```

### 3.3 ProviderError

```rust
#[derive(Debug, thiserror::Error)]
pub enum ProviderError {
    #[error("Network: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Serialization: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("Model not supported: {0}")]
    ModelNotSupported(String),
    #[error("Provider unavailable")]
    Unavailable,
    #[error("Unauthorized")]
    Unauthorized,
    #[error("Internal: {0}")]
    Internal(String),
}
```

### 3.4 ProviderRegistry

```rust
pub struct ProviderRegistry { /* HashMap<String, Arc<dyn LLMProvider>> */ }

impl ProviderRegistry {
    pub fn new() -> Self;
    pub fn add(&mut self, name: String, provider: Arc<dyn LLMProvider>) -> Result<(), String>;
    pub fn get(&self, name: &str) -> Option<Arc<dyn LLMProvider>>;
    pub fn list(&self) -> Vec<ProviderInfo>;
    pub async fn health_check_all(&self) -> Vec<(String, Result<(), ProviderError>)>;
}
```

### 3.5 Concrete Providers

**OllamaProvider:** HTTP client to `localhost:11434` (configurable). Uses `/api/chat` for completion, `/api/chat` with `stream: true` + SSE parsing for streaming, `/api/tags` for health_check and list_models. Vision via base64-encoded images in message content.

**OpenAIProvider:** HTTP client to `api.openai.com` (configurable base_url for **any OpenAI-compatible API** — Groq, Mistral, Together, LM Studio, vLLM, etc.). Uses `/v1/chat/completions` for completion, SSE streaming with `stream: true`, `/v1/models` for list_models. API key retrieved from keychain at construction time. This single provider covers dozens of services that expose OpenAI-compatible endpoints.

**Future Providers (Phase 2+):** `AnthropicProvider` (Messages API), `GoogleProvider` (Gemini API). Each is a new file implementing `LLMProvider` — no changes to existing code. The architecture guarantees **zero vendor lock-in**: no SDK dependency, pure HTTP via `reqwest`.

### 3.6 Streaming Data Flow

```
React invoke("stream_completion")
  → Tauri Command creates mpsc channel (buffer: 100)
    → VidaEngine.send_message_stream(session_id, content, tx)
      → Provider.chat_completion_stream(messages, options, tx)
        → Provider spawns tokio task, parses SSE, pushes StreamEvent into tx
      ← Command reads from rx in a loop
    ← Each StreamEvent emitted via app_handle.emit("llm-stream-{session_id}", event)
  → React listen("llm-stream-{session_id}") receives tokens incrementally
```

## 4. Security & Auth (`vida-security`)

### 4.1 Keychain Manager

Uses the `keyring` crate for cross-platform OS keychain access:
- **Linux:** Secret Service (GNOME Keyring / KDE Wallet)
- **macOS:** macOS Keychain
- **Windows:** Credential Manager

```rust
pub trait SecretStore: Send + Sync {
    fn store(&self, key: &str, value: &str) -> Result<(), SecurityError>;
    fn get(&self, key: &str) -> Result<String, SecurityError>;
    fn delete(&self, key: &str) -> Result<(), SecurityError>;
    fn list(&self) -> Result<Vec<String>, SecurityError>;
}

pub struct KeychainManager { service: String }
impl SecretStore for KeychainManager { /* delegates to keyring crate */ }
```

The `SecretStore` trait enables dependency injection — `VidaEngine` takes `Box<dyn SecretStore>`, allowing `MockSecretStore` (in-memory HashMap) for testing.

### 4.2 PIN Manager

Optional application lock. PIN hash stored in SQLite `pin_config` table (singleton row).

```rust
pub struct PinManager;

impl PinManager {
    pub fn is_configured(db: &SqlitePool) -> Result<bool, SecurityError>;
    pub async fn set_pin(db: &SqlitePool, pin: &str) -> Result<(), SecurityError>;
    pub async fn verify_pin(db: &SqlitePool, pin: &str) -> Result<bool, SecurityError>;
    pub async fn remove_pin(db: &SqlitePool) -> Result<(), SecurityError>;
}
```

Argon2id parameters: m=65536, t=3, p=4. Random 16-byte salt stored alongside hash.

**Password vs PIN:** The spec uses "PIN" but the UI will present it as a **password field** (alphanumeric, 4-32 characters, not limited to digits). The term "PIN" refers to the implementation pattern (local app lock, not a user account), but the UX allows full passwords. A **"Change password" option** is exposed in the Settings UI via the `set_pin` / `remove_pin` Tauri commands.

### 4.3 Boot Sequence

1. App starts → checks if PIN is configured (`pin_config` table)
2. **PIN active** → show PIN screen → verify Argon2id hash
3. **No PIN** → direct access (OS session = trust)
4. VidaEngine initializes → loads config from SQLite → retrieves API keys from OS keychain
5. ProviderRegistry instantiates providers with keys → health_check_all()
6. Main UI displays with available providers

### 4.4 SecurityError

```rust
#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Keychain access: {0}")]
    KeychainAccess(String),
    #[error("Secret not found: {0}")]
    SecretNotFound(String),
    #[error("Invalid PIN")]
    InvalidPin,
    #[error("Hashing failed: {0}")]
    HashingFailed(String),
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
}
```

## 5. Database Layer (`vida-db`)

### 5.1 Schema

```sql
CREATE TABLE app_config (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,          -- JSON serialized
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE provider_configs (
    id            TEXT PRIMARY KEY,    -- "ollama", "openai"
    provider_type TEXT NOT NULL,       -- "local" | "cloud"
    base_url      TEXT,               -- e.g. "http://localhost:11434"
    default_model TEXT,               -- e.g. "llama3"
    enabled       INTEGER NOT NULL DEFAULT 1,
    config_json   TEXT,               -- additional options (JSON)
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);
-- Note: API keys are in OS Keychain, NOT in this table

CREATE TABLE sessions (
    id            TEXT PRIMARY KEY,    -- UUID v4
    title         TEXT,               -- generated or user-defined
    provider_id   TEXT NOT NULL,
    model         TEXT NOT NULL,
    system_prompt TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (provider_id) REFERENCES provider_configs(id)
);

CREATE TABLE messages (
    id          TEXT PRIMARY KEY,      -- UUID v4
    session_id  TEXT NOT NULL,
    role        TEXT NOT NULL,         -- "system" | "user" | "assistant"
    content     TEXT NOT NULL,
    token_count INTEGER,              -- tokens consumed (optional)
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE TABLE pin_config (
    id         INTEGER PRIMARY KEY CHECK (id = 1),  -- singleton
    hash       TEXT NOT NULL,          -- Argon2id hash
    salt       BLOB NOT NULL,          -- 16 bytes
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_messages_session ON messages(session_id, created_at);
CREATE INDEX idx_sessions_updated ON sessions(updated_at DESC);
```

### 5.2 Repository API

```rust
pub struct Database { pool: SqlitePool }

impl Database {
    pub async fn connect(path: &str) -> Result<Self, DbError>;
    pub async fn run_migrations(&self) -> Result<(), DbError>;

    // Config
    pub async fn get_config(&self, key: &str) -> Result<Option<String>, DbError>;
    pub async fn set_config(&self, key: &str, value: &str) -> Result<(), DbError>;

    // Providers
    pub async fn upsert_provider(&self, config: &ProviderConfigRow) -> Result<(), DbError>;
    pub async fn list_providers(&self) -> Result<Vec<ProviderConfigRow>, DbError>;
    pub async fn get_provider(&self, id: &str) -> Result<Option<ProviderConfigRow>, DbError>;

    // Sessions
    pub async fn create_session(&self, session: &SessionRow) -> Result<(), DbError>;
    pub async fn list_sessions(&self, limit: u32) -> Result<Vec<SessionRow>, DbError>;
    pub async fn get_session(&self, id: &str) -> Result<Option<SessionRow>, DbError>;
    pub async fn delete_session(&self, id: &str) -> Result<(), DbError>;

    // Messages
    pub async fn insert_message(&self, msg: &MessageRow) -> Result<(), DbError>;
    pub async fn get_messages(&self, session_id: &str) -> Result<Vec<MessageRow>, DbError>;
}
```

### 5.3 Design Principles

- **SQLx compile-time checked queries** — SQL verified at compilation
- **Versioned migrations** — `migrations/` directory, applied at startup
- **UUID v4 for all IDs** — no autoincrement (future-proof for sync)
- **ON DELETE CASCADE** — deleting a session deletes its messages
- **JSON in app_config** — flexible schema for evolving settings

### 5.4 Out of Scope (Phase 1)

- Workspace table (Phase 4)
- Team/Agent config table (Phase 3)
- MCP server config table (Phase 5)
- Database encryption (YAGNI)
- Full-text search on messages

## 6. Core Orchestration (`vida-core`)

### 6.1 VidaEngine

```rust
pub struct VidaEngine {
    db: Database,                        // vida-db
    providers: ProviderRegistry,         // vida-providers
    secrets: Box<dyn SecretStore>,       // vida-security (injectable)
    config: AppConfig,                   // loaded from SQLite
}

impl VidaEngine {
    /// Initialize with real keychain (production)
    pub async fn init(data_dir: &Path) -> Result<Self, VidaError>;

    /// Initialize with custom SecretStore (for testing with MockSecretStore)
    pub async fn init_with_secrets(data_dir: &Path, secrets: Box<dyn SecretStore>) -> Result<Self, VidaError>;

    // Chat
    pub async fn send_message(&self, session_id: &str, content: &str) -> Result<CompletionResponse, VidaError>;
    pub async fn send_message_stream(&self, session_id: &str, content: &str, tx: mpsc::Sender<StreamEvent>) -> Result<(), VidaError>;

    // Sessions
    pub async fn create_session(&self, provider_id: &str, model: &str) -> Result<SessionRow, VidaError>;
    pub async fn list_sessions(&self, limit: u32) -> Result<Vec<SessionRow>, VidaError>;
    pub async fn get_session_messages(&self, session_id: &str) -> Result<Vec<MessageRow>, VidaError>;
    pub async fn delete_session(&self, id: &str) -> Result<(), VidaError>;

    // Providers
    pub async fn list_providers(&self) -> Vec<ProviderInfo>;
    pub async fn list_models(&self, provider_id: &str) -> Result<Vec<String>, VidaError>;
    pub async fn health_check_all(&self) -> Vec<(String, bool)>;

    // Security
    pub fn is_pin_configured(&self) -> Result<bool, VidaError>;
    pub async fn verify_pin(&self, pin: &str) -> Result<bool, VidaError>;
    pub async fn set_pin(&self, pin: &str) -> Result<(), VidaError>;
    pub async fn store_api_key(&self, provider_id: &str, key: &str) -> Result<(), VidaError>;
    pub async fn remove_api_key(&self, provider_id: &str) -> Result<(), VidaError>;
}
```

### 6.2 send_message_stream Internal Sequence

1. Retrieve session from DB → get provider_id + model
2. Load message history for this session from DB
3. Insert new "user" message into DB
4. Build `Vec<ChatMessage>` (system_prompt + history + new message)
5. Get provider via `registry.get(provider_id)`
6. Call `provider.chat_completion_stream(messages, options, tx)`
7. When `StreamEvent::Done` received → concatenate all tokens
8. Insert complete "assistant" message into DB

### 6.3 VidaError

```rust
#[derive(Debug, thiserror::Error)]
pub enum VidaError {
    #[error("Provider: {0}")]
    Provider(#[from] ProviderError),
    #[error("Security: {0}")]
    Security(#[from] SecurityError),
    #[error("Database: {0}")]
    Database(#[from] DbError),
    #[error("Session not found: {0}")]
    SessionNotFound(String),
    #[error("Config: {0}")]
    Config(String),
}
```

## 7. Tauri IPC Bridge (`src-tauri`)

### 7.1 State Management

VidaEngine is wrapped in `Arc<RwLock<VidaEngine>>` via Tauri State. Initialized in `setup()`, shared across all commands.

```rust
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            let engine = block_on(VidaEngine::init(&data_dir))?;
            app.manage(Arc::new(RwLock::new(engine)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![...])
        .run(tauri::generate_context!())
        .expect("error while running vida-ai");
}
```

### 7.2 Commands

| Command | Type | Delegates to |
|---|---|---|
| `stream_completion` | Command + Event | engine.send_message_stream() → emit("llm-stream-{session_id}") |
| `send_message` | Command | engine.send_message() |
| `create_session` | Command | engine.create_session() |
| `list_sessions` | Command | engine.list_sessions() |
| `get_messages` | Command | engine.get_session_messages() |
| `delete_session` | Command | engine.delete_session() |
| `list_providers` | Command | engine.list_providers() |
| `list_models` | Command | engine.list_models() |
| `health_check` | Command | engine.health_check_all() |
| `verify_pin` | Command | engine.verify_pin() |
| `set_pin` | Command | engine.set_pin() |
| `store_api_key` | Command | engine.store_api_key() |
| `is_pin_configured` | Command | engine.is_pin_configured() |

`src-tauri` contains zero business logic — it is a thin IPC bridge.

## 8. Testing Strategy

### 8.1 TDD Workflow

Red (write test first, FAIL) → Green (minimum code to pass) → Refactor (clean up, tests protect).

### 8.2 Test Matrix

| Crate | Unit Tests | Integration Tests | Mocking |
|---|---|---|---|
| vida-providers | Serialization, ProviderRegistry add/get/list, HTTP request construction | Ollama health_check + chat (real, `#[ignore]`) | `MockLLMProvider` via mockall |
| vida-security | AES-GCM roundtrip, Argon2id roundtrip, PIN set/verify/remove | Keychain store/get/delete (`#[ignore]`, needs D-Bus) | `MockSecretStore` (in-memory HashMap) |
| vida-db | Model serialization, data validation | Full CRUD on SQLite **in-memory** (fast, no cleanup), migrations, CASCADE delete | Not needed — SQLite in-memory is fast |
| vida-core | AppConfig load/save, VidaError conversions | VidaEngine::init() full, send_message with mock, stream events verification, full scenario: create_session → send → get_messages | MockLLMProvider + MockSecretStore + SQLite in-memory → **100% testable without network or keychain** |

### 8.3 Test Commands

```bash
cargo test --workspace                              # All tests (excluding #[ignore])
cargo test -p vida-providers                        # Single crate
cargo test -p vida-db test_cascade_delete -- --nocapture  # Single test with output
cargo test --workspace -- --include-ignored          # Include integration tests
cargo check --workspace                             # Compile check only
```

## 9. Key Rust Dependencies

| Crate | Version | Used in | Purpose |
|---|---|---|---|
| tokio | 1.x (full) | All | Async runtime |
| sqlx | 0.7 (sqlite, migrate, chrono, uuid) | vida-db | SQLite async |
| serde / serde_json | 1.x | All | Serialization |
| reqwest | 0.11 (json, stream) | vida-providers | HTTP client |
| async-trait | 0.1 | vida-providers | Async trait support |
| thiserror | 1.x | All | Error derive |
| keyring | 3.x | vida-security | OS keychain |
| argon2 | 0.5 | vida-security | PIN hashing |
| aes-gcm | 0.10 | vida-security | Generic encryption |
| uuid | 1.x (v4, serde) | vida-db, vida-core | ID generation |
| chrono | 0.4 (serde) | vida-db | Timestamps |
| tauri | 2.x | src-tauri | Desktop framework |
| mockall | 0.12 | dev-deps | Trait mocking |
| tempfile | 3.x | dev-deps | Temp dirs for tests |

## 10. Internationalization (i18n)

### 10.1 Launch Languages (Phase 1)

| Language | Code | Rationale |
|---|---|---|
| English | `en` | Default, Silicon Valley AI ecosystem |
| Simplified Chinese | `zh-CN` | China AI ecosystem, 2nd largest market |
| French | `fr` | Europe francophone, Mistral AI ecosystem |

### 10.2 Architecture Requirements

- **12-language ready from day 1** — the i18n system must support adding new locales without code changes
- **Automatic language detection** — detect OS locale at startup, fallback to `en`
- **Community translations** — locale files in a standard format (JSON) that open-source contributors can add to
- **Frontend i18n** — React side uses a lightweight library (`react-i18next` or equivalent) with namespace-based JSON files in `src/locales/{lang}/`
- **Backend i18n** — Rust error messages and Tauri command responses remain in English (machine-consumed). Only UI-facing strings are translated.

### 10.3 Locale File Structure

```
src/
└── locales/
    ├── en/
    │   ├── common.json       # Shared UI strings
    │   ├── chat.json         # Chat interface
    │   ├── providers.json    # Provider management
    │   └── settings.json     # Settings screens
    ├── zh-CN/
    │   └── (same structure)
    └── fr/
        └── (same structure)
```

### 10.4 Future Locale Expansion (post-Phase 1)

Target 12 languages: en, zh-CN, fr, es, de, ja, ko, pt-BR, ru, ar, hi, it. Community PRs welcome via standard JSON locale files.

## 11. Product Vision & Principles

### 11.1 Core Principles

- **Zero vendor lock-in:** No proprietary SDK. All provider communication via pure HTTP (`reqwest`). The `LLMProvider` trait is the only abstraction needed.
- **Open source:** Project will be released under an open-source license (AGPLv3 or MIT, to be decided). All dependencies are open-source. No telemetry, no phoning home.
- **Universal provider support:** Ollama (local), any OpenAI-compatible API (OpenAI, Groq, Mistral, Together, LM Studio, vLLM, Cerebras, NVIDIA NIM…), with dedicated providers for Anthropic and Google Gemini in Phase 2+.
- **Multi-deployment:** Desktop (Linux .deb/.rpm/.AppImage, macOS .dmg, Windows .exe), LXC container (Proxmox), Docker, headless server mode.

### 11.2 Full Feature Matrix — Phase Mapping

| Feature | Phase | Details |
|---|---|---|
| Provider abstraction (trait, no SDK) | **1** ✅ | `LLMProvider` trait + Ollama + OpenAI-compatible |
| Additional providers (Anthropic, Google) | **2** | New files implementing `LLMProvider` |
| Login screen + change password | **1** ✅ | PIN/password with Argon2id + Settings UI |
| OS keychain for API keys | **1** ✅ | `keyring` crate, cross-platform |
| Liquid Glass design system | **2** | React components, Framer Motion animations |
| Chat interface + streaming | **2** | React UI consuming Tauri Events |
| File import + drag & drop | **2** | Frontend file handling → backend processing |
| Vision (image analysis) | **1** ✅ (backend) / **2** (UI) | `vision_completion` in trait; UI in Phase 2 |
| Voice chat (Whisper.cpp) | **2** | Local speech-to-text, no cloud dependency |
| Team creation (checkbox per model) | **3** | `vida-team` crate, multi-agent orchestration |
| Sidebar agent list + activity animations | **3** | Agent state (Idle/Working/Error) → color pulse |
| Workspace selector (.vida/config.json) | **4** | `vida-workspace` crate, directory-based |
| Permission system (Yolo/Sandbox/Ask) | **4** | `vida-permissions` crate, per-workspace |
| Permission bypass / total control | **4** | Granular: file write, shell exec, network scan |
| MCP server management | **5** | `vida-mcp` crate, process spawning, tool routing |
| MCP prompt rewriting (non-tool-calling models) | **5** | Adapter layer for models without native tool use |
| Skills system | **5** | Integrated with MCP, per-workspace config |
| Remote access (HTTP/WS server) | **6** | `vida-remote` crate, embedded server |
| Telegram bot connector | **6** | Bot API integration for remote commands |
| i18n (en, zh-CN, fr + 9 more) | **1** ✅ | react-i18next, JSON locale files, community PRs |
| Packaging (.deb, .rpm, .AppImage, .dmg, .exe) | **6** | Tauri bundler + CI/CD |
| LXC / Docker / headless deployment | **6** | Dockerfile + LXC script, server mode without GUI |
| Open-source release | **6** | License, documentation, contribution guide |

## 12. Future Phase Specs (to be written)

| Phase | Spec | Status |
|---|---|---|
| 2 | UI/Design System + Chat + Vision UI + Voice | Pending |
| 3 | Team/Agent Engine + Sidebar animations | Pending |
| 4 | Workspace Manager + Permissions | Pending |
| 5 | MCP Integration + Skills + Prompt Rewriting | Pending |
| 6 | Remote Access + Packaging + Open-source release | Pending |
