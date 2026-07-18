use a4n_core::models::ai::system_settings;

use super::types::invalid_argument;
use super::{ProjectOverrideIdentity, PromptOverrideDbError};

pub fn normalize_project_override_identity(
    project_id: i64,
    name: &str,
) -> Result<ProjectOverrideIdentity, PromptOverrideDbError> {
    let project_id = validate_project_override_project_id(project_id)?;
    let name = name.trim();
    validate_project_override_name(name, "提示词覆盖 name")?;

    Ok(ProjectOverrideIdentity {
        project_id,
        name: name.to_string(),
    })
}

pub fn parse_project_override_key(
    key: &str,
) -> Result<Option<ProjectOverrideIdentity>, PromptOverrideDbError> {
    let Some(rest) = key.strip_prefix(system_settings::PROMPT_OVERRIDE_KEY_PREFIX) else {
        return Ok(None);
    };
    let (project_id, name) = rest
        .split_once('_')
        .ok_or_else(|| invalid_argument(format!("提示词覆盖 key 格式无效: {key}")))?;
    let project_id = project_id
        .parse::<i64>()
        .map_err(|_| invalid_argument(format!("提示词覆盖 key project_id 无效: {key}")))?;
    let project_id = validate_project_override_project_id(project_id)
        .map_err(|_| invalid_argument(format!("提示词覆盖 key project_id 必须为正整数: {key}")))?;
    if name.trim() != name {
        return Err(invalid_argument(format!(
            "提示词覆盖 key name 不能包含首尾空白: {key}"
        )));
    }
    validate_project_override_name(name, "提示词覆盖 key name")
        .map_err(|err| invalid_argument(format!("{err}: {key}")))?;

    Ok(Some(ProjectOverrideIdentity {
        project_id,
        name: name.to_string(),
    }))
}

pub fn project_override_key(identity: &ProjectOverrideIdentity) -> String {
    system_settings::prompt_override_key(identity.project_id, &identity.name)
}

pub fn validate_project_override_content(content: &str) -> Result<(), PromptOverrideDbError> {
    if content.trim().is_empty() {
        return Err(invalid_argument("提示词覆盖内容不能为空"));
    }
    Ok(())
}

pub fn validate_project_override_project_id(project_id: i64) -> Result<i64, PromptOverrideDbError> {
    if project_id <= 0 {
        return Err(invalid_argument("项目 ID 必须大于 0"));
    }
    Ok(project_id)
}

fn validate_project_override_name(name: &str, label: &str) -> Result<(), PromptOverrideDbError> {
    if name.is_empty() {
        return Err(invalid_argument(format!("{label} 不能为空")));
    }
    if name.chars().any(char::is_control) {
        return Err(invalid_argument(format!("{label} 不能包含控制字符")));
    }
    Ok(())
}
