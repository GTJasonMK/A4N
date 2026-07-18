mod checkpoint;
mod contracts;
mod finalization;
mod persistence;
mod request;
mod reuse;

pub(super) use checkpoint::*;
pub(super) use contracts::*;
pub(super) use finalization::*;
pub(super) use persistence::*;
pub(super) use request::*;
pub(super) use reuse::*;

use std::collections::{BTreeSet, HashMap, HashSet};

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_db::workflow;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::video::library::available_video_image_ids;
use crate::video::prompt_context::{
    FramePromptContext, VIDEO_PREVIOUS_OUTPUT_SEGMENT_PLAN, VideoContextManifestInput,
    VideoContextPreviousOutput, build_single_frame_prompt_user, build_video_context_manifest,
};
use crate::video::prompt_generation::frame_prompts::{
    FramePromptDraftsInput, FramePromptGenerationSpec, NewChapterVideoPrompt,
    VideoPromptDraftCommonInput, attach_generation_trace_to_draft,
    available_assets_with_previous_boundary_frame, boundary_frame_generation_context,
    frame_prompt_available_assets_context, frame_prompt_drafts,
    previous_boundary_frame_asset_context,
};
use crate::video::prompt_json::{VideoFramePrompt, VideoFramePromptDefaults, VideoSegmentPlan};
use crate::video::{
    ChapterVideoSourceContext, OrchestratorContext, VideoPromptChunk, VideoPromptProgress,
};
use crate::workflow::workflow_checkpoint_contract_app_error;
use a4n_core::prompt_language::PromptOutputLanguage;
use a4n_core::source_spans::SourceSpan;
use a4n_llm::context::LlmContextManifest;
use a4n_prompt::rendering::PromptRenderer;

use super::super::{
    ProgressSpan, RenderOrRestoreJsonWithValidationInput, RenderedVideoPromptTrace,
    VideoPromptStepProgress, apply_boundary_frame_style_template, backend_frame_reference_ids,
    boundary_frame_expected_duration, boundary_frame_expected_segment_numbers,
    frame_prompt_progress_spec, parse_single_video_frame_prompt,
    render_or_restore_json_with_validation_with_context, report_video_prompt_progress,
    serialize_video_value, validate_frame_prompts, validate_partial_boundary_frame_prompts,
    validate_single_boundary_frame_prompt, video_prompt_generation_trace, video_stage_fingerprint,
};
