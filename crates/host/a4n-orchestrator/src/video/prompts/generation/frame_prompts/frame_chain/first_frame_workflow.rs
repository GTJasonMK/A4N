use std::collections::{HashMap, HashSet};

use a4n_db::workflow;
use a4n_error::AppError;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

use crate::video::prompts::generation::frame_prompts::{
    FramePromptGenerationSpec, NewChapterVideoPrompt, VideoPromptDraftCommonInput,
};
use crate::video::prompts::json::{VideoFramePrompt, VideoSegmentPlan};
use crate::video::{
    ChapterVideoSourceContext, OrchestratorContext, VideoPromptChunk, VideoPromptProgress,
};
use crate::workflow::workflow_checkpoint_contract_app_error;
use a4n_domain::prompt_language::PromptOutputLanguage;
use a4n_domain::source_spans::SourceSpan;
use a4n_prompt::rendering::PromptRenderer;

use crate::video::prompts::generation::frame_prompts::{
    FirstFramePromptDraftsInput, FramePromptContext, RenderOrRestoreJsonWithValidationInput,
    VIDEO_PREVIOUS_OUTPUT_SEGMENT_PLAN, VideoContextManifestInput, VideoContextPreviousOutput,
    attach_generation_trace_to_all_drafts, build_first_frame_prompt_user,
    build_video_context_manifest, ensure_existing_frame_prompts_can_regenerate,
    first_frame_prompt_drafts, frame_prompt_available_assets_context, frame_prompt_progress_spec,
    parse_video_frame_prompts_with_defaults, render_or_restore_json_with_validation_with_context,
    report_video_prompt_item_committed, report_video_prompt_progress, serialize_video_value,
    validate_first_frame_prompts, video_prompt_generation_trace, video_stage_fingerprint,
};
use crate::video::prompts::generation::{ProgressSpan, VideoPromptStepProgress};
use crate::video::prompts::json::VideoFramePromptDefaults;

pub(super) struct FirstFrameWorkflowInput<'a, 'source, 'progress, 'chunk> {
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
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub aspect_ratio: &'a str,
    pub frame_source_prompt_type: &'a str,
    pub workflow_run_id: Option<i64>,
    pub expected_frame_count: usize,
    pub regenerate_existing: bool,
    pub frame_draft_common: VideoPromptDraftCommonInput<'a>,
    pub segment_draft: Option<&'a NewChapterVideoPrompt>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub cancel_token: Option<&'a CancellationToken>,
    pub progress: &'a mut Option<VideoPromptProgress<'progress>>,
    pub chunks: &'a mut Option<VideoPromptChunk<'chunk>>,
}

