use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActorRole {
    SuperAdmin,
    Architect,
    Operator,
    Agent,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccessAction {
    Read,
    Create,
    Modify,
    Delete,
    Execute,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AccessResource {
    SystemFiles,
    ProjectFiles,
    TeamConfig,
    IaConfig,
    CriticalCode,
    LogsAudit,
    ShellEscalation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessDecision {
    Allow,
    Deny,
    RequireHumanApproval,
}

#[derive(Debug, Clone)]
pub struct AccessRequest {
    pub actor: ActorRole,
    pub action: AccessAction,
    pub resource: AccessResource,
}

#[derive(Debug, Clone)]
pub struct AgentToolContext {
    pub workspace_root: Option<PathBuf>,
    pub sandbox_root: PathBuf,
}

pub fn evaluate_access(request: &AccessRequest) -> AccessDecision {
    use AccessAction::*;
    use AccessDecision::*;
    use AccessResource::*;
    use ActorRole::*;

    match request.actor {
        SuperAdmin => match (request.resource, request.action) {
            (TeamConfig, Create | Modify | Delete) => RequireHumanApproval,
            (CriticalCode, Modify | Delete) => RequireHumanApproval,
            _ => Allow,
        },
        Architect => match (request.resource, request.action) {
            (SystemFiles, Read) => Allow,
            (ProjectFiles, _) => Allow,
            (IaConfig, Modify) => Allow,
            (TeamConfig, Create | Modify | Delete) => RequireHumanApproval,
            (CriticalCode, Modify | Delete) => RequireHumanApproval,
            (ShellEscalation, Execute) => RequireHumanApproval,
            (LogsAudit, Delete) => Deny,
            _ => Deny,
        },
        Operator => match (request.resource, request.action) {
            (ProjectFiles, Read | Create) => Allow,
            (ShellEscalation, Execute) => RequireHumanApproval,
            _ => Deny,
        },
        Agent => match (request.resource, request.action) {
            (ProjectFiles, Read | Create | Modify | Delete) => Allow,
            _ => Deny,
        },
    }
}

pub fn authorize_agent_tool_call(
    tool_name: &str,
    arguments: &serde_json::Value,
    context: &AgentToolContext,
) -> Result<(), String> {
    let action = infer_tool_action(tool_name);

    if matches!(action, AccessAction::Execute) {
        return Err("Agent shell escalation is forbidden".to_string());
    }

    let paths = collect_candidate_paths(arguments, context);
    if matches!(
        action,
        AccessAction::Create | AccessAction::Modify | AccessAction::Delete
    ) && paths.is_empty()
    {
        return Err(format!(
            "Agent write operation '{tool_name}' is missing an explicit path and was denied"
        ));
    }

    if paths.is_empty() {
        return Ok(());
    }

    for path in paths {
        if matches!(action, AccessAction::Read) {
            let Some(workspace_root) = &context.workspace_root else {
                return Err(format!(
                    "Agent read operation '{tool_name}' requires an open workspace"
                ));
            };

            if !path.starts_with(workspace_root) && !path.starts_with(&context.sandbox_root) {
                return Err(format!(
                    "Agent read outside the project scope is forbidden: {}",
                    path.display()
                ));
            }
        }

        if matches!(
            action,
            AccessAction::Create | AccessAction::Modify | AccessAction::Delete
        ) && !path.starts_with(&context.sandbox_root)
        {
            return Err(format!(
                "Agent write outside the sandbox is forbidden: {}",
                path.display()
            ));
        }

        let resource = classify_path(&path, context);
        let decision = evaluate_access(&AccessRequest {
            actor: ActorRole::Agent,
            action,
            resource,
        });

        if !matches!(decision, AccessDecision::Allow) {
            return Err(format!(
                "Agent access denied for {:?} on {}",
                resource,
                path.display()
            ));
        }
    }

    Ok(())
}

pub fn infer_tool_action(tool_name: &str) -> AccessAction {
    let lower = tool_name.to_ascii_lowercase();

    if ["bash", "shell", "exec", "execute", "command", "run"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        AccessAction::Execute
    } else if ["delete", "remove"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        AccessAction::Delete
    } else if ["create", "new", "save", "write"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        AccessAction::Create
    } else if ["edit", "modify", "update", "rename", "move"]
        .iter()
        .any(|needle| lower.contains(needle))
    {
        AccessAction::Modify
    } else {
        AccessAction::Read
    }
}

pub fn classify_path(path: &Path, context: &AgentToolContext) -> AccessResource {
    if path.starts_with(&context.sandbox_root) {
        return AccessResource::ProjectFiles;
    }

    if looks_like_logs_or_audit(path) {
        return AccessResource::LogsAudit;
    }

    if let Some(workspace_root) = &context.workspace_root {
        if path.starts_with(workspace_root) {
            if looks_like_critical_code(path) {
                return AccessResource::CriticalCode;
            }
            return AccessResource::ProjectFiles;
        }
    }

    AccessResource::SystemFiles
}

fn collect_candidate_paths(value: &serde_json::Value, context: &AgentToolContext) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    collect_paths_recursive(value, None, context, &mut paths);
    paths
}

fn collect_paths_recursive(
    value: &serde_json::Value,
    parent_key: Option<&str>,
    context: &AgentToolContext,
    out: &mut Vec<PathBuf>,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map {
                collect_paths_recursive(child, Some(key), context, out);
            }
        }
        serde_json::Value::Array(items) => {
            for child in items {
                collect_paths_recursive(child, parent_key, context, out);
            }
        }
        serde_json::Value::String(text) => {
            let Some(key) = parent_key else {
                return;
            };
            let key = key.to_ascii_lowercase();
            if [
                "path",
                "file",
                "filepath",
                "target_path",
                "source_path",
                "destination_path",
            ]
            .iter()
            .any(|needle| key.contains(needle))
            {
                out.push(resolve_path(text, context));
            }
        }
        _ => {}
    }
}

