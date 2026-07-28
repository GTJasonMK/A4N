use a4n_orchestrator::character::CharacterDiscoveryInput;

use crate::{
    AppState,
    actions::{access, character_ai::CharacterAiPreviewStep},
    prompt_renderer::RuntimePromptRenderer,
};

pub(crate) async fn character_discovery_preview_steps(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_number: Option<i32>,
    initial_state: &str,
    initial_note: Option<String>,
) -> Result<Vec<CharacterAiPreviewStep>, String> {
    access::verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|error| error.to_string())?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let preview = state
        .character()
        .prepare_character_discovery_request_preview_validated(
            CharacterDiscoveryInput {
                project_id,
                chapter_number,
            },
            &prompt_renderer,
        )
        .await
        .map_err(|error| error.to_string())?;
    let note = match initial_note {
        Some(note) => format!("{note}；分析章节数：{}", preview.analyzed_chapter_count),
        None => format!("分析章节数：{}", preview.analyzed_chapter_count),
    };
    Ok(vec![CharacterAiPreviewStep::llm(
        "character_discovery",
        "角色发现初始 LLM 请求",
        "character_discovery",
        initial_state,
        Some(note),
        preview.request,
    )])
}
