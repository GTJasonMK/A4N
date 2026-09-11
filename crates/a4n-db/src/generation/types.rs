mod artifacts;
mod catalog;
mod checkpoint;
mod error;
mod prompts;
mod runs;

pub use self::artifacts::GenerationRunTraceTarget;
pub(crate) use self::artifacts::append_generation_artifact_target;
pub use self::catalog::{
    GENERATION_ENTITY_KINDS, GENERATION_ENTITY_ROLES, GENERATION_MEDIA_REF_ROLES,
    GENERATION_SPEC_STATUSES, GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE,
    GENERATION_SPEC_TARGET_ENTITY_KINDS, GENERATION_SPEC_TARGET_KIND_CANVAS_ASSEMBLY,
    GENERATION_SPEC_TARGET_KIND_CANVAS_IMAGE, GENERATION_SPEC_TARGET_KIND_CANVAS_PROMPT,
    GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO, GENERATION_SPEC_TARGET_KINDS,
    GENERATION_TEXT_ANCHOR_ROLES, SPEC_STATUS_DRAFT, is_valid_generation_entity_kind,
    is_valid_generation_entity_role, is_valid_generation_media_ref_role,
    is_valid_generation_spec_status, is_valid_generation_spec_target_entity_kind,
    is_valid_generation_spec_target_kind, is_valid_generation_text_anchor_role,
};
pub use self::checkpoint::{
    GenerationRunCheckpointContext, generation_checkpoint_fingerprint_sha256,
};
pub use self::error::GenerationDbError;
pub(super) use self::prompts::INTERNAL_PROMPT_TOKENS;
pub use self::prompts::{
    GENERATION_PROMPT_KIND_CANVAS_PROMPT, GENERATION_PROMPT_KIND_CANVAS_SHOT_SCRIPT,
    GENERATION_PROMPT_KIND_VIDEO_PROMPT, GENERATION_PROMPT_KINDS, PromptArtifactInsertInput,
};
pub(in crate::generation) use self::prompts::{
    GENERATION_PROMPT_KIND_IMAGE_PROMPT, VIDEO_PROMPT_REQUIRED_SECTIONS,
};
pub(super) use self::runs::CreateGenerationRunInput;
pub use self::runs::{
    AttachGenerationRunsToWorkflowInput, RecordCompletedGenerationRunInput,
    RecordCompletedGenerationRunWithCheckpointInput, RecordStartedGenerationRunInput,
};
