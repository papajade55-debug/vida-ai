# Vida AI Phase 1 — Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the Rust backend foundation for Vida AI — multi-crate workspace with LLM provider abstraction, security (keychain + password), SQLite persistence, and Tauri v2 IPC bridge.

**Architecture:** Cargo workspace with 4 library crates (`vida-providers`, `vida-security`, `vida-db`, `vida-core`) and 1 binary crate (`src-tauri`). React 19 frontend communicates via Tauri Commands (request/response) and Events (streaming). TDD: tests first, then implementation.

**Tech Stack:** Rust 2021, Tauri v2, tokio, sqlx (SQLite), reqwest, keyring, argon2, aes-gcm, async-trait, thiserror, serde, mockall. Frontend: React 19, TypeScript, Vite 6, Tailwind CSS 4, Framer Motion.

**Spec:** `docs/superpowers/specs/2026-03-28-vida-ai-phase1-core-design.md`

---

## File Map

### Workspace Root
- Create: `Cargo.toml` (workspace definition, replaces existing)
- Modify: `package.json` (already exists, add i18next deps)
- Keep: `vite.config.ts`, `tsconfig.json`, `index.html`

### crates/vida-providers/
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/traits.rs` — LLMProvider trait, ChatMessage, CompletionOptions, etc.
- Create: `src/registry.rs` — ProviderRegistry
- Create: `src/ollama.rs` — OllamaProvider
- Create: `src/openai.rs` — OpenAIProvider

### crates/vida-security/
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/keychain.rs` — SecretStore trait + KeychainManager
- Create: `src/pin.rs` — PinManager (Argon2id)
- Create: `src/encryption.rs` — AES-256-GCM helpers

### crates/vida-db/
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/models.rs` — Row structs
- Create: `src/repository.rs` — Database struct + CRUD
- Create: `migrations/001_initial.sql`

### crates/vida-core/
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/error.rs` — VidaError
- Create: `src/config.rs` — AppConfig
- Create: `src/engine.rs` — VidaEngine

### src-tauri/
- Create: `Cargo.toml`
- Create: `tauri.conf.json`
- Create: `src/main.rs`
- Create: `src/commands/mod.rs`
- Create: `src/commands/auth.rs`
- Create: `src/commands/providers.rs`
- Create: `src/commands/chat.rs`
- Create: `src/commands/config.rs`

### src/ (Frontend — minimal shell for Phase 1)
- Modify: `src/App.tsx`
- Modify: `src/main.tsx`
- Create: `src/lib/tauri.ts` — typed invoke/listen wrappers
- Create: `src/locales/en/common.json`
- Create: `src/locales/zh-CN/common.json`
- Create: `src/locales/fr/common.json`

### Removed (old skeleton)
- Delete: `src/lib.rs`, `src/main.rs`, `src/core/`, `src/database/`, `src/mcp/`, `src/permissions/`, `src/providers/`, `src/remote/`, `src/security/`, `src/team/`, `src/tools/`, `src/utils/`, `src/workspace/`

---

## Task 1: Scaffold Cargo Workspace

**Files:**
- Create: `Cargo.toml` (workspace root)
- Create: `crates/vida-providers/Cargo.toml`
- Create: `crates/vida-providers/src/lib.rs`
- Create: `crates/vida-security/Cargo.toml`
- Create: `crates/vida-security/src/lib.rs`
- Create: `crates/vida-db/Cargo.toml`
- Create: `crates/vida-db/src/lib.rs`
- Create: `crates/vida-core/Cargo.toml`
- Create: `crates/vida-core/src/lib.rs`
- Delete: `src/lib.rs`, `src/main.rs`, and old Rust skeleton directories

- [ ] **Step 1: Remove old Rust skeleton**

```bash
rm -f src/lib.rs src/main.rs
rm -rf src/core src/database src/mcp src/permissions src/providers src/remote src/security src/team src/tools src/utils src/workspace
```

- [ ] **Step 2: Create workspace root Cargo.toml**

Replace the existing `Cargo.toml` with:

```toml
[workspace]
members = [
    "crates/vida-providers",
    "crates/vida-security",
    "crates/vida-db",
    "crates/vida-core",
    "src-tauri",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
license = "MIT"

[workspace.dependencies]
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
async-trait = "0.1"
uuid = { version = "1.6", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
```

- [ ] **Step 3: Create vida-providers crate**

```bash
mkdir -p crates/vida-providers/src
```

`crates/vida-providers/Cargo.toml`:
```toml
[package]
name = "vida-providers"
version.workspace = true
edition.workspace = true

[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
reqwest = { version = "0.12", features = ["json", "stream"] }

[dev-dependencies]
mockall = "0.12"
tokio = { workspace = true, features = ["test-util"] }
```

`crates/vida-providers/src/lib.rs`:
```rust
pub mod traits;
pub mod registry;
pub mod ollama;
pub mod openai;
```

- [ ] **Step 4: Create vida-security crate**

```bash
mkdir -p crates/vida-security/src
```

`crates/vida-security/Cargo.toml`:
```toml
[package]
name = "vida-security"
version.workspace = true
edition.workspace = true

[dependencies]
thiserror = { workspace = true }
serde = { workspace = true }
keyring = "3"
argon2 = "0.5"
aes-gcm = "0.10"
rand = "0.8"
base64 = "0.22"

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
```

`crates/vida-security/src/lib.rs`:
```rust
pub mod keychain;
pub mod pin;
pub mod encryption;
```

- [ ] **Step 5: Create vida-db crate**

```bash
mkdir -p crates/vida-db/src
mkdir -p crates/vida-db/migrations
```

`crates/vida-db/Cargo.toml`:
```toml
[package]
name = "vida-db"
version.workspace = true
edition.workspace = true

[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
uuid = { workspace = true }
chrono = { workspace = true }
sqlx = { version = "0.8", features = ["runtime-tokio", "sqlite", "migrate", "chrono", "uuid"] }

[dev-dependencies]
tokio = { workspace = true, features = ["test-util"] }
```

`crates/vida-db/src/lib.rs`:
```rust
pub mod models;
pub mod repository;
```

- [ ] **Step 6: Create vida-core crate**

```bash
mkdir -p crates/vida-core/src
```

`crates/vida-core/Cargo.toml`:
```toml
[package]
name = "vida-core"
version.workspace = true
edition.workspace = true

[dependencies]
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
thiserror = { workspace = true }
uuid = { workspace = true }
vida-providers = { path = "../vida-providers" }
vida-security = { path = "../vida-security" }
vida-db = { path = "../vida-db" }

[dev-dependencies]
mockall = "0.12"
tempfile = "3.8"
tokio = { workspace = true, features = ["test-util"] }
```

`crates/vida-core/src/lib.rs`:
```rust
pub mod error;
pub mod config;
pub mod engine;
```

- [ ] **Step 7: Verify workspace compiles**

```bash
cargo check --workspace
```

