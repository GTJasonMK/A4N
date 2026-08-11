use super::super::{CharacterAiPreviewStep, SyncCharacterProfileInput};
use crate::{AppState, actions::access};
use a4n_orchestrator::character::CharacterProfileSyncApplicationInput;

pub(crate) async fn character_sync_preview_steps(
    state: &AppState,
    user_id: i64,
    input: SyncCharacterProfileInput,
) -> Result<Vec<CharacterAiPreviewStep>, String> {
    access::verify_project_editor(state, user_id, input.project_id)
        .await
        .map_err(|error| error.to_string())?;
    character_sync_preview_steps_for_input(
        state,
        CharacterProfileSyncApplicationInput::preview_by_name(
            input.project_id,
            input.name,
            input.chapter_number,
        ),
    )
    .await
}

pub(in crate::actions::character_ai) async fn character_sync_preview_steps_for_input(
    state: &AppState,
    input: CharacterProfileSyncApplicationInput,
) -> Result<Vec<CharacterAiPreviewStep>, String> {
    let prompt_renderer = state;
    state
        .orchestrator()
        .prepare_character_profile_sync_preview_validated(input, prompt_renderer)
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
