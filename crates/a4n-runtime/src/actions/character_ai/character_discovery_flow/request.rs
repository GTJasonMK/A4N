use a4n_api_contract::character::DiscoverCharacterProfilesInput;
use a4n_orchestrator::character::{CHARACTER_DISCOVERY_CALL_KEY, CHARACTER_DISCOVERY_KIND};

use crate::{
    AppState,
    actions::{access, request_preview::ChatStatusPreviewStep},
};

pub(crate) async fn character_discovery_preview_step(
    state: &AppState,
    user_id: i64,
    input: DiscoverCharacterProfilesInput,
) -> Result<ChatStatusPreviewStep, String> {
    access::verify_project_editor(state, user_id, input.project_id)
        .await
        .map_err(|error| error.to_string())?;
    let prompt_renderer = state;
    let preview = state
        .orchestrator()
        .prepare_character_discovery_request_preview_validated(input, prompt_renderer)
        .await
        .map_err(|error| error.to_string())?;
    Ok(ChatStatusPreviewStep::llm(
        CHARACTER_DISCOVERY_CALL_KEY,
        "角色发现初始 LLM 请求",
        Some(CHARACTER_DISCOVERY_KIND.to_string()),
        "ready",
        Some(format!("分析章节数：{}", preview.analyzed_chapter_count)),
        Some(preview.request),
    ))
}