Expected: compiles with warnings about empty modules (that's fine).

- [ ] **Step 8: Commit**

```bash
git add -A
git commit -m "feat: scaffold Cargo workspace with 4 crates (vida-providers, vida-security, vida-db, vida-core)"
```

---

## Task 2: Provider Types & Trait (`vida-providers`)

**Files:**
- Create: `crates/vida-providers/src/traits.rs`

- [ ] **Step 1: Write tests for data types serialization**

Add to end of `crates/vida-providers/src/traits.rs`:

```rust
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

// ── Data Types ──

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompletionOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repeat_penalty: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub content: String,
    pub model: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamEvent {
    Token { content: String },
    Error { error: String },
    Done,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ProviderType {
    Local,
    Cloud,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub name: String,
    pub provider_type: ProviderType,
    pub models: Vec<String>,
}

// ── Error ──

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

// ── Trait ──

#[async_trait]
pub trait LLMProvider: Send + Sync {
    async fn chat_completion(
        &self,
        messages: &[ChatMessage],
        options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError>;

    async fn chat_completion_stream(
        &self,
        messages: &[ChatMessage],
        options: Option<CompletionOptions>,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), ProviderError>;

    async fn vision_completion(
        &self,
        image_data: Vec<u8>,
        prompt: &str,
        options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError>;

    async fn health_check(&self) -> Result<(), ProviderError>;

    fn info(&self) -> ProviderInfo;

    async fn list_models(&self) -> Result<Vec<String>, ProviderError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_role_serialization() {
        let json = serde_json::to_string(&ChatRole::User).unwrap();
        assert_eq!(json, "\"user\"");
        let role: ChatRole = serde_json::from_str("\"assistant\"").unwrap();
        assert_eq!(role, ChatRole::Assistant);
    }

    #[test]
    fn test_chat_message_serialization() {
        let msg = ChatMessage {
            role: ChatRole::System,
            content: "You are helpful.".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"system\""));
        assert!(json.contains("You are helpful."));
    }

    #[test]
    fn test_completion_options_skip_none() {
        let opts = CompletionOptions {
            temperature: Some(0.7),
            ..Default::default()
        };
        let json = serde_json::to_string(&opts).unwrap();
        assert!(json.contains("\"temperature\":0.7"));
        assert!(!json.contains("\"model\""));
        assert!(!json.contains("\"max_tokens\""));
    }

    #[test]
    fn test_stream_event_variants() {
        let token = StreamEvent::Token { content: "Hello".to_string() };
        let json = serde_json::to_string(&token).unwrap();
        assert!(json.contains("Token"));
        assert!(json.contains("Hello"));

        let done = StreamEvent::Done;
        let json = serde_json::to_string(&done).unwrap();
        assert!(json.contains("Done"));
    }

    #[test]
    fn test_provider_type_serialization() {
        let json = serde_json::to_string(&ProviderType::Local).unwrap();
        assert_eq!(json, "\"local\"");
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p vida-providers -- --nocapture
```

Expected: all 5 tests PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/vida-providers/src/traits.rs
git commit -m "feat(vida-providers): add LLMProvider trait, data types, and serialization tests"
```

---

## Task 3: ProviderRegistry (`vida-providers`)

**Files:**
- Create: `crates/vida-providers/src/registry.rs`

- [ ] **Step 1: Write the ProviderRegistry with tests**

`crates/vida-providers/src/registry.rs`:
```rust
use std::collections::HashMap;
use std::sync::Arc;

use crate::traits::{LLMProvider, ProviderError, ProviderInfo};

pub struct ProviderRegistry {
    providers: HashMap<String, Arc<dyn LLMProvider>>,
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    pub fn add(&mut self, name: String, provider: Arc<dyn LLMProvider>) -> Result<(), String> {
        if self.providers.contains_key(&name) {
            return Err(format!("Provider '{}' already registered", name));
        }
        self.providers.insert(name, provider);
        Ok(())
    }

    pub fn get(&self, name: &str) -> Option<Arc<dyn LLMProvider>> {
        self.providers.get(name).cloned()
    }

    pub fn list(&self) -> Vec<ProviderInfo> {
        self.providers.values().map(|p| p.info()).collect()
    }

    pub async fn health_check_all(&self) -> Vec<(String, Result<(), ProviderError>)> {
        let mut results = Vec::new();
        for (name, provider) in &self.providers {
            let result = provider.health_check().await;
            results.push((name.clone(), result));
        }
        results
    }
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::*;
    use async_trait::async_trait;
    use tokio::sync::mpsc;

    struct FakeProvider {
        name: String,
        healthy: bool,
    }

    #[async_trait]
    impl LLMProvider for FakeProvider {
        async fn chat_completion(&self, _: &[ChatMessage], _: Option<CompletionOptions>) -> Result<CompletionResponse, ProviderError> {
            Ok(CompletionResponse {
                content: "fake".to_string(),
                model: "fake-model".to_string(),
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 0,
            })
        }
        async fn chat_completion_stream(&self, _: &[ChatMessage], _: Option<CompletionOptions>, _: mpsc::Sender<StreamEvent>) -> Result<(), ProviderError> {
            Ok(())
        }
        async fn vision_completion(&self, _: Vec<u8>, _: &str, _: Option<CompletionOptions>) -> Result<CompletionResponse, ProviderError> {
            Err(ProviderError::Internal("not supported".to_string()))
        }
        async fn health_check(&self) -> Result<(), ProviderError> {
            if self.healthy { Ok(()) } else { Err(ProviderError::Unavailable) }
        }
        fn info(&self) -> ProviderInfo {
            ProviderInfo {
                name: self.name.clone(),
                provider_type: ProviderType::Local,
                models: vec!["fake-model".to_string()],
            }
        }
        async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
            Ok(vec!["fake-model".to_string()])
        }
    }

    #[test]
    fn test_registry_add_and_get() {
        let mut reg = ProviderRegistry::new();
        let provider = Arc::new(FakeProvider { name: "test".to_string(), healthy: true });
        assert!(reg.add("test".to_string(), provider).is_ok());
        assert!(reg.get("test").is_some());
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_duplicate_add_fails() {
        let mut reg = ProviderRegistry::new();
        let p1 = Arc::new(FakeProvider { name: "test".to_string(), healthy: true });
        let p2 = Arc::new(FakeProvider { name: "test".to_string(), healthy: true });
        assert!(reg.add("test".to_string(), p1).is_ok());
        assert!(reg.add("test".to_string(), p2).is_err());
    }

    #[test]
    fn test_registry_list() {
        let mut reg = ProviderRegistry::new();
        reg.add("a".to_string(), Arc::new(FakeProvider { name: "A".to_string(), healthy: true })).unwrap();
        reg.add("b".to_string(), Arc::new(FakeProvider { name: "B".to_string(), healthy: true })).unwrap();
        let list = reg.list();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn test_registry_health_check_all() {
        let mut reg = ProviderRegistry::new();
        reg.add("healthy".to_string(), Arc::new(FakeProvider { name: "H".to_string(), healthy: true })).unwrap();
        reg.add("sick".to_string(), Arc::new(FakeProvider { name: "S".to_string(), healthy: false })).unwrap();
        let results = reg.health_check_all().await;
        assert_eq!(results.len(), 2);
        let healthy_result = results.iter().find(|(n, _)| n == "healthy").unwrap();
        assert!(healthy_result.1.is_ok());
        let sick_result = results.iter().find(|(n, _)| n == "sick").unwrap();
        assert!(sick_result.1.is_err());
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p vida-providers -- --nocapture
```

Expected: all 9 tests PASS (5 from Task 2 + 4 new).

- [ ] **Step 3: Commit**

```bash
git add crates/vida-providers/src/registry.rs
git commit -m "feat(vida-providers): add ProviderRegistry with add/get/list/health_check_all"
```

---

## Task 4: OllamaProvider (`vida-providers`)

**Files:**
- Create: `crates/vida-providers/src/ollama.rs`

- [ ] **Step 1: Write OllamaProvider**

`crates/vida-providers/src/ollama.rs`:
```rust
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::traits::*;

pub struct OllamaProvider {
    client: Client,
    base_url: String,
}

impl OllamaProvider {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }
}

// ── Ollama API types ──

#[derive(Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    images: Option<Vec<String>>,
}

#[derive(Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repeat_penalty: Option<f32>,
}

#[derive(Deserialize)]
struct OllamaChatResponse {
    message: OllamaResponseMessage,
    model: String,
    #[serde(default)]
    prompt_eval_count: u32,
    #[serde(default)]
    eval_count: u32,
    #[serde(default)]
    done: bool,
}

#[derive(Deserialize)]
struct OllamaResponseMessage {
    content: String,
}

#[derive(Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

#[derive(Deserialize)]
struct OllamaModel {
    name: String,
}

// ── Helpers ──

fn to_ollama_messages(messages: &[ChatMessage]) -> Vec<OllamaMessage> {
    messages
        .iter()
        .map(|m| OllamaMessage {
            role: match m.role {
                ChatRole::System => "system".to_string(),
                ChatRole::User => "user".to_string(),
                ChatRole::Assistant => "assistant".to_string(),
            },
            content: m.content.clone(),
            images: None,
        })
        .collect()
}

fn to_ollama_options(options: &Option<CompletionOptions>) -> Option<OllamaOptions> {
    options.as_ref().map(|o| OllamaOptions {
        temperature: o.temperature,
        num_predict: o.max_tokens,
        top_p: o.top_p,
        top_k: o.top_k,
        repeat_penalty: o.repeat_penalty,
    })
}

#[async_trait]
impl LLMProvider for OllamaProvider {
    async fn chat_completion(
        &self,
        messages: &[ChatMessage],
        options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError> {
        let model = options
            .as_ref()
            .and_then(|o| o.model.clone())
            .unwrap_or_else(|| "llama3".to_string());

        let request = OllamaChatRequest {
            model: model.clone(),
            messages: to_ollama_messages(messages),
            stream: false,
            options: to_ollama_options(&options),
        };

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api(format!("Ollama {}: {}", status, body)));
        }

        let ollama_resp: OllamaChatResponse = resp.json().await?;

        Ok(CompletionResponse {
            content: ollama_resp.message.content,
            model: ollama_resp.model,
            prompt_tokens: ollama_resp.prompt_eval_count,
            completion_tokens: ollama_resp.eval_count,
            total_tokens: ollama_resp.prompt_eval_count + ollama_resp.eval_count,
        })
    }

    async fn chat_completion_stream(
        &self,
        messages: &[ChatMessage],
        options: Option<CompletionOptions>,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), ProviderError> {
        let model = options
            .as_ref()
            .and_then(|o| o.model.clone())
            .unwrap_or_else(|| "llama3".to_string());

        let request = OllamaChatRequest {
            model,
            messages: to_ollama_messages(messages),
            stream: true,
            options: to_ollama_options(&options),
        };

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            let _ = tx.send(StreamEvent::Error { error: format!("Ollama {}: {}", status, body) }).await;
            let _ = tx.send(StreamEvent::Done).await;
            return Err(ProviderError::Api(format!("Ollama {}", status)));
        }

        let mut stream = resp.bytes_stream();
        use futures_util::StreamExt;
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));
                    // Ollama sends newline-delimited JSON
                    while let Some(pos) = buffer.find('\n') {
                        let line = buffer[..pos].to_string();
                        buffer = buffer[pos + 1..].to_string();
                        if line.trim().is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<OllamaChatResponse>(&line) {
                            Ok(resp) => {
                                if resp.done {
                                    let _ = tx.send(StreamEvent::Done).await;
                                    return Ok(());
                                }
                                if !resp.message.content.is_empty() {
                                    let _ = tx.send(StreamEvent::Token {
                                        content: resp.message.content,
                                    }).await;
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(StreamEvent::Error { error: e.to_string() }).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(StreamEvent::Error { error: e.to_string() }).await;
                    let _ = tx.send(StreamEvent::Done).await;
                    return Err(ProviderError::Network(e));
                }
            }
        }
        let _ = tx.send(StreamEvent::Done).await;
        Ok(())
    }

    async fn vision_completion(
        &self,
        image_data: Vec<u8>,
        prompt: &str,
        options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError> {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&image_data);

        let model = options
            .as_ref()
            .and_then(|o| o.model.clone())
            .unwrap_or_else(|| "llava".to_string());

        let request = OllamaChatRequest {
            model: model.clone(),
            messages: vec![OllamaMessage {
                role: "user".to_string(),
                content: prompt.to_string(),
                images: Some(vec![b64]),
            }],
            stream: false,
            options: to_ollama_options(&options),
        };

        let resp = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api(format!("Ollama {}: {}", status, body)));
        }

        let ollama_resp: OllamaChatResponse = resp.json().await?;

        Ok(CompletionResponse {
            content: ollama_resp.message.content,
            model: ollama_resp.model,
            prompt_tokens: ollama_resp.prompt_eval_count,
            completion_tokens: ollama_resp.eval_count,
            total_tokens: ollama_resp.prompt_eval_count + ollama_resp.eval_count,
        })
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        let resp = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await?;

        if resp.status().is_success() {
            Ok(())
        } else {
            Err(ProviderError::Unavailable)
        }
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "Ollama".to_string(),
            provider_type: ProviderType::Local,
            models: vec![], // populated by list_models()
        }
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let resp = self
            .client
            .get(format!("{}/api/tags", self.base_url))
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ProviderError::Unavailable);
        }

        let tags: OllamaTagsResponse = resp.json().await?;
        Ok(tags.models.into_iter().map(|m| m.name).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_ollama_messages() {
        let messages = vec![
            ChatMessage { role: ChatRole::System, content: "Be helpful.".to_string() },
            ChatMessage { role: ChatRole::User, content: "Hello".to_string() },
        ];
        let ollama_msgs = to_ollama_messages(&messages);
        assert_eq!(ollama_msgs.len(), 2);
        assert_eq!(ollama_msgs[0].role, "system");
        assert_eq!(ollama_msgs[1].role, "user");
    }

    #[test]
    fn test_to_ollama_options() {
        let opts = Some(CompletionOptions {
            temperature: Some(0.5),
            max_tokens: Some(100),
            ..Default::default()
        });
        let ollama_opts = to_ollama_options(&opts).unwrap();
        assert_eq!(ollama_opts.temperature, Some(0.5));
        assert_eq!(ollama_opts.num_predict, Some(100));
    }

    #[test]
    fn test_ollama_provider_info() {
        let provider = OllamaProvider::new("http://localhost:11434");
        let info = provider.info();
        assert_eq!(info.name, "Ollama");
        assert_eq!(info.provider_type, ProviderType::Local);
    }
}
```

- [ ] **Step 2: Add futures-util and base64 deps**

Add to `crates/vida-providers/Cargo.toml` under `[dependencies]`:
```toml
futures-util = "0.3"
base64 = "0.22"
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p vida-providers -- --nocapture
```

Expected: all 12 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/vida-providers/
git commit -m "feat(vida-providers): add OllamaProvider with chat, streaming, vision, health_check"
```

