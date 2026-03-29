# Vida AI — Phase 6 Design Spec: Remote Access + Packaging + Open Source

**Date:** 2026-03-29
**Status:** Approved
**Scope:** Phase 6 — Telegram bot, HTTP/WebSocket server, packaging, open-source release
**Depends on:** Phases 1-5

## 1. Remote Access Module

### 1.1 Embedded HTTP/WebSocket Server
- Lightweight HTTP server embedded in vida-core (using `axum` or `warp`)
- Serves a minimal REST API mirroring Tauri commands
- WebSocket endpoint for streaming
- Optional — disabled by default, user enables in settings
- Configurable port (default: 3690)

### 1.2 Endpoints
```
POST /api/chat/send          {session_id, content} → CompletionResponse
POST /api/chat/stream        {session_id, content} → WebSocket upgrade → StreamEvents
POST /api/sessions/create    {provider_id, model} → SessionRow
GET  /api/sessions            → Vec<SessionRow>
GET  /api/providers           → Vec<ProviderInfo>
GET  /api/health              → {status: "ok", providers: [...]}
```

### 1.3 Authentication
- Bearer token auth (token generated on first enable, stored in keychain)
- Rate limiting: 60 req/min per token

### 1.4 Telegram Bot Connector
- Uses `teloxide` crate (Rust Telegram bot framework)
- User configures: bot token + allowed chat IDs in settings
- Commands: `/chat <message>` → sends to default session, `/models` → list, `/health` → status
- Streaming: sends partial messages as edits (Telegram edit_message_text)
- Feature-gated: `#[cfg(feature = "telegram")]`

## 2. Packaging

### 2.1 Desktop Builds (Tauri Bundler)
- Linux: `.deb`, `.AppImage`
- macOS: `.dmg`
- Windows: `.exe` (NSIS installer)
- Built via `cargo tauri build` with CI/CD (GitHub Actions)

### 2.2 Server/Headless Mode
- `vida-ai --headless` flag → starts only the HTTP/WS server, no GUI
- Docker: `Dockerfile` with multi-stage build (Rust compile → minimal runtime)
- LXC: install script for Proxmox (`install-lxc.sh`)

### 2.3 CI/CD (GitHub Actions)
```yaml
# .github/workflows/build.yml
jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    steps:
      - cargo test --workspace
      - npm run build
      - cargo tauri build
      - Upload artifacts
  release:
    on: tag push (v*)
    steps: create GitHub release with all platform builds
```

## 3. Open Source Release

### 3.1 License
- MIT License (permissive, maximizes adoption)

### 3.2 Repository Structure
```
vida-ai/
├── LICENSE
├── README.md
├── CONTRIBUTING.md
├── .github/
│   ├── workflows/build.yml
│   └── ISSUE_TEMPLATE/
├── Cargo.toml
├── crates/
├── src-tauri/
├── src/
└── docs/
```

### 3.3 README.md
- Logo + screenshots
- Features list
- Quick start (npm install, npm run dev)
- Provider setup guide (Ollama, OpenAI, Anthropic, Google)
- Build from source instructions
- Contributing guide link

## 4. Implementation Scope for This Phase

Focus on what's implementable now:
1. ✅ Embedded HTTP server (axum) with REST endpoints
2. ✅ WebSocket streaming endpoint
3. ✅ Bearer token auth
4. ✅ Telegram bot connector (teloxide)
5. ✅ Headless mode flag
6. ✅ Dockerfile
7. ✅ LXC install script
8. ✅ LICENSE + README + CONTRIBUTING
9. ⏭ CI/CD (needs GitHub repo — defer to actual publish)
10. ⏭ Tauri bundler builds (needs platform-specific testing)
