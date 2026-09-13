use a4n_api_contract::character::{
    DiscoverCharacterProfilesInput, DiscoverCharacterProfilesResult,
};
use a4n_error::AppError;
use a4n_orchestrator::character::CharacterDiscoveryProgress;

use crate::{AppState, actions::character_ai::CharacterAiProgress};

pub async fn discover_character_profiles(
    state: &AppState,
    user_id: i64,
    input: DiscoverCharacterProfilesInput,
    progress: Option<&CharacterAiProgress<'_>>,
) -> Result<DiscoverCharacterProfilesResult, String> {
    let prompt_renderer = state;
    let reporter = |current, total, message: &str| match progress {
        Some(progress) => progress
            .report(current, total, message, "角色发现已停止")
            .map_err(|_| AppError::Cancelled),
        None => Ok(()),
    };
    let stream_reporter = |text: &str| {
        if let Some(progress) = progress {
            progress.stream_text(text);
        }
    };
    let application_progress = progress.map(|progress| CharacterDiscoveryProgress {
        reporter: Some(&reporter),
        stream_reporter: Some(&stream_reporter),
        cancel_token: progress.cancel_token,
        active_task_id: progress.task_id,
    });
    state
        .character_discovery_application()
        .discover(user_id, input, prompt_renderer, application_progress)
        .await
        .map_err(|error| error.to_string())
}
