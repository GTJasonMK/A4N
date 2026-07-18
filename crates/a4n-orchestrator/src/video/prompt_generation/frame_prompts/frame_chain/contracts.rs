use std::collections::HashMap;

use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::video::prompt_generation::ProgressSpan;
use crate::video::prompt_generation::frame_prompts::FramePromptGenerationSpec;
use crate::video::{
    ChapterVideoPromptOptions, ChapterVideoPromptStageInput, ChapterVideoSourceContext,
    OrchestratorContext,
};
use a4n_core::source_spans::SourceSpan;
use a4n_prompt::rendering::PromptRenderer;

pub(in crate::video::prompt_generation::frame_prompts) struct SegmentPlanForSpecInput<'a, 'source> {
    pub service: &'a OrchestratorContext,
    pub spec: &'a FramePromptGenerationSpec,
    pub stage: ChapterVideoPromptStageInput<'a, 'source>,
}

pub(in crate::video::prompt_generation::frame_prompts) struct FramePromptChainInput<'a, 'source> {
    pub service: &'a OrchestratorContext,
    pub spec: &'a FramePromptGenerationSpec,
    pub project_id: i64,
    pub chapter_id: i64,
    pub source: ChapterVideoSourceContext<'source>,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub options: ChapterVideoPromptOptions,
    pub cancel_token: Option<&'a CancellationToken>,
}

pub(in crate::video::prompt_generation::frame_prompts) struct ShootingScriptProjectionPreparationInput<
    'a,
    'source,
> {
    pub service: &'a OrchestratorContext,
    pub project_id: i64,
    pub chapter_id: i64,
    pub source: ChapterVideoSourceContext<'source>,
    pub source_spans: &'a [SourceSpan],
    pub project_context: Option<&'a Value>,
    pub style: &'a str,
    pub duration_secs: i32,
    pub aspect_ratio: &'a str,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub variables: &'a HashMap<String, Value>,
    pub workflow_run_id: Option<i64>,
    pub progress_span: ProgressSpan,
    pub cancel_token: Option<&'a CancellationToken>,
}

pub(super) struct SegmentPlanFromShootingScriptProjectionInput<'a, 'source> {
    pub shooting_script_projection: ShootingScriptProjectionPreparationInput<'a, 'source>,
    pub segment_context: &'a str,
    pub segment_progress: &'a str,
    pub segment_span: ProgressSpan,
}
