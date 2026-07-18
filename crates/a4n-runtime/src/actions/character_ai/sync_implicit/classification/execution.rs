mod call;

use std::collections::{BTreeSet, HashMap};

use serde_json::{Value, json};

use crate::actions::character_ai::json_utils::{category_attributes, required_i32};
use crate::actions::character_ai::state_projection::{
    apply_sync_change_to_state, project_attribute_changes,
};
use crate::actions::prompt_records::render_prompt_template;
use crate::state::AppState;

use self::call::run_implicit_classification_call;
use super::request::{PreparedImplicitClassificationCall, prepare_implicit_classification_call};
use crate::actions::character_ai::implicit_classification::{
    apply_implicit_classification_to_behavior, set_behavior_classification_results,
};

pub(in crate::actions::character_ai) async fn classify_behaviors(
    state: &AppState,
    project_id: i64,
    workflow_run_id: Option<i64>,
    profile_name: &str,
    changes: &[Value],
    behaviors: &mut [Value],
    final_attributes: &mut Value,
) -> Result<Vec<Value>, String> {
    if behaviors.is_empty() {
        *final_attributes = project_attribute_changes(final_attributes.clone(), changes)?;
        return Ok(Vec::new());
    }

    let system_prompt = render_prompt_template(
        state,
        "implicit_classification",
        HashMap::new(),
        Some(project_id),
    )
    .await?;
    let mut change_index = 0usize;
    let mut suggestions = Vec::new();
    let mut suggested_keys = BTreeSet::new();

    for (behavior_index, behavior) in behaviors.iter_mut().enumerate() {
        let chapter = required_i32(behavior, "chapter_number")?;
        while change_index < changes.len()
            && required_i32(&changes[change_index], "chapter_number")? <= chapter
        {
            apply_sync_change_to_state(final_attributes, &changes[change_index])?;
            change_index += 1;
        }

        let implicit_attributes = category_attributes(final_attributes, "implicit")?;
        if implicit_attributes
            .as_object()
            .is_none_or(serde_json::Map::is_empty)
        {
            set_behavior_classification_results(behavior, json!({}))?;
            continue;
        }

        let PreparedImplicitClassificationCall {
            request,
            call_key,
            workflow_run_id,
            input_fingerprint_json,
        } = prepare_implicit_classification_call(
            project_id,
            workflow_run_id,
            profile_name,
            behavior_index,
            chapter,
            behavior,
            &implicit_attributes,
            &system_prompt,
        )?;
        let parsed = run_implicit_classification_call(
            state,
            request,
            workflow_run_id,
            &call_key,
            &input_fingerprint_json,
            behavior_index,
            &implicit_attributes,
        )
        .await?;
        suggestions.extend(apply_implicit_classification_to_behavior(
            behavior,
            &parsed,
            final_attributes,
            &mut suggested_keys,
            behavior_index,
        )?);
    }

    while change_index < changes.len() {
        apply_sync_change_to_state(final_attributes, &changes[change_index])?;
        change_index += 1;
    }

    Ok(suggestions)
}
