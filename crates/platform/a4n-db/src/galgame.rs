#[cfg(test)]
use a4n_db_models::models::media::media_asset;
use a4n_db_models::models::{
    ai::generated_image,
    character::{character_portrait, protagonist_profile},
    galgame::galgame_scene,
    generation::{generation_spec, prompt_artifact},
    project::{novel_project, source_snapshot_profile},
};
use a4n_domain::galgame::GalgamePromptArtifactUpsertInput;
use a4n_domain::project::ProjectKind;
use chrono::Utc;
#[cfg(test)]
use sea_orm::DbErr;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
#[cfg(test)]
use std::collections::BTreeSet;

const GALGAME_ENTITY_SCENE: &str = "galgame_scene";
const GALGAME_ENTITY_LINE: &str = "galgame_line";
const GALGAME_ENTITY_CHARACTER_PROFILE: &str = "character_profile";
const GALGAME_TARGET_PORTRAIT_EXPRESSION: &str = "portrait_expression";
const GALGAME_SCENE_ENTRY_MAIN: &str = "main";
const GALGAME_SCENE_ENTRY_BRANCH: &str = "branch";
const GALGAME_LINE_TYPE_CHOICE: &str = "choice";
const GALGAME_SPEC_STATUS_DRAFT: &str = "draft";
const GALGAME_SPEC_SOURCE_CONTEXT_SNAPSHOT_BACKED: &str = "snapshot_backed";
const MEDIA_KIND_IMAGE: &str = "image";
const MEDIA_BINDING_ROLE_BACKGROUND: &str = "background";
const MEDIA_BINDING_ROLE_CG: &str = "cg";
const MEDIA_STATUS_COMPLETED: &str = "completed";
const GALGAME_SPEC_REF_INSERT_CHUNK_SIZE: usize = 1000;

fn require_galgame_media_text<'a>(
    value: &'a str,
    field: &str,
) -> Result<&'a str, GalgameMediaRecordError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(GalgameMediaRecordError::InvalidArgument(format!(
            "{field} 不能为空"
        )));
    }
    Ok(value)
}

fn validate_galgame_generation_id(
    value: i64,
    field: &str,
) -> Result<(), GalgameGenerationRecordError> {
    if value <= 0 {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "{field} 必须是正整数"
        )));
    }
    Ok(())
}

fn require_galgame_generation_text<'a>(
    value: &'a str,
    field: &str,
) -> Result<&'a str, GalgameGenerationRecordError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "{field} 不能为空"
        )));
    }
    Ok(value)
}

mod types;
pub use types::{
    CompletedGalgameVisualBindingRecords, GalgameAssetPlanSpecRecordError,
    GalgameAssetPlanningReadModel, GalgameAssetPlanningSceneRecord,
    GalgameAssetPlanningSourceChapterRecord, GalgameCompletedVisualAssetRecord,
    GalgameCompletedVisualBindingRecord, GalgameContextRecordError, GalgameGenerationRecordError,
    GalgameGenerationSpecUpsert, GalgameGenerationSpecUpsertOutcome, GalgameLineRecordError,
    GalgameLineReorderRecordError, GalgameMediaAssetReadRecord, GalgameMediaBindingReadModel,
    GalgameMediaBindingReadRecord, GalgameMediaBindingResult, GalgameMediaRecordError,
    GalgamePersistValidationScopeReadModel, GalgamePersistValidationSourceSnapshotChapterRecord,
    GalgamePersistValidationSpeakerProfileRecord, GalgamePersistValidationTextAnchorRecord,
    GalgamePipelinePromptChapterRecord, GalgamePipelinePromptChapterSourceRecord,
    GalgamePipelinePromptContextRecord, GalgamePipelinePromptSpeakerProfileRecord,
    GalgamePipelinePromptTextAnchorRecord, GalgamePortraitSpecTargetInput,
    GalgamePortraitSpecTargetProfile, GalgameProjectRecordError, GalgameSceneRecordError,
    GalgameScriptDetailChoiceRecord, GalgameScriptDetailReadModel, GalgameScriptDetailSceneRecord,
    GalgameScriptDetailScriptRecord, GalgameScriptRecordError, GalgameTargetContext,
    InsertGalgameLineRecordInput, PersistGalgameChoiceRecordInput, PersistGalgameLineRecordInput,
    PersistGalgameSceneRecordInput, PersistGalgameScriptRecordInput,
    ReplaceGalgameChoiceRecordInput, UpdateGalgameLineRecordInput, UpdateGalgameSceneRecordInput,
};
pub(in crate::galgame) use types::{
    GalgameGenerationSpecEntityRefRecordInput, GalgameGenerationSpecRefsRecordInput,
    GalgameGenerationSpecTextAnchorRefRecordInput,
};

mod read_models;
pub use read_models::{
    galgame_asset_planning_read_model, galgame_line_in_project_record,
    galgame_persist_validation_scope_read_model, galgame_pipeline_prompt_context_record,
    galgame_scene_in_project_record, galgame_script_detail_read_model,
    galgame_speaker_profile_records, validate_galgame_speaker_profile_in_root_project,
    validate_galgame_text_anchor_in_source_snapshot,
};

mod generation_specs;
pub(crate) use generation_specs::galgame_portrait_spec_references_lines;
pub use generation_specs::{
    completed_galgame_portrait_record, galgame_asset_generation_specs,
    galgame_generation_spec_content_changed, galgame_portrait_spec_target_profile,
    galgame_prompt_artifact_by_generation_spec, galgame_webgal_portraits,
    mark_galgame_generation_spec_status_record, primary_galgame_portrait_reference_image_ids,
    upsert_galgame_asset_plan_spec_records_transaction, upsert_galgame_prompt_artifact_record,
};
#[cfg(test)]
pub(in crate::galgame) use generation_specs::{
    validate_galgame_asset_generation_spec_query, validate_galgame_portrait_spec_target_input,
};

mod media;
pub use media::{
    bind_galgame_media_records, completed_galgame_visual_binding_records,
    delete_media_asset_with_provenance_records, galgame_generated_image_by_id,
    galgame_media_target_context, unbind_galgame_media_records,
    validate_galgame_portrait_position_samples,
};
pub(in crate::galgame) use media::{
    delete_galgame_line_generation_media_graph_record,
    delete_galgame_line_visual_generation_media_graph_record,
    delete_galgame_scene_visual_generation_media_graph_record,
    delete_galgame_target_generation_media_graph_record, galgame_media_binding_read_model,
    validate_galgame_media_asset_id, validate_galgame_media_target_ids,
    validate_galgame_portrait_spec_family_context, validate_galgame_portrait_spec_source_chapter,
    validate_galgame_portrait_spec_source_lines,
};
#[cfg(test)]
pub(in crate::galgame) use media::{
    validate_completed_visual_asset, validate_galgame_media_asset_source_context,
};

mod script_records;
pub(crate) use script_records::{
    GalgameScriptCleanupResult, delete_galgame_script_records_for_project, line_record,
    scene_record,
};
pub use script_records::{
    delete_galgame_line_with_generation_media_graph_record, insert_galgame_line_record_transaction,
    reorder_galgame_line_records_transaction, replace_galgame_choice_records_transaction,
    replace_galgame_script_for_project_record, update_galgame_line_with_generation_cleanup_record,
    update_galgame_scene_with_generation_cleanup_record,
};

#[cfg(test)]
mod tests;
