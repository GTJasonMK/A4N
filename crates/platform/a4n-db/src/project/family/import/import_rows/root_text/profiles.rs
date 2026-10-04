use a4n_db_models::models::{
    asset::object_profile, character::protagonist_profile, scene::scene_profile,
};
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseTransaction, EntityTrait, QueryFilter, Set};

use crate::project::family::import::{
    DatabaseProjectFamilyImportPackage, ImportIdMaps, ProjectFamilyImportError, map_required,
};

pub(super) async fn insert_profile_rows(
    txn: &DatabaseTransaction,
    root_project_id: i64,
    package: &DatabaseProjectFamilyImportPackage,
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
    insert_scene_profiles(txn, package, maps).await
}

pub(in crate::project::family::import) async fn insert_scene_profiles(
    txn: &DatabaseTransaction,
    package: &DatabaseProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    let profiles = package
        .text_core
        .scene_profiles
        .iter()
        .filter(|profile| {
            maps.project_ids.contains_key(&profile.project_id)
                && !maps.scene_profile_ids.contains_key(&profile.id)
        })
        .collect::<Vec<_>>();
    if !profiles.is_empty() {
        let rows = profiles
            .iter()
            .map(|profile| {
                let mut active: scene_profile::ActiveModel = (**profile).clone().into();
                active.id = sea_orm::NotSet;
                active.project_id = Set(map_required(
                    &maps.project_ids,
                    profile.project_id,
                    "场景所属项目",
                )?);
                active.parent_scene_id = Set(None);
                Ok(active)
            })
            .collect::<Result<Vec<_>, ProjectFamilyImportError>>()?;
        let inserted = scene_profile::Entity::insert_many(rows)
            .exec_with_returning_many(txn)
            .await
            .map_err(|source| ProjectFamilyImportError::DatabaseContext {
                context: "导入场景档案失败",
                source,
            })?;
        if inserted.len() != profiles.len() {
            return Err(ProjectFamilyImportError::invalid_package(format!(
                "导入场景档案批量写入返回行数不一致: inserted={}, expected={}",
                inserted.len(),
                profiles.len()
            )));
        }
        for (profile, inserted) in profiles.iter().zip(inserted) {
            maps.scene_profile_ids.insert(profile.id, inserted.id);
        }
    }
    let mut parent_scene_id_case = CaseStatement::new();
    let mut profile_ids = Vec::new();
    for profile in profiles {
        let Some(parent_id) = profile.parent_scene_id else {
            continue;
        };
        let new_profile_id = map_required(&maps.scene_profile_ids, profile.id, "场景档案 ID")?;
        let new_parent_id = map_required(&maps.scene_profile_ids, parent_id, "场景父级 ID")?;
        parent_scene_id_case = parent_scene_id_case.case(
            scene_profile::Column::Id.eq(new_profile_id),
            Expr::value(Some(new_parent_id)),
        );
        profile_ids.push(new_profile_id);
    }
    if profile_ids.is_empty() {
        return Ok(());
    }
    let updated = scene_profile::Entity::update_many()
        .col_expr(
            scene_profile::Column::ParentSceneId,
            parent_scene_id_case
                .finally(Expr::col(scene_profile::Column::ParentSceneId))
                .into(),
        )
        .filter(scene_profile::Column::Id.is_in(profile_ids.clone()))
        .exec(txn)
        .await
        .map_err(|source| ProjectFamilyImportError::DatabaseContext {
            context: "更新场景父级失败",
            source,
        })?;
    if updated.rows_affected != profile_ids.len() as u64 {
        return Err(ProjectFamilyImportError::invalid_package(format!(
            "更新场景父级数量不一致: updated={}, expected={}",
            updated.rows_affected,
            profile_ids.len()
        )));
    }
    Ok(())
}
