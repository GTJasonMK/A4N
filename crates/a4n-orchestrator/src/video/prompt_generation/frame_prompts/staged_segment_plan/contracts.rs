use std::collections::HashMap;

use a4n_core::source_spans::SourceSpan;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::super::{
    ChapterVideoSourceContext, OrchestratorContext, ProgressSpan, RenderedVideoPromptTrace,
    VideoPromptChunk, VideoPromptProgress, VideoSegmentPlan, VideoShootingScriptProjection,
};

pub(super) type StagedSegmentPlanResult = (
    Option<RenderedVideoPromptTrace>,
    Value,
    Vec<VideoSegmentPlan>,
    Value,
);

pub(in crate::video::prompt_generation::frame_prompts) struct StagedSegmentPlanInput<
    'a,
    'source,
    'progress,
    'chunk,
> {
    pub(in crate::video::prompt_generation::frame_prompts) service: &'a OrchestratorContext,
    pub(in crate::video::prompt_generation::frame_prompts) prompt_renderer: &'a dyn PromptRenderer,
    pub(in crate::video::prompt_generation::frame_prompts) project_id: i64,
    pub(in crate::video::prompt_generation::frame_prompts) workflow_run_id: Option<i64>,
    pub(in crate::video::prompt_generation::frame_prompts) variables: &'a HashMap<String, Value>,
    pub(in crate::video::prompt_generation::frame_prompts) source:
        ChapterVideoSourceContext<'source>,
    pub(in crate::video::prompt_generation::frame_prompts) source_spans: &'a [SourceSpan],
    pub(in crate::video::prompt_generation::frame_prompts) project_context: Option<&'a Value>,
    pub(in crate::video::prompt_generation::frame_prompts) shooting_script_projection_json:
        &'a Value,
    pub(in crate::video::prompt_generation::frame_prompts) shooting_script_projection:
        &'a VideoShootingScriptProjection,
    pub(in crate::video::prompt_generation::frame_prompts) default_aspect_ratio: &'a str,
    pub(in crate::video::prompt_generation::frame_prompts) style: &'a str,
    pub(in crate::video::prompt_generation::frame_prompts) duration_secs: i32,
    pub(in crate::video::prompt_generation::frame_prompts) context: &'a str,
    pub(in crate::video::prompt_generation::frame_prompts) progress_message: &'a str,
    pub(in crate::video::prompt_generation::frame_prompts) progress_span: ProgressSpan,
    pub(in crate::video::prompt_generation::frame_prompts) cancel_token:
        Option<&'a CancellationToken>,
    pub(in crate::video::prompt_generation::frame_prompts) progress:
        &'a mut Option<VideoPromptProgress<'progress>>,
    pub(in crate::video::prompt_generation::frame_prompts) chunks:
        &'a mut Option<VideoPromptChunk<'chunk>>,
}

pub(super) struct CompletedSegmentPlanRecoveryInput<'a, 'progress> {
    pub(super) service: &'a OrchestratorContext,
    pub(super) workflow_run_id: i64,
    pub(super) fingerprint_json: &'a Value,
    pub(super) shooting_script_projection: &'a VideoShootingScriptProjection,
    pub(super) progress_message: &'a str,
    pub(super) progress_span: ProgressSpan,
    pub(super) progress: &'a mut Option<VideoPromptProgress<'progress>>,
}

pub(super) struct RenderedSegmentPlanShots {
    pub(super) segment_inputs: Vec<Value>,
    pub(super) stage_outputs: Vec<Value>,
    pub(super) last_trace: Option<RenderedVideoPromptTrace>,
}

pub(super) struct FinalizeStagedSegmentPlanInput<'a> {
    pub(super) service: &'a OrchestratorContext,
    pub(super) workflow_run_id: i64,
    pub(super) fingerprint_json: Value,
    pub(super) source_spans: &'a [SourceSpan],
    pub(super) shooting_script_projection: &'a VideoShootingScriptProjection,
    pub(super) duration_secs: i32,
    pub(super) default_aspect_ratio: &'a str,
    pub(super) rendered_shots: RenderedSegmentPlanShots,
}
