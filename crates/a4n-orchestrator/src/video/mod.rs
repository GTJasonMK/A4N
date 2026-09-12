// 视频生成域：章节视频提示词与视频产物管理。

mod access;
mod assembly_clip_rules;
mod assembly_postprocess_rules;
mod assembly_push_rules;
mod boundary_frame_rules;
mod chapter_video_generation;
mod chapter_video_request_rules;
mod config;
mod contracts;
mod errors;
mod first_frame_chain_rules;
mod generation;
mod generation_run_rules;
mod job_status_rules;
mod library;
mod prompt_application;
mod prompt_chain_rules;
mod prompt_context;
mod prompt_generation;
mod prompt_generation_task;
mod prompt_json;
mod prompt_optimization;
mod prompt_optimization_checkpoint;
mod prompt_optimization_rules;
mod prompt_ports;
mod prompt_records;
mod prompt_store;
mod prompt_workflow_rules;
mod prompts;
mod provider_execution;
mod provider_prompt_text_rules;
mod provider_request_rules;
mod provider_workflow_rules;
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
pub use a4n_db::generation::RecordStartedGenerationRunInput;
pub use a4n_db::video::{
    CreateGeneratedImageRecordDraftInput, CreateGeneratedVideoRecordDraftInput,
    UpdateGeneratedImageRecordInput, UpdateGeneratedVideoRecordInput,
};
pub use a4n_db::video_prompt::VideoPromptDownstreamDependencies;
#[cfg(test)]
pub(in crate::video) use a4n_error::AppError;

