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
            "INSERT INTO sessions (id, title, provider_id, model, system_prompt, team_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))"
        )
        .bind(&session.id)
        .bind(&session.title)
        .bind(&session.provider_id)
        .bind(&session.model)
        .bind(&session.system_prompt)
        .bind(&session.team_id)
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
            "INSERT INTO messages (id, session_id, role, content, token_count, agent_id, agent_name, agent_color, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))"
        )
        .bind(&msg.id)
        .bind(&msg.session_id)
        .bind(&msg.role)
        .bind(&msg.content)
        .bind(msg.token_count)
        .bind(&msg.agent_id)
        .bind(&msg.agent_name)
        .bind(&msg.agent_color)
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

    // ── Teams ──

    pub async fn create_team(&self, team: &TeamRow) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO teams (id, name, mode, created_at)
             VALUES (?, ?, ?, datetime('now'))"
        )
        .bind(&team.id)
        .bind(&team.name)
        .bind(&team.mode)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_teams(&self) -> Result<Vec<TeamRow>, DbError> {
        let rows = sqlx::query_as::<_, TeamRow>(
            "SELECT * FROM teams ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn get_team(&self, id: &str) -> Result<Option<TeamRow>, DbError> {
        let row = sqlx::query_as::<_, TeamRow>("SELECT * FROM teams WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row)
    }

    pub async fn delete_team(&self, id: &str) -> Result<(), DbError> {
        sqlx::query("DELETE FROM teams WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn add_team_member(&self, member: &TeamMemberRow) -> Result<(), DbError> {
        sqlx::query(
            "INSERT INTO team_members (id, team_id, provider_id, model, display_name, color, role, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, datetime('now'))"
        )
        .bind(&member.id)
        .bind(&member.team_id)
        .bind(&member.provider_id)
        .bind(&member.model)
        .bind(&member.display_name)
        .bind(&member.color)
        .bind(&member.role)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_team_members(&self, team_id: &str) -> Result<Vec<TeamMemberRow>, DbError> {
        let rows = sqlx::query_as::<_, TeamMemberRow>(
            "SELECT * FROM team_members WHERE team_id = ? ORDER BY created_at ASC"
        )
        .bind(team_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows)
    }

    pub async fn remove_team_member(&self, id: &str) -> Result<(), DbError> {
        sqlx::query("DELETE FROM team_members WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
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
            team_id: None,
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
            team_id: None,
        };
        db.create_session(&session).await.unwrap();

        let msg = MessageRow {
            id: "msg-1".to_string(), session_id: "sess-1".to_string(),
            role: "user".to_string(), content: "Hello".to_string(), token_count: Some(5), created_at: String::new(),
            agent_id: None, agent_name: None, agent_color: None,
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
            team_id: None,
        };
        db.create_session(&session).await.unwrap();
        let msg = MessageRow {
            id: "msg-1".to_string(), session_id: "sess-1".to_string(),
            role: "user".to_string(), content: "Hi".to_string(), token_count: None, created_at: String::new(),
            agent_id: None, agent_name: None, agent_color: None,
        };
        db.insert_message(&msg).await.unwrap();

        // Delete session -> messages should be cascade deleted
        db.delete_session("sess-1").await.unwrap();
        let messages = db.get_messages("sess-1").await.unwrap();
        assert!(messages.is_empty());
    }

    // ── Team Tests ──

    async fn setup_db_with_provider() -> Database {
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
        db
    }

    #[tokio::test]
    async fn test_team_create_and_get() {
        let db = setup_db().await;
        let team = TeamRow {
            id: "team-1".to_string(),
            name: "Test Team".to_string(),
            mode: "parallel".to_string(),
            created_at: String::new(),
        };
        db.create_team(&team).await.unwrap();

        let fetched = db.get_team("team-1").await.unwrap();
        assert!(fetched.is_some());
        let fetched = fetched.unwrap();
        assert_eq!(fetched.name, "Test Team");
        assert_eq!(fetched.mode, "parallel");
    }

    #[tokio::test]
    async fn test_team_list() {
        let db = setup_db().await;
        let team1 = TeamRow {
            id: "team-1".to_string(),
            name: "Alpha".to_string(),
            mode: "parallel".to_string(),
            created_at: String::new(),
        };
        let team2 = TeamRow {
            id: "team-2".to_string(),
            name: "Beta".to_string(),
            mode: "parallel".to_string(),
            created_at: String::new(),
        };
        db.create_team(&team1).await.unwrap();
        db.create_team(&team2).await.unwrap();

        let teams = db.list_teams().await.unwrap();
        assert_eq!(teams.len(), 2);
    }

    #[tokio::test]
    async fn test_team_delete() {
        let db = setup_db().await;
        let team = TeamRow {
            id: "team-1".to_string(),
            name: "Delete Me".to_string(),
            mode: "parallel".to_string(),
            created_at: String::new(),
        };
        db.create_team(&team).await.unwrap();
        db.delete_team("team-1").await.unwrap();

        let fetched = db.get_team("team-1").await.unwrap();
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn test_team_member_crud() {
        let db = setup_db_with_provider().await;
        let team = TeamRow {
            id: "team-1".to_string(),
            name: "Test Team".to_string(),
            mode: "parallel".to_string(),
            created_at: String::new(),
        };
        db.create_team(&team).await.unwrap();

        let member = TeamMemberRow {
            id: "member-1".to_string(),
            team_id: "team-1".to_string(),
            provider_id: "ollama".to_string(),
            model: "llama3".to_string(),
            display_name: Some("Agent Alpha".to_string()),
            color: "#6366f1".to_string(),
            role: None,
            created_at: String::new(),
        };
        db.add_team_member(&member).await.unwrap();

        let members = db.get_team_members("team-1").await.unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].display_name, Some("Agent Alpha".to_string()));
        assert_eq!(members[0].color, "#6366f1");

        // Add second member
        let member2 = TeamMemberRow {
            id: "member-2".to_string(),
            team_id: "team-1".to_string(),
            provider_id: "ollama".to_string(),
            model: "mistral".to_string(),
            display_name: None,
            color: "#ec4899".to_string(),
            role: Some("reviewer".to_string()),
            created_at: String::new(),
        };
        db.add_team_member(&member2).await.unwrap();

        let members = db.get_team_members("team-1").await.unwrap();
        assert_eq!(members.len(), 2);

        // Remove first member
        db.remove_team_member("member-1").await.unwrap();
        let members = db.get_team_members("team-1").await.unwrap();
        assert_eq!(members.len(), 1);
        assert_eq!(members[0].id, "member-2");
    }

    #[tokio::test]
    async fn test_team_cascade_delete_members() {
        let db = setup_db_with_provider().await;
        let team = TeamRow {
            id: "team-1".to_string(),
            name: "Cascade Team".to_string(),
            mode: "parallel".to_string(),
            created_at: String::new(),
        };
        db.create_team(&team).await.unwrap();

        let member = TeamMemberRow {
            id: "member-1".to_string(),
            team_id: "team-1".to_string(),
            provider_id: "ollama".to_string(),
            model: "llama3".to_string(),
            display_name: None,
            color: "#6366f1".to_string(),
            role: None,
            created_at: String::new(),
        };
        db.add_team_member(&member).await.unwrap();

        // Deleting team should cascade-delete members
        db.delete_team("team-1").await.unwrap();
        let members = db.get_team_members("team-1").await.unwrap();
        assert!(members.is_empty());
    }

    #[tokio::test]
    async fn test_session_with_team_id() {
        let db = setup_db_with_provider().await;
        let team = TeamRow {
            id: "team-1".to_string(),
            name: "Team Session".to_string(),
            mode: "parallel".to_string(),
            created_at: String::new(),
        };
        db.create_team(&team).await.unwrap();

        let session = SessionRow {
            id: "sess-team".to_string(),
            title: Some("Team chat".to_string()),
            provider_id: "ollama".to_string(),
            model: "llama3".to_string(),
            system_prompt: None,
            created_at: String::new(),
            updated_at: String::new(),
            team_id: Some("team-1".to_string()),
        };
        db.create_session(&session).await.unwrap();

        let fetched = db.get_session("sess-team").await.unwrap().unwrap();
        assert_eq!(fetched.team_id, Some("team-1".to_string()));
    }

    #[tokio::test]
    async fn test_message_with_agent_fields() {
        let db = setup_db_with_provider().await;
        let session = SessionRow {
            id: "sess-1".to_string(), title: None, provider_id: "ollama".to_string(),
            model: "llama3".to_string(), system_prompt: None, created_at: String::new(),
            updated_at: String::new(), team_id: None,
        };
        db.create_session(&session).await.unwrap();

        let msg = MessageRow {
            id: "msg-agent".to_string(),
            session_id: "sess-1".to_string(),
            role: "assistant".to_string(),
            content: "Hello from agent!".to_string(),
            token_count: None,
            created_at: String::new(),
            agent_id: Some("member-1".to_string()),
            agent_name: Some("Agent Alpha".to_string()),
            agent_color: Some("#6366f1".to_string()),
        };
        db.insert_message(&msg).await.unwrap();

        let messages = db.get_messages("sess-1").await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].agent_id, Some("member-1".to_string()));
        assert_eq!(messages[0].agent_name, Some("Agent Alpha".to_string()));
        assert_eq!(messages[0].agent_color, Some("#6366f1".to_string()));
    }
}
