mod generation;

use std::collections::{HashMap, HashSet};

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_core::prompt_language::PromptOutputLanguage;
use a4n_core::source_spans::SourceSpan;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::video::prompt_generation::frame_prompts::{
    FramePromptGenerationSpec, NewChapterVideoPrompt, VideoPromptDraftCommonInput,
    ensure_existing_frame_prompts_can_regenerate, prompt_row_draft, report_video_prompt_progress,
    storyboard_prompts_from_rows, validate_storyboard_prompts,
};
use crate::video::prompt_json::VideoSegmentPlan;
use crate::video::{
    ChapterVideoSourceContext, OrchestratorContext, VideoPromptChunk, VideoPromptProgress,
};

use self::generation::{StoryboardFrameGenerationInput, generate_storyboard_frames};

pub(super) struct StoryboardWorkflowInput<'a, 'source, 'progress, 'chunk> {
    pub service: &'a OrchestratorContext,
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub chapter_id: i64,
    pub source: ChapterVideoSourceContext<'source>,
    pub source_spans: &'a [SourceSpan],
    pub variables: &'a HashMap<String, Value>,
    pub project_context: Option<&'a Value>,
    pub available_assets: &'a Value,
    pub reference_image_ids: &'a [i64],
    pub reference_ids: &'a Value,
    pub allowed_reference_ids: &'a HashSet<i64>,
    pub segment_json: &'a Value,
    pub segment_plan: &'a [VideoSegmentPlan],
    pub video_units: &'a [crate::video::prompt_generation::frame_prompts::VideoUnitPlan],
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub aspect_ratio: &'a str,
    pub frame_source_prompt_type: &'a str,
    pub workflow_run_id: Option<i64>,
    pub expected_frame_count: usize,
    pub prompt_batch_size: usize,
    pub regenerate_existing: bool,
    pub reusable_rows: Vec<chapter_video_prompt::Model>,
    pub frame_draft_common: VideoPromptDraftCommonInput<'a>,
    pub segment_draft: Option<&'a NewChapterVideoPrompt>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub cancel_token: Option<&'a CancellationToken>,
    pub progress: &'a mut Option<VideoPromptProgress<'progress>>,
    pub chunks: &'a mut Option<VideoPromptChunk<'chunk>>,
}

pub(super) async fn generate_storyboard_workflow(
    input: StoryboardWorkflowInput<'_, '_, '_, '_>,
) -> Result<Vec<NewChapterVideoPrompt>, AppError> {
    let StoryboardWorkflowInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        source_spans,
        variables,
        project_context,
        available_assets,
        reference_image_ids,
        reference_ids,
        allowed_reference_ids,
        segment_json,
        segment_plan,
        video_units,
        style,
        prompt_output_language,
        aspect_ratio,
        frame_source_prompt_type,
        workflow_run_id,
        expected_frame_count,
        prompt_batch_size,
        regenerate_existing,
        reusable_rows,
        frame_draft_common,
        segment_draft,
        prompt_renderer,
        cancel_token,
        mut progress,
        mut chunks,
    } = input;

    let mut prefix_rows = Vec::new();
    for row in reusable_rows {
        let expected = i32::try_from(prefix_rows.len())
            .map_err(|_| {
                AppError::InvalidArgument(format!(
                    "可复用分镜提示词数量超过 i32 范围：{}",
                    prefix_rows.len()
                ))
            })?
            .checked_add(1)
            .ok_or_else(|| {
                AppError::InvalidArgument("可复用分镜提示词编号超过 i32 范围".to_string())
            })?;
        if row.shot_number == expected {
            if storyboard_prompts_from_rows(std::slice::from_ref(&row)).is_err() {
                break;
            }
            prefix_rows.push(row);
        } else if row.shot_number > expected {
            break;
        }
    }

    let storyboard_prompts = storyboard_prompts_from_rows(&prefix_rows)?;
    if storyboard_prompts.len() == expected_frame_count {
        validate_storyboard_prompts(&storyboard_prompts, segment_plan, allowed_reference_ids)?;
        report_video_prompt_progress(
            &mut progress,
            90,
            100,
            &format!(
                "已复用 {} 条{}，正在写入记录",
                prefix_rows.len(),
                spec.reused_frame_label
            ),
        )?;
        return prefix_rows
            .iter()
            .map(prompt_row_draft)
            .collect::<Result<Vec<_>, _>>();
    }

    if !regenerate_existing && prefix_rows.is_empty() {
        ensure_existing_frame_prompts_can_regenerate(
            service,
            project_id,
            chapter_id,
            spec.workflow_mode,
            spec.frame_prompt_type,
            prompt_output_language,
        )
        .await?;
    }
    let frame_prompt_available_assets =
        crate::video::prompt_generation::frame_prompts::frame_prompt_available_assets_context(
            available_assets,
        )?;

    generate_storyboard_frames(StoryboardFrameGenerationInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        source_spans,
        variables,
        project_context,
        frame_prompt_available_assets: &frame_prompt_available_assets,
        reference_image_ids,
        reference_ids,
        allowed_reference_ids,
        segment_json,
        segment_plan,
        video_units,
        style,
        prompt_output_language,
        aspect_ratio,
        frame_source_prompt_type,
        workflow_run_id,
        expected_frame_count,
        prompt_batch_size,
        regenerate_existing,
        storyboard_prompts,
        frame_draft_common,
        segment_draft,
        prompt_renderer,
        cancel_token,
        progress: &mut progress,
        chunks: &mut chunks,
    })
    .await
}