pub(super) async fn generate_first_frame_workflow(
    input: FirstFrameWorkflowInput<'_, '_, '_, '_>,
) -> Result<Vec<NewChapterVideoPrompt>, AppError> {
    let FirstFrameWorkflowInput {
        service,
        spec,
        project_id,
        chapter_id,
        source,
        source_spans,
        variables: vars,
        project_context,
        available_assets,
        reference_image_ids,
        reference_ids,
        allowed_reference_ids,
        segment_json,
        segment_plan,
        style,
        prompt_output_language,
        aspect_ratio,
        frame_source_prompt_type,
        workflow_run_id,
        expected_frame_count,
        regenerate_existing,
        frame_draft_common,
        segment_draft,
        prompt_renderer,
        cancel_token,
        progress,
        chunks,
    } = input;
    let validate_frame_prompts_for_spec =
        |frame_prompts: &[VideoFramePrompt]| -> Result<(), AppError> {
            validate_first_frame_prompts(frame_prompts, segment_plan, allowed_reference_ids)
        };
    let message = if regenerate_existing {
        spec.frame_regenerate_progress
    } else {
        spec.frame_generate_progress
    };
    if !regenerate_existing {
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
    report_video_prompt_progress(progress, 60, 100, message)?;
    let frame_prompt_available_assets = frame_prompt_available_assets_context(available_assets)?;
    let frame_prompt_context = FramePromptContext {
        prompt_renderer,
        project_id,
        chapter_summary: source.summary,
        chapter_analysis: source.analysis,
        segment_plan: segment_json,
        available_assets: &frame_prompt_available_assets,
        style,
        prompt_output_language,
        aspect_ratio,
        reference_ids,
    };
    let frame_user = build_first_frame_prompt_user(frame_prompt_context, source.content).await?;
    let frame_context_manifest = build_video_context_manifest(VideoContextManifestInput {
        prompt_renderer,
        project_id,
        variables: vars,
        source: &source,
        source_spans,
        prompt_names: &["chapter_video_segment_plan", spec.frame_prompt_name],
        previous_outputs: &[VideoContextPreviousOutput {
            pack_kind: VIDEO_PREVIOUS_OUTPUT_SEGMENT_PLAN,
            source_name: "chapter_video_segment_plan",
            parsed_output: segment_json,
            raw_output: None,
            placeholder: false,
        }],
        project_context,
        asset_context: Some(&frame_prompt_available_assets),
        dynamic_user_content: frame_user.clone(),
    })
    .await?;
    let frame_step_key = format!("{}:batch:first_frame", spec.frame_prompt_name);
    let frame_fingerprint_json = video_stage_fingerprint(
        spec.frame_prompt_name,
        &frame_context_manifest,
        json!({
            "project_id": project_id,
            "chapter_id": chapter_id,
            "frame_prompt_type": spec.frame_prompt_type,
            "expected_frame_count": expected_frame_count,
        }),
    );
    let frame_workflow_run_id =
        workflow::required_llm_call_workflow_run_id(workflow_run_id, &frame_step_key)
            .map_err(workflow_checkpoint_contract_app_error)?;
    let (frame_trace, frame_prompts) = render_or_restore_json_with_validation_with_context(
        RenderOrRestoreJsonWithValidationInput {
            service,
            workflow_run_id: frame_workflow_run_id,
            call_key: frame_step_key,
            call_order: 300,
            input_fingerprint_json: frame_fingerprint_json,
            prompt_renderer,
            prompt_name: spec.frame_prompt_name,
            project_id,
            variables: vars,
            context_manifest: frame_context_manifest,
            temperature: 0.45,
            cancel_token,
            step: VideoPromptStepProgress::new(60, 100, message)
                .with_span(ProgressSpan::new(60, 90)),
            progress,
            chunks,
            structured_progress: Some(frame_prompt_progress_spec(
                "首帧",
                Some(expected_frame_count),
            )),
            checkpoint_context: "首帧提示词断点",
        },
        |raw| {
            let first_segment = segment_plan.first().ok_or_else(|| {
                AppError::InvalidArgument("首帧串联片段规划至少需要 1 个片段".to_string())
            })?;
            let frame_prompts = parse_video_frame_prompts_with_defaults(
                raw,
                &[VideoFramePromptDefaults {
                    frame_number: 1,
                    duration_secs: first_segment.duration_secs,
                    aspect_ratio,
                    segment_numbers: vec![first_segment.segment_number],
                    reference_image_ids: reference_image_ids.to_vec(),
                }],
                prompt_output_language,
            )?;
            validate_frame_prompts_for_spec(&frame_prompts)?;
            Ok(frame_prompts)
        },
        |frame_prompts| serialize_video_value(frame_prompts, "首帧提示词断点"),
    )
    .await?;
    let frame_drafts = first_frame_prompt_drafts(FirstFramePromptDraftsInput {
        common: frame_draft_common,
        source_prompt_type: frame_source_prompt_type,
        first_frame_prompts: &frame_prompts,
    });
    let frame_drafts = attach_generation_trace_to_all_drafts(
        frame_drafts,
        video_prompt_generation_trace(frame_trace, workflow_run_id),
    );
    let mut checkpoint_drafts = Vec::new();
    if let Some(draft) = segment_draft {
        checkpoint_drafts.push(draft.clone());
    }
    checkpoint_drafts.extend(frame_drafts.clone());
    service
        .sync_chapter_prompts(chapter_id, checkpoint_drafts)
        .await?;
    report_video_prompt_item_committed(
        progress,
        90,
        100,
        spec.frame_prompt_type,
        1,
        1,
        expected_frame_count,
        spec.frame_saved_progress,
    )?;
    Ok(frame_drafts)
}
