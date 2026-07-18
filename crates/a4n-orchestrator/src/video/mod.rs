// 视频生成域：章节视频提示词与视频产物管理。

mod access;
mod config;
mod contracts;
mod duration;
mod errors;
mod images;
mod library;
mod materials;
mod prompt_context;
mod prompt_generation;
mod prompt_json;
mod prompt_preview;
mod prompt_records;
mod prompt_store;
mod prompts;
mod record_facade;
mod records;
mod source;

#[cfg(test)]
mod facade_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
pub(crate) use test_support::{test_video_orchestrator, test_video_orchestrator_with_llm};

pub use a4n_core::prompt_language::PromptOutputLanguage;
use a4n_core::{
    error::AppError,
    models::{
        ai::{generated_image, generated_video},
        chapter::{chapter, chapter_video_prompt},
    },
};
pub use a4n_db::generation::RecordStartedGenerationRunInput;
pub use a4n_db::video::{
    CreateGeneratedImageRecordDraftInput, CreateGeneratedVideoRecordDraftInput,
    UpdateGeneratedImageRecordInput, UpdateGeneratedVideoRecordInput,
};
pub use a4n_db::video_prompt::{UpdateChapterVideoPromptInput, VideoPromptDownstreamDependencies};

pub use contracts::{
    ChapterVideoPromptOptions, ChapterVideoPromptStageInput, ChapterVideoSource,
    ChapterVideoSourceContext, ProjectChapterVideoSource,
};
pub use duration::VideoDurationCapability;
pub use materials::is_project_reusable_video_material_image;
use serde_json::Value;
#[cfg(test)]
use tokio_util::sync::CancellationToken;

use crate::OrchestratorContext;
use contracts::{
    FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT, IMPORT_VIDEO_PROMPT_TYPE, VideoPromptChunk,
    VideoPromptProgress,
};
use errors::video_llm_step_error;
use materials::is_video_frame_input_metadata;

pub use self::config::{
    UpdateVideoConfigParams, VideoConfigParams, duration_capability_for_config,
    pick_provider_for_config, validate_video_config_model, validate_video_config_params,
    validate_video_duration_for_config_secs, validate_video_max_concurrent,
};
pub use self::prompt_context::validate_prompt_options_with_reference_limit;
pub use self::prompt_json::{
    validate_direct_prompt_text, video_motion_prompt_has_temporal_storyboard,
    video_prompt_missing_required_section_for_language, video_prompt_stale_section_for_language,
    video_prompt_upload_status_term,
};
pub use self::prompt_preview::{VideoPromptRequestPreview, VideoPromptRequestPreviewStep};
#[cfg(test)]
pub use self::prompt_records::VideoPromptUpdateInput;
pub use contracts::{
    BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION, CHAT_VIDEO_CONTRACT_VERSION,
    FIRST_FRAME_CHAIN_CONTRACT_VERSION, FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
    STORYBOARD_PROMPT_CONTRACT_VERSION, STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION,
};

pub(crate) use errors::video_db_error;

#[cfg(test)]
mod tests;
