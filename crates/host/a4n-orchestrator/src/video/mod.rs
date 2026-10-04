mod assembly;
mod provider;
// 视频生成域：章节视频提示词与视频产物管理。

mod access;

mod boundary_frame_rules;
mod chapter_video_generation;
mod chapter_video_request_rules;
mod config;
mod contracts;
mod errors;
mod first_frame_chain_rules;
mod generation;

mod job_status_rules;
mod library;

mod prompts;

mod records;
mod reference_media_generation;
mod reference_media_rules;
mod source;
mod storyboard_frame_rules;
mod tail_frame_rules;
mod timeline;

#[cfg(test)]
mod facade_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
pub(in crate::video) use test_support::{
    test_video_orchestrator, test_video_orchestrator_with_llm,
};

// 子模块通过 `use crate::video::{...}` / `super::` 共享的类型与错误映射。
pub(crate) use a4n_domain::video_prompt::VideoPromptDownstreamDependencies;
#[cfg(test)]
pub(in crate::video) use a4n_error::AppError;

pub(crate) use a4n_video::ensure_provider_generation_run_updated;
pub(in crate::video) use assembly::clip_rules::{
    AssemblyClipOrderAnchor, assembly_clip_display_shot_number,
    assembly_clip_insert_after_video_id, assembly_clip_sort_order,
    assembly_insert_metadata_with_position, is_assembly_clip_video_type,
    next_assembly_insert_sort_order,
};
pub(in crate::video) use assembly::postprocess_rules::{
    parse_postprocess_edit_metadata, postprocess_edit_timing,
};
pub(in crate::video) use assembly::push_rules::ASSEMBLY_PUSH_STATUS_PENDING;
pub(in crate::video) use assembly::push_rules::{
    ASSEMBLY_PUSH_STATUS_ACCEPTED, ASSEMBLY_PUSH_STATUS_REJECTED, active_assembly_push_source_ids,
    assembly_push_metadata_with_status, assembly_push_records,
    ensure_active_pushes_match_clip_prefix, ensure_insert_position_not_inside_pushed_prefix,
    latest_non_rejected_push_index, parse_assembly_push_metadata, selected_assembly_push_end_index,
    validate_assembly_workflow_mode,
};
pub(in crate::video) use boundary_frame_rules::{
    boundary_frame_numbers_for_prompt, frame_prompt_matches_video_prompt,
};
pub(in crate::video) use chapter_video_generation::GenerateChapterVideosBatchInput;
pub(in crate::video) use contracts::ProjectChapterVideoSource;
pub(in crate::video) use contracts::{
    BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION, CHAT_VIDEO_CONTRACT_VERSION,
    ChapterVideoPromptStageInput, ChapterVideoSourceContext, FIRST_FRAME_CHAIN_CONTRACT_VERSION,
    FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT, FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
    STORYBOARD_PROMPT_CONTRACT_VERSION, STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION,
    VIDEO_PROMPT_CHECKPOINT_STAGE_CODE, VideoPromptChunk, VideoPromptProgress,
    VideoPromptProgressUpdate,
};
pub(crate) use contracts::{ChapterVideoPromptOptions, ChapterVideoSource};
pub(in crate::video) use first_frame_chain_rules::first_frame_prompt_matches_video_prompt;
pub(in crate::video) use generation::started_video_generation_run_input;
pub(in crate::video) use job_status_rules::{
    active_provider_job_id, canvas_video_workflow_context, has_active_provider_job,
    is_canvas_video_record,
};
pub(crate) use job_status_rules::{
    can_adopt_authoritative_video_completion, ensure_positive_generated_video_id,
    ensure_positive_project_id_for_video, ensure_positive_video_prompt_id,
    normalized_provider_job_id, provider_poll_config_id, required_generated_video_failure_message,
    required_provider_job_id, validate_generated_video_waiting_status,
};
pub(crate) use library::{
    image_type_rank, is_video_reference_image_candidate, resolve_video_reference_image_project_ids,
    video_reference_scope_rank,
};
pub(in crate::video) use prompts::application::{
    GenerateChapterVideoPromptInput, VideoPromptApplicationContext,
};
pub(crate) use prompts::application::{
    validate_video_final_prompt_generation_options, validate_video_frame_prompt_generation_options,
    validate_video_segment_plan_generation_options,
};
pub(in crate::video) use prompts::chain_rules::{
    PromptChainFields, prompt_chain_matches, prompt_chain_matches_for_initial_frame_image,
    prompt_source_type_matches,
};
pub(crate) use prompts::generation::validate_video_inserted_segment_generation_options;
pub(in crate::video) use prompts::generation::{
    generate_inserted_segment_draft, load_video_segment_plan_draft_checkpoints,
    load_video_segment_plan_draft_recipe_state, update_video_segment_plan_draft_recipe_checkpoint,
};
pub(crate) use prompts::workflow::{
    video_prompt_optimization_workflow_run_input, video_prompt_workflow_run_input,
};
pub(in crate::video) use provider::execution::VideoGenerationCheckpointContext;
pub(in crate::video) use provider::execution::{
    VIDEO_GENERATION_PROGRESS_TOTAL, VideoGenerationProgress, VideoProviderExecutionInput,
};
pub(in crate::video) use provider::prompt_text_rules::{
    MIN_PROVIDER_MOTION_PROMPT_CHARS, validate_provider_model_text,
};
pub(in crate::video) use provider::request_rules::prompt_provider_workflow_for_recipe;
pub(in crate::video) use provider::request_rules::{
    MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES, ProviderGenerationWorkflow,
    boundary_frame_missing_image_message, ensure_newapi_first_frame_image_supported,
    ensure_provider_generation_workflow, first_frame_chain_missing_start_image_message,
    linked_video_preview_config_id, provider_image_binding_violation_messages,
    reference_ids_with_storyboard_image, required_prompt_source_prompt_type,
    required_video_execution_identity, storyboard_missing_image_message,
};
pub(crate) use provider::workflow_rules::{
    video_provider_progress_workflow_run_input, video_provider_workflow_business_key,
    video_provider_workflow_fingerprint,
};
pub(crate) use reference_media_rules::validate_reference_media_start_input;
pub(in crate::video) use reference_media_rules::{
    MIN_PROVIDER_VISUAL_PROMPT_CHARS, validate_reference_media_import_options,
};
pub(in crate::video) use storyboard_frame_rules::{
    latest_storyboard_image_id, storyboard_prompt_matches_video_prompt,
};
pub(crate) use tail_frame_rules::{tail_frame_reference_belongs_to_video, video_shot_number};

pub(in crate::video) use crate::OrchestratorContext;
#[cfg(test)]
pub(in crate::video) use tokio_util::sync::CancellationToken;

pub(in crate::video) use self::config::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, GEMINI_VEO_VIDEO_BACKEND, NEWAPI_VIDEO_BACKEND,
    OPENAI_CHAT_VIDEO_BACKEND, OPENAI_COMPATIBLE_VIDEO_BACKEND, SEEDANCE2_VIDEO_BACKEND,
    duration_capability_for_provider_config,
};
pub(crate) use self::config::{
    validate_video_duration_for_provider_config_secs, video_provider_config_from_record,
};

pub(crate) use errors::video_db_error;
pub(in crate::video) use errors::video_llm_step_error;
pub(crate) use records::{VideoGenerationRecordInput, generated_video_summary};

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(in crate::video) use tests::{
    insert_derived_video_project, insert_video_chapter_without_shooting_script,
    insert_video_project,
};
