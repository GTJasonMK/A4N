//! Galgame 剧本、资产规划与生成流水线编排。

mod pipeline;

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

mod preview_application;
mod read_models;
mod rules;
mod script_records;
mod webgal_export_application;
mod webgal_export_rules;

#[cfg(test)]
use asset_plan::{GalgamePortraitAssetDraft, portrait_variant};
use contracts::SCRIPT_STATUS_READY;

use pipeline::workflow::{GALGAME_PIPELINE_WORKFLOW_KIND, galgame_pipeline_workflow_run_input};

pub(crate) use contracts::{
    BindGalgameMediaInput, GALGAME_ENTITY_CHARACTER_PROFILE, GALGAME_ENTITY_LINE,
    GALGAME_ENTITY_SCENE, GALGAME_EXPRESSIONS, GALGAME_PIPELINE_TOTAL_STEPS,
    GALGAME_TARGET_BACKGROUND, GALGAME_TARGET_CG, GALGAME_TARGET_PORTRAIT_EXPRESSION,
    GalgameAssetGenerationItem, GalgameAssetGenerationSummary, GalgameAssetPlan, GalgameAudioRole,
    GalgameCgImportance, GalgameChapterAdaptation, GalgameChoice, GalgameGenerationTargetKind,
    GalgameLine, GalgameLinePresentationDocumentV3, GalgameLinePresentationSchemaVersion,
    GalgameLinePresentationV2, GalgameLineType, GalgameMediaAsset, GalgameMediaBinding,
    GalgameMediaBindingSummary, GalgameMediaRole, GalgameMediaTargetKind, GalgamePipelineBeatRefV4,
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
pub(in crate::galgame) use pipeline::json::validate_staged_presentation;
pub(crate) use pipeline::json::{
    GalgameAdaptationProjection, GalgameBranchDesign, GalgameCharacterStaging,
    GalgamePipelineBackgroundPlan, GalgamePipelineBranchSceneLineStaged, GalgamePipelineChoicePlan,
    GalgamePipelineEventCgPlan, GalgamePipelineLineDraft, GalgamePipelineLineStaged,
    GalgamePipelineScene, GalgameScriptAdaptation, GalgameVisualPlan,
    adaptation_projection_from_value, branch_design_from_value, character_staging_from_value,
    script_adaptation_from_value, visual_plan_from_value,
};
#[cfg(test)]
use pipeline::json::{GalgamePipelineSceneLineDraft, GalgamePipelineSceneLineStaged};
#[cfg(test)]
pub(crate) use pipeline::persist::GalgameAdaptationSceneRef;
pub(crate) use pipeline::persist::{
    GalgamePersistChoice, GalgamePersistLine, GalgamePersistScene, GalgamePipelineSourceSpan,
    GalgameScriptPersistPlan, build_galgame_script_persist_plan, validate_galgame_branch_design,
};
pub(crate) use pipeline::prompt::{
    GalgamePipelinePromptChapter, GalgamePipelinePromptSourceSpan,
    galgame_branch_design_user_message, galgame_character_staging_user_message,
    galgame_pipeline_source_spans_from_prompt_chapters, galgame_prompt_chapter_from_snapshot,
    galgame_script_adaptation_user_message, galgame_speaker_profiles_from_prompt_records,
    galgame_visual_planning_user_message,
};
pub(in crate::galgame) use pipeline::prompt::{
    galgame_branch_design_session_user_message, galgame_script_adaptation_session_user_message,
    galgame_visual_planning_session_user_message,
};
pub(crate) use pipeline::run::validate_source_sync_scene_regeneration_stage_outputs;
pub(crate) use pipeline::run::{
    GalgamePipelineRunOptions, GalgameSceneRegenerationInput, GalgameSceneRegenerationPlan,
    galgame_adaptation_projection,
};
pub(crate) use pipeline::state::should_restore_galgame_stage_checkpoint;
pub use preview_application::GalgamePipelinePreviewRequest;
#[cfg(test)]
use read_models::galgame_media_binding_summaries;
pub(crate) use read_models::{galgame_line_from_model, galgame_line_from_record};
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

use a4n_db::galgame::{GalgamePortraitSpecTargetInput, GalgameTargetContext};
use a4n_domain::galgame::GalgamePromptArtifactUpsertInput;

#[cfg(test)]
use a4n_domain::galgame::{GalgameFacingV2, GalgameStageLayerV2};

pub(crate) use a4n_galgame_host_contract::GalgameAssetGenerationHostPort;
