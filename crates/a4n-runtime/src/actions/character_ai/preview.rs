use super::{
    CharacterAiPreviewStep, SyncAllCharacterProfilesInput, SyncCharacterProfileInput,
    character_discovery_preview_steps, character_sync_preview_steps,
    sync_all_character_profiles_preview_steps,
};
use crate::{
    AppState,
    actions::request_preview::{
        ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
    },
};

pub async fn preview_discover_character_profiles_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
) -> Result<ModelRequestPreview, String> {
    character_ai_preview(
        state,
        "character_discovery",
        "角色发现 LLM 请求预览",
        character_discovery_preview_steps(
            state,
            user_id,
            project_id,
            chapter_number,
            "ready",
            None,
        )
        .await?,
    )
    .await
}

pub async fn preview_sync_character_from_chapters_request(
    state: &AppState,
    user_id: i64,
    input: SyncCharacterProfileInput,
) -> Result<ModelRequestPreview, String> {
    let title = format!("{} 角色同步 LLM 请求预览", input.name);
    character_ai_preview(
        state,
        "character_profile_sync",
        title,
        character_sync_preview_steps(state, user_id, input).await?,
    )
    .await
}

pub async fn preview_sync_all_character_profiles_request(
    state: &AppState,
    user_id: i64,
    input: SyncAllCharacterProfilesInput,
) -> Result<ModelRequestPreview, String> {
    character_ai_preview(
        state,
        "character_profile_sync_all",
        "批量角色同步 LLM 请求预览",
        sync_all_character_profiles_preview_steps(state, user_id, input).await?,
    )
    .await
}

async fn character_ai_preview(
    state: &AppState,
    workflow: impl Into<String>,
    title: impl Into<String>,
    source_steps: Vec<CharacterAiPreviewStep>,
) -> Result<ModelRequestPreview, String> {
    chat_status_preview_for_state(
        state,
        workflow,
        title,
        source_steps.into_iter().map(|step| {
            ChatStatusPreviewStep::llm(
                step.key,
                step.title,
                step.prompt_name,
                step.state,
                step.note,
                step.request,
            )
        }),
        "llm_chat",
    )
    .await
}
