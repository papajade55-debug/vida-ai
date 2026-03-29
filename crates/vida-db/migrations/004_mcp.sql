CREATE TABLE IF NOT EXISTS mcp_server_configs (
    id             TEXT PRIMARY KEY,
    workspace_path TEXT,
    name           TEXT NOT NULL,
    command        TEXT NOT NULL,
    args_json      TEXT,
    env_json       TEXT,
    enabled        INTEGER NOT NULL DEFAULT 1,
    created_at     TEXT NOT NULL DEFAULT (datetime('now'))
);
