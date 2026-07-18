use super::super::{
    AppState, GenerateChapterVideoInput, GenerateChapterVideosBatchInput,
    VIDEO_GENERATION_PROGRESS_TOTAL, VideoGenerationTaskScope, access,
};

pub async fn chapter_video_generation_task_scope(
    state: &AppState,
    input: &GenerateChapterVideoInput,
) -> Result<VideoGenerationTaskScope, String> {
    video_generation_task_scope(state, input.user_id, input.project_id).await
}

pub async fn chapter_videos_batch_generation_task_scope(
    state: &AppState,
    input: &GenerateChapterVideosBatchInput,
) -> Result<VideoGenerationTaskScope, String> {
    access::verify_project_editor_string(state, input.user_id, input.project_id).await?;
    let prompt_count = input.prompt_ids.len().max(1);
    let prompt_count =
        u32::try_from(prompt_count).map_err(|_| "批量视频提示词数量超出 u32 范围".to_string())?;
    let total = prompt_count
        .checked_mul(VIDEO_GENERATION_PROGRESS_TOTAL)
        .ok_or_else(|| "批量视频进度总量超出 u32 范围".to_string())?;
    Ok(VideoGenerationTaskScope {
        project_id: input.project_id,
        total,
    })
}

pub(in crate::actions::video_generation) async fn video_generation_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<VideoGenerationTaskScope, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    Ok(VideoGenerationTaskScope {
        project_id,
        total: VIDEO_GENERATION_PROGRESS_TOTAL,
    })
}
