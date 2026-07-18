use a4n_core::error::AppError;
use a4n_core::video_execution::VideoShootingScriptIdentityV1;
use serde_json::Value;

use super::super::{
    ChapterVideoSourceContext, OrchestratorContext, PreparedVideoPromptPreviewInputs,
    PromptRenderer, VIDEO_PREVIOUS_OUTPUT_SHOOTING_SCRIPT, VideoContextManifestInput,
    VideoContextPreviousOutput, VideoFramePrompt, VideoPlanningPromptContext,
    VideoPromptRequestPreviewStep, build_segment_plan_for_shot_user, build_video_context_manifest,
    build_video_prompt_request, first_cinematic_shot_for_preview, step_from_request,
};
use crate::shooting_script::ChapterShootingScriptSourceInput;
use crate::video::prompt_generation::video_projection_from_shooting_script;

pub(super) struct SegmentPlanStepPreviewInput<'a, 'source> {
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) project_id: i64,
    pub(super) source: &'a ChapterVideoSourceContext<'source>,
    pub(super) input: &'a PreparedVideoPromptPreviewInputs,
    pub(super) project_context: Option<&'a Value>,
    pub(super) shooting_script_projection: &'a Value,
    pub(super) current_shot: &'a Value,
    pub(super) previous_output_placeholder: bool,
    pub(super) title: &'a str,
}

pub(in crate::video::prompt_preview::local) struct SegmentPlanPreviewStepsInput<'a, 'source> {
    pub(in crate::video::prompt_preview::local) prompt_renderer: &'a dyn PromptRenderer,
    pub(in crate::video::prompt_preview::local) project_id: i64,
    pub(in crate::video::prompt_preview::local) chapter_id: i64,
    pub(in crate::video::prompt_preview::local) source: &'a ChapterVideoSourceContext<'source>,
    pub(in crate::video::prompt_preview::local) input: &'a PreparedVideoPromptPreviewInputs,
    pub(in crate::video::prompt_preview::local) project_context: Option<&'a Value>,
    pub(in crate::video::prompt_preview::local) segment_title: &'a str,
}

impl OrchestratorContext {
    pub(in crate::video::prompt_preview::local) async fn push_segment_plan_preview_steps(
        &self,
        steps: &mut Vec<VideoPromptRequestPreviewStep>,
        input: SegmentPlanPreviewStepsInput<'_, '_>,
    ) -> Result<(), AppError> {
        let SegmentPlanPreviewStepsInput {
            prompt_renderer,
            project_id,
            chapter_id,
            source,
            input,
            project_context,
            segment_title,
        } = input;
        let resolved = self
            .require_current_chapter_shooting_script(ChapterShootingScriptSourceInput {
                project_id,
                source_chapter_id: chapter_id,
            })
            .await?;
        let record = &resolved.shooting_script.record;
        let projection = video_projection_from_shooting_script(
            &resolved.shooting_script.script,
            VideoShootingScriptIdentityV1 {
                record_id: record.id,
                revision: record.revision,
                source_revision_key: record.source_revision_key.clone(),
                source_content_sha256: record.source_content_sha256.clone(),
                script_sha256: record.script_sha256.clone(),
            },
            input.duration_secs,
        )?;
        let shooting_script_projection_json = serde_json::to_value(projection)?;
        steps.push(
            segment_plan_step_from_preview_input(SegmentPlanStepPreviewInput {
                prompt_renderer,
                project_id,
                source,
                input,
                project_context,
                shooting_script_projection: &shooting_script_projection_json,
                current_shot: &first_cinematic_shot_for_preview(&shooting_script_projection_json)?,
                previous_output_placeholder: false,
                title: segment_title,
            })
            .await?,
        );
        Ok(())
    }
}

pub(super) async fn segment_plan_step_from_preview_input(
    input: SegmentPlanStepPreviewInput<'_, '_>,
) -> Result<VideoPromptRequestPreviewStep, AppError> {
    let SegmentPlanStepPreviewInput {
        prompt_renderer,
        project_id,
        source,
        input,
        project_context,
        shooting_script_projection,
        current_shot,
        previous_output_placeholder,
        title,
    } = input;
    let context = VideoPlanningPromptContext {
        prompt_renderer,
        project_id,
        chapter_summary: source.summary,
        chapter_analysis: source.analysis,
        project_context,
        style: &input.style,
        duration_secs: input.duration_secs,
        aspect_ratio: &input.aspect_ratio,
    };
    let user = build_segment_plan_for_shot_user(context, current_shot, &input.source_spans).await?;
    let context_manifest = build_video_context_manifest(VideoContextManifestInput {
        prompt_renderer,
        project_id,
        variables: &input.vars,
        source,
        source_spans: &input.source_spans,
        prompt_names: &["chapter_video_segment_plan"],
        previous_outputs: &[VideoContextPreviousOutput {
            pack_kind: VIDEO_PREVIOUS_OUTPUT_SHOOTING_SCRIPT,
            source_name: "chapter_shooting_script_video_projection",
            parsed_output: shooting_script_projection,
            raw_output: None,
            placeholder: previous_output_placeholder,
        }],
        project_context,
        asset_context: None,
        dynamic_user_content: user.clone(),
    })
    .await?;
    let request = build_video_prompt_request(
        prompt_renderer,
        "chapter_video_segment_plan",
        project_id,
        &input.vars,
        context_manifest,
        0.35,
    )
    .await?;
    step_from_request("segment_plan", title, "chapter_video_segment_plan", request)
}

pub(in crate::video::prompt_preview::local) fn frame_prompts_from_preview_context(
    frame_json: &Value,
) -> Result<Vec<VideoFramePrompt>, AppError> {
    let frames = frame_json
        .get("frames")
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument("视频提示词预览缺少 frames 上下文".to_string()))?;
    frames
        .iter()
        .cloned()
        .map(|frame| {
            serde_json::from_value::<VideoFramePrompt>(frame).map_err(|err| {
                AppError::InvalidArgument(format!("视频提示词预览 frames 上下文无效: {err}"))
            })
        })
        .collect()
}