pub(crate) use a4n_video::ensure_provider_generation_run_updated;
pub use a4n_video_contract::{
    ChapterVideoGenerationHostPort, ReferenceMediaVideoGenerationHostPort, VideoGenerationHostPort,
};
pub(crate) use assembly_clip_rules::{
    AssemblyClipOrderAnchor, assembly_clip_display_shot_number,
    assembly_clip_insert_after_video_id, assembly_clip_sort_order,
    assembly_insert_metadata_with_position, is_assembly_clip_video_type,
    next_assembly_insert_sort_order,
};
pub(crate) use assembly_postprocess_rules::{
    parse_postprocess_edit_metadata, postprocess_edit_timing,
};
pub(in crate::video) use assembly_push_rules::ASSEMBLY_PUSH_STATUS_PENDING;
pub(in crate::video) use assembly_push_rules::{
    ASSEMBLY_PUSH_STATUS_ACCEPTED, ASSEMBLY_PUSH_STATUS_REJECTED, active_assembly_push_source_ids,
    assembly_push_metadata_with_status, assembly_push_records,
    ensure_active_pushes_match_clip_prefix, ensure_insert_position_not_inside_pushed_prefix,
    latest_non_rejected_push_index, parse_assembly_push_metadata, selected_assembly_push_end_index,
    validate_assembly_workflow_mode,
};
pub use boundary_frame_rules::latest_boundary_frame_image_id;
pub(in crate::video) use boundary_frame_rules::{
    boundary_frame_numbers_for_prompt, frame_prompt_matches_video_prompt,
};
pub use chapter_video_generation::GenerateChapterVideosBatchInput;
pub use chapter_video_request_rules::{
    BoundaryFrameImageIds, ChapterVideoProviderRequestContext, ProviderPromptImageBinding,
    append_provider_input_note, build_video_generation_provider_prompt_with_renderer,
    provider_specific_negative_prompt,
};
pub use contracts::ProjectChapterVideoSource;
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
pub(in crate::video) use generation_run_rules::started_video_generation_run_input;
pub(in crate::video) use job_status_rules::{
    active_provider_job_id, canvas_video_workflow_context, has_active_provider_job,
    is_canvas_video_record,
};
pub(crate) use job_status_rules::{
    can_adopt_authoritative_video_completion, ensure_positive_generated_video_id,
    ensure_positive_project_id_for_video, ensure_positive_video_prompt_id,
    normalized_provider_job_id, provider_poll_config_id, required_generated_video_failure_message,
    required_provider_job_id, validate_generated_video_interrupted_status,
    validate_generated_video_waiting_status,
};
pub use library::{
    image_type_rank, is_video_reference_image_candidate, resolve_video_reference_image_project_ids,
    video_reference_scope_rank,
};
pub(in crate::video) use prompt_application::{
    GenerateChapterVideoPromptInput, VideoPromptApplicationContext,
};
pub(crate) use prompt_application::{
    validate_video_final_prompt_generation_options, validate_video_frame_prompt_generation_options,
    validate_video_segment_plan_generation_options,
};
pub use prompt_chain_rules::{PromptChainFields, prompt_chain_matches, prompt_source_type_matches};
pub(crate) use prompt_generation::validate_video_inserted_segment_generation_options;
pub(in crate::video) use prompt_generation::{
    generate_inserted_segment_draft, load_video_segment_plan_draft_checkpoints,
    load_video_segment_plan_draft_recipe_state, update_video_segment_plan_draft_recipe_checkpoint,
};
pub(crate) use prompt_workflow_rules::{
    video_prompt_optimization_workflow_run_input, video_prompt_workflow_run_input,
};
pub(crate) use provider_execution::VideoGenerationCheckpointContext;
pub use provider_execution::{
    VIDEO_GENERATION_PROGRESS_TOTAL, VideoGenerationProgress, VideoProviderExecutionInput,
};
pub use provider_prompt_text_rules::{
    MIN_FRAME_PROMPT_CHARS, MIN_PROVIDER_MOTION_PROMPT_CHARS, ProviderPromptTextViolation,
    provider_prompt_text_violation, validate_provider_model_text,
};
pub(crate) use provider_request_rules::prompt_provider_workflow_for_recipe;
pub use provider_request_rules::{
    MAX_CLIPROXY_VIDEO_PROMPT_BYTES, MAX_CLIPROXY_VIDEO_REFERENCE_IMAGES,
    ProviderGenerationWorkflow, boundary_frame_missing_image_message,
    cliproxy_prompt_length_violation_message, ensure_newapi_first_frame_image_supported,
    ensure_provider_generation_workflow, first_frame_chain_missing_start_image_message,
    linked_video_preview_config_id, provider_image_binding_violation_messages,
    reference_ids_with_storyboard_image, required_prompt_source_prompt_type,
    required_video_execution_identity, storyboard_missing_image_message,
};
pub use provider_workflow_rules::{
    video_provider_progress_workflow_run_input, video_provider_workflow_business_key,
    video_provider_workflow_fingerprint,
};
pub(crate) use reference_media_rules::validate_reference_media_start_input;
pub(in crate::video) use reference_media_rules::{
    MIN_PROVIDER_VISUAL_PROMPT_CHARS, validate_reference_media_import_options,
};
pub use storyboard_frame_rules::{
    latest_storyboard_image_id, storyboard_prompt_matches_video_prompt,
};
pub(crate) use tail_frame_rules::{
    is_first_frame_chain_video, should_capture_video_tail_frame,
    tail_frame_reference_belongs_to_video, video_shot_number,
};

pub(in crate::video) use crate::OrchestratorContext;
#[cfg(test)]
pub(in crate::video) use tokio_util::sync::CancellationToken;

pub(in crate::video) use self::config::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, GEMINI_VEO_VIDEO_BACKEND, NEWAPI_VIDEO_BACKEND,
    OPENAI_CHAT_VIDEO_BACKEND, OPENAI_COMPATIBLE_VIDEO_BACKEND, SEEDANCE2_VIDEO_BACKEND,
    duration_capability_for_provider_config,
};
pub(crate) use self::config::{
    validate_video_config_model, validate_video_duration_for_provider_config_secs,
    video_provider_config_from_model,
};
pub use self::prompt_json::{
    validate_direct_prompt_text, video_motion_prompt_has_temporal_storyboard,
    video_prompt_missing_required_section_for_language, video_prompt_stale_section_for_language,
    video_prompt_upload_status_term,
};
#[cfg(test)]
pub use self::prompt_records::VideoPromptUpdateInput;

pub(crate) use errors::video_db_error;
pub(in crate::video) use errors::video_llm_step_error;
pub use records::{
    PreparedImportedChapterVideoRecord, VideoGenerationRecordInput, generated_video_summary,
};

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(in crate::video) use tests::{
    insert_derived_video_project, insert_video_chapter_without_shooting_script,
    insert_video_project,
};
