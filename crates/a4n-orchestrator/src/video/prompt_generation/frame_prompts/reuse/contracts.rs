use a4n_core::models::chapter::chapter_video_prompt;
use serde_json::Value;

use crate::video::prompt_store::PromptWorkflowMode;
use crate::video::{OrchestratorContext, PromptOutputLanguage};
use a4n_core::source_spans::SourceSpan;

use super::super::FramePromptGenerationSpec;
use crate::video::prompt_json::VideoShootingScriptProjection;

pub(in crate::video::prompt_generation::frame_prompts) struct ReusableSegmentPlanQuery<'a> {
    pub service: &'a OrchestratorContext,
    pub project_id: i64,
    pub chapter_id: i64,
    pub workflow_mode: PromptWorkflowMode,
    pub prompt_type: &'a str,
    pub source_metadata: &'a Value,
    pub source_spans: &'a [SourceSpan],
    pub style: &'a str,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub shooting_script_projection: &'a VideoShootingScriptProjection,
    pub contract_version: &'a str,
    pub context: &'a str,
}

pub(in crate::video::prompt_generation::frame_prompts) struct ReusableSegmentPlanForSpecQuery<'a> {
    pub service: &'a OrchestratorContext,
    pub project_id: i64,
    pub chapter_id: i64,
    pub spec: &'a FramePromptGenerationSpec,
    pub source_metadata: &'a Value,
    pub source_spans: &'a [SourceSpan],
    pub style: &'a str,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
}

pub(crate) struct ReusableSegmentPlanForPreviewQuery<'a> {
    pub service: &'a OrchestratorContext,
    pub project_id: i64,
    pub chapter_id: i64,
    pub workflow_mode: &'a str,
    pub source_metadata: &'a Value,
    pub source_spans: &'a [SourceSpan],
    pub style: &'a str,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
}

pub(in crate::video::prompt_generation::frame_prompts) struct DownstreamSegmentPlanningGuardInput<
    'a,
> {
    pub service: &'a OrchestratorContext,
    pub project_id: i64,
    pub chapter_id: i64,
    pub spec: &'a FramePromptGenerationSpec,
    pub source_metadata: &'a Value,
    pub style: &'a str,
    pub aspect_ratio: &'a str,
    pub operation: &'a str,
}

pub(in crate::video::prompt_generation::frame_prompts) struct ReusableSegmentPlanFromRowInput<'a> {
    pub row: chapter_video_prompt::Model,
    pub source_metadata: &'a Value,
    pub source_spans: &'a [SourceSpan],
    pub style: &'a str,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub shooting_script_projection: &'a VideoShootingScriptProjection,
    pub contract_version: &'a str,
    pub context: &'a str,
    pub draft_workflow_mode: PromptWorkflowMode,
    pub draft_prompt_type: &'a str,
    pub draft_title: &'a str,
}

pub(in crate::video::prompt_generation::frame_prompts) struct ReusableFrameRowsQuery<'a> {
    pub service: &'a OrchestratorContext,
    pub project_id: i64,
    pub chapter_id: i64,
    pub workflow_mode: PromptWorkflowMode,
    pub prompt_type: &'a str,
    pub source_metadata: &'a Value,
    pub style: &'a str,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub reference_ids: &'a Value,
    pub generation_asset_fingerprint: &'a Value,
    pub source_prompt_type: &'a str,
    pub video_execution_identity: &'a Value,
    pub prompt_output_language: PromptOutputLanguage,
}

pub(in crate::video) struct FrameImageLookupContext<'a> {
    pub workflow_mode: PromptWorkflowMode,
    pub prompt_type: &'a str,
    pub source_metadata: &'a Value,
    pub style: &'a str,
    pub aspect_ratio: &'a str,
    pub source_prompt_type: &'a str,
    pub video_execution_identity: &'a Value,
    pub prompt_output_language: PromptOutputLanguage,
}

pub(in crate::video) struct FrameImageLookupRowsQuery<'a> {
    pub service: &'a OrchestratorContext,
    pub project_id: i64,
    pub chapter_id: i64,
    pub context: FrameImageLookupContext<'a>,
}
