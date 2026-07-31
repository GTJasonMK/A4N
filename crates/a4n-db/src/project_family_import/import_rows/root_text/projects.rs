use a4n_core::models::project::{novel_project, project_derivation};
use sea_orm::{ActiveModelTrait, DatabaseTransaction, NotSet, Set};

use crate::project_family_import::{
    ImportIdMaps, ProjectFamilyImportError, ProjectFamilyImportPackage, map_required,
};

pub(in crate::project_family_import::import_rows) async fn insert_root_project(
    txn: &DatabaseTransaction,
    user_id: i64,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<novel_project::Model, ProjectFamilyImportError> {
    let root = package
        .projects
        .iter()
        .find(|project| project.id == package.root_project_id)
        .ok_or_else(|| {
            ProjectFamilyImportError::InvalidPackage(format!(
                "项目族导入包缺少根文本项目: {}",
                package.root_project_id
            ))
        })?;
    let root_imported = insert_project_row(txn, user_id, root, None, None, None).await?;
    maps.project_ids.insert(root.id, root_imported.id);
    Ok(root_imported)
}

pub(in crate::project_family_import::import_rows) async fn insert_derived_projects(
    txn: &DatabaseTransaction,
    user_id: i64,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<Vec<novel_project::Model>, ProjectFamilyImportError> {
    let mut imported = Vec::new();
    for project in package
        .projects
        .iter()
        .filter(|project| project.id != package.root_project_id)
    {
        let new_root_id = map_required(
            &maps.project_ids,
            package.root_project_id,
            "派生项目 root_project_id",
        )?;
        let old_snapshot_id = project.source_snapshot_id.ok_or_else(|| {
            ProjectFamilyImportError::InvalidPackage(format!(
                "派生项目缺少 source_snapshot_id: {}",
                project.id
            ))
        })?;
        let new_snapshot_id = map_required(
            &maps.source_snapshot_ids,
            old_snapshot_id,
            "派生项目 source_snapshot_id",
        )?;
        let imported_project = insert_project_row(
            txn,
            user_id,
            project,
            Some(new_root_id),
            Some(new_root_id),
            Some(new_snapshot_id),
        )
        .await?;
        maps.project_ids.insert(project.id, imported_project.id);
        imported.push(imported_project);
    }
    insert_import_rows!(
        txn,
        &package.project_derivations,
        derivation,
        project_derivation::ActiveModel,
        "导入项目派生关系失败",
        |active| {
            active.root_project_id = Set(map_required(
                &maps.project_ids,
                derivation.root_project_id,
                "派生关系 root_project_id",
            )?);
            active.derived_project_id = Set(map_required(
                &maps.project_ids,
                derivation.derived_project_id,
                "派生关系 derived_project_id",
            )?);
            active.source_snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                derivation.source_snapshot_id,
                "派生关系 source_snapshot_id",
            )?);
        }
    );
    Ok(imported)
}

async fn insert_project_row(
    txn: &DatabaseTransaction,
    user_id: i64,
    project: &novel_project::Model,
    root_project_id: Option<i64>,
    parent_project_id: Option<i64>,
    source_snapshot_id: Option<i64>,
) -> Result<novel_project::Model, ProjectFamilyImportError> {
    Ok(insert_import_row!(
        txn,
        project,
        novel_project::ActiveModel,
        "导入项目失败",
        |active| {
            active.user_id = Set(user_id);
            active.root_project_id = Set(root_project_id);
            active.parent_project_id = Set(parent_project_id);
            active.source_snapshot_id = Set(source_snapshot_id);
            active.lifecycle_gate = Set("active".to_string());
        }
    ))
}
