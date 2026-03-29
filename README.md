# Vida AI

**Your AI team, your rules.**

A cross-platform desktop AI assistant with multi-provider support, team-based multi-agent collaboration, and a Liquid Glass UI. Built with Rust (Tauri v2) and React 19. Zero vendor lock-in — your data stays local.

## Features

- **Multi-provider**: Ollama (local), OpenAI, Anthropic, Google Gemini — any OpenAI-compatible endpoint
- **Team multi-agent**: Create teams, all agents respond in parallel, compare outputs side-by-side
- **Liquid Glass UI**: Adaptive glassmorphism design with light/dark themes
- **MCP integration**: Launch and manage MCP servers, route tool calls to any model
- **Workspace & permissions**: Per-directory configs, Yolo/Ask/Sandbox permission modes
- **Remote access**: Embedded HTTP/WebSocket server + Telegram bot connector
- **Vision**: Drag & drop images, automatic vision model routing
- **i18n**: English, 简体中文, Français — community translations welcome
- **Open source**: MIT license, no telemetry, no cloud dependency

## Quick Start

```bash
# Install frontend dependencies
npm install

# Start development server
npm run dev

# Run the desktop app (requires Tauri CLI)
cargo tauri dev
```

## Provider Setup

| Provider | Setup |
|----------|-------|
| **Ollama** | Install from [ollama.ai](https://ollama.ai), runs locally on port 11434 |
| **OpenAI** | Add API key in Settings → Providers |
| **Anthropic** | Add API key in Settings → Providers |
| **Google Gemini** | Add API key in Settings → Providers |
| **Any OpenAI-compatible** | Use OpenAI provider with custom base URL (Groq, Mistral, Together, etc.) |

## Build from Source

### Prerequisites
- [Rust](https://rustup.rs/) (1.77+)
- [Node.js](https://nodejs.org/) (22+)
- [Tauri CLI](https://tauri.app/start/): `cargo install tauri-cli`

### Commands
```bash
git clone https://github.com/vida-ai/vida-ai.git
cd vida-ai
npm install
cargo tauri build
```

Builds output to `src-tauri/target/release/bundle/` (.deb, .AppImage, .dmg, .exe).

## Architecture

Cargo workspace with 4 library crates + 1 Tauri binary:

```
crates/
├── vida-providers   # LLMProvider trait + 4 providers (Ollama, OpenAI, Anthropic, Google)
├── vida-security    # OS Keychain + PIN (Argon2id) + AES-256-GCM
├── vida-db          # SQLite + SQLx + migrations
└── vida-core        # VidaEngine orchestration, MCP, permissions, workspaces, remote
```

Frontend: React 19 + TypeScript + Tailwind CSS 4 + Framer Motion + Zustand.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

[MIT](LICENSE) — free to use, modify, and distribute.
