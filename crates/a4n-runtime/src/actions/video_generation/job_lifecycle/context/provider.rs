use a4n_core::models::ai::generated_video;

pub(crate) fn is_first_frame_chain_video(video: &generated_video::Model) -> bool {
    video.workflow_mode.as_deref() == Some("first_frame_chain")
}

pub(crate) fn should_capture_video_tail_frame(video: &generated_video::Model) -> bool {
    video.video_type == "chapter_video"
        && matches!(
            video.workflow_mode.as_deref(),
            Some("boundary_frames" | "first_frame_chain" | "chat_video")
        )
}

pub(crate) fn active_provider_job_id<'a>(
    provider_job_id: Option<&'a str>,
    status: &str,
) -> Option<&'a str> {
    if is_terminal_generated_video_status(status) {
        return None;
    }
    normalized_provider_job_id(provider_job_id)
}

pub(crate) fn has_active_provider_job(provider_job_id: Option<&str>, status: &str) -> bool {
    active_provider_job_id(provider_job_id, status).is_some()
}

pub(crate) fn normalized_provider_job_id(provider_job_id: Option<&str>) -> Option<&str> {
    provider_job_id
        .map(str::trim)
        .filter(|job_id| !job_id.is_empty())
}

pub(crate) fn is_terminal_generated_video_status(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "cancelled")
}

pub(crate) fn required_video_chapter_id(
    chapter_id: Option<i64>,
    video_id: i64,
) -> Result<i64, String> {
    chapter_id.ok_or_else(|| format!("视频 #{video_id} 缺少 chapter_id，无法保存生成结果"))
}
