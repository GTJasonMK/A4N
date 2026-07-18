//! Galgame 剧本、资产规划与生成流水线编排。

use crate::OrchestratorContext;

mod asset_plan;
mod context;
mod contracts;
mod errors;
mod generation;
mod media;
mod mutations;
mod persistence;
mod pipeline_json;
mod pipeline_persist;
mod pipeline_prompt;
mod pipeline_run;
mod pipeline_state;
mod read_models;
mod rules;
mod script_records;

pub(crate) use asset_plan::validate_presentation_json;
pub use asset_plan::{
    GalgameAssetPlan, GalgameAssetPlanItem, GalgameAssetPlanOutcome, GalgameAssetPlanSummary,
};
#[cfg(test)]
use asset_plan::{GalgamePortraitAssetDraft, event_cg_directive, portrait_variant};
use contracts::SCRIPT_STATUS_READY;
pub use contracts::*;
use errors::{
    galgame_asset_plan_spec_record_error, galgame_context_record_error,
    galgame_generation_record_error, galgame_line_record_error, galgame_line_reorder_record_error,
    galgame_media_record_error, galgame_project_record_error, galgame_scene_record_error,
    galgame_script_record_error,
};
#[cfg(test)]
use persistence::GalgamePersistValidationScope;
pub(in crate::galgame) use pipeline_json::validate_staged_presentation;
pub use pipeline_json::{
    GalgameBranchDesign, GalgameCharacterStaging, GalgameFacingV2,
    GalgameLinePresentationDocumentV2, GalgameLinePresentationV2, GalgamePipelineBackgroundPlan,
    GalgamePipelineBeatRefV3, GalgamePipelineBranchSceneLineDraft,
    GalgamePipelineBranchSceneLineStaged, GalgamePipelineChoiceOptionPlan,
    GalgamePipelineChoicePlan, GalgamePipelineEventCgPlan, GalgamePipelineLineDraft,
    GalgamePipelineLineStaged, GalgamePipelineScene, GalgamePipelineSceneLineDraft,
    GalgamePipelineSceneLineStaged, GalgamePipelineShootingBeat, GalgameScriptAdaptation,
    GalgameShootingScriptProjection, GalgameStageLayerV2, GalgameStagePositionV2,
    GalgameVisibleCastMemberV2, GalgameVisualPlan, branch_design_from_value,
    character_staging_from_value, parse_galgame_json_object, script_adaptation_from_value,
    shooting_script_projection_from_value, visual_plan_from_value,
};
pub use pipeline_persist::{
    GalgamePersistChoice, GalgamePersistLine, GalgamePersistScene, GalgamePipelineSourceSpan,
    GalgameScriptPersistPlan, build_galgame_script_persist_plan,
};
pub use pipeline_prompt::{
    GalgamePipelinePromptChapter, GalgamePipelinePromptSourceSpan,
    galgame_branch_design_user_message, galgame_character_staging_user_message,
    galgame_pipeline_source_spans_from_prompt_chapters, galgame_prompt_chapter_from_snapshot,
    galgame_script_adaptation_user_message, galgame_speaker_profiles_from_models,
    galgame_visual_planning_user_message,
};
pub(crate) use pipeline_run::validate_source_sync_scene_regeneration_stage_outputs;
pub use pipeline_run::{
    GalgamePipelineProgressUpdate, GalgamePipelineRunOptions, GalgameProgressReporter,
    GalgameSceneRegenerationInput, GalgameSceneRegenerationPlan,
    galgame_shooting_script_projection,
};
pub use pipeline_state::{
    GALGAME_PIPELINE_TOTAL_STEPS, GalgamePipelineStartStage, galgame_pipeline_stage_keys,
    parse_galgame_start_stage, should_restore_galgame_stage_checkpoint,
};
#[cfg(test)]
use read_models::galgame_media_binding_summaries;
#[cfg(test)]
use read_models::{
    galgame_pipeline_prompt_context_from_record, galgame_script_detail_from_read_model,
};
pub(crate) use rules::validate_and_project_presentation;
use rules::{
    ensure_json_object, normalize_optional_positive_id, normalize_optional_text, required_text,
    to_i32, validate_positive_id, validate_scene_entry_kind,
};
pub(crate) use rules::{normalize_line_type, normalize_position, validate_line_semantics};

#[cfg(test)]
mod tests;

pub use a4n_db::galgame::{
    CompletedGalgameVisualBindingRecords, GalgameAssetPlanSpecRecordError,
    GalgameAssetPlanSpecRecordInput, GalgameAssetPlanningReadModel, GalgameGenerationRecordError,
    GalgameGenerationSpecEntityRefRecordInput, GalgameGenerationSpecRefsRecordInput,
    GalgameGenerationSpecTextAnchorRefRecordInput, GalgameGenerationSpecUpsert,
    GalgameGenerationSpecUpsertInput, GalgameGenerationSpecUpsertOutcome,
    GalgameMediaBindingReadModel, GalgameMediaBindingResult, GalgameMediaRecordError,
    GalgamePersistValidationScopeReadModel, GalgamePipelinePromptChapterRecord,
    GalgamePipelinePromptContextRecord, GalgamePortraitSpecTargetInput,
    GalgamePromptArtifactUpsertInput, GalgameScriptDetailReadModel, GalgameTargetContext,
    InsertGalgameLineRecordInput, PersistGalgameChoiceRecordInput, PersistGalgameLineRecordInput,
    PersistGalgameSceneRecordInput, PersistGalgameScriptRecordInput,
    ReplaceGalgameChoiceRecordInput, UpdateGalgameLineRecordInput, UpdateGalgameSceneRecordInput,
    validate_galgame_media_asset_source_context, validate_galgame_portrait_position_samples,
    validate_galgame_portrait_spec_family_context, validate_galgame_portrait_spec_source_chapter,
};
