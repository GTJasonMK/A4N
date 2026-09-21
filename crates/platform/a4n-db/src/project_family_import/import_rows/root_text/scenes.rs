use a4n_db_models::models::scene::{scene_entity_state, scene_occurrence, scene_transition};
use sea_orm::{ActiveModelTrait, DatabaseTransaction, Set};

use crate::project_family_import::{
    ImportIdMaps, ProjectFamilyImportError, ProjectFamilyImportPackage, map_optional, map_required,
    remap_optional_entity_id,
};

pub(super) async fn insert_scene_rows(
    txn: &DatabaseTransaction,
    root_project_id: i64,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    insert_mapped_import_rows!(
        txn,
        &package.text_core.scene_occurrences,
        occurrence,
        scene_occurrence::ActiveModel,
        maps.scene_occurrence_ids,
        "导入场景出场失败",
        |active| {
            active.project_id = Set(root_project_id);
            active.scene_profile_id = Set(map_required(
                &maps.scene_profile_ids,
                occurrence.scene_profile_id,
                "场景出场 scene_profile_id",
            )?);
            active.chapter_id = Set(map_required(
                &maps.chapter_ids,
                occurrence.chapter_id,
                "场景出场 chapter_id",
            )?);
        }
    );
    insert_import_rows!(
        txn,
        &package.text_core.scene_transitions,
        transition,
        scene_transition::ActiveModel,
        "导入场景位移失败",
        |active| {
            active.project_id = Set(root_project_id);
            active.chapter_id = Set(map_required(
                &maps.chapter_ids,
                transition.chapter_id,
                "场景位移 chapter_id",
            )?);
            active.from_scene_profile_id = Set(map_optional(
                &maps.scene_profile_ids,
                transition.from_scene_profile_id,
                "场景位移 from_scene_profile_id",
            )?);
            active.to_scene_profile_id = Set(map_required(
                &maps.scene_profile_ids,
                transition.to_scene_profile_id,
                "场景位移 to_scene_profile_id",
            )?);
            active.occurrence_id = Set(map_optional(
                &maps.scene_occurrence_ids,
                transition.occurrence_id,
                "场景位移 occurrence_id",
            )?);
        }
    );
    insert_import_rows!(
        txn,
        &package.text_core.scene_entity_states,
        state,
        scene_entity_state::ActiveModel,
        "导入场景实体状态失败",
        |active| {
            active.project_id = Set(root_project_id);
            active.scene_profile_id = Set(map_required(
                &maps.scene_profile_ids,
                state.scene_profile_id,
                "场景实体状态 scene_profile_id",
            )?);
            active.occurrence_id = Set(map_required(
                &maps.scene_occurrence_ids,
                state.occurrence_id,
                "场景实体状态 occurrence_id",
            )?);
            active.entity_id = Set(remap_optional_entity_id(
                &state.entity_type,
                state.entity_id,
                maps,
                "场景实体状态 entity_id",
            )?);
        }
    );
    Ok(())
}
