use std::collections::{BTreeSet, HashMap, HashSet};

use a4n_core::models::chapter::chapter_video_prompt;
use a4n_core::prompt_language::PromptOutputLanguage;
use a4n_core::source_spans::SourceSpan;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::video::prompt_generation::frame_prompts::{
    FramePromptGenerationSpec, NewChapterVideoPrompt, OrchestratorContext, ProgressSpan,
    VideoPromptDraftCommonInput, VideoPromptProgress,
};
use crate::video::prompt_json::VideoFramePrompt;
use crate::video::{ChapterVideoSourceContext, VideoPromptChunk};

use super::super::boundary_frames::BoundaryFrameReuseInput;

pub(in crate::video::prompt_generation::frame_prompts::frame_chain) struct BoundaryFrameWorkflowInput<
    'a,
    'source,
    'progress,
    'chunk,
> {
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) service:
        &'a OrchestratorContext,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) spec:
        &'a FramePromptGenerationSpec,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) project_id: i64,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) chapter_id: i64,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) source:
        ChapterVideoSourceContext<'source>,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) source_metadata: &'a Value,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) source_spans:
        &'a [SourceSpan],
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) variables:
        &'a HashMap<String, Value>,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) project_context:
        Option<&'a Value>,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) available_assets: &'a Value,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) reference_image_ids:
        &'a [i64],
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) segment_json: &'a Value,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) segment_plan:
        &'a [crate::video::prompt_json::VideoSegmentPlan],
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) style: &'a str,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) prompt_output_language:
        PromptOutputLanguage,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) aspect_ratio: &'a str,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) frame_source_prompt_type:
        &'a str,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) workflow_run_id:
        Option<i64>,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) expected_frame_count: usize,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) prompt_batch_size: usize,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) regenerate_existing: bool,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) reusable_rows:
        Vec<chapter_video_prompt::Model>,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) allowed_reference_ids:
        &'a HashSet<i64>,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) frame_draft_common:
        VideoPromptDraftCommonInput<'a>,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) segment_draft:
        Option<&'a NewChapterVideoPrompt>,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) prompt_renderer:
        &'a dyn PromptRenderer,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) cancel_token:
        Option<&'a CancellationToken>,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) progress:
        &'a mut Option<VideoPromptProgress<'progress>>,
    pub(in crate::video::prompt_generation::frame_prompts::frame_chain) chunks:
        &'a mut Option<VideoPromptChunk<'chunk>>,
}

pub(super) struct BoundaryWorkflowPreparationInput<'a, 'progress> {
    pub(super) reuse: BoundaryFrameReuseInput<'a>,
    pub(super) expected_frame_count: usize,
    pub(super) reused_frame_label: &'a str,
    pub(super) progress: &'a mut Option<VideoPromptProgress<'progress>>,
}

pub(super) enum PreparedBoundaryWorkflow {
    Complete(Vec<NewChapterVideoPrompt>),
    Pending(PendingBoundaryWorkflow),
}

pub(super) struct PendingBoundaryWorkflow {
    pub(super) boundary_frame_image_id_by_number: HashMap<i32, i64>,
    pub(super) allowed_reference_ids: HashSet<i64>,
    pub(super) frame_prompts: Vec<VideoFramePrompt>,
    pub(super) kept_prompt_ids: BTreeSet<i64>,
}

pub(super) struct BoundaryIterationInput<'a, 'progress> {
    pub(super) service: &'a OrchestratorContext,
    pub(super) spec: &'a FramePromptGenerationSpec,
    pub(super) project_id: i64,
    pub(super) chapter_id: i64,
    pub(super) expected_frame_count: usize,
    pub(super) prompt_batch_size: usize,
    pub(super) generated_this_run: usize,
    pub(super) regenerate_existing: bool,
    pub(super) current_frame_count: usize,
    pub(super) kept_prompt_ids: &'a BTreeSet<i64>,
    pub(super) progress: &'a mut Option<VideoPromptProgress<'progress>>,
}

pub(super) struct PreparedBoundaryIteration {
    pub(super) frame_number: i32,
    pub(super) generation_message: String,
    pub(super) progress_span: ProgressSpan,
}
