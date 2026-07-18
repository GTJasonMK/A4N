use crate::AppState;
use crate::actions::access;

use super::super::super::CharacterAiPreviewStep;
use super::super::contracts::ChapterAnchorMode;
use super::build::build_character_discovery_request;

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
        .map_err(|err| err.to_string())?;
    let context = build_character_discovery_request(
        state,
        project_id,
        chapter_number,
        ChapterAnchorMode::RequireExistingCurrent,
    )
    .await?;
    let note = match initial_note {
        Some(note) => format!("{note}；分析章节数：{}", context.chapters.len()),
        None => format!("分析章节数：{}", context.chapters.len()),
    };
    Ok(vec![CharacterAiPreviewStep::llm(
        "character_discovery",
        "角色发现初始 LLM 请求",
        "character_discovery",
        initial_state,
        Some(note),
        context.request,
    )])
}
