use serde::{Deserialize, Serialize};

use crate::access::ActorRole;
use vida_db::UserRow;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthSession {
    pub user_id: String,
    pub username: String,
    pub role: ActorRole,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthStatus {
    pub has_users: bool,
    pub actor: Option<AuthSession>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: String,
    pub username: String,
    pub role: ActorRole,
    pub active: bool,
    pub created_at: String,
}

impl TryFrom<&UserRow> for AuthSession {
    type Error = String;

    fn try_from(value: &UserRow) -> Result<Self, Self::Error> {
        Ok(Self {
            user_id: value.id.clone(),
            username: value.username.clone(),
            role: parse_actor_role(&value.role)?,
        })
    }
}

impl TryFrom<UserRow> for AuthUser {
    type Error = String;

    fn try_from(value: UserRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            username: value.username,
            role: parse_actor_role(&value.role)?,
            active: value.active != 0,
            created_at: value.created_at,
        })
    }
}

pub fn parse_actor_role(role: &str) -> Result<ActorRole, String> {
    match role.trim().to_ascii_lowercase().as_str() {
        "super_admin" => Ok(ActorRole::SuperAdmin),
        "architect" => Ok(ActorRole::Architect),
        "operator" => Ok(ActorRole::Operator),
        "agent" => Ok(ActorRole::Agent),
        other => Err(format!("Unknown actor role '{other}'")),
    }
}

pub fn actor_role_storage(role: ActorRole) -> &'static str {
    match role {
        ActorRole::SuperAdmin => "super_admin",
        ActorRole::Architect => "architect",
        ActorRole::Operator => "operator",
        ActorRole::Agent => "agent",
    }
}
