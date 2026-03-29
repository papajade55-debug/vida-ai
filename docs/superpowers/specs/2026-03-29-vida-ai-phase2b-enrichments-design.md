# Vida AI — Phase 2B Design Spec: File Import, Vision, Voice, Providers, Settings

**Date:** 2026-03-29
**Status:** Approved
**Scope:** Phase 2B — Drag&drop files, Vision UI, Voice input (Whisper.cpp), Anthropic/Google providers, Settings modal
**Depends on:** Phase 1, 2A, 3

## 1. Overview

Phase 2B enriches Vida AI with file handling (drag&drop + vision), voice input via local Whisper.cpp, two new LLM providers (Anthropic, Google), and a complete Settings UI.

## 2. Drag & Drop Files

### 2.1 Approach
- HTML5 native drag events (`ondrop`/`ondragover`) on ChatArea — no Tauri plugin needed
- Files are read via `FileReader` in the browser
- Images: if provider supports vision → call `vision_completion`. If not → describe image via a small vision-capable model first
- Text files (.txt, .md, .py, .rs, .json, etc.): content appended to message as code block

### 2.2 Frontend Components
- Modify `ChatArea.tsx`: add drop zone overlay (shows "Drop files here" on drag)
- Create `src/components/chat/FilePreview.tsx`: thumbnail preview for images, filename+size for text files, remove button
- Modify `ChatInput.tsx`: track attached files in local state, send with message

### 2.3 Backend
- No backend changes needed — `vision_completion` already exists in the trait
- Frontend sends image as base64 via a new Tauri command `send_vision_message(session_id, image_base64, prompt)`
- Add command to `src-tauri/src/commands/chat.rs`

## 3. Anthropic Provider

### 3.1 Implementation
- Create `crates/vida-providers/src/anthropic.rs`
- HTTP to `https://api.anthropic.com` (configurable base_url)
- Uses `/v1/messages` endpoint (NOT OpenAI-compatible)
- Auth: `x-api-key` header (not Bearer)
- Streaming: SSE with `event: content_block_delta`, `data: {"delta":{"text":"..."}}`
- Vision: content array with `{"type":"image","source":{"type":"base64","media_type":"...","data":"..."}}`

### 3.2 Key Differences from OpenAI
- System prompt is a top-level `system` field, not a message
- Streaming events are `content_block_delta` not `choices[0].delta`
- Stop reason field: `stop_reason` not `finish_reason`
- Model names: `claude-3-5-sonnet-20241022`, etc.

## 4. Google Gemini Provider

### 4.1 Implementation
- Create `crates/vida-providers/src/google.rs`
- HTTP to `https://generativelanguage.googleapis.com` (configurable)
- Uses `/v1beta/models/{model}:generateContent` (non-streaming)
- Uses `/v1beta/models/{model}:streamGenerateContent?alt=sse` (streaming)
- Auth: `key` query parameter (API key)
- Vision: inline_data with base64 in parts array

### 4.2 Key Differences
- Messages are called "contents" with "parts"
- Roles: "user" and "model" (not "assistant")
- System instruction is a separate `system_instruction` field
- Streaming uses SSE but response format differs

## 5. Voice Input (Whisper.cpp)

### 5.1 Architecture
- Rust crate: `whisper-rs` (bindings to whisper.cpp)
- Audio capture: `cpal` crate (cross-platform audio input)
- Model: Whisper Base (~150MB, good speed/quality tradeoff)
- Model file stored in app data dir, downloaded on first use

### 5.2 Flow
1. User clicks microphone button in ChatInput
2. Frontend invokes `start_recording` Tauri command
3. Backend starts `cpal` audio capture (16kHz mono WAV)
4. User clicks stop (or silence detection after 3s)
5. Frontend invokes `stop_recording` → returns audio buffer
6. Backend runs Whisper inference → returns transcribed text
7. Text injected into ChatInput

### 5.3 New Crate: vida-voice (optional)
- Could be a new crate or stay in vida-core
- Decision: keep in vida-core for now (YAGNI on separate crate)
- Feature-gated: `#[cfg(feature = "voice")]` so it can be disabled

### 5.4 Frontend
- Add microphone button to ChatInput (lucide `Mic` icon)
- Button states: idle → recording (red pulse) → processing (spinner)
- Create `src/hooks/useVoiceInput.ts`

## 6. Settings Modal (complete)

### 6.1 Sections
1. **General**: Language selector (en/zh-CN/fr), Theme toggle
2. **Security**: Change password, Remove password
3. **Providers**: List all providers with health status. Add/edit API key per provider. Test connection button.
4. **Default Model**: Select default provider + model for new solo sessions
5. **About**: Version, links

### 6.2 Frontend
- Expand existing `src/components/settings/SettingsModal.tsx` with tabbed interface
- Create `src/components/settings/GeneralSettings.tsx`
- Create `src/components/settings/SecuritySettings.tsx`
- Create `src/components/settings/ProviderSettings.tsx`
- Modify existing `src/components/settings/ApiKeyForm.tsx`

## 7. Out of Scope
- Whisper model auto-download UI (Phase 4+)
- Voice output / TTS
- File editing (Phase 4 — workspace)
- OCR on images
