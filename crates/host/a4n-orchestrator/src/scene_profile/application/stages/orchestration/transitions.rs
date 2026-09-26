use a4n_error::AppError;
use serde_json::{Value, json};

use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_progress::ProgressSpan;
use a4n_prompt::rendering::PromptRenderer;

use super::super::super::parsing::parse_scene_transitions;
use crate::{
    OrchestratorContext,
    scene_profile::{SceneSyncInput, SceneTransitionDraft},
};

use super::super::super::progress::{
    SCENE_TRANSITION_PROGRESS_FIELDS, SceneProfileSyncProgress, scene_stage_progress_spec,
};
use super::super::super::types::{
    SCENE_TRANSITION_MOVEMENT_TYPE_RULE, SCENE_TRANSITION_PROMPT, SCENE_TRANSITION_REFERENCE_RULE,
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
    scenes: &[Value],
    occurrences: &[Value],
    known: &SceneSyncInput,
    response_chars: &mut Vec<Value>,
) -> Result<(ChatRequest, Vec<SceneTransitionDraft>), AppError> {
    let mut request = build_scene_profile_stage_request(
        prompt_renderer,
        context,
        Some(workflow_run_id),
        SCENE_TRANSITION_PROMPT,
        "基于场景与出场链抽取场景间转场，只输出 transitions",
        json!({ "scenes": scenes, "occurrences": occurrences }),
        json!({
            "format": "json_object",
            "required_root_field": "transitions",
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
    let (_, parsed) = run_scene_profile_stage_with_checkpoint(
        state,
        workflow_run_id,
        &mut request,
        SCENE_TRANSITION_PROMPT,
        scene_profile_stage_step_key(context, "transitions"),
        scene_profile_stage_step_order(chapter_index, 2)?,
        scene_stage_progress_spec(
            "transitions",
            "场景转场",
            SCENE_TRANSITION_PROGRESS_FIELDS,
            model_span.split(2, 4),
        ),
        progress,
        response_chars,
        |value| parse_scene_transitions(value, context, known),
    )
    .await?;
    Ok((request, parsed))
}
