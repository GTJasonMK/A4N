mod checkpoint;

use std::collections::{HashMap, HashSet};

use a4n_domain::prompt_language::PromptOutputLanguage;
use a4n_domain::source_spans::SourceSpan;
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::video::prompt_generation::ProgressSpan;
use crate::video::prompt_generation::frame_prompts::{
    FramePromptGenerationSpec, NewChapterVideoPrompt, StoryboardPromptDraftsInput,
    VideoPromptDraftCommonInput, VideoUnitPlan, attach_generation_trace_to_draft,
    report_video_prompt_item_committed, report_video_prompt_progress, storyboard_prompt_drafts,
    validate_storyboard_prompt, validate_storyboard_prompt_for_unit, validate_storyboard_prompts,
    video_prompt_generation_trace,
};
use crate::video::prompt_json::{VideoFramePrompt, VideoSegmentPlan};
use crate::video::{
    ChapterVideoSourceContext, OrchestratorContext, VideoPromptChunk, VideoPromptProgress,
};

use self::checkpoint::{StoryboardFrameCheckpointInput, render_storyboard_frame_checkpoint};

pub(super) struct StoryboardFrameGenerationInput<'a, 'source, 'progress, 'chunk> {
    pub service: &'a OrchestratorContext,
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub chapter_id: i64,
    pub source: ChapterVideoSourceContext<'source>,
    pub source_spans: &'a [SourceSpan],
    pub variables: &'a HashMap<String, Value>,
    pub project_context: Option<&'a Value>,
    pub frame_prompt_available_assets: &'a Value,
    pub reference_image_ids: &'a [i64],
    pub reference_ids: &'a Value,
    pub allowed_reference_ids: &'a HashSet<i64>,
    pub segment_json: &'a Value,
    pub segment_plan: &'a [VideoSegmentPlan],
    pub video_units: &'a [VideoUnitPlan],
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub aspect_ratio: &'a str,
    pub frame_source_prompt_type: &'a str,
    pub workflow_run_id: Option<i64>,
    pub expected_frame_count: usize,
    pub target_shot_number: Option<i32>,
    pub prompt_batch_size: usize,
    pub regenerate_existing: bool,
    pub storyboard_prompts: Vec<VideoFramePrompt>,
    pub frame_draft_common: VideoPromptDraftCommonInput<'a>,
    pub segment_draft: Option<&'a NewChapterVideoPrompt>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub cancel_token: Option<&'a CancellationToken>,
    pub progress: &'a mut Option<VideoPromptProgress<'progress>>,
    pub chunks: &'a mut Option<VideoPromptChunk<'chunk>>,
}

