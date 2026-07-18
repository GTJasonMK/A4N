use super::*;

mod requests;

pub use requests::{
    preview_chapter_video_frame_prompt_model_request, preview_chapter_video_frame_prompt_requests,
    preview_chapter_video_prompt_model_request, preview_chapter_video_prompt_requests,
};

pub async fn generate_chapter_video_frame_prompts(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    progress: Option<VideoPromptProgress<'_>>,
    chunks: Option<VideoPromptChunk<'_>>,
    cancel_token: CancellationToken,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    generate_chapter_video_frame_prompts_inner(state, input, progress, chunks, cancel_token).await
}

pub async fn generate_chapter_video_segment_plan(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    progress: Option<VideoPromptProgress<'_>>,
    chunks: Option<VideoPromptChunk<'_>>,
    cancel_token: CancellationToken,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    generate_chapter_video_segment_plan_inner(state, input, progress, chunks, cancel_token).await
}

pub async fn generate_chapter_video_prompts(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    progress: Option<VideoPromptProgress<'_>>,
    chunks: Option<VideoPromptChunk<'_>>,
    cancel_token: CancellationToken,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    generate_chapter_video_prompts_inner(state, input, progress, chunks, cancel_token).await
}

pub async fn chapter_video_prompt_task_scope(
    state: &AppState,
    input: &GenerateChapterVideoPromptInput,
) -> Result<VideoPromptTaskScope, String> {
    access::verify_project_editor_string(state, input.user_id, input.project_id).await?;
    Ok(VideoPromptTaskScope {
        project_id: input.project_id,
        total: 100,
    })
}

pub(crate) async fn ensure_prompt_matches_current_chapter(
    state: &AppState,
    prompt: &chapter_video_prompt::Model,
) -> Result<(), String> {
    let source = resolve_project_chapter_video_source(
        state,
        prompt.project_id,
        prompt.chapter_id,
        "校验视频提示词",
        false,
    )
    .await?
    .source;
    let Some(source_hash) = metadata_source_content_sha256(prompt.metadata_json.as_ref()) else {
        return Ok(());
    };
    let current_hash = source_content_sha256(&source.content);
    if source_hash != current_hash {
        return Err("当前提示词基于旧正文内容，请重新生成视频提示词".into());
    }
    Ok(())
}
