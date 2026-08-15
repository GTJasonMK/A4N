use a4n_orchestrator::character::CharacterDiscoveryInput;

use crate::{
    AppState,
    actions::{access, request_preview::ChatStatusPreviewStep},
};

pub(crate) async fn character_discovery_preview_steps(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
) -> Result<Vec<ChatStatusPreviewStep>, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|error| error.to_string())?;
    let prompt_renderer = state;
    let preview = state
        .orchestrator()
        .prepare_character_discovery_request_preview_validated(
            CharacterDiscoveryInput {
                project_id,
                chapter_number,
            },
            prompt_renderer,
        )
        .await
        .map_err(|error| error.to_string())?;
    Ok(vec![ChatStatusPreviewStep::llm(
        "character_discovery",
        "角色发现初始 LLM 请求",
        Some("character_discovery".to_string()),
        "ready",
        Some(format!("分析章节数：{}", preview.analyzed_chapter_count)),
        Some(preview.request),
    )])
}
