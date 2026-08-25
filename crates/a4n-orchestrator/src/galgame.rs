//! Galgame 剧本、资产规划与生成流水线编排。

use crate::OrchestratorContext;

mod application;
mod asset_generation_application;
mod asset_plan;
mod context;
mod contracts;
mod errors;
mod media_application;
mod mutations;
mod persistence;
mod pipeline_json;
mod pipeline_persist;
mod pipeline_prompt;
mod pipeline_run;
mod pipeline_state;
mod pipeline_workflow_rules;
mod preview_application;
mod read_models;
mod rules;
mod script_records;
mod webgal_export_application;
mod webgal_export_rules;

#[cfg(test)]
use asset_plan::{GalgamePortraitAssetDraft, portrait_variant};
use contracts::SCRIPT_STATUS_READY;

use pipeline_workflow_rules::{
    GALGAME_PIPELINE_WORKFLOW_KIND, galgame_pipeline_workflow_run_input,
};

pub(crate) use contracts::{
    BindGalgameMediaInput, GALGAME_ENTITY_CHARACTER_PROFILE, GALGAME_ENTITY_LINE,
    GALGAME_ENTITY_SCENE, GALGAME_EXPRESSIONS, GALGAME_PIPELINE_TOTAL_STEPS,
    GALGAME_TARGET_BACKGROUND, GALGAME_TARGET_CG, GALGAME_TARGET_PORTRAIT_EXPRESSION,
    GalgameAssetGenerationItem, GalgameAssetGenerationSummary, GalgameAssetPlan, GalgameAudioRole,
    GalgameCgImportance, GalgameChapterShootingScript, GalgameChoice, GalgameGenerationTargetKind,
    GalgameLine, GalgameLinePresentationDocumentV2, GalgameLinePresentationSchemaVersion,
    GalgameLinePresentationV2, GalgameLineType, GalgameMediaAsset, GalgameMediaBinding,
    GalgameMediaBindingSummary, GalgameMediaRole, GalgameMediaTargetKind, GalgamePipelineBeatRefV3,
    GalgamePipelinePromptContext, GalgamePipelineStartStage, GalgameScene, GalgameSceneDetail,
    GalgameSceneEntryKind, GalgameScriptDetail, GalgameSpeakerProfile, GalgameStagePositionV2,
    GalgameTargetEntityKind, GalgameTextAnchorSummary, GalgameVisibleCastMemberV2,
    ImportGalgameAudioInput, InsertGalgameLineInput, LINE_TYPE_CHOICE, LINE_TYPE_DIALOGUE,
    LINE_TYPE_MONOLOGUE, LINE_TYPE_NARRATION, LINE_TYPE_STAGE_DIRECTION, ReorderGalgameLinesInput,
    RunGalgameAssetGenerationInput, SCENE_ENTRY_BRANCH, SCENE_ENTRY_MAIN, UnbindGalgameMediaInput,
    UpdateGalgameChoicesInput, UpdateGalgameLineInput, UpdateGalgameSceneInput,
};
pub(crate) use errors::{
    galgame_asset_plan_spec_record_error, galgame_context_record_error,
    galgame_generation_record_error, galgame_line_record_error, galgame_line_reorder_record_error,
    galgame_media_record_error, galgame_project_record_error, galgame_scene_record_error,
    galgame_script_record_error,
};
#[cfg(test)]
use persistence::GalgamePersistValidationScope;
pub(in crate::galgame) use pipeline_json::validate_staged_presentation;
pub(crate) use pipeline_json::{
    GalgameBranchDesign, GalgameCharacterStaging, GalgamePipelineBackgroundPlan,
    GalgamePipelineBranchSceneLineStaged, GalgamePipelineChoicePlan, GalgamePipelineEventCgPlan,
    GalgamePipelineLineStaged, GalgamePipelineScene, GalgamePipelineShootingBeat,
    GalgameScriptAdaptation, GalgameShootingScriptProjection, GalgameVisualPlan,
    branch_design_from_value, character_staging_from_value, script_adaptation_from_value,
    shooting_script_projection_from_value, visual_plan_from_value,
};
#[cfg(test)]
use pipeline_json::{
    GalgamePipelineLineDraft, GalgamePipelineSceneLineDraft, GalgamePipelineSceneLineStaged,
};
pub(crate) use pipeline_persist::{
    GalgamePersistChoice, GalgamePersistLine, GalgamePersistScene, GalgamePipelineSourceSpan,
    GalgameScriptPersistPlan, build_galgame_script_persist_plan,
};
pub(crate) use pipeline_prompt::{
    GalgamePipelinePromptChapter, GalgamePipelinePromptSourceSpan,
    galgame_branch_design_user_message, galgame_character_staging_user_message,
    galgame_pipeline_source_spans_from_prompt_chapters, galgame_prompt_chapter_from_snapshot,
    galgame_script_adaptation_user_message, galgame_speaker_profiles_from_prompt_records,
    galgame_visual_planning_user_message,
};
pub(in crate::galgame) use pipeline_prompt::{
    galgame_branch_design_session_user_message, galgame_script_adaptation_session_user_message,
    galgame_visual_planning_session_user_message,
};
pub(crate) use pipeline_run::validate_source_sync_scene_regeneration_stage_outputs;
pub use pipeline_run::{GalgamePipelineProgressUpdate, GalgameProgressReporter};
pub(crate) use pipeline_run::{
    GalgamePipelineRunOptions, GalgameSceneRegenerationInput, GalgameSceneRegenerationPlan,
    galgame_shooting_script_projection,
};
pub(crate) use pipeline_state::should_restore_galgame_stage_checkpoint;
pub use preview_application::GalgamePipelinePreviewRequest;
pub(crate) use read_models::galgame_line_from_model;
#[cfg(test)]
use read_models::galgame_media_binding_summaries;
#[cfg(test)]
use read_models::{
    galgame_pipeline_prompt_context_from_record, galgame_script_detail_from_read_model,
};
pub(crate) use rules::{normalize_line_type, normalize_position, validate_line_semantics};
use rules::{
    normalize_optional_positive_id, normalize_optional_text, required_text, to_i32,
    validate_positive_id, validate_scene_entry_kind,
};
pub(crate) use rules::{
    parse_and_validate_presentation_projection, validate_and_project_presentation,
};

#[cfg(test)]
mod tests;

use a4n_db::galgame::{
    GalgamePortraitSpecTargetInput, GalgamePromptArtifactUpsertInput, GalgameTargetContext,
};

#[cfg(test)]
use a4n_domain::galgame::{GalgameFacingV2, GalgameStageLayerV2};

pub use application::GalgamePipelineApplicationInput;
pub use asset_generation_application::GalgameAssetGenerationHostPort;
