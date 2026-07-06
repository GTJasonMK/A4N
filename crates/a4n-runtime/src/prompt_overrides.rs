//! 项目级提示词覆盖在 system_settings 中的持久化边界。

use a4n_core::models::ai::system_settings;
use a4n_core::models::project::novel_project;
use sea_orm::{DatabaseConnection, EntityTrait};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectOverrideIdentity {
    pub(crate) project_id: i64,
    pub(crate) name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MissingProjectMessage {
    Command,
    Hydrate,
}

pub(crate) fn normalize_project_override_identity(
    project_id: i64,
    name: &str,
) -> Result<ProjectOverrideIdentity, String> {
    let project_id = validate_project_override_project_id(project_id)?;
    let name = name.trim();
    validate_project_override_name(name, "提示词覆盖 name")?;

    Ok(ProjectOverrideIdentity {
        project_id,
        name: name.to_string(),
    })
}

pub(crate) fn parse_project_override_key(
    key: &str,
) -> Result<Option<ProjectOverrideIdentity>, String> {
    let Some(rest) = key.strip_prefix(system_settings::PROMPT_OVERRIDE_KEY_PREFIX) else {
        return Ok(None);
    };
    let (project_id, name) = rest
        .split_once('_')
        .ok_or_else(|| format!("提示词覆盖 key 格式无效: {key}"))?;
    let project_id = project_id
        .parse::<i64>()
        .map_err(|_| format!("提示词覆盖 key project_id 无效: {key}"))?;
    let project_id = validate_project_override_project_id(project_id)
        .map_err(|_| format!("提示词覆盖 key project_id 必须为正整数: {key}"))?;
    if name.trim() != name {
        return Err(format!("提示词覆盖 key name 不能包含首尾空白: {key}"));
    }
    validate_project_override_name(name, "提示词覆盖 key name")
        .map_err(|err| format!("{err}: {key}"))?;

    Ok(Some(ProjectOverrideIdentity {
        project_id,
        name: name.to_string(),
    }))
}

pub(crate) fn project_override_key(identity: &ProjectOverrideIdentity) -> String {
    system_settings::prompt_override_key(identity.project_id, &identity.name)
}

pub(crate) fn validate_project_override_content(content: &str) -> Result<(), String> {
    if content.trim().is_empty() {
        return Err("提示词覆盖内容不能为空".to_string());
    }
    Ok(())
}

pub(crate) fn validate_project_override_project_id(project_id: i64) -> Result<i64, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    Ok(project_id)
}

pub(crate) async fn ensure_project_exists(
    db: &DatabaseConnection,
    project_id: i64,
    missing_message: MissingProjectMessage,
) -> Result<(), String> {
    let exists = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(|e| e.to_string())?
        .is_some();
    if exists {
        return Ok(());
    }

    match missing_message {
        MissingProjectMessage::Command => Err(format!("项目不存在: {project_id}")),
        MissingProjectMessage::Hydrate => Err(format!("提示词覆盖关联项目不存在: {project_id}")),
    }
}

fn validate_project_override_name(name: &str, label: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err(format!("{label} 不能为空"));
    }
    if name.chars().any(char::is_control) {
        return Err(format!("{label} 不能包含控制字符"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_project_override_key_accepts_prompt_names_with_underscores() {
        let identity = parse_project_override_key("prompt_override_42_foo_bar")
            .unwrap()
            .unwrap();

        assert_eq!(identity.project_id, 42);
        assert_eq!(identity.name, "foo_bar");
    }

    #[test]
    fn parse_project_override_key_ignores_unrelated_settings() {
        assert_eq!(parse_project_override_key("llm_config").unwrap(), None);
    }

    #[test]
    fn parse_project_override_key_rejects_blank_name() {
        let err = parse_project_override_key("prompt_override_42_")
            .expect_err("blank persisted override name should fail");

        assert!(err.contains("提示词覆盖 key name 不能为空"));
    }

    #[test]
    fn parse_project_override_key_rejects_name_with_boundary_whitespace() {
        let err = parse_project_override_key("prompt_override_42_ prompt")
            .expect_err("persisted override key names must not be guessed by trimming");

        assert!(err.contains("提示词覆盖 key name 不能包含首尾空白"));
    }

    #[test]
    fn parse_project_override_key_rejects_control_character_name() {
        let err = parse_project_override_key("prompt_override_42_bad\nname")
            .expect_err("control characters are not valid prompt override names");

        assert!(err.contains("提示词覆盖 key name 不能包含控制字符"));
    }

    #[test]
    fn normalize_project_override_identity_trims_user_input_name() {
        let identity = normalize_project_override_identity(42, " prompt ").unwrap();

        assert_eq!(
            identity,
            ProjectOverrideIdentity {
                project_id: 42,
                name: "prompt".to_string(),
            }
        );
    }
}
