use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;

use crate::video::prompt_generation::frame_prompts::{
    FramePromptGenerationSpec, NewChapterVideoPrompt, OrchestratorContext, VideoPromptProgress,
    report_video_prompt_progress,
};

use crate::video::prompt_generation::frame_prompts::FramePromptLayout;

pub(super) struct FrameChainFinalizationInput<'a, 'progress> {
    pub service: &'a OrchestratorContext,
    pub spec: &'a FramePromptGenerationSpec,
    pub chapter_id: i64,
    pub expected_frame_count: usize,
    pub regenerate_existing: bool,
    pub segment_draft: Option<NewChapterVideoPrompt>,
    pub frame_drafts: Vec<NewChapterVideoPrompt>,
    pub progress: &'a mut Option<VideoPromptProgress<'progress>>,
}

pub(super) async fn finalize_frame_chain(
    input: FrameChainFinalizationInput<'_, '_>,
) -> Result<Vec<chapter_video_prompt::Model>, AppError> {
    let expected_frame_count = i32::try_from(input.expected_frame_count).map_err(|_| {
        AppError::InvalidArgument(format!(
            "视频提示词目标记录数超过 i32 范围：{}",
            input.expected_frame_count
        ))
    })?;
    if !(1..=expected_frame_count).any(|shot_number| {
        input.frame_drafts.iter().any(|draft| {
            draft.matches_identity(
                input.spec.workflow_mode,
                input.spec.frame_prompt_type,
                shot_number,
            )
        })
    }) {
        return Err(AppError::InvalidArgument(format!(
            "本次{}工作流未产生任何目标记录；请检查本次生成数量、片段规划和已有提示词状态",
            input.spec.reused_frame_label
        )));
    }

    let mut drafts = Vec::new();
    if let Some(draft) = input.segment_draft {
        drafts.push(draft);
    }
    drafts.extend(input.frame_drafts);
    let rows = if matches!(input.spec.layout, FramePromptLayout::BoundaryFrames)
        && !input.regenerate_existing
    {
        input
            .service
            .sync_chapter_prompts_preserving_generated_frame_prompts(input.chapter_id, drafts)
            .await?
    } else {
        input
            .service
            .sync_chapter_prompts(input.chapter_id, drafts)
            .await?
    };
    report_video_prompt_progress(input.progress, 100, 100, input.spec.final_progress)?;
    Ok(rows)
}
