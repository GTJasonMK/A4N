use a4n_error::AppError;
use serde_json::{Value, json};

use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_progress::ProgressSpan;
use a4n_prompt::rendering::PromptRenderer;

use crate::OrchestratorContext;

use super::super::super::progress::{
    SCENE_CANDIDATE_PROGRESS_FIELDS, SceneProfileSyncProgress, scene_stage_progress_spec,
};
use super::super::super::types::{
    SCENE_CANDIDATE_ACTUAL_OCCURRENCE_RULE, SCENE_PROFILE_CANDIDATES_PROMPT, SCENE_TYPE_RULE,
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
    response_chars: &mut Vec<Value>,
) -> Result<(ChatRequest, Value), AppError> {
    let request = build_scene_profile_stage_request(
        prompt_renderer,
        context,
        Some(workflow_run_id),
        SCENE_PROFILE_CANDIDATES_PROMPT,
        "识别或匹配当前章节中的可复用场景档案，只输出 scenes",
        json!({}),
        json!({
            "format": "json_object",
            "required_root_field": "scenes",
            "strict_rules": [
                "只返回 scenes 数组，不要输出 occurrences、transitions 或 entity_states",
                SCENE_CANDIDATE_ACTUAL_OCCURRENCE_RULE,
                SCENE_TYPE_RULE,
                "importance 只能是 primary/secondary/minor",
                "evidence_anchor_ids 必须来自 input.text_anchors[].id",
                "scene_profile_id 和 parent_scene_id 只能引用 input.existing_scene_profiles[].id 或 null"
            ]
        }),
    )
    .await?;
    let output = run_scene_profile_stage_with_checkpoint(
        state,
        workflow_run_id,
        request.clone(),
        context,
        SCENE_PROFILE_CANDIDATES_PROMPT,
        "scenes",
        scene_profile_stage_step_key(context, "scenes"),
        scene_profile_stage_step_order(chapter_index, 0)?,
        scene_stage_progress_spec(
            "scenes",
            "场景",
            SCENE_CANDIDATE_PROGRESS_FIELDS,
            model_span.split(0, 4),
        ),
        progress,
        response_chars,
    )
    .await?;
    Ok((request, output))
}
