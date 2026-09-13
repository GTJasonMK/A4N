use a4n_api_contract::character::DiscoverCharacterProfilesInput;
use a4n_orchestrator::character::{CHARACTER_DISCOVERY_CALL_KEY, CHARACTER_DISCOVERY_KIND};

use crate::{AppState, actions::request_preview::ChatStatusPreviewStep};

pub(crate) async fn character_discovery_preview_step(
    state: &AppState,
    user_id: i64,
    input: DiscoverCharacterProfilesInput,
) -> Result<ChatStatusPreviewStep, String> {
    let prompt_renderer = state;
    let preview = state
        .character_discovery_application()
        .preview_discovery(user_id, input, prompt_renderer)
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
