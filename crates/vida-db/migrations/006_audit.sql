CREATE TABLE IF NOT EXISTS audit_events (
    id TEXT PRIMARY KEY,
    actor_username TEXT,
    actor_role TEXT,
    event_type TEXT NOT NULL,
    resource TEXT,
    details_json TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_audit_events_created_at
    ON audit_events(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_audit_events_actor_username
    ON audit_events(actor_username);
