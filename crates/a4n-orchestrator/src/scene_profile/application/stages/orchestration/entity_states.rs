use serde_json::{Value, json};

use a4n_core::structured_progress::ProgressSpan;
use a4n_llm::provider::ChatRequest;
use a4n_prompt::rendering::PromptRenderer;

use crate::OrchestratorContext;

use super::super::super::progress::{
    SCENE_ENTITY_STATE_PROGRESS_FIELDS, SceneProfileSyncProgress, scene_stage_progress_spec,
};
use super::super::super::types::{
    SCENE_ENTITY_STATE_KEY_RULE, SCENE_ENTITY_STATE_PROMPT, SCENE_ENTITY_TYPE_RULE,
    SceneProfileSyncRequestContext,
};
use super::super::execution::run_scene_profile_stage_with_checkpoint;
use super::super::request::{
    build_scene_profile_stage_request, scene_profile_stage_step_key, scene_profile_stage_step_order,
};

pub(super) async fn run(
    state: &OrchestratorContext,
    prompt_renderer: &dyn PromptRenderer,
    workflow_run_id: i64,
    context: &SceneProfileSyncRequestContext,
    progress: Option<&SceneProfileSyncProgress<'_>>,
    model_span: ProgressSpan,
    chapter_index: usize,
    scenes: &Value,
    occurrences: &Value,
    response_chars: &mut Vec<Value>,
) -> Result<(ChatRequest, Value), String> {
    let request = build_scene_profile_stage_request(
        prompt_renderer,
        context,
        Some(workflow_run_id),
        SCENE_ENTITY_STATE_PROMPT,
        "基于场景与出场链抽取场景内实体状态，只输出 entity_states",
        json!({ "scenes": scenes.clone(), "occurrences": occurrences.clone() }),
        json!({
            "format": "json_object",
            "required_root_field": "entity_states",
            "strict_rules": [
                "只返回 entity_states 数组，不要输出 scenes、occurrences 或 transitions",
                "scene_ref 必须来自 input.previous_stages.scenes[].scene_ref",
                "occurrence_ref 必须来自 input.previous_stages.occurrences[].occurrence_ref",
                SCENE_ENTITY_TYPE_RULE,
                SCENE_ENTITY_STATE_KEY_RULE,
                "实体显示名称由系统根据 entity_id 或动作描述生成，模型只需输出实体类型、ID 和状态变化",
                "source_anchor_ids 必须来自 input.text_anchors[].id"
            ]
        }),
    )
    .await?;
    let output = run_scene_profile_stage_with_checkpoint(
        state,
        workflow_run_id,
        request.clone(),
        SCENE_ENTITY_STATE_PROMPT,
        "entity_states",
        scene_profile_stage_step_key(context, "entity_states"),
        scene_profile_stage_step_order(chapter_index, 3)?,
        scene_stage_progress_spec(
            "entity_states",
            "场景实体状态",
            SCENE_ENTITY_STATE_PROGRESS_FIELDS,
            model_span.split(3, 4),
        ),
        progress,
        response_chars,
    )
    .await?;
    Ok((request, output))
}
