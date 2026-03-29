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
pub struct MessageRow {
    pub id: String,
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub token_count: Option<i32>,
    pub created_at: String,
    pub agent_id: Option<String>,
    pub agent_name: Option<String>,
    pub agent_color: Option<String>,
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
    pub team_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamRow {
    pub id: String,
    pub name: String,
    pub mode: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TeamMemberRow {
    pub id: String,
    pub team_id: String,
    pub provider_id: String,
    pub model: String,
    pub display_name: Option<String>,
    pub color: String,
    pub role: Option<String>,
    pub created_at: String,
}