---

## Task 5: OpenAIProvider (`vida-providers`)

**Files:**
- Create: `crates/vida-providers/src/openai.rs`

- [ ] **Step 1: Write OpenAIProvider**

`crates/vida-providers/src/openai.rs`:
```rust
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

use crate::traits::*;

pub struct OpenAIProvider {
    client: Client,
    base_url: String,
    api_key: String,
    default_model: String,
}

impl OpenAIProvider {
    /// Create a new OpenAI-compatible provider.
    /// `base_url` can be any OpenAI-compatible endpoint (OpenAI, Groq, Mistral, Together, etc.)
    pub fn new(base_url: &str, api_key: &str, default_model: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            default_model: default_model.to_string(),
        }
    }
}

// ── OpenAI API types ──

#[derive(Serialize)]
struct OpenAIChatRequest {
    model: String,
    messages: Vec<OpenAIMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    stream: bool,
}

#[derive(Serialize)]
struct OpenAIMessage {
    role: String,
    content: serde_json::Value,
}

#[derive(Deserialize)]
struct OpenAIChatResponse {
    choices: Vec<OpenAIChoice>,
    model: String,
    #[serde(default)]
    usage: Option<OpenAIUsage>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIResponseMessage,
}

#[derive(Deserialize)]
struct OpenAIResponseMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Deserialize)]
struct OpenAIStreamChunk {
    choices: Vec<OpenAIStreamChoice>,
}

#[derive(Deserialize)]
struct OpenAIStreamChoice {
    delta: OpenAIStreamDelta,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIStreamDelta {
    content: Option<String>,
}

#[derive(Deserialize)]
struct OpenAIModelsResponse {
    data: Vec<OpenAIModelEntry>,
}

#[derive(Deserialize)]
struct OpenAIModelEntry {
    id: String,
}

#[derive(Deserialize)]
struct OpenAIErrorResponse {
    error: OpenAIErrorBody,
}

#[derive(Deserialize)]
struct OpenAIErrorBody {
    message: String,
}

// ── Helpers ──

fn to_openai_messages(messages: &[ChatMessage]) -> Vec<OpenAIMessage> {
    messages
        .iter()
        .map(|m| OpenAIMessage {
            role: match m.role {
                ChatRole::System => "system".to_string(),
                ChatRole::User => "user".to_string(),
                ChatRole::Assistant => "assistant".to_string(),
            },
            content: serde_json::Value::String(m.content.clone()),
        })
        .collect()
}

#[async_trait]
impl LLMProvider for OpenAIProvider {
    async fn chat_completion(
        &self,
        messages: &[ChatMessage],
        options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError> {
        let model = options
            .as_ref()
            .and_then(|o| o.model.clone())
            .unwrap_or_else(|| self.default_model.clone());

        let request = OpenAIChatRequest {
            model,
            messages: to_openai_messages(messages),
            temperature: options.as_ref().and_then(|o| o.temperature),
            max_tokens: options.as_ref().and_then(|o| o.max_tokens),
            top_p: options.as_ref().and_then(|o| o.top_p),
            stream: false,
        };

        let resp = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;

        if resp.status() == 401 {
            return Err(ProviderError::Unauthorized);
        }

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            if let Ok(err_resp) = serde_json::from_str::<OpenAIErrorResponse>(&body) {
                return Err(ProviderError::Api(err_resp.error.message));
            }
            return Err(ProviderError::Api(body));
        }

        let oai_resp: OpenAIChatResponse = resp.json().await?;
        let choice = oai_resp.choices.first().ok_or(ProviderError::Internal("No choices in response".to_string()))?;
        let usage = oai_resp.usage.unwrap_or(OpenAIUsage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 });

        Ok(CompletionResponse {
            content: choice.message.content.clone().unwrap_or_default(),
            model: oai_resp.model,
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        })
    }

    async fn chat_completion_stream(
        &self,
        messages: &[ChatMessage],
        options: Option<CompletionOptions>,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), ProviderError> {
        let model = options
            .as_ref()
            .and_then(|o| o.model.clone())
            .unwrap_or_else(|| self.default_model.clone());

        let request = OpenAIChatRequest {
            model,
            messages: to_openai_messages(messages),
            temperature: options.as_ref().and_then(|o| o.temperature),
            max_tokens: options.as_ref().and_then(|o| o.max_tokens),
            top_p: options.as_ref().and_then(|o| o.top_p),
            stream: true,
        };

        let resp = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            let _ = tx.send(StreamEvent::Error { error: body.clone() }).await;
            let _ = tx.send(StreamEvent::Done).await;
            return Err(ProviderError::Api(body));
        }

        use futures_util::StreamExt;
        let mut stream = resp.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk) = stream.next().await {
            match chunk {
                Ok(bytes) => {
                    buffer.push_str(&String::from_utf8_lossy(&bytes));
                    while let Some(pos) = buffer.find('\n') {
                        let line = buffer[..pos].trim().to_string();
                        buffer = buffer[pos + 1..].to_string();
                        if line.is_empty() || !line.starts_with("data: ") {
                            continue;
                        }
                        let data = &line[6..];
                        if data == "[DONE]" {
                            let _ = tx.send(StreamEvent::Done).await;
                            return Ok(());
                        }
                        match serde_json::from_str::<OpenAIStreamChunk>(data) {
                            Ok(chunk) => {
                                if let Some(choice) = chunk.choices.first() {
                                    if let Some(content) = &choice.delta.content {
                                        if !content.is_empty() {
                                            let _ = tx.send(StreamEvent::Token {
                                                content: content.clone(),
                                            }).await;
                                        }
                                    }
                                    if choice.finish_reason.is_some() {
                                        let _ = tx.send(StreamEvent::Done).await;
                                        return Ok(());
                                    }
                                }
                            }
                            Err(e) => {
                                let _ = tx.send(StreamEvent::Error { error: e.to_string() }).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    let _ = tx.send(StreamEvent::Error { error: e.to_string() }).await;
                    let _ = tx.send(StreamEvent::Done).await;
                    return Err(ProviderError::Network(e));
                }
            }
        }
        let _ = tx.send(StreamEvent::Done).await;
        Ok(())
    }

    async fn vision_completion(
        &self,
        image_data: Vec<u8>,
        prompt: &str,
        options: Option<CompletionOptions>,
    ) -> Result<CompletionResponse, ProviderError> {
        use base64::Engine;
        let b64 = base64::engine::general_purpose::STANDARD.encode(&image_data);

        let model = options
            .as_ref()
            .and_then(|o| o.model.clone())
            .unwrap_or_else(|| self.default_model.clone());

        let content = serde_json::json!([
            { "type": "text", "text": prompt },
            { "type": "image_url", "image_url": { "url": format!("data:image/png;base64,{}", b64) } }
        ]);

        let request = serde_json::json!({
            "model": model,
            "messages": [{ "role": "user", "content": content }],
            "max_tokens": options.as_ref().and_then(|o| o.max_tokens).unwrap_or(1024),
        });

        let resp = self
            .client
            .post(format!("{}/v1/chat/completions", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&request)
            .send()
            .await?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ProviderError::Api(body));
        }

        let oai_resp: OpenAIChatResponse = resp.json().await?;
        let choice = oai_resp.choices.first().ok_or(ProviderError::Internal("No choices".to_string()))?;
        let usage = oai_resp.usage.unwrap_or(OpenAIUsage { prompt_tokens: 0, completion_tokens: 0, total_tokens: 0 });

        Ok(CompletionResponse {
            content: choice.message.content.clone().unwrap_or_default(),
            model: oai_resp.model,
            prompt_tokens: usage.prompt_tokens,
            completion_tokens: usage.completion_tokens,
            total_tokens: usage.total_tokens,
        })
    }

    async fn health_check(&self) -> Result<(), ProviderError> {
        let resp = self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await?;

        if resp.status() == 401 {
            return Err(ProviderError::Unauthorized);
        }
        if resp.status().is_success() {
            Ok(())
        } else {
            Err(ProviderError::Unavailable)
        }
    }

    fn info(&self) -> ProviderInfo {
        ProviderInfo {
            name: "OpenAI".to_string(),
            provider_type: ProviderType::Cloud,
            models: vec![],
        }
    }

    async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
        let resp = self
            .client
            .get(format!("{}/v1/models", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(ProviderError::Unavailable);
        }

        let models: OpenAIModelsResponse = resp.json().await?;
        Ok(models.data.into_iter().map(|m| m.id).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_openai_messages() {
        let messages = vec![
            ChatMessage { role: ChatRole::User, content: "Hi".to_string() },
        ];
        let oai_msgs = to_openai_messages(&messages);
        assert_eq!(oai_msgs.len(), 1);
        assert_eq!(oai_msgs[0].role, "user");
    }

    #[test]
    fn test_openai_provider_info() {
        let provider = OpenAIProvider::new("https://api.openai.com", "sk-test", "gpt-4o");
        let info = provider.info();
        assert_eq!(info.name, "OpenAI");
        assert_eq!(info.provider_type, ProviderType::Cloud);
    }

    #[test]
    fn test_openai_compatible_base_url() {
        let provider = OpenAIProvider::new("https://api.groq.com/openai", "gsk-test", "llama3-70b");
        assert_eq!(provider.base_url, "https://api.groq.com/openai");
        assert_eq!(provider.default_model, "llama3-70b");
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p vida-providers -- --nocapture
```

