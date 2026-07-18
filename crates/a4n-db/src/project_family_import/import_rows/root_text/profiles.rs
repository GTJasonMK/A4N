use a4n_core::models::{
    asset::object_profile, character::protagonist_profile, scene::scene_profile,
};
use sea_orm::{ActiveModelTrait, DatabaseTransaction, EntityTrait, NotSet, Set};

use crate::project_family_import::{
    ImportIdMaps, ProjectFamilyImportError, ProjectFamilyImportPackage, map_required,
};

pub(super) async fn insert_profile_rows(
    txn: &DatabaseTransaction,
    root_project_id: i64,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    insert_project_scoped_mapped_import_rows!(
        txn,
        &package.text_core.character_profiles,
        profile,
        protagonist_profile::ActiveModel,
        maps.character_profile_ids,
        "导入人物档案失败",
        root_project_id
    );
    insert_project_scoped_mapped_import_rows!(
        txn,
        &package.text_core.object_profiles,
        profile,
        object_profile::ActiveModel,
        maps.object_profile_ids,
        "导入物体档案失败",
        root_project_id
    );
    insert_scene_profiles(txn, root_project_id, package, maps).await
}

async fn insert_scene_profiles(
    txn: &DatabaseTransaction,
    root_project_id: i64,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    insert_mapped_import_rows!(
        txn,
        &package.text_core.scene_profiles,
        profile,
        scene_profile::ActiveModel,
        maps.scene_profile_ids,
        "导入场景档案失败",
        |active| {
            active.project_id = Set(root_project_id);
            active.parent_scene_id = Set(None);
        }
    );
    for profile in &package.text_core.scene_profiles {
        let Some(parent_id) = profile.parent_scene_id else {
            continue;
        };
        let new_profile_id = map_required(&maps.scene_profile_ids, profile.id, "场景档案 ID")?;
        let new_parent_id = map_required(&maps.scene_profile_ids, parent_id, "场景父级 ID")?;
        let stored = scene_profile::Entity::find_by_id(new_profile_id)
            .one(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "读取导入场景档案失败",
                source,
            })?
            .ok_or_else(|| {
                ProjectFamilyImportError::InvalidPackage(format!(
                    "导入场景档案不存在: {new_profile_id}"
                ))
            })?;
        let mut active: scene_profile::ActiveModel = stored.into();
        active.parent_scene_id = Set(Some(new_parent_id));
        active
            .update(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "更新场景父级失败",
                source,
            })?;
    }
    Ok(())
}
