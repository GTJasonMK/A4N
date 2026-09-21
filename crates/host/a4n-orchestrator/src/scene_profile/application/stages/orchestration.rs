mod entity_states;
mod occurrences;
mod scenes;
mod transitions;

use a4n_error::AppError;
use serde_json::Value;

use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_progress::ProgressSpan;
use a4n_prompt::rendering::PromptRenderer;

use crate::OrchestratorContext;

use super::super::progress::{SceneProfileSyncProgress, subspan};
use super::super::types::{SceneProfileSyncRequestContext, SceneProfileSyncStageResponses};

pub(in crate::scene_profile::application) async fn run_scene_profile_sync_stages(
    state: &OrchestratorContext,
    prompt_renderer: &dyn PromptRenderer,
    workflow_run_id: i64,
    context: &SceneProfileSyncRequestContext,
    progress: Option<&SceneProfileSyncProgress<'_>>,
    chapter_span: ProgressSpan,
    chapter_index: usize,
) -> Result<SceneProfileSyncStageResponses, AppError> {
    let mut requests = Vec::<ChatRequest>::new();
    let mut response_chars = Vec::<Value>::new();
    let model_span = subspan(chapter_span, 0, 4, 6);

    let (scene_request, scenes) = scenes::run(
        state,
        prompt_renderer,
        workflow_run_id,
        context,
        progress,
        model_span,
        chapter_index,
        &mut response_chars,
    )
    .await?;
    requests.push(scene_request);

    let (occurrence_request, occurrences) = occurrences::run(
        state,
        prompt_renderer,
        workflow_run_id,
        context,
        progress,
        model_span,
        chapter_index,
        &scenes,
        &mut response_chars,
    )
    .await?;
    requests.push(occurrence_request);

    let (transition_request, transitions) = transitions::run(
        state,
        prompt_renderer,
        workflow_run_id,
        context,
        progress,
        model_span,
        chapter_index,
        &scenes,
        &occurrences,
        &mut response_chars,
    )
    .await?;
    requests.push(transition_request);

    let (entity_request, entity_states) = entity_states::run(
        state,
        prompt_renderer,
        workflow_run_id,
        context,
        progress,
        model_span,
        chapter_index,
        &scenes,
        &occurrences,
        &mut response_chars,
    )
    .await?;
    requests.push(entity_request);

    Ok(SceneProfileSyncStageResponses {
        scenes,
        occurrences,
        transitions,
        entity_states,
        requests,
        response_chars,
    })
}
