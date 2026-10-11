use a4n_error::AppError;
use serde_json::Value;

use a4n_llm_provider_contract::provider::ChatRequest;

use super::super::super::parsing::{ParsedSceneProfiles, parse_scene_profiles};
use super::super::super::progress::{SCENE_CANDIDATE_PROGRESS_FIELDS, scene_stage_progress_spec};
use super::super::super::types::SCENE_PROFILE_CANDIDATES_PROMPT;
use super::super::SceneProfileStageContext;
use super::super::execution::run_scene_profile_stage_with_checkpoint;
use super::super::request::{
    build_scene_profile_candidates_request, scene_profile_stage_step_key,
    scene_profile_stage_step_order,
};

pub(super) async fn run(
    context: &mut SceneProfileStageContext<'_, '_>,
) -> Result<(ChatRequest, Vec<Value>, ParsedSceneProfiles), AppError> {
    let mut request = build_scene_profile_candidates_request(
        context.prompt_renderer,
        context.request_context,
        Some(context.workflow_run_id),
    )
    .await?;
    let request_context = context.request_context;
    let step_key = scene_profile_stage_step_key(request_context, "scenes");
    let step_order = scene_profile_stage_step_order(context.chapter_index, 0)?;
    let stage_progress = scene_stage_progress_spec(
        "scenes",
        "场景",
        SCENE_CANDIDATE_PROGRESS_FIELDS,
        context.model_span.split(0, 4),
    );
    let (output, parsed) = run_scene_profile_stage_with_checkpoint(
        context,
        &mut request,
        SCENE_PROFILE_CANDIDATES_PROMPT,
        step_key,
        step_order,
        stage_progress,
        |value| parse_scene_profiles(value, request_context),
    )
    .await?;
    Ok((request, output, parsed))
}
