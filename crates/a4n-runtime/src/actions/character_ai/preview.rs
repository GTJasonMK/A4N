use super::{
    DiscoverCharacterProfilesInput, SyncAllCharacterProfilesInput, SyncCharacterProfileInput,
    character_discovery_preview_step, character_sync_preview_step,
    sync_all_character_profiles_preview_steps,
};
use crate::{
    AppState,
    actions::request_preview::{
        ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
    },
};
use a4n_orchestrator::character::{CHARACTER_DISCOVERY_KIND, CharacterProfileSyncPreviewStep};

pub async fn preview_discover_character_profiles_request(
    state: &AppState,
    user_id: i64,
    input: DiscoverCharacterProfilesInput,
) -> Result<ModelRequestPreview, String> {
    chat_status_preview_for_state(
        state,
        CHARACTER_DISCOVERY_KIND,
        "角色发现 LLM 请求预览",
        [character_discovery_preview_step(state, user_id, input).await?],
        "llm_chat",
    )
    .await
}

pub async fn preview_sync_character_from_chapters_request(
    state: &AppState,
    user_id: i64,
    input: SyncCharacterProfileInput,
) -> Result<ModelRequestPreview, String> {
    let title = format!("{} 角色同步 LLM 请求预览", input.name);
    chat_status_preview_for_state(
        state,
        "character_profile_sync",
        title,
        character_sync_preview_step(state, user_id, input)
            .await?
            .into_iter()
            .map(profile_sync_preview_step),
        "llm_chat",
    )
    .await
}

pub async fn preview_sync_all_character_profiles_request(
    state: &AppState,
    user_id: i64,
    input: SyncAllCharacterProfilesInput,
) -> Result<ModelRequestPreview, String> {
    chat_status_preview_for_state(
        state,
        "character_profile_sync_all",
        "批量角色同步 LLM 请求预览",
        sync_all_character_profiles_preview_steps(state, user_id, input)
            .await?
            .into_iter()
            .map(profile_sync_preview_step),
        "llm_chat",
    )
    .await
}

fn profile_sync_preview_step(step: CharacterProfileSyncPreviewStep) -> ChatStatusPreviewStep {
    ChatStatusPreviewStep::llm(
        step.id,
        step.title,
        Some(step.prompt_name.to_string()),
        "ready",
        Some(step.note),
        Some(step.request),
    )
}
