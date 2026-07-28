use super::super::super::CharacterAiPreviewStep;
use crate::{AppState, actions::access, prompt_renderer::RuntimePromptRenderer};
use a4n_orchestrator::character::CharacterProfileSyncApplicationInput;

pub(crate) async fn character_sync_preview_steps(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
    chapter_number: i32,
) -> Result<Vec<CharacterAiPreviewStep>, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|error| error.to_string())?;
    character_sync_preview_steps_for_input(
        state,
        CharacterProfileSyncApplicationInput::preview_by_name(project_id, name, chapter_number),
    )
    .await
}

pub(in crate::actions::character_ai) async fn character_sync_preview_steps_for_input(
    state: &AppState,
    input: CharacterProfileSyncApplicationInput,
) -> Result<Vec<CharacterAiPreviewStep>, String> {
    let prompt_renderer = RuntimePromptRenderer::new(state);
    state
        .character()
        .prepare_character_profile_sync_preview_validated(input, &prompt_renderer)
        .await
        .map_err(|error| error.to_string())
        .map(|steps| {
            steps
                .into_iter()
                .map(|step| match step.request {
                    Some(request) => CharacterAiPreviewStep::llm(
                        step.id,
                        step.title,
                        step.prompt_name,
                        step.status,
                        step.note,
                        request,
                    ),
                    None => CharacterAiPreviewStep::status(
                        step.id,
                        step.title,
                        step.prompt_name,
                        step.status,
                        step.note,
                    ),
                })
                .collect()
        })
}
