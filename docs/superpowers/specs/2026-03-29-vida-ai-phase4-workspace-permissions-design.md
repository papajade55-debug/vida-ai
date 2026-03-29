# Vida AI — Phase 4 Design Spec: Workspace Manager + Permissions

**Date:** 2026-03-29
**Status:** Approved
**Scope:** Phase 4 — Workspace management (.vida/config.json) + Permission system (Yolo/Ask/Sandbox)
**Depends on:** Phases 1, 2A, 2B, 3

## 1. Workspace Manager

### 1.1 Concept
A workspace = a directory on disk containing a `.vida/config.json` file. Each workspace has its own configuration: default provider/model, system prompt, permission mode. The app remembers the last used workspace.

### 1.2 Config File
```json
{
  "name": "My Project",
  "default_provider": "ollama",
  "default_model": "llama3",
  "system_prompt": "You are a helpful assistant for this project.",
  "permission_mode": "ask",
  "permissions": {
    "file_read": true,
    "file_write": false,
    "shell_execute": false,
    "network_access": true
  }
}
```

### 1.3 Backend (vida-core)
- `WorkspaceManager` struct: load/save `.vida/config.json`, list recent workspaces
- Recent workspaces list stored in app SQLite DB (table `recent_workspaces`)
- Tauri commands: `open_workspace(path)`, `create_workspace(path, name)`, `list_recent_workspaces`, `get_workspace_config`, `set_workspace_config`
- On workspace open: load config, apply permission mode, set default provider

### 1.4 Frontend
- Workspace selector dropdown at top of Sidebar (shows current workspace name)
- "Open Folder" button → Tauri file dialog to pick directory
- Recent workspaces list in dropdown
- Workspace settings in Settings modal (new tab)

### 1.5 DB Table
```sql
CREATE TABLE IF NOT EXISTS recent_workspaces (
    path       TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    last_used  TEXT NOT NULL DEFAULT (datetime('now'))
);
```

## 2. Permission System

### 2.1 Three Modes
- **Yolo**: All actions allowed without confirmation
- **Ask**: Popup confirmation for each action (default)
- **Sandbox**: All actions denied by default, user must explicitly grant per-action

### 2.2 Permission Types
- `file_read` — Read files from disk
- `file_write` — Write/create/delete files
- `shell_execute` — Execute shell commands
- `network_access` — Make network requests (beyond LLM API calls)

### 2.3 Backend (vida-core)
```rust
pub enum PermissionMode { Yolo, Ask, Sandbox }

pub enum PermissionType { FileRead, FileWrite, ShellExecute, NetworkAccess }

pub struct PermissionManager {
    mode: PermissionMode,
    grants: HashMap<PermissionType, bool>,
}

impl PermissionManager {
    pub fn check(&self, perm: PermissionType) -> PermissionResult;
    pub fn grant(&mut self, perm: PermissionType);
    pub fn revoke(&mut self, perm: PermissionType);
}

pub enum PermissionResult { Allowed, Denied, NeedsApproval }
```

- `check()` in Yolo mode → always Allowed
- `check()` in Sandbox mode → Denied unless explicitly granted
- `check()` in Ask mode → NeedsApproval (frontend must confirm)

### 2.4 Permission Flow (Ask mode)
1. Backend needs to do an action (e.g., write file)
2. Calls `permission_manager.check(FileWrite)`
3. Returns `NeedsApproval`
4. Backend emits Tauri Event `permission-request` with `{action, path, description}`
5. Frontend shows confirmation popup
6. User clicks Allow/Deny
7. Frontend invokes `respond_permission(request_id, allowed)`
8. Backend receives response, proceeds or aborts

### 2.5 Tauri Commands
- `get_permission_mode() → String`
- `set_permission_mode(mode: String)`
- `respond_permission(request_id: String, allowed: bool)`
- Permission requests via Tauri Events (not commands)

### 2.6 Frontend
- Permission popup component: shows action description, Allow/Deny/Allow Always buttons
- Permission settings in workspace config tab (toggle per permission type)
- Mode selector in workspace settings (Yolo/Ask/Sandbox radio buttons)

## 3. Out of Scope
- File editing tools (Phase 5 — MCP)
- Shell execution UI (Phase 5)
- Workspace sync between machines
