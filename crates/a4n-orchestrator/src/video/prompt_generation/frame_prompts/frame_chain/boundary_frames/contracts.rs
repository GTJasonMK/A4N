use std::collections::{HashMap, HashSet};

use a4n_core::models::chapter::chapter_video_prompt;
use a4n_core::prompt_language::PromptOutputLanguage;
use a4n_core::source_spans::SourceSpan;
use a4n_llm::context::LlmContextManifest;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::video::prompt_generation::frame_prompts::{
    FramePromptGenerationSpec, NewChapterVideoPrompt, VideoPromptDraftCommonInput,
};
use crate::video::prompt_json::{VideoFramePrompt, VideoSegmentPlan};
use crate::video::{
    ChapterVideoSourceContext, OrchestratorContext, VideoPromptChunk, VideoPromptProgress,
};

use crate::video::prompt_generation::ProgressSpan;
use crate::video::prompt_generation::frame_prompts::RenderedVideoPromptTrace;

pub(in crate::video::prompt_generation::frame_prompts::frame_chain) struct BoundaryFrameReuseInput<
    'a,
> {
    pub service: &'a OrchestratorContext,
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub chapter_id: i64,
    pub source_metadata: &'a Value,
    pub style: &'a str,
    pub aspect_ratio: &'a str,
    pub execution_identity: &'a Value,
    pub frame_source_prompt_type: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub segment_plan: &'a [VideoSegmentPlan],
    pub allowed_reference_ids: &'a HashSet<i64>,
    pub reusable_rows: Vec<chapter_video_prompt::Model>,
    pub regenerate_existing: bool,
}

pub(in crate::video::prompt_generation::frame_prompts::frame_chain) struct PreparedBoundaryFrameReuse
{
    pub boundary_frame_image_id_by_number: HashMap<i32, i64>,
    pub allowed_reference_ids: HashSet<i64>,
    pub prefix_rows: Vec<chapter_video_prompt::Model>,
    pub frame_prompts: Vec<VideoFramePrompt>,
}

pub(in crate::video::prompt_generation::frame_prompts::frame_chain) struct BoundaryFrameRequestInput<
    'a,
    'source,
> {
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub chapter_id: i64,
    pub frame_number: i32,
    pub source: ChapterVideoSourceContext<'source>,
    pub source_spans: &'a [SourceSpan],
    pub variables: &'a HashMap<String, Value>,
    pub project_context: Option<&'a Value>,
    pub available_assets: &'a Value,
    pub reference_image_ids: &'a [i64],
    pub segment_json: &'a Value,
    pub segment_plan: &'a [VideoSegmentPlan],
    pub frame_prompts: &'a [VideoFramePrompt],
    pub boundary_frame_image_id_by_number: &'a HashMap<i32, i64>,
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub aspect_ratio: &'a str,
}

pub(in crate::video::prompt_generation::frame_prompts::frame_chain) struct PreparedBoundaryFrameRequest
{
    pub previous_frame_image_id: Option<i64>,
    pub allowed_reference_ids: HashSet<i64>,
    pub reference_image_ids: Vec<i64>,
    pub context_manifest: LlmContextManifest,
    pub step_key: String,
    pub fingerprint_json: Value,
}

pub(in crate::video::prompt_generation::frame_prompts::frame_chain) struct BoundaryFrameCheckpointInput<
    'a,
    'progress,
    'chunk,
> {
    pub service: &'a OrchestratorContext,
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub frame_number: i32,
    pub workflow_run_id: Option<i64>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub cancel_token: Option<&'a CancellationToken>,
    pub progress_span: ProgressSpan,
    pub generation_message: &'a str,
    pub prepared_request: PreparedBoundaryFrameRequest,
    pub segment_plan: &'a [VideoSegmentPlan],
    pub aspect_ratio: &'a str,
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
    pub progress: &'a mut Option<VideoPromptProgress<'progress>>,
    pub chunks: &'a mut Option<VideoPromptChunk<'chunk>>,
}

pub(in crate::video::prompt_generation::frame_prompts::frame_chain) struct BoundaryFramePersistenceInput<
    'a,
> {
    pub service: &'a OrchestratorContext,
    pub spec: &'a FramePromptGenerationSpec,
    pub chapter_id: i64,
    pub frame_number: i32,
    pub workflow_run_id: Option<i64>,
    pub frame_draft_common: VideoPromptDraftCommonInput<'a>,
    pub frame_source_prompt_type: &'a str,
    pub segment_draft: Option<&'a NewChapterVideoPrompt>,
    pub frame_trace: RenderedVideoPromptTrace,
    pub frame_prompt: VideoFramePrompt,
    pub frame_prompts: &'a mut Vec<VideoFramePrompt>,
    pub allowed_reference_ids: &'a mut HashSet<i64>,
}

pub(in crate::video::prompt_generation::frame_prompts::frame_chain) struct BoundaryFrameFinalizationInput<
    'a,
    'progress,
> {
    pub spec: &'a FramePromptGenerationSpec,
    pub expected_frame_count: usize,
    pub generated_this_run: usize,
    pub frame_draft_common: VideoPromptDraftCommonInput<'a>,
    pub frame_source_prompt_type: &'a str,
    pub frame_prompts: &'a [VideoFramePrompt],
    pub segment_plan: &'a [VideoSegmentPlan],
    pub allowed_reference_ids: &'a HashSet<i64>,
    pub boundary_frame_image_id_by_number: &'a HashMap<i32, i64>,
    pub progress: &'a mut Option<VideoPromptProgress<'progress>>,
}
