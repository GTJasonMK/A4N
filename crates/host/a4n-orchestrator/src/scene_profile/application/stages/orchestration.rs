mod entity_states;
mod occurrences;
mod scenes;
mod transitions;

use a4n_error::AppError;

use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_progress::ProgressSpan;
use a4n_prompt::rendering::PromptRenderer;

use crate::{OrchestratorContext, scene_profile::SceneSyncInput};

use super::super::progress::SceneProfileSyncProgress;
use super::super::types::{SceneProfileSyncRequestContext, SceneProfileSyncStageResponses};
use super::SceneProfileStageContext;

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
    let model_span = ProgressSpan::new(chapter_span.start(), chapter_span.split(4, 6).start());
    let mut stage_context = SceneProfileStageContext {
        state,
        prompt_renderer,
        workflow_run_id,
        request_context: context,
        progress,
        model_span,
        chapter_index,
        response_chars: Vec::new(),
    };

    let (scene_request, scenes, profiles) = scenes::run(&mut stage_context).await?;
    requests.push(scene_request);
    let mut input = SceneSyncInput {
        profiles: profiles.drafts,
        occurrences: Vec::new(),
        transitions: Vec::new(),
        entity_states: Vec::new(),
    };

    let (occurrence_request, occurrences, parsed_occurrences) =
        occurrences::run(&mut stage_context, &scenes, &input).await?;
    requests.push(occurrence_request);
    input.occurrences = parsed_occurrences;

    let (transition_request, transitions) =
        transitions::run(&mut stage_context, &scenes, &occurrences, &input).await?;
    requests.push(transition_request);
    input.transitions = transitions;

    let (entity_request, entity_states) =
        entity_states::run(&mut stage_context, &scenes, &occurrences, &input).await?;
    requests.push(entity_request);
    input.entity_states = entity_states;

    Ok(SceneProfileSyncStageResponses {
        input,
        profile_evidence: profiles.evidence_refs,
        requests,
        response_chars: stage_context.response_chars,
    })
}
