use super::*;

pub(super) async fn ensure_no_running_video_prompts(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    message: &str,
) -> Result<(), String> {
    let running = state
        .orchestrator
        .has_running_chapter_video_prompts(project_id, chapter_id)
        .await
        .map_err(crate::actions::app_error_message)?;
    if running {
        return Err(message.to_string());
    }
    Ok(())
}

pub(super) fn report_progress(
    progress: &mut Option<VideoPromptProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
) -> Result<(), String> {
    if let Some(progress) = progress.as_deref_mut() {
        progress(current, total, message).map_err(|err| err.to_string())?;
    }
    Ok(())
}

pub(super) fn check_cancelled(
    cancel_token: &CancellationToken,
    message: &str,
) -> Result<(), String> {
    if cancel_token.is_cancelled() {
        return Err(message.to_string());
    }
    Ok(())
}

pub(super) fn prompt_to_summary(p: &chapter_video_prompt::Model) -> ChapterVideoPromptSummary {
    ChapterVideoPromptSummary {
        id: p.id,
        project_id: p.project_id,
        chapter_id: p.chapter_id,
        style: p.style.clone(),
        prompt_type: p.prompt_type.clone(),
        workflow_mode: p.workflow_mode.clone(),
        shot_number: p.shot_number,
        title: p.title.clone(),
        scene_summary: p.scene_summary.clone(),
        visual_prompt: p.visual_prompt.clone(),
        motion_prompt: p.motion_prompt.clone(),
        negative_prompt: p.negative_prompt.clone(),
        duration_secs: p.duration_secs,
        aspect_ratio: p.aspect_ratio.clone(),
        reference_image_ids_json: p.reference_image_ids_json.clone(),
        status: p.status.clone(),
        generated_video_id: p.generated_video_id,
        metadata_json: p.metadata_json.clone(),
        created_at: Some(p.created_at.to_string()),
        updated_at: Some(p.updated_at.to_string()),
    }
}
