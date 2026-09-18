use a4n_db_models::models::project::novel_project;
use sea_orm::{DatabaseConnection, TransactionTrait};

use super::{
    ProjectFamilyImportError, ProjectFamilyImportMappings, ProjectFamilyImportPackage,
    ProjectFamilyMediaFileImporter, import_rows,
};

pub async fn import_project_family_package_rows<Importer>(
    db: &DatabaseConnection,
    user_id: i64,
    package: &ProjectFamilyImportPackage,
    media_importer: &mut Importer,
) -> Result<Vec<novel_project::Model>, ProjectFamilyImportError>
where
    Importer: ProjectFamilyMediaFileImporter,
{
    let (imported_projects, _) =
        import_project_family_package_rows_with_mappings(db, user_id, package, media_importer)
            .await?;
    Ok(imported_projects)
}

pub async fn import_project_family_package_rows_with_mappings<Importer>(
    db: &DatabaseConnection,
    user_id: i64,
    package: &ProjectFamilyImportPackage,
    media_importer: &mut Importer,
) -> Result<(Vec<novel_project::Model>, ProjectFamilyImportMappings), ProjectFamilyImportError>
where
    Importer: ProjectFamilyMediaFileImporter,
{
    let txn = db.begin().await?;
    let (imported_projects, maps) =
        import_rows::import_package_rows(user_id, &txn, package, media_importer).await?;
    txn.commit()
        .await
        .map_err(ProjectFamilyImportError::Commit)?;
    Ok((
        imported_projects,
        ProjectFamilyImportMappings {
            chapter_ids: maps.chapter_ids,
            source_snapshot_ids: maps.source_snapshot_ids,
            source_snapshot_chapter_ids: maps.source_snapshot_chapter_ids,
        },
    ))
}