pub(super) async fn generate_storyboard_frames(
    input: StoryboardFrameGenerationInput<'_, '_, '_, '_>,
) -> Result<Vec<NewChapterVideoPrompt>, AppError> {
    let StoryboardFrameGenerationInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        source_spans,
        variables,
        project_context,
        frame_prompt_available_assets,
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
        target_shot_number,
        prompt_batch_size,
        regenerate_existing,
        mut storyboard_prompts,
        frame_draft_common,
        segment_draft,
        prompt_renderer,
        cancel_token,
        progress,
        chunks,
    } = input;
    let message = if regenerate_existing {
        spec.frame_regenerate_progress
    } else {
        spec.frame_generate_progress
    };
    let mut generated_frame_numbers = storyboard_prompts
        .iter()
        .map(|prompt| prompt.frame_number)
        .collect::<HashSet<_>>();
    let eligible_units = video_units
        .iter()
        .filter(|unit| target_shot_number.is_none_or(|target| unit.unit_number == target))
        .collect::<Vec<_>>();
    let mut next_unit_index = 0usize;
    let mut generated_this_run = 0usize;
    while storyboard_prompts.len() < expected_frame_count && generated_this_run < prompt_batch_size
    {
        let index = storyboard_prompts.len();
        while eligible_units
            .get(next_unit_index)
            .is_some_and(|unit| generated_frame_numbers.contains(&unit.unit_number))
        {
            next_unit_index += 1;
        }
        let unit = eligible_units
            .get(next_unit_index)
            .copied()
            .ok_or_else(|| {
                AppError::InvalidArgument("分镜提示词缺少目标视频单元规划".to_string())
            })?;
        let frame_number = unit.unit_number;
        let generation_message = if target_shot_number.is_some() {
            format!(
                "{message}：正在生成目标片段 #{frame_number} 的分镜，本批第 {}/{} 条",
                generated_this_run + 1,
                prompt_batch_size
            )
        } else {
            format!(
                "{message}：正在生成第 {}/{} 个视频单元分镜，本批第 {}/{} 条",
                frame_number,
                expected_frame_count,
                generated_this_run + 1,
                prompt_batch_size
            )
        };
        let frame_progress_span = ProgressSpan::new(60, 90).split(index, expected_frame_count);
        report_video_prompt_progress(
            progress,
            frame_progress_span.start(),
            100,
            &generation_message,
        )?;
        let (frame_trace, mut frame_prompt) =
            render_storyboard_frame_checkpoint(StoryboardFrameCheckpointInput {
                service,
                spec,
                project_id,
                chapter_id,
                source,
                source_spans,
                variables,
                project_context,
                frame_prompt_available_assets,
                reference_image_ids,
                reference_ids,
                allowed_reference_ids,
                segment_json,
                segment_plan,
                unit,
                style,
                prompt_output_language,
                aspect_ratio,
                workflow_run_id,
                expected_frame_count,
                prompt_renderer,
                cancel_token,
                frame_progress_span,
                generation_message: &generation_message,
                progress,
                chunks,
            })
            .await?;
        frame_prompt.frame_number = frame_number;
        if target_shot_number.is_some() {
            validate_storyboard_prompt_for_unit(
                &frame_prompt,
                segment_plan,
                unit,
                allowed_reference_ids,
            )?;
        } else {
            validate_storyboard_prompt(&frame_prompt, segment_plan, allowed_reference_ids)?;
        }
        storyboard_prompts.push(frame_prompt);
        generated_frame_numbers.insert(frame_number);
        next_unit_index += 1;
        generated_this_run += 1;

        let frame_drafts = storyboard_prompt_drafts(StoryboardPromptDraftsInput {
            common: frame_draft_common,
            source_prompt_type: frame_source_prompt_type,
            video_units,
            storyboard_prompts: &storyboard_prompts,
        });
        let frame_drafts = attach_generation_trace_to_draft(
            frame_drafts,
            spec.workflow_mode,
            spec.frame_prompt_type,
            frame_number,
            video_prompt_generation_trace(frame_trace, workflow_run_id),
        );
        let mut checkpoint_drafts = Vec::new();
        if let Some(draft) = segment_draft {
            checkpoint_drafts.push(draft.clone());
        }
        checkpoint_drafts.extend(frame_drafts);
        service
            .sync_chapter_prompts(chapter_id, checkpoint_drafts)
            .await?;
        report_video_prompt_item_committed(
            progress,
            frame_progress_span.end(),
            100,
            spec.frame_prompt_type,
            frame_number,
            storyboard_prompts.len(),
            expected_frame_count,
            &format!(
                "分镜提示词第 {}/{} 条已保存，本批已生成 {} 条",
                storyboard_prompts.len(),
                expected_frame_count,
                generated_this_run
            ),
        )?;
    }
    if storyboard_prompts.len() == expected_frame_count {
        if target_shot_number.is_some() {
            let unit = video_units
                .iter()
                .find(|unit| unit.storyboard_number == Some(storyboard_prompts[0].frame_number))
                .ok_or_else(|| {
                    AppError::InvalidArgument("分镜提示词缺少目标视频单元规划".to_string())
                })?;
            validate_storyboard_prompt_for_unit(
                &storyboard_prompts[0],
                segment_plan,
                unit,
                allowed_reference_ids,
            )?;
        } else {
            validate_storyboard_prompts(&storyboard_prompts, segment_plan, allowed_reference_ids)?;
        }
    }
    let final_message = if storyboard_prompts.len() == expected_frame_count {
        format!(
            "分镜提示词已全部生成，共 {}/{} 条",
            storyboard_prompts.len(),
            expected_frame_count
        )
    } else {
        format!(
            "本批已生成 {} 条分镜提示词，当前共 {}/{} 条",
            generated_this_run,
            storyboard_prompts.len(),
            expected_frame_count
        )
    };
    report_video_prompt_progress(progress, 90, 100, &final_message)?;
    Ok(storyboard_prompt_drafts(StoryboardPromptDraftsInput {
        common: frame_draft_common,
        source_prompt_type: frame_source_prompt_type,
        video_units,
        storyboard_prompts: &storyboard_prompts,
    }))
}