Expected: all 15 tests PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/vida-providers/src/openai.rs
git commit -m "feat(vida-providers): add OpenAIProvider compatible with any OpenAI-compat endpoint"
```

---

## Task 6: Security — SecretStore trait + encryption (`vida-security`)

**Files:**
- Create: `crates/vida-security/src/keychain.rs`
- Create: `crates/vida-security/src/encryption.rs`

- [ ] **Step 1: Write SecretStore trait and KeychainManager**

`crates/vida-security/src/keychain.rs`:
```rust
use crate::SecurityError;

pub trait SecretStore: Send + Sync {
    fn store(&self, key: &str, value: &str) -> Result<(), SecurityError>;
    fn get(&self, key: &str) -> Result<String, SecurityError>;
    fn delete(&self, key: &str) -> Result<(), SecurityError>;
    fn list(&self) -> Result<Vec<String>, SecurityError>;
}

/// Production implementation using OS keychain via `keyring` crate.
pub struct KeychainManager {
    service: String,
    /// Track stored keys (keyring doesn't support listing)
    stored_keys: std::sync::Mutex<Vec<String>>,
}

impl KeychainManager {
    pub fn new(service: &str) -> Self {
        Self {
            service: service.to_string(),
            stored_keys: std::sync::Mutex::new(Vec::new()),
        }
    }
}

impl SecretStore for KeychainManager {
    fn store(&self, key: &str, value: &str) -> Result<(), SecurityError> {
        let entry = keyring::Entry::new(&self.service, key)
            .map_err(|e| SecurityError::KeychainAccess(e.to_string()))?;
        entry
            .set_password(value)
            .map_err(|e| SecurityError::KeychainAccess(e.to_string()))?;
        let mut keys = self.stored_keys.lock().unwrap();
        if !keys.contains(&key.to_string()) {
            keys.push(key.to_string());
        }
        Ok(())
    }

    fn get(&self, key: &str) -> Result<String, SecurityError> {
        let entry = keyring::Entry::new(&self.service, key)
            .map_err(|e| SecurityError::KeychainAccess(e.to_string()))?;
        entry
            .get_password()
            .map_err(|e| SecurityError::SecretNotFound(format!("{}: {}", key, e)))
    }

    fn delete(&self, key: &str) -> Result<(), SecurityError> {
        let entry = keyring::Entry::new(&self.service, key)
            .map_err(|e| SecurityError::KeychainAccess(e.to_string()))?;
        entry
            .delete_credential()
            .map_err(|e| SecurityError::KeychainAccess(e.to_string()))?;
        let mut keys = self.stored_keys.lock().unwrap();
        keys.retain(|k| k != key);
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>, SecurityError> {
        let keys = self.stored_keys.lock().unwrap();
        Ok(keys.clone())
    }
}

/// In-memory mock for testing — no OS keychain needed.
pub struct MockSecretStore {
    secrets: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl MockSecretStore {
    pub fn new() -> Self {
        Self {
            secrets: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }
}

impl Default for MockSecretStore {
    fn default() -> Self {
        Self::new()
    }
}

impl SecretStore for MockSecretStore {
    fn store(&self, key: &str, value: &str) -> Result<(), SecurityError> {
        self.secrets.lock().unwrap().insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn get(&self, key: &str) -> Result<String, SecurityError> {
        self.secrets
            .lock()
            .unwrap()
            .get(key)
            .cloned()
            .ok_or_else(|| SecurityError::SecretNotFound(key.to_string()))
    }

    fn delete(&self, key: &str) -> Result<(), SecurityError> {
        self.secrets.lock().unwrap().remove(key);
        Ok(())
    }

    fn list(&self) -> Result<Vec<String>, SecurityError> {
        Ok(self.secrets.lock().unwrap().keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_store_roundtrip() {
        let store = MockSecretStore::new();
        store.store("api-key", "sk-123").unwrap();
        assert_eq!(store.get("api-key").unwrap(), "sk-123");
    }

    #[test]
    fn test_mock_store_not_found() {
        let store = MockSecretStore::new();
        assert!(store.get("nonexistent").is_err());
    }

    #[test]
    fn test_mock_store_delete() {
        let store = MockSecretStore::new();
        store.store("key", "value").unwrap();
        store.delete("key").unwrap();
        assert!(store.get("key").is_err());
    }

    #[test]
    fn test_mock_store_list() {
        let store = MockSecretStore::new();
        store.store("a", "1").unwrap();
        store.store("b", "2").unwrap();
        let list = store.list().unwrap();
        assert_eq!(list.len(), 2);
    }
}
```

- [ ] **Step 2: Write encryption helpers**

`crates/vida-security/src/encryption.rs`:
```rust
use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;

use crate::SecurityError;

/// Encrypt data with AES-256-GCM. Returns nonce (12 bytes) + ciphertext as base64.
pub fn encrypt(key: &[u8; 32], plaintext: &[u8]) -> Result<String, SecurityError> {
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?;

    let mut combined = Vec::with_capacity(12 + ciphertext.len());
    combined.extend_from_slice(&nonce_bytes);
    combined.extend_from_slice(&ciphertext);

    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.encode(combined))
}

/// Decrypt base64(nonce + ciphertext) with AES-256-GCM.
pub fn decrypt(key: &[u8; 32], encoded: &str) -> Result<Vec<u8>, SecurityError> {
    use base64::Engine;
    let combined = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?;

    if combined.len() < 13 {
        return Err(SecurityError::EncryptionFailed("Data too short".to_string()));
    }

    let (nonce_bytes, ciphertext) = combined.split_at(12);
    let cipher = Aes256Gcm::new_from_slice(key)
        .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))?;
    let nonce = Nonce::from_slice(nonce_bytes);

    cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| SecurityError::EncryptionFailed(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let key = [42u8; 32];
        let plaintext = b"Hello, Vida AI!";
        let encrypted = encrypt(&key, plaintext).unwrap();
        let decrypted = decrypt(&key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let key1 = [42u8; 32];
        let key2 = [99u8; 32];
        let encrypted = encrypt(&key1, b"secret").unwrap();
        assert!(decrypt(&key2, &encrypted).is_err());
    }

    #[test]
    fn test_decrypt_short_data_fails() {
        let key = [42u8; 32];
        use base64::Engine;
        let short = base64::engine::general_purpose::STANDARD.encode([0u8; 5]);
        assert!(decrypt(&key, &short).is_err());
    }
}
```

- [ ] **Step 3: Write lib.rs with SecurityError**

`crates/vida-security/src/lib.rs`:
```rust
pub mod keychain;
pub mod pin;
pub mod encryption;

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

- [ ] **Step 4: Run tests**

```bash
cargo test -p vida-security -- --nocapture
```

Expected: 7 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vida-security/
git commit -m "feat(vida-security): add SecretStore trait, MockSecretStore, KeychainManager, AES-256-GCM encryption"
```

---

## Task 7: Security — PinManager (`vida-security`)

**Files:**
- Create: `crates/vida-security/src/pin.rs`

- [ ] **Step 1: Write PinManager with Argon2id**

`crates/vida-security/src/pin.rs`:
```rust
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2, Params,
};

use crate::SecurityError;

pub struct PinManager;

impl PinManager {
    /// Hash a password/PIN using Argon2id. Returns the PHC string (contains salt + hash).
    pub fn hash_password(password: &str) -> Result<String, SecurityError> {
        let salt = SaltString::generate(&mut OsRng);
        let params = Params::new(65536, 3, 4, None)
            .map_err(|e| SecurityError::HashingFailed(e.to_string()))?;
        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

        let hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| SecurityError::HashingFailed(e.to_string()))?;

        Ok(hash.to_string())
    }

    /// Verify a password/PIN against a stored PHC hash string.
    pub fn verify_password(password: &str, hash: &str) -> Result<bool, SecurityError> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| SecurityError::HashingFailed(e.to_string()))?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify_correct() {
        let hash = PinManager::hash_password("mySecurePin123").unwrap();
        assert!(PinManager::verify_password("mySecurePin123", &hash).unwrap());
    }

