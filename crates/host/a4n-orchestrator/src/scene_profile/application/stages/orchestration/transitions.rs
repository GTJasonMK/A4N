use a4n_domain::json_contract::positive_i64_array_schema;
use a4n_error::AppError;
use serde_json::{Value, json};

use a4n_llm_provider_contract::provider::ChatRequest;

use super::super::super::parsing::parse_scene_transitions;
use crate::scene_profile::{SceneSyncInput, SceneTransitionDraft};

use super::super::super::progress::{SCENE_TRANSITION_PROGRESS_FIELDS, scene_stage_progress_spec};
use super::super::super::types::{
    SCENE_TRANSITION_MOVEMENT_TYPE_RULE, SCENE_TRANSITION_PROMPT, SCENE_TRANSITION_REFERENCE_RULE,
};
use super::super::SceneProfileStageContext;
use super::super::execution::run_scene_profile_stage_with_checkpoint;
use super::super::request::{
    build_scene_profile_stage_request, scene_profile_stage_step_key, scene_profile_stage_step_order,
};

pub(super) async fn run(
    context: &mut SceneProfileStageContext<'_, '_>,
    scenes: &[Value],
    occurrences: &[Value],
    known: &SceneSyncInput,
) -> Result<(ChatRequest, Vec<SceneTransitionDraft>), AppError> {
    let request_context = context.request_context;
    let mut request = build_scene_profile_stage_request(
        context.prompt_renderer,
        request_context,
        Some(context.workflow_run_id),
        SCENE_TRANSITION_PROMPT,
        "基于场景与出场链抽取场景间转场，只输出 transitions",
        json!({ "scenes": scenes, "occurrences": occurrences }),
        json!({
            "format": "json_object",
            "required_root_field": "transitions",
            "field_types": {
                "from_scene_ref":{"type":["string","null"]}, "to_scene_ref":{"type":"string","minLength":1},
                "occurrence_ref":{"type":["string","null"]},
                "movement_type":{"type":"string","enum":crate::scene_profile::SCENE_TRANSITION_MOVEMENT_TYPES},
                "route_description":{"type":"string","minLength":1}, "time_cost":{"type":["string","null"]}, "distance_hint":{"type":["string","null"]},
                "participants":{"type":"array","items":a4n_domain::json_contract::positive_i64_schema()},
                "object_refs":{"type":"array","items":a4n_domain::json_contract::positive_i64_schema()},
                "source_anchor_ids": positive_i64_array_schema(true)
            },
            "strict_rules": [
                "只返回 transitions 数组，不要输出 scenes、occurrences 或 entity_states",
                SCENE_TRANSITION_REFERENCE_RULE,
                "occurrence_ref 必须来自 input.previous_stages.occurrences[].occurrence_ref 或 null",
                SCENE_TRANSITION_MOVEMENT_TYPE_RULE,
                "source_anchor_ids 必须选择 chapter.content 内 ⟦id⟧ 标记的 ID，不含括号"
            ]
        }),
    )
    .await?;
    let step_key = scene_profile_stage_step_key(request_context, "transitions");
    let step_order = scene_profile_stage_step_order(context.chapter_index, 2)?;
    let stage_progress = scene_stage_progress_spec(
        "transitions",
        "场景转场",
        SCENE_TRANSITION_PROGRESS_FIELDS,
        context.model_span.split(2, 4),
    );
    let (_, parsed) = run_scene_profile_stage_with_checkpoint(
        context,
        &mut request,
        SCENE_TRANSITION_PROMPT,
        step_key,
        step_order,
        stage_progress,
        |value| parse_scene_transitions(value, request_context, known),
    )
    .await?;
    Ok((request, parsed))
}
