use super::super::{
    AppState, ChapterVideoPromptGenerationSpec, ChapterVideoPromptStageInput,
    ChapterVideoPromptSummary, GenerateChapterVideoPromptInput, VIDEO_WORKFLOW_MODE_CHAT_VIDEO,
    VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN, VIDEO_WORKFLOW_MODE_STORYBOARD, VideoPromptChunk,
    VideoPromptProgress, check_cancelled, finalize_video_prompt_workflow,
    prepare_chapter_video_prompt_generation, prompt_to_summary, report_progress, source_context,
};
use tokio_util::sync::CancellationToken;

pub(in crate::actions::video_prompts) async fn generate_chapter_video_prompts_inner(
    state: &AppState,
    input: GenerateChapterVideoPromptInput,
    mut progress: Option<VideoPromptProgress<'_>>,
    chunks: Option<VideoPromptChunk<'_>>,
    cancel_token: CancellationToken,
) -> Result<Vec<ChapterVideoPromptSummary>, String> {
    let prepared = prepare_chapter_video_prompt_generation(
        state,
        input,
        &mut progress,
        &cancel_token,
        ChapterVideoPromptGenerationSpec {
            initial_stop_message: "视频提示词生成已停止",
            operation: "生成视频提示词",
            empty_content_message: "章节正文为空，无法生成视频提示词",
            running_message: "当前章节已有视频任务正在运行，请先等待完成后再重新生成视频提示词",
            workflow_operation: "生成视频提示词",
            workflow_kind: "video_prompts",
            workflow_step_key: "video_prompts",
        },
    )
    .await?;
    let stop_message = if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN {
        "首帧串联视频提示词生成已停止"
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        "Chat 视频提示词生成已停止"
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_STORYBOARD {
        "分镜视频提示词生成已停止"
    } else {
        "视频提示词生成已停止"
    };
    let prepare_message = if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN {
        "正在准备章节正文、首帧和素材"
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        "正在准备章节正文和 Chat 上传素材"
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_STORYBOARD {
        "正在准备章节正文、分镜图和素材"
    } else {
        "正在准备章节正文、边界帧和素材"
    };
    report_progress(&mut progress, 10, 100, prepare_message)?;
    check_cancelled(&cancel_token, stop_message)?;
    let source_context = source_context(&prepared.source);
    let rows_result = if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_FIRST_FRAME_CHAIN {
        state
            .orchestrator
            .generate_chapter_first_frame_video_prompts(
                ChapterVideoPromptStageInput {
                    project_id: prepared.project_id,
                    chapter_id: prepared.chapter_id,
                    workflow_mode: &prepared.workflow_mode,
                    source: source_context,
                    prompt_renderer: state,
                    variables: &prepared.vars,
                    options: prepared.prompt_options,
                    cancel_token: Some(&cancel_token),
                },
                chunks,
                progress,
            )
            .await
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_CHAT_VIDEO {
        state
            .orchestrator
            .generate_chapter_chat_video_prompts(
                ChapterVideoPromptStageInput {
                    project_id: prepared.project_id,
                    chapter_id: prepared.chapter_id,
                    workflow_mode: &prepared.workflow_mode,
                    source: source_context,
                    prompt_renderer: state,
                    variables: &prepared.vars,
                    options: prepared.prompt_options,
                    cancel_token: Some(&cancel_token),
                },
                chunks,
                progress,
            )
            .await
    } else if prepared.workflow_mode == VIDEO_WORKFLOW_MODE_STORYBOARD {
        state
            .orchestrator
            .generate_chapter_storyboard_video_prompts(
                ChapterVideoPromptStageInput {
                    project_id: prepared.project_id,
                    chapter_id: prepared.chapter_id,
                    workflow_mode: &prepared.workflow_mode,
                    source: source_context,
                    prompt_renderer: state,
                    variables: &prepared.vars,
                    options: prepared.prompt_options,
                    cancel_token: Some(&cancel_token),
                },
                chunks,
                progress,
            )
            .await
    } else {
        state
            .orchestrator
            .generate_chapter_video_prompts(
                ChapterVideoPromptStageInput {
                    project_id: prepared.project_id,
                    chapter_id: prepared.chapter_id,
                    workflow_mode: &prepared.workflow_mode,
                    source: source_context,
                    prompt_renderer: state,
                    variables: &prepared.vars,
                    options: prepared.prompt_options,
                    cancel_token: Some(&cancel_token),
                },
                chunks,
                progress,
            )
            .await
    };
    let rows = finalize_video_prompt_workflow(state, prepared.workflow_run_id, rows_result).await?;
    Ok(rows.iter().map(prompt_to_summary).collect())
}