    #[test]
    fn test_verify_wrong_password() {
        let hash = PinManager::hash_password("correct").unwrap();
        assert!(!PinManager::verify_password("wrong", &hash).unwrap());
    }

    #[test]
    fn test_hash_is_unique_per_call() {
        let h1 = PinManager::hash_password("same").unwrap();
        let h2 = PinManager::hash_password("same").unwrap();
        assert_ne!(h1, h2); // Different salts
    }

    #[test]
    fn test_empty_password() {
        let hash = PinManager::hash_password("").unwrap();
        assert!(PinManager::verify_password("", &hash).unwrap());
        assert!(!PinManager::verify_password("x", &hash).unwrap());
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p vida-security -- --nocapture
```

Expected: 11 tests PASS (7 from Task 6 + 4 new).

- [ ] **Step 3: Commit**

```bash
git add crates/vida-security/src/pin.rs
git commit -m "feat(vida-security): add PinManager with Argon2id hashing (m=65536, t=3, p=4)"
```

---

## Task 8: Database — Models, Migrations, Repository (`vida-db`)

**Files:**
- Create: `crates/vida-db/src/models.rs`
- Create: `crates/vida-db/src/repository.rs`
- Create: `crates/vida-db/migrations/001_initial.sql`
- Modify: `crates/vida-db/src/lib.rs`

- [ ] **Step 1: Write the SQL migration**

`crates/vida-db/migrations/001_initial.sql`:
```sql
CREATE TABLE IF NOT EXISTS app_config (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS provider_configs (
    id            TEXT PRIMARY KEY,
    provider_type TEXT NOT NULL,
    base_url      TEXT,
    default_model TEXT,
    enabled       INTEGER NOT NULL DEFAULT 1,
    config_json   TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE IF NOT EXISTS sessions (
    id            TEXT PRIMARY KEY,
    title         TEXT,
    provider_id   TEXT NOT NULL,
    model         TEXT NOT NULL,
    system_prompt TEXT,
    created_at    TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at    TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (provider_id) REFERENCES provider_configs(id)
);

CREATE TABLE IF NOT EXISTS messages (
    id          TEXT PRIMARY KEY,
    session_id  TEXT NOT NULL,
    role        TEXT NOT NULL,
    content     TEXT NOT NULL,
    token_count INTEGER,
    created_at  TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS pin_config (
    id         INTEGER PRIMARY KEY CHECK (id = 1),
    hash       TEXT NOT NULL,
    salt       BLOB NOT NULL,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_messages_session ON messages(session_id, created_at);
CREATE INDEX IF NOT EXISTS idx_sessions_updated ON sessions(updated_at DESC);
```

- [ ] **Step 2: Write models**

`crates/vida-db/src/models.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProviderConfigRow {
    pub id: String,
    pub provider_type: String,
    pub base_url: Option<String>,
    pub default_model: Option<String>,
    pub enabled: i32,
    pub config_json: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct SessionRow {
    pub id: String,
    pub title: Option<String>,
    pub provider_id: String,
    pub model: String,
    pub system_prompt: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct MessageRow {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub token_count: Option<i32>,
    pub created_at: String,
}
```

- [ ] **Step 3: Write repository with tests**

`crates/vida-db/src/repository.rs`:
```rust
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

use crate::models::*;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("SQLx: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Migration: {0}")]
    Migration(#[from] sqlx::migrate::MigrateError),
    #[error("Not found: {0}")]
    NotFound(String),
}

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn connect(path: &str) -> Result<Self, DbError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(path)
            .await?;
        Ok(Self { pool })
    }

    pub async fn connect_in_memory() -> Result<Self, DbError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await?;
        // Enable foreign keys
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&pool)
            .await?;
        Ok(Self { pool })
    }

    pub async fn run_migrations(&self) -> Result<(), DbError> {
        sqlx::migrate!("./migrations").run(&self.pool).await?;
        Ok(())
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    // ── Config ──

    pub async fn get_config(&self, key: &str) -> Result<Option<String>, DbError> {
        let row: Option<(String,)> = sqlx::query_as("SELECT value FROM app_config WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| r.0))
    }

    pub async fn set_config(&self, key: &str, value: &str) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO app_config (key, value, updated_at) VALUES (?, ?, datetime('now'))
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')"
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Providers ──

    pub async fn upsert_provider(&self, config: &ProviderConfigRow) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO provider_configs (id, provider_type, base_url, default_model, enabled, config_json, created_at)
             VALUES (?, ?, ?, ?, ?, ?, datetime('now'))
             ON CONFLICT(id) DO UPDATE SET provider_type = excluded.provider_type, base_url = excluded.base_url,
             default_model = excluded.default_model, enabled = excluded.enabled, config_json = excluded.config_json"
        )
        .bind(&config.id)
        .bind(&config.provider_type)
        .bind(&config.base_url)
        .bind(&config.default_model)
        .bind(config.enabled)
        .bind(&config.config_json)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_providers(&self) -> Result<Vec<ProviderConfigRow>, DbError> {
        let rows = sqlx::query_as::<_, ProviderConfigRow>("SELECT * FROM provider_configs")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows)
    }

    pub async fn get_provider(&self, id: &str) -> Result<Option<ProviderConfigRow>, DbError> {
        let row = sqlx::query_as::<_, ProviderConfigRow>("SELECT * FROM provider_configs WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    // ── Sessions ──

    pub async fn create_session(&self, session: &SessionRow) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO sessions (id, title, provider_id, model, system_prompt, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, datetime('now'), datetime('now'))"
        )
        .bind(&session.id)
        .bind(&session.title)
        .bind(&session.provider_id)
        .bind(&session.model)
        .bind(&session.system_prompt)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_sessions(&self, limit: u32) -> Result<Vec<SessionRow>, DbError> {
        let rows = sqlx::query_as::<_, SessionRow>(
            "SELECT * FROM sessions ORDER BY updated_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_session(&self, id: &str) -> Result<Option<SessionRow>, DbError> {
        let row = sqlx::query_as::<_, SessionRow>("SELECT * FROM sessions WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    pub async fn delete_session(&self, id: &str) -> Result<(), DbError> {
        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Messages ──

    pub async fn insert_message(&self, msg: &MessageRow) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO messages (id, session_id, role, content, token_count, created_at)
             VALUES (?, ?, ?, ?, ?, datetime('now'))"
        )
        .bind(&msg.id)
        .bind(&msg.session_id)
        .bind(&msg.role)
        .bind(&msg.content)
        .bind(msg.token_count)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_messages(&self, session_id: &str) -> Result<Vec<MessageRow>, DbError> {
        let rows = sqlx::query_as::<_, MessageRow>(
            "SELECT * FROM messages WHERE session_id = ? ORDER BY created_at ASC"
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_db() -> Database {
        let db = Database::connect_in_memory().await.unwrap();
        db.run_migrations().await.unwrap();
        db
    }

    #[tokio::test]
    async fn test_config_set_and_get() {
        let db = setup_db().await;
        db.set_config("theme", "\"dark\"").await.unwrap();
        let val = db.get_config("theme").await.unwrap();
        assert_eq!(val, Some("\"dark\"".to_string()));
    }

    #[tokio::test]
    async fn test_config_upsert() {
        let db = setup_db().await;
        db.set_config("key", "v1").await.unwrap();
        db.set_config("key", "v2").await.unwrap();
        let val = db.get_config("key").await.unwrap();
        assert_eq!(val, Some("v2".to_string()));
    }

    #[tokio::test]
    async fn test_provider_upsert_and_list() {
        let db = setup_db().await;
        let provider = ProviderConfigRow {
            id: "ollama".to_string(),
            provider_type: "local".to_string(),
            base_url: Some("http://localhost:11434".to_string()),
            default_model: Some("llama3".to_string()),
            enabled: 1,
            config_json: None,
            created_at: String::new(),
        };
        db.upsert_provider(&provider).await.unwrap();
        let providers = db.list_providers().await.unwrap();
        assert_eq!(providers.len(), 1);
        assert_eq!(providers[0].id, "ollama");
    }

    #[tokio::test]
    async fn test_session_crud() {
        let db = setup_db().await;
        // Need a provider first
        let provider = ProviderConfigRow {
            id: "ollama".to_string(),
            provider_type: "local".to_string(),
            base_url: None,
            default_model: None,
            enabled: 1,
            config_json: None,
            created_at: String::new(),
        };
        db.upsert_provider(&provider).await.unwrap();

        let session = SessionRow {
            id: "sess-1".to_string(),
            title: Some("Test session".to_string()),
            provider_id: "ollama".to_string(),
            model: "llama3".to_string(),
            system_prompt: None,
            created_at: String::new(),
            updated_at: String::new(),
        };
        db.create_session(&session).await.unwrap();

        let sessions = db.list_sessions(10).await.unwrap();
        assert_eq!(sessions.len(), 1);

        let fetched = db.get_session("sess-1").await.unwrap();
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().title, Some("Test session".to_string()));
    }

    #[tokio::test]
    async fn test_messages_crud() {
        let db = setup_db().await;
        let provider = ProviderConfigRow {
            id: "ollama".to_string(),
            provider_type: "local".to_string(),
            base_url: None, default_model: None, enabled: 1, config_json: None, created_at: String::new(),
        };
        db.upsert_provider(&provider).await.unwrap();
        let session = SessionRow {
            id: "sess-1".to_string(), title: None, provider_id: "ollama".to_string(),
            model: "llama3".to_string(), system_prompt: None, created_at: String::new(), updated_at: String::new(),
        };
        db.create_session(&session).await.unwrap();

        let msg = MessageRow {
            id: "msg-1".to_string(), session_id: "sess-1".to_string(),
            role: "user".to_string(), content: "Hello".to_string(), token_count: Some(5), created_at: String::new(),
        };
        db.insert_message(&msg).await.unwrap();

        let messages = db.get_messages("sess-1").await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].content, "Hello");
    }

    #[tokio::test]
    async fn test_cascade_delete() {
        let db = setup_db().await;
        let provider = ProviderConfigRow {
            id: "ollama".to_string(), provider_type: "local".to_string(),
            base_url: None, default_model: None, enabled: 1, config_json: None, created_at: String::new(),
        };
        db.upsert_provider(&provider).await.unwrap();
        let session = SessionRow {
            id: "sess-1".to_string(), title: None, provider_id: "ollama".to_string(),
            model: "llama3".to_string(), system_prompt: None, created_at: String::new(), updated_at: String::new(),
        };
        db.create_session(&session).await.unwrap();
        let msg = MessageRow {
            id: "msg-1".to_string(), session_id: "sess-1".to_string(),
            role: "user".to_string(), content: "Hi".to_string(), token_count: None, created_at: String::new(),
        };
        db.insert_message(&msg).await.unwrap();

        // Delete session → messages should be cascade deleted
        db.delete_session("sess-1").await.unwrap();
        let messages = db.get_messages("sess-1").await.unwrap();
        assert!(messages.is_empty());
    }
}
```

- [ ] **Step 4: Update lib.rs**

`crates/vida-db/src/lib.rs`:
```rust
pub mod models;
pub mod repository;

pub use models::*;
pub use repository::{Database, DbError};
```

- [ ] **Step 5: Run tests**

```bash
cargo test -p vida-db -- --nocapture
```

Expected: 6 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add crates/vida-db/
git commit -m "feat(vida-db): add SQLite schema, models, repository with CRUD and cascade delete tests"
```

---

## Task 9: VidaError + AppConfig (`vida-core`)

**Files:**
- Create: `crates/vida-core/src/error.rs`
- Create: `crates/vida-core/src/config.rs`

- [ ] **Step 1: Write VidaError**

`crates/vida-core/src/error.rs`:
```rust
use vida_db::DbError;
use vida_providers::traits::ProviderError;
use vida_security::SecurityError;

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
    #[error("Provider not found: {0}")]
    ProviderNotFound(String),
    #[error("Config: {0}")]
    Config(String),
}

// Serialize for Tauri commands
impl serde::Serialize for VidaError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}
```

- [ ] **Step 2: Write AppConfig**

`crates/vida-core/src/config.rs`:
```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub language: String,
    pub theme: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            theme: "dark".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.language, "en");
        assert_eq!(config.theme, "dark");
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig { language: "fr".to_string(), theme: "light".to_string() };
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.language, "fr");
    }
}
```

- [ ] **Step 3: Update lib.rs**

`crates/vida-core/src/lib.rs`:
```rust
pub mod error;
pub mod config;
pub mod engine;

pub use error::VidaError;
pub use config::AppConfig;
```

- [ ] **Step 4: Run tests**

```bash
cargo test -p vida-core -- --nocapture
```

Expected: 2 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/vida-core/src/error.rs crates/vida-core/src/config.rs crates/vida-core/src/lib.rs
git commit -m "feat(vida-core): add VidaError (unified error) and AppConfig"
```

---

## Task 10: VidaEngine (`vida-core`)

**Files:**
- Create: `crates/vida-core/src/engine.rs`

This is the largest task — the central orchestrator. The engine integrates all crates.

- [ ] **Step 1: Write VidaEngine implementation**

`crates/vida-core/src/engine.rs`:
```rust
use std::path::Path;
use tokio::sync::mpsc;
use uuid::Uuid;

use vida_db::{Database, MessageRow, SessionRow};
use vida_providers::traits::*;
use vida_providers::registry::ProviderRegistry;
use vida_security::keychain::{SecretStore, KeychainManager, MockSecretStore};

use crate::config::AppConfig;
use crate::error::VidaError;

pub struct VidaEngine {
    pub db: Database,
    pub providers: ProviderRegistry,
    pub secrets: Box<dyn SecretStore>,
    pub config: AppConfig,
}

impl VidaEngine {
    /// Initialize with real OS keychain (production).
    pub async fn init(data_dir: &Path) -> Result<Self, VidaError> {
        let keychain = KeychainManager::new("vida-ai");
        Self::init_with_secrets(data_dir, Box::new(keychain)).await
    }

    /// Initialize with custom SecretStore (for testing).
    pub async fn init_with_secrets(
        data_dir: &Path,
        secrets: Box<dyn SecretStore>,
    ) -> Result<Self, VidaError> {
        std::fs::create_dir_all(data_dir).map_err(|e| VidaError::Config(e.to_string()))?;
        let db_path = format!("sqlite:{}/vida.db?mode=rwc", data_dir.display());
        let db = Database::connect(&db_path).await?;
        db.run_migrations().await?;

        // Load config from DB or use defaults
        let config = match db.get_config("app_config").await? {
            Some(json) => serde_json::from_str(&json).unwrap_or_default(),
            None => AppConfig::default(),
        };

        let providers = ProviderRegistry::new();

        Ok(Self { db, providers, secrets, config })
    }

    /// Initialize with in-memory DB (for testing).
    pub async fn init_in_memory() -> Result<Self, VidaError> {
        let db = Database::connect_in_memory().await?;
        db.run_migrations().await?;
        let secrets = Box::new(MockSecretStore::new());
        let config = AppConfig::default();
        let providers = ProviderRegistry::new();
        Ok(Self { db, providers, secrets, config })
    }

    // ── Chat ──

    pub async fn send_message(
        &self,
        session_id: &str,
        content: &str,
    ) -> Result<CompletionResponse, VidaError> {
        let session = self.db.get_session(session_id).await?
            .ok_or_else(|| VidaError::SessionNotFound(session_id.to_string()))?;

        let provider = self.providers.get(&session.provider_id)
            .ok_or_else(|| VidaError::ProviderNotFound(session.provider_id.clone()))?;

        // Load history
        let history = self.db.get_messages(session_id).await?;

        // Insert user message
        let user_msg = MessageRow {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: "user".to_string(),
            content: content.to_string(),
            token_count: None,
            created_at: String::new(),
        };
        self.db.insert_message(&user_msg).await?;

        // Build ChatMessage list
        let mut messages = Vec::new();
        if let Some(ref prompt) = session.system_prompt {
            messages.push(ChatMessage { role: ChatRole::System, content: prompt.clone() });
        }
        for msg in &history {
            let role = match msg.role.as_str() {
                "system" => ChatRole::System,
                "user" => ChatRole::User,
                _ => ChatRole::Assistant,
            };
            messages.push(ChatMessage { role, content: msg.content.clone() });
        }
        messages.push(ChatMessage { role: ChatRole::User, content: content.to_string() });

        let options = CompletionOptions {
            model: Some(session.model.clone()),
            ..Default::default()
        };

        let response = provider.chat_completion(&messages, Some(options)).await?;

        // Insert assistant message
        let assistant_msg = MessageRow {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: "assistant".to_string(),
            content: response.content.clone(),
            token_count: Some(response.total_tokens as i32),
            created_at: String::new(),
        };
        self.db.insert_message(&assistant_msg).await?;

        Ok(response)
    }

    pub async fn send_message_stream(
        &self,
        session_id: &str,
        content: &str,
        tx: mpsc::Sender<StreamEvent>,
    ) -> Result<(), VidaError> {
        let session = self.db.get_session(session_id).await?
            .ok_or_else(|| VidaError::SessionNotFound(session_id.to_string()))?;

        let provider = self.providers.get(&session.provider_id)
            .ok_or_else(|| VidaError::ProviderNotFound(session.provider_id.clone()))?;

        let history = self.db.get_messages(session_id).await?;

        // Insert user message
        let user_msg = MessageRow {
            id: Uuid::new_v4().to_string(),
            session_id: session_id.to_string(),
            role: "user".to_string(),
            content: content.to_string(),
            token_count: None,
            created_at: String::new(),
        };
        self.db.insert_message(&user_msg).await?;

        // Build messages
        let mut messages = Vec::new();
        if let Some(ref prompt) = session.system_prompt {
            messages.push(ChatMessage { role: ChatRole::System, content: prompt.clone() });
        }
        for msg in &history {
            let role = match msg.role.as_str() {
                "system" => ChatRole::System,
                "user" => ChatRole::User,
                _ => ChatRole::Assistant,
            };
            messages.push(ChatMessage { role, content: msg.content.clone() });
        }
        messages.push(ChatMessage { role: ChatRole::User, content: content.to_string() });

        let options = CompletionOptions {
            model: Some(session.model.clone()),
            ..Default::default()
        };

        // Intercept stream to collect full response
        let (inner_tx, mut inner_rx) = mpsc::channel::<StreamEvent>(100);
        let tx_clone = tx.clone();
        let db = &self.db;
        let sid = session_id.to_string();

        // Spawn provider streaming
        let provider_clone = provider.clone();
        let messages_clone = messages.clone();
        let options_clone = options.clone();
        tokio::spawn(async move {
            let _ = provider_clone.chat_completion_stream(&messages_clone, Some(options_clone), inner_tx).await;
        });

        // Forward events and collect tokens
        let mut full_content = String::new();
        while let Some(event) = inner_rx.recv().await {
            match &event {
                StreamEvent::Token { content } => {
                    full_content.push_str(content);
                }
                StreamEvent::Done => {
                    let _ = tx_clone.send(event).await;
                    break;
                }
                _ => {}
            }
            let _ = tx_clone.send(event).await;
        }

        // Insert assistant message with full content
        if !full_content.is_empty() {
            let assistant_msg = MessageRow {
                id: Uuid::new_v4().to_string(),
                session_id: sid,
                role: "assistant".to_string(),
                content: full_content,
                token_count: None,
                created_at: String::new(),
            };
            db.insert_message(&assistant_msg).await?;
        }

        Ok(())
    }

    // ── Sessions ──

    pub async fn create_session(
        &self,
        provider_id: &str,
        model: &str,
    ) -> Result<SessionRow, VidaError> {
        let session = SessionRow {
            id: Uuid::new_v4().to_string(),
            title: None,
            provider_id: provider_id.to_string(),
            model: model.to_string(),
            system_prompt: None,
            created_at: String::new(),
            updated_at: String::new(),
        };
        self.db.create_session(&session).await?;
        Ok(session)
    }

    pub async fn list_sessions(&self, limit: u32) -> Result<Vec<SessionRow>, VidaError> {
        Ok(self.db.list_sessions(limit).await?)
    }

    pub async fn get_session_messages(&self, session_id: &str) -> Result<Vec<MessageRow>, VidaError> {
        Ok(self.db.get_messages(session_id).await?)
    }

    pub async fn delete_session(&self, id: &str) -> Result<(), VidaError> {
        Ok(self.db.delete_session(id).await?)
    }

    // ── Providers ──

    pub fn list_providers(&self) -> Vec<ProviderInfo> {
        self.providers.list()
    }

    pub async fn list_models(&self, provider_id: &str) -> Result<Vec<String>, VidaError> {
        let provider = self.providers.get(provider_id)
            .ok_or_else(|| VidaError::ProviderNotFound(provider_id.to_string()))?;
        Ok(provider.list_models().await?)
    }

    pub async fn health_check_all(&self) -> Vec<(String, bool)> {
        self.providers.health_check_all().await
            .into_iter()
            .map(|(name, result)| (name, result.is_ok()))
            .collect()
    }

    // ── Security ──

    pub fn is_pin_configured(&self) -> Result<bool, VidaError> {
        // Check if pin_config table has a row
        // For Phase 1, delegate to DB query
        Ok(false) // Placeholder until we wire DB query
    }

    pub async fn store_api_key(&self, provider_id: &str, key: &str) -> Result<(), VidaError> {
        self.secrets.store(&format!("{}-api-key", provider_id), key)?;
        Ok(())
    }

    pub async fn remove_api_key(&self, provider_id: &str) -> Result<(), VidaError> {
        self.secrets.delete(&format!("{}-api-key", provider_id))?;
        Ok(())
    }

    pub fn get_api_key(&self, provider_id: &str) -> Result<String, VidaError> {
        Ok(self.secrets.get(&format!("{}-api-key", provider_id))?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use vida_providers::traits::*;
    use async_trait::async_trait;
    use vida_db::ProviderConfigRow;

    struct MockProvider;

    #[async_trait]
    impl LLMProvider for MockProvider {
        async fn chat_completion(&self, _: &[ChatMessage], _: Option<CompletionOptions>) -> Result<CompletionResponse, ProviderError> {
            Ok(CompletionResponse {
                content: "Hello from mock!".to_string(),
                model: "mock-model".to_string(),
                prompt_tokens: 10,
                completion_tokens: 5,
                total_tokens: 15,
            })
        }
        async fn chat_completion_stream(&self, _: &[ChatMessage], _: Option<CompletionOptions>, tx: mpsc::Sender<StreamEvent>) -> Result<(), ProviderError> {
            let _ = tx.send(StreamEvent::Token { content: "Hello ".to_string() }).await;
            let _ = tx.send(StreamEvent::Token { content: "world!".to_string() }).await;
            let _ = tx.send(StreamEvent::Done).await;
            Ok(())
        }
        async fn vision_completion(&self, _: Vec<u8>, _: &str, _: Option<CompletionOptions>) -> Result<CompletionResponse, ProviderError> {
            Err(ProviderError::Internal("not supported".to_string()))
        }
        async fn health_check(&self) -> Result<(), ProviderError> { Ok(()) }
        fn info(&self) -> ProviderInfo {
            ProviderInfo { name: "Mock".to_string(), provider_type: ProviderType::Local, models: vec!["mock-model".to_string()] }
        }
        async fn list_models(&self) -> Result<Vec<String>, ProviderError> {
            Ok(vec!["mock-model".to_string()])
        }
    }

    async fn setup_engine() -> VidaEngine {
        let mut engine = VidaEngine::init_in_memory().await.unwrap();

        // Register mock provider in DB and registry
        let config = ProviderConfigRow {
            id: "mock".to_string(),
            provider_type: "local".to_string(),
            base_url: None, default_model: Some("mock-model".to_string()),
            enabled: 1, config_json: None, created_at: String::new(),
        };
        engine.db.upsert_provider(&config).await.unwrap();
        engine.providers.add("mock".to_string(), Arc::new(MockProvider)).unwrap();
        engine
    }

    #[tokio::test]
    async fn test_engine_init_in_memory() {
        let engine = VidaEngine::init_in_memory().await;
        assert!(engine.is_ok());
    }

    #[tokio::test]
    async fn test_create_session() {
        let engine = setup_engine().await;
        let session = engine.create_session("mock", "mock-model").await.unwrap();
        assert_eq!(session.provider_id, "mock");
        assert_eq!(session.model, "mock-model");
    }

    #[tokio::test]
    async fn test_send_message() {
        let engine = setup_engine().await;
        let session = engine.create_session("mock", "mock-model").await.unwrap();
        let response = engine.send_message(&session.id, "Hi").await.unwrap();
        assert_eq!(response.content, "Hello from mock!");

        // Check messages in DB
        let messages = engine.get_session_messages(&session.id).await.unwrap();
        assert_eq!(messages.len(), 2); // user + assistant
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[1].role, "assistant");
    }

    #[tokio::test]
    async fn test_send_message_session_not_found() {
        let engine = VidaEngine::init_in_memory().await.unwrap();
        let result = engine.send_message("nonexistent", "Hi").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_store_and_get_api_key() {
        let engine = VidaEngine::init_in_memory().await.unwrap();
        engine.store_api_key("openai", "sk-test123").await.unwrap();
        let key = engine.get_api_key("openai").unwrap();
        assert_eq!(key, "sk-test123");
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let engine = setup_engine().await;
        engine.create_session("mock", "mock-model").await.unwrap();
        engine.create_session("mock", "mock-model").await.unwrap();
        let sessions = engine.list_sessions(10).await.unwrap();
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_delete_session_cascades() {
        let engine = setup_engine().await;
        let session = engine.create_session("mock", "mock-model").await.unwrap();
        engine.send_message(&session.id, "test").await.unwrap();
        engine.delete_session(&session.id).await.unwrap();
        let messages = engine.get_session_messages(&session.id).await.unwrap();
        assert!(messages.is_empty());
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test -p vida-core -- --nocapture
```

Expected: 7 tests PASS.

- [ ] **Step 3: Run full workspace tests**

```bash
cargo test --workspace -- --nocapture
```

Expected: all tests across all crates PASS.

- [ ] **Step 4: Commit**

```bash
git add crates/vida-core/
git commit -m "feat(vida-core): add VidaEngine with chat, sessions, providers, security integration"
```

---

## Task 11: Tauri v2 Bootstrap (`src-tauri`)

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/tauri.conf.json`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/auth.rs`
- Create: `src-tauri/src/commands/providers.rs`
- Create: `src-tauri/src/commands/chat.rs`
- Create: `src-tauri/src/commands/config.rs`

- [ ] **Step 1: Create src-tauri/Cargo.toml**

```toml
[package]
name = "vida-ai"
version.workspace = true
edition.workspace = true

[dependencies]
tauri = { version = "2", features = [] }
tauri-build = { version = "2", features = [] }
tokio = { workspace = true }
serde = { workspace = true }
serde_json = { workspace = true }
vida-core = { path = "../crates/vida-core" }
vida-providers = { path = "../crates/vida-providers" }

[build-dependencies]
tauri-build = { version = "2", features = [] }
```

- [ ] **Step 2: Create tauri.conf.json**

`src-tauri/tauri.conf.json`:
```json
{
  "$schema": "https://raw.githubusercontent.com/nicedoc/tauri/tauri-v2/crates/tauri-cli/config.schema.json",
  "productName": "Vida AI",
  "version": "0.1.0",
  "identifier": "ai.vida.app",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:3000",
    "beforeDevCommand": "npm run dev",
    "beforeBuildCommand": "npm run build"
  },
  "app": {
    "windows": [
      {
        "title": "Vida AI",
        "width": 1200,
        "height": 800,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ]
  }
}
```

- [ ] **Step 3: Create build.rs**

`src-tauri/build.rs`:
```rust
fn main() {
    tauri_build::build()
}
```

- [ ] **Step 4: Create Tauri commands**

`src-tauri/src/commands/mod.rs`:
```rust
pub mod auth;
pub mod providers;
pub mod chat;
pub mod config;
```

`src-tauri/src/commands/auth.rs`:
```rust
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use vida_core::VidaEngine;

#[tauri::command]
pub async fn is_pin_configured(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<bool, String> {
    let e = engine.read().await;
    e.is_pin_configured().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn store_api_key(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    provider_id: String,
    key: String,
) -> Result<(), String> {
    let e = engine.read().await;
    e.store_api_key(&provider_id, &key).await.map_err(|e| e.to_string())
}
```

`src-tauri/src/commands/providers.rs`:
```rust
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use vida_core::VidaEngine;
use vida_providers::traits::ProviderInfo;

#[tauri::command]
pub async fn list_providers(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<Vec<ProviderInfo>, String> {
    let e = engine.read().await;
    Ok(e.list_providers())
}

#[tauri::command]
pub async fn list_models(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    provider_id: String,
) -> Result<Vec<String>, String> {
    let e = engine.read().await;
    e.list_models(&provider_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn health_check(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<Vec<(String, bool)>, String> {
    let e = engine.read().await;
    Ok(e.health_check_all().await)
}
```

`src-tauri/src/commands/chat.rs`:
```rust
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::{mpsc, RwLock};
use vida_core::VidaEngine;
use vida_providers::traits::{CompletionResponse, StreamEvent};

#[tauri::command]
pub async fn send_message(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    session_id: String,
    content: String,
) -> Result<CompletionResponse, String> {
    let e = engine.read().await;
    e.send_message(&session_id, &content).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stream_completion(
    app: AppHandle,
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    session_id: String,
    content: String,
) -> Result<(), String> {
    let (tx, mut rx) = mpsc::channel::<StreamEvent>(100);
    let event_name = format!("llm-stream-{}", session_id);

    let e = engine.read().await;
    let sid = session_id.clone();

    // Spawn the streaming in background
    tokio::spawn({
        let engine_ref = engine.inner().clone();
        async move {
            let e = engine_ref.read().await;
            let _ = e.send_message_stream(&sid, &content, tx).await;
        }
    });

    // Forward events to frontend
    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let is_done = matches!(event, StreamEvent::Done);
            let _ = app.emit(&event_name, &event);
            if is_done { break; }
        }
    });

    Ok(())
}

#[tauri::command]
pub async fn create_session(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    provider_id: String,
    model: String,
) -> Result<vida_db::SessionRow, String> {
    let e = engine.read().await;
    e.create_session(&provider_id, &model).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_sessions(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    limit: u32,
) -> Result<Vec<vida_db::SessionRow>, String> {
    let e = engine.read().await;
    e.list_sessions(limit).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_messages(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    session_id: String,
) -> Result<Vec<vida_db::MessageRow>, String> {
    let e = engine.read().await;
    e.get_session_messages(&session_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_session(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
    session_id: String,
) -> Result<(), String> {
    let e = engine.read().await;
    e.delete_session(&session_id).await.map_err(|e| e.to_string())
}
```

`src-tauri/src/commands/config.rs`:
```rust
use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;
use vida_core::{AppConfig, VidaEngine};

#[tauri::command]
pub async fn get_config(
    engine: State<'_, Arc<RwLock<VidaEngine>>>,
) -> Result<AppConfig, String> {
    let e = engine.read().await;
    Ok(e.config.clone())
}
```

- [ ] **Step 5: Create main.rs**

`src-tauri/src/main.rs`:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;

use std::sync::Arc;
use tokio::sync::RwLock;
use vida_core::VidaEngine;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let data_dir = app.path().app_data_dir().expect("Failed to get app data dir");
            let rt = tokio::runtime::Runtime::new().unwrap();
            let engine = rt.block_on(VidaEngine::init(&data_dir))
                .expect("Failed to initialize VidaEngine");
            app.manage(Arc::new(RwLock::new(engine)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::auth::is_pin_configured,
            commands::auth::store_api_key,
            commands::providers::list_providers,
            commands::providers::list_models,
            commands::providers::health_check,
            commands::chat::send_message,
            commands::chat::stream_completion,
            commands::chat::create_session,
            commands::chat::list_sessions,
            commands::chat::get_messages,
            commands::chat::delete_session,
            commands::config::get_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running vida-ai");
}
```

- [ ] **Step 6: Verify compilation**

```bash
cargo check --workspace
```

Expected: compiles (Tauri may show warnings about missing icons — that's fine for now).

- [ ] **Step 7: Commit**

```bash
git add src-tauri/
git commit -m "feat(src-tauri): add Tauri v2 bootstrap with 12 IPC commands"
```

---

## Task 12: Frontend i18n Setup

**Files:**
- Modify: `package.json`
- Create: `src/locales/en/common.json`
- Create: `src/locales/zh-CN/common.json`
- Create: `src/locales/fr/common.json`
- Create: `src/lib/tauri.ts`

- [ ] **Step 1: Add i18next dependencies**

```bash
cd /home/hackos0911/AI/projects/IA/Vida\ ui && npm install react-i18next i18next i18next-browser-languagedetector
```

- [ ] **Step 2: Create locale files**

`src/locales/en/common.json`:
```json
{
  "app": {
    "name": "Vida AI",
    "loading": "Loading...",
    "error": "An error occurred"
  },
  "auth": {
    "enterPassword": "Enter your password",
    "unlock": "Unlock",
    "wrongPassword": "Wrong password",
    "noPasswordSet": "No password configured"
  },
  "providers": {
    "title": "Providers",
    "noProviders": "No providers configured",
    "addApiKey": "Add API Key",
    "healthy": "Connected",
    "unhealthy": "Unavailable"
  },
  "chat": {
    "placeholder": "Type a message...",
    "send": "Send",
    "newSession": "New Chat"
  },
  "settings": {
    "title": "Settings",
    "language": "Language",
    "theme": "Theme",
    "changePassword": "Change Password",
    "removePassword": "Remove Password"
  }
}
```

`src/locales/zh-CN/common.json`:
```json
{
  "app": {
    "name": "Vida AI",
    "loading": "加载中...",
    "error": "发生错误"
  },
  "auth": {
    "enterPassword": "请输入密码",
    "unlock": "解锁",
    "wrongPassword": "密码错误",
    "noPasswordSet": "未设置密码"
  },
  "providers": {
    "title": "提供商",
    "noProviders": "未配置提供商",
    "addApiKey": "添加 API 密钥",
    "healthy": "已连接",
    "unhealthy": "不可用"
  },
  "chat": {
    "placeholder": "输入消息...",
    "send": "发送",
    "newSession": "新对话"
  },
  "settings": {
    "title": "设置",
    "language": "语言",
    "theme": "主题",
    "changePassword": "修改密码",
    "removePassword": "移除密码"
  }
}
```

`src/locales/fr/common.json`:
```json
{
  "app": {
    "name": "Vida AI",
    "loading": "Chargement...",
    "error": "Une erreur est survenue"
  },
  "auth": {
    "enterPassword": "Entrez votre mot de passe",
    "unlock": "Déverrouiller",
    "wrongPassword": "Mot de passe incorrect",
    "noPasswordSet": "Aucun mot de passe configuré"
  },
  "providers": {
    "title": "Fournisseurs",
    "noProviders": "Aucun fournisseur configuré",
    "addApiKey": "Ajouter une clé API",
    "healthy": "Connecté",
    "unhealthy": "Indisponible"
  },
  "chat": {
    "placeholder": "Tapez un message...",
    "send": "Envoyer",
    "newSession": "Nouvelle conversation"
  },
  "settings": {
    "title": "Paramètres",
    "language": "Langue",
    "theme": "Thème",
    "changePassword": "Changer le mot de passe",
    "removePassword": "Supprimer le mot de passe"
  }
}
```

- [ ] **Step 3: Create Tauri typed wrappers**

`src/lib/tauri.ts`:
```typescript
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
```

- [ ] **Step 4: Commit**

```bash
git add src/locales/ src/lib/tauri.ts package.json
git commit -m "feat(frontend): add i18n locale files (en, zh-CN, fr) and typed Tauri API wrappers"
```

---

## Task 13: Final — Workspace Verification

- [ ] **Step 1: Run full test suite**

```bash
cargo test --workspace -- --nocapture
```

Expected: ALL tests pass across vida-providers, vida-security, vida-db, vida-core.

- [ ] **Step 2: Check workspace compiles**

```bash
cargo check --workspace
```

Expected: no errors.

- [ ] **Step 3: Verify file structure**

```bash
find crates/ src-tauri/ src/locales/ src/lib/ -type f | sort
```

Expected: all files from the File Map are present.

- [ ] **Step 4: Run frontend build**

```bash
npm run lint
```

Expected: TypeScript type-check passes.

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "chore: Phase 1 complete — Vida AI Core foundation verified"
```
