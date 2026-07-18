use crate::AppState;
use crate::actions::access;
use crate::actions::dependency_conflict::DownstreamConflictStrategy;
pub use crate::actions::request_preview::{ModelRequestPreview, ModelRequestPreviewStep};
use crate::actions::{image_records, video_records};
use a4n_core::error::AppError;
use a4n_core::models::chapter::{chapter, chapter_video_prompt};
use a4n_core::source_spans::metadata_source_content_sha256;
use a4n_core::source_spans::source_text_hash as source_content_sha256;
pub use a4n_orchestrator::video::ChapterVideoSource;
pub use a4n_orchestrator::video::VideoPromptRequestPreview;
use a4n_orchestrator::video::{
    ChapterVideoPromptOptions, ChapterVideoPromptStageInput, ChapterVideoSourceContext,
};
use a4n_orchestrator::video::{
    ProjectChapterVideoSource, UpdateChapterVideoPromptInput as DbUpdateChapterVideoPromptInput,
    VideoPromptDownstreamDependencies,
};
use a4n_orchestrator::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap};
use tokio_util::sync::CancellationToken;

mod optimization;

pub use self::optimization::{OptimizeChapterVideoPromptInput, optimize_chapter_video_prompt};

mod api;
mod generation;
mod helpers;
mod mutation;
mod preparation;
mod preview;
mod records;
mod source;
mod types;
mod workflow;

pub(crate) use self::{
    api::ensure_prompt_matches_current_chapter, source::resolve_project_chapter_video_source,
};
pub use self::{
    api::{
        chapter_video_prompt_task_scope, generate_chapter_video_frame_prompts,
        generate_chapter_video_prompts, generate_chapter_video_segment_plan,
        preview_chapter_video_frame_prompt_model_request,
        preview_chapter_video_frame_prompt_requests, preview_chapter_video_prompt_model_request,
        preview_chapter_video_prompt_requests,
    },
    records::{
        delete_chapter_video_prompt, delete_chapter_video_prompt_group, list_chapter_video_prompts,
        update_chapter_video_prompt,
    },
    types::{
        ChapterVideoPromptSummary, DeleteChapterVideoPromptGroupResult,
        GenerateChapterVideoPromptInput, GeneratePromptOptionsInput, UpdateChapterVideoPromptInput,
        VideoPromptChunk, VideoPromptProgress, VideoPromptTaskScope,
    },
};
use self::{
    generation::*,
    helpers::*,
    mutation::*,
    preparation::*,
    preview::*,
    source::{
        build_video_prompt_vars, source_context, validate_generate_prompt_options,
        validate_generate_prompt_options_for_preview, video_prompt_workflow_fingerprint,
    },
    types::*,
    workflow::*,
};

#[cfg(test)]
mod tests;
