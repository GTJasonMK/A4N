use a4n_core::error::AppError;
use a4n_core::models::ai::optimization_session;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{
    OPTIMIZATION_PATCH_MAX_COUNT, OPTIMIZATION_PATCH_NEW_TEXT_MAX_CHARS,
    OPTIMIZATION_PATCH_OLD_TEXT_MAX_CHARS,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OptimizeAgentPermissions {
    pub read_project_detail: bool,
    pub read_blueprint: bool,
    pub read_outlines: bool,
    pub read_character_profiles: bool,
    pub read_foreshadowing: bool,
    #[serde(default)]
    pub automatic_writeback: bool,
}

impl Default for OptimizeAgentPermissions {
    fn default() -> Self {
        Self {
            read_project_detail: true,
            read_blueprint: true,
            read_outlines: true,
            read_character_profiles: true,
            read_foreshadowing: true,
            automatic_writeback: false,
        }
    }
}

impl OptimizeAgentPermissions {
    pub(super) fn from_variables(variables: &std::collections::HashMap<String, Value>) -> Self {
        let Some(value) = variables
            .get("agent_permissions")
            .and_then(Value::as_object)
        else {
            return Self::default();
        };
        Self::from_object(value)
    }

    pub fn as_value(&self) -> Value {
        json!({
            "read_project_detail": self.read_project_detail,
            "read_blueprint": self.read_blueprint,
            "read_outlines": self.read_outlines,
            "read_character_profiles": self.read_character_profiles,
            "read_foreshadowing": self.read_foreshadowing,
            "automatic_writeback": self.automatic_writeback,
        })
    }

    pub(super) fn from_session(session: &optimization_session::Model) -> Self {
        let Some(value) = session
            .agent_permissions
            .as_ref()
            .and_then(Value::as_object)
        else {
            return Self::default();
        };
        Self::from_object(value)
    }

    fn from_object(value: &serde_json::Map<String, Value>) -> Self {
        let defaults = Self::default();
        Self {
            read_project_detail: permission_bool(
                value,
                "read_project_detail",
                defaults.read_project_detail,
            ),
            read_blueprint: permission_bool(value, "read_blueprint", defaults.read_blueprint),
            read_outlines: permission_bool(value, "read_outlines", defaults.read_outlines),
            read_character_profiles: permission_bool(
                value,
                "read_character_profiles",
                defaults.read_character_profiles,
            ),
            read_foreshadowing: permission_bool(
                value,
                "read_foreshadowing",
                defaults.read_foreshadowing,
            ),
            automatic_writeback: permission_bool(
                value,
                "automatic_writeback",
                defaults.automatic_writeback,
            ),
        }
    }
}

fn permission_bool(
    object: &serde_json::Map<String, Value>,
    key: &str,
    default_value: bool,
) -> bool {
    object
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(default_value)
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
