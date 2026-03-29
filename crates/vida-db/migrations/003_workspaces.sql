CREATE TABLE IF NOT EXISTS recent_workspaces (
    path       TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    last_used  TEXT NOT NULL DEFAULT (datetime('now'))
);
