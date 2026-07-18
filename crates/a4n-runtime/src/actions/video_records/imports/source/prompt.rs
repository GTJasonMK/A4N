use super::super::prelude::{
    AppState, chapter_video_prompt, ensure_prompt_matches_current_chapter,
};

pub(in crate::actions::video_records::imports) async fn import_video_prompt(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    prompt_id: i64,
) -> Result<chapter_video_prompt::Model, String> {
    let prompt = state
        .orchestrator
        .chapter_video_prompt_for_import(project_id, chapter_id, prompt_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    ensure_prompt_matches_current_chapter(state, &prompt).await?;
    Ok(prompt)
}

pub(in crate::actions::video_records::imports) fn imported_video_prompt_text(
    prompt: &chapter_video_prompt::Model,
) -> String {
    let mut sections = vec![prompt.visual_prompt.trim().to_string()];
    if let Some(motion) = prompt
        .motion_prompt
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        sections.push(format!("Motion Prompt:\n{motion}"));
    }
    sections.join("\n\n")
}
