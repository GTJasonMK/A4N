use a4n_error::AppError;
use serde_json::Value;

use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_progress::ProgressSpan;
use a4n_prompt::rendering::PromptRenderer;

use crate::OrchestratorContext;

use super::super::super::parsing::{ParsedSceneProfiles, parse_scene_profiles};
use super::super::super::progress::{
    SCENE_CANDIDATE_PROGRESS_FIELDS, SceneProfileSyncProgress, scene_stage_progress_spec,
};
use super::super::super::types::{SCENE_PROFILE_CANDIDATES_PROMPT, SceneProfileSyncRequestContext};
use super::super::execution::run_scene_profile_stage_with_checkpoint;
use super::super::request::{
    build_scene_profile_candidates_request, scene_profile_stage_step_key,
    scene_profile_stage_step_order,
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
) -> Result<(ChatRequest, Vec<Value>, ParsedSceneProfiles), AppError> {
    let mut request =
        build_scene_profile_candidates_request(prompt_renderer, context, Some(workflow_run_id))
            .await?;
    let (output, parsed) = run_scene_profile_stage_with_checkpoint(
        state,
        workflow_run_id,
        &mut request,
        SCENE_PROFILE_CANDIDATES_PROMPT,
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
        |value| parse_scene_profiles(value, context),
    )
    .await?;
    Ok((request, output, parsed))
}
