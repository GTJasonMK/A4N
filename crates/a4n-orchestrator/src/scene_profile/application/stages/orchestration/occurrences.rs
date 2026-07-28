use serde_json::{Value, json};

use a4n_core::structured_progress::ProgressSpan;
use a4n_llm::provider::ChatRequest;
use a4n_prompt::rendering::PromptRenderer;

use crate::OrchestratorContext;

use super::super::super::progress::{
    SCENE_OCCURRENCE_PROGRESS_FIELDS, SceneProfileSyncProgress, scene_stage_progress_spec,
};
use super::super::super::types::{
    SCENE_OCCURRENCE_COVERAGE_RULE, SCENE_OCCURRENCE_PROMPT, SceneProfileSyncRequestContext,
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
    response_chars: &mut Vec<Value>,
) -> Result<(ChatRequest, Value), String> {
    let request = build_scene_profile_stage_request(
        prompt_renderer,
        context,
        Some(workflow_run_id),
        SCENE_OCCURRENCE_PROMPT,
        "基于已确认场景抽取当前章节中的场景出场，只输出 occurrences",
        json!({ "scenes": scenes.clone() }),
        json!({
            "format": "json_object",
            "required_root_field": "occurrences",
            "strict_rules": [
                "只返回 occurrences 数组，不要输出 scenes、transitions 或 entity_states",
                "scene_ref 必须来自 input.previous_stages.scenes[].scene_ref",
                SCENE_OCCURRENCE_COVERAGE_RULE,
                "source_anchor_ids 必须来自 input.text_anchors[].id"
            ]
        }),
    )
    .await?;
    let output = run_scene_profile_stage_with_checkpoint(
        state,
        workflow_run_id,
        request.clone(),
        SCENE_OCCURRENCE_PROMPT,
        "occurrences",
        scene_profile_stage_step_key(context, "occurrences"),
        scene_profile_stage_step_order(chapter_index, 1)?,
        scene_stage_progress_spec(
            "occurrences",
            "场景出场",
            SCENE_OCCURRENCE_PROGRESS_FIELDS,
            model_span.split(1, 4),
        ),
        progress,
        response_chars,
    )
    .await?;
    Ok((request, output))
}