fn resolve_path(raw: &str, context: &AgentToolContext) -> PathBuf {
    let path = PathBuf::from(raw);
    if path.is_absolute() {
        path
    } else if let Some(workspace_root) = &context.workspace_root {
        workspace_root.join(path)
    } else {
        context.sandbox_root.join(path)
    }
}

fn looks_like_critical_code(path: &Path) -> bool {
    const CRITICAL_FILES: &[&str] = &[
        "cargo.toml",
        "cargo.lock",
        "package.json",
        "pnpm-lock.yaml",
        "package-lock.json",
        "tauri.conf.json",
        "vite.config.ts",
        "vite.config.js",
        "install-lxc.sh",
    ];
    const CRITICAL_EXTENSIONS: &[&str] = &[
        "rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "c", "cpp", "h", "hpp", "toml", "yaml",
        "yml", "json", "sql", "sh",
    ];
    const CRITICAL_DIRS: &[&str] = &["src", "src-tauri", "crates", "scripts"];

    if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
        let lower = name.to_ascii_lowercase();
        if CRITICAL_FILES.contains(&lower.as_str()) {
            return true;
        }
    }

    if path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .any(|component| CRITICAL_DIRS.contains(&component))
    {
        return true;
    }

    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| CRITICAL_EXTENSIONS.contains(&ext))
        .unwrap_or(false)
}

fn looks_like_logs_or_audit(path: &Path) -> bool {
    path.components()
        .filter_map(|component| component.as_os_str().to_str())
        .any(|component| {
            let lower = component.to_ascii_lowercase();
            lower.contains("log") || lower.contains("audit")
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_super_admin_needs_approval_for_team_config() {
        let decision = evaluate_access(&AccessRequest {
            actor: ActorRole::SuperAdmin,
            action: AccessAction::Modify,
            resource: AccessResource::TeamConfig,
        });
        assert_eq!(decision, AccessDecision::RequireHumanApproval);
    }

    #[test]
    fn test_operator_denied_ia_config() {
        let decision = evaluate_access(&AccessRequest {
            actor: ActorRole::Operator,
            action: AccessAction::Modify,
            resource: AccessResource::IaConfig,
        });
        assert_eq!(decision, AccessDecision::Deny);
    }

    #[test]
    fn test_agent_denied_shell() {
        let context = AgentToolContext {
            workspace_root: Some(PathBuf::from("/workspace")),
            sandbox_root: PathBuf::from("/workspace/.vida/sandboxes/team-a"),
        };
        let result =
            authorize_agent_tool_call("exec_shell", &serde_json::json!({"command":"ls"}), &context);
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_write_must_stay_in_sandbox() {
        let context = AgentToolContext {
            workspace_root: Some(PathBuf::from("/workspace")),
            sandbox_root: PathBuf::from("/workspace/.vida/sandboxes/team-a"),
        };
        let result = authorize_agent_tool_call(
            "write_file",
            &serde_json::json!({"path":"src/main.rs","content":"fn main() {}"}),
            &context,
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_agent_write_in_sandbox_allowed() {
        let context = AgentToolContext {
            workspace_root: Some(PathBuf::from("/workspace")),
            sandbox_root: PathBuf::from("/workspace/.vida/sandboxes/team-a"),
        };
        let result = authorize_agent_tool_call(
            "write_file",
            &serde_json::json!({
                "path":"/workspace/.vida/sandboxes/team-a/output.txt",
                "content":"ok"
            }),
            &context,
        );
        assert!(result.is_ok());
    }
}
