use a4n_db_models::models::project::novel_project;
use sea_orm::{DatabaseConnection, TransactionTrait};

use super::{
    ProjectFamilyImportError, ProjectFamilyImportPackage, ProjectFamilyMediaFileImporter,
    import_rows,
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
    let txn = db.begin().await?;
    let imported_projects =
        import_rows::import_package_rows(user_id, &txn, package, media_importer).await?;
    txn.commit()
        .await
        .map_err(ProjectFamilyImportError::Commit)?;
    Ok(imported_projects)
}
