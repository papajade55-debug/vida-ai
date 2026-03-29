-- Teams table
CREATE TABLE IF NOT EXISTS teams (
    id         TEXT PRIMARY KEY,
    name       TEXT NOT NULL,
    mode       TEXT NOT NULL DEFAULT 'parallel',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Team members table
CREATE TABLE IF NOT EXISTS team_members (
    id           TEXT PRIMARY KEY,
    team_id      TEXT NOT NULL,
    provider_id  TEXT NOT NULL,
    model        TEXT NOT NULL,
    display_name TEXT,
    color        TEXT NOT NULL DEFAULT '#6366f1',
    role         TEXT,
    created_at   TEXT NOT NULL DEFAULT (datetime('now')),
    FOREIGN KEY (team_id) REFERENCES teams(id) ON DELETE CASCADE,
    FOREIGN KEY (provider_id) REFERENCES provider_configs(id)
);

CREATE INDEX IF NOT EXISTS idx_team_members_team ON team_members(team_id);

-- Add team_id to sessions (nullable for solo sessions)
ALTER TABLE sessions ADD COLUMN team_id TEXT;

-- Add agent fields to messages (nullable for solo sessions)
ALTER TABLE messages ADD COLUMN agent_id TEXT;
ALTER TABLE messages ADD COLUMN agent_name TEXT;
ALTER TABLE messages ADD COLUMN agent_color TEXT;
