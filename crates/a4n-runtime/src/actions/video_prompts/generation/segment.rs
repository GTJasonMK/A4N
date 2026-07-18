use super::super::{
    AppState, ChapterVideoPromptGenerationSpec, ChapterVideoPromptStageInput,
    ChapterVideoPromptSummary, GenerateChapterVideoPromptInput, VideoPromptChunk,
    VideoPromptProgress, check_cancelled, finalize_video_prompt_workflow,
    prepare_chapter_video_prompt_generation, prompt_to_summary, report_progress, source_context,
};
use tokio_util::sync::CancellationToken;

pub(in crate::actions::video_prompts) async fn generate_chapter_video_segment_plan_inner(
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
            initial_stop_message: "片段规划生成已停止",
            operation: "生成片段规划",
            empty_content_message: "章节正文为空，无法生成片段规划",
            running_message: "当前章节已有视频任务正在运行，请先等待完成后再重新生成片段规划",
            workflow_operation: "生成片段规划",
            workflow_kind: "video_segment_plan",
            workflow_step_key: "segment_plan",
        },
    )
    .await?;
    report_progress(&mut progress, 10, 100, "正在准备章节正文、素材和时长档位")?;
    check_cancelled(&cancel_token, "片段规划生成已停止")?;
    let rows_result = state
        .orchestrator
        .generate_chapter_segment_plan(
            ChapterVideoPromptStageInput {
                project_id: prepared.project_id,
                chapter_id: prepared.chapter_id,
                workflow_mode: &prepared.workflow_mode,
                source: source_context(&prepared.source),
                prompt_renderer: state,
                variables: &prepared.vars,
                options: prepared.prompt_options,
                cancel_token: Some(&cancel_token),
            },
            chunks,
            progress,
        )
        .await;
    let rows = finalize_video_prompt_workflow(state, prepared.workflow_run_id, rows_result).await?;
    Ok(rows.iter().map(prompt_to_summary).collect())
}
