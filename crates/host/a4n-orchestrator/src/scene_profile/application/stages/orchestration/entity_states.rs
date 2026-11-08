use a4n_domain::json_contract::positive_i64_array_schema;
use a4n_error::AppError;
use serde_json::{Value, json};

use a4n_llm_provider_contract::provider::ChatRequest;

use super::super::super::parsing::parse_scene_entity_states;
use crate::scene_profile::{SceneEntityStateDraft, SceneSyncInput};

use super::super::super::progress::{
    SCENE_ENTITY_STATE_PROGRESS_FIELDS, scene_stage_progress_spec,
};
use super::super::super::types::{
    SCENE_ENTITY_STATE_KEY_RULE, SCENE_ENTITY_STATE_PROMPT, SCENE_ENTITY_TYPE_RULE,
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
) -> Result<(ChatRequest, Vec<SceneEntityStateDraft>), AppError> {
    let request_context = context.request_context;
    let mut request = build_scene_profile_stage_request(
        context.prompt_renderer,
        request_context,
        Some(context.workflow_run_id),
        SCENE_ENTITY_STATE_PROMPT,
        "基于场景与出场链抽取场景内实体状态，只输出 entity_states",
        json!({ "scenes": scenes, "occurrences": occurrences }),
        json!({
            "format": "json_object",
            "required_root_field": "entity_states",
            "field_types": { "source_anchor_ids": positive_i64_array_schema(true) },
            "strict_rules": [
                "只返回 entity_states 数组，不要输出 scenes、occurrences 或 transitions",
                "scene_ref 必须来自 input.previous_stages.scenes[].scene_ref",
                "occurrence_ref 必须来自 input.previous_stages.occurrences[].occurrence_ref",
                SCENE_ENTITY_TYPE_RULE,
                SCENE_ENTITY_STATE_KEY_RULE,
                "实体显示名称由系统根据 entity_id 或动作描述生成，模型只需输出实体类型、ID 和状态变化",
                "source_anchor_ids 必须选择 chapter.content 内 ⟦id⟧ 标记的 ID，不含括号"
            ]
        }),
    )
    .await?;
    let step_key = scene_profile_stage_step_key(request_context, "entity_states");
    let step_order = scene_profile_stage_step_order(context.chapter_index, 3)?;
    let stage_progress = scene_stage_progress_spec(
        "entity_states",
        "场景实体状态",
        SCENE_ENTITY_STATE_PROGRESS_FIELDS,
        context.model_span.split(3, 4),
    );
    let (_, parsed) = run_scene_profile_stage_with_checkpoint(
        context,
        &mut request,
        SCENE_ENTITY_STATE_PROMPT,
        step_key,
        step_order,
        stage_progress,
        |value| parse_scene_entity_states(value, request_context, known),
    )
    .await?;
    Ok((request, parsed))
}
