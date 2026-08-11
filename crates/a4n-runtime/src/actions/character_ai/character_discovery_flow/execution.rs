use a4n_core::error::AppError;
use a4n_orchestrator::character::{CharacterDiscoveryInput, CharacterDiscoveryProgress};
use serde_json::Value;

use crate::{
    AppState,
    actions::{access, character_ai::CharacterAiProgress},
};

pub async fn discover_character_profiles_for_chapters(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
    progress: Option<&CharacterAiProgress<'_>>,
) -> Result<Value, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())?;
    let prompt_renderer = state;
    let reporter = |current, total, message: &str| match progress {
        Some(progress) => progress
            .report(current, total, message, "角色发现已停止")
            .map_err(|_| AppError::Cancelled),
        None => Ok(()),
    };
    let stream_reporter = |text: &str| match progress {
        Some(progress) => progress.stream_text(text),
        None => {}
    };
    let application_progress = progress.map(|progress| CharacterDiscoveryProgress {
        reporter: Some(&reporter),
        stream_reporter: Some(&stream_reporter),
        cancel_token: progress.cancel_token,
        active_task_id: progress.task_id,
    });
    state
        .orchestrator()
        .discover_character_profiles_validated(
            CharacterDiscoveryInput {
                project_id,
                chapter_number,
            },
            prompt_renderer,
            application_progress,
        )
        .await
        .map_err(|error| error.to_string())
}
