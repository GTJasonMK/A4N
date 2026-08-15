use std::collections::HashMap;

use a4n_core::error::AppError;
use a4n_core::models::ai::optimization_session;
use serde_json::{Value, json};

pub use a4n_api_contract::optimize::OptimizationAgentPermissions as OptimizeAgentPermissions;

use super::{
    OPTIMIZATION_PATCH_MAX_COUNT, OPTIMIZATION_PATCH_NEW_TEXT_MAX_CHARS,
    OPTIMIZATION_PATCH_OLD_TEXT_MAX_CHARS,
};

pub(super) fn optimization_permissions_from_variables(
    variables: &HashMap<String, Value>,
) -> Result<OptimizeAgentPermissions, AppError> {
    let value = variables.get("agent_permissions").ok_or_else(|| {
        AppError::InvalidArgument("缺少内容优化权限 agent_permissions".to_string())
    })?;
    serde_json::from_value(value.clone()).map_err(|error| {
        AppError::InvalidArgument(format!("内容优化权限 agent_permissions 无效: {error}"))
    })
}

pub(super) fn optimization_permissions_from_session(
    session: &optimization_session::Model,
) -> Result<OptimizeAgentPermissions, AppError> {
    let value = session
        .agent_permissions
        .clone()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_agent_permissions".to_string(),
            operation: format!("读取内容优化会话权限: session_id={}", session.id),
        })?;
    serde_json::from_value(value).map_err(|error| AppError::InvalidState {
        current: "invalid_agent_permissions".to_string(),
        operation: format!(
            "读取内容优化会话权限失败: session_id={}, error={error}",
            session.id
        ),
    })
}

pub fn optimization_agent_variables(
    permissions: &OptimizeAgentPermissions,
) -> HashMap<String, Value> {
    HashMap::from([("agent_permissions".to_string(), permissions.as_value())])
}

pub fn optimization_analysis_variables(
    mode: &str,
    scope: Option<&str>,
    permissions: &OptimizeAgentPermissions,
    agent_instruction: Option<&str>,
) -> HashMap<String, Value> {
    let mut variables = optimization_agent_variables(permissions);
    variables.insert("mode".to_string(), Value::String(mode.to_string()));
    if let Some(scope) = scope {
        variables.insert("scope".to_string(), Value::String(scope.to_string()));
    }
    if let Some(instruction) = agent_instruction
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        variables.insert(
            "agent_instruction".to_string(),
            Value::String(instruction.to_string()),
        );
    }
    variables
}

pub(super) fn optimization_agent_capabilities(permissions: &OptimizeAgentPermissions) -> Value {
    json!({
        "readable_context": {
            "project_detail": permissions.read_project_detail,
            "blueprint": permissions.read_blueprint,
            "outlines": permissions.read_outlines,
            "character_profiles": permissions.read_character_profiles,
            "foreshadowing": permissions.read_foreshadowing,
            "current_chapter_content": true,
            "nearby_chapter_summaries": true,
        },
        "actions": {
            "chat": true,
            "analyze": true,
            "get_project_information": true,
            "edit_article": true,
            "apply_local_patches": {
                "available": true,
                "requires_user_confirmation": true,
                "review_mode": if permissions.automatic_writeback { "batch" } else { "per_patch" },
                "requires_base_content_hash": true,
                "requires_unique_old_text_match": true,
            },
            "full_chapter_rewrite": false,
            "automatic_writeback": permissions.automatic_writeback,
        },
        "execution_policy": {
            "ordinary_user_text": "chat_only",
            "analysis_and_patch_generation_require_explicit_user_action": true,
            "missing_precondition_behavior": "tell_user_the_required_next_step_without_running_it",
            "llm_may_not_choose_or_execute_system_actions": true,
        },
        "patch_contract": {
            "operations": ["replace", "delete", "insert_before", "insert_after"],
            "max_patch_count": OPTIMIZATION_PATCH_MAX_COUNT,
            "max_old_text_chars": OPTIMIZATION_PATCH_OLD_TEXT_MAX_CHARS,
            "max_new_text_chars": OPTIMIZATION_PATCH_NEW_TEXT_MAX_CHARS,
            "old_text_must_match_once": true,
            "replace_new_text_must_differ_from_old_text": true,
            "full_chapter_rewrite_forbidden": true,
        }
    })
}

pub(super) fn insert_optimization_agent_capability_variables(
    prompt_vars: &mut std::collections::HashMap<String, Value>,
    permissions: &OptimizeAgentPermissions,
) -> Result<(), AppError> {
    let capabilities = optimization_agent_capabilities(permissions);
    prompt_vars.insert("agent_capabilities".to_string(), capabilities.clone());
    prompt_vars.insert(
        "agent_capabilities_prompt".to_string(),
        Value::String(serde_json::to_string_pretty(&capabilities)?),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn permissions() -> OptimizeAgentPermissions {
        OptimizeAgentPermissions {
            read_project_detail: true,
            read_blueprint: true,
            read_outlines: true,
            read_character_profiles: true,
            read_foreshadowing: true,
            automatic_writeback: false,
        }
    }

    #[test]
    fn analysis_variables_keep_required_mode_and_permissions() {
        let permissions = permissions();
        let variables = optimization_analysis_variables("review", None, &permissions, None);

        assert_eq!(variables.get("mode"), Some(&Value::String("review".into())));
        assert!(variables.contains_key("agent_permissions"));
        assert!(!variables.contains_key("scope"));
        assert!(!variables.contains_key("agent_instruction"));
    }

    #[test]
    fn analysis_variables_trim_instruction_but_preserve_scope() {
        let variables = optimization_analysis_variables(
            "rewrite",
            Some(" chapter "),
            &permissions(),
            Some("  保留叙事视角  "),
        );

        assert_eq!(
            variables.get("scope"),
            Some(&Value::String(" chapter ".into()))
        );
        assert_eq!(
            variables.get("agent_instruction"),
            Some(&Value::String("保留叙事视角".into()))
        );
    }

    #[test]
    fn analysis_variables_drop_blank_instruction() {
        let permissions = permissions();
        let variables = optimization_analysis_variables("review", None, &permissions, Some("  "));

        assert!(!variables.contains_key("agent_instruction"));
    }
}
