use a4n_core::{error::AppError, models::project::novel_project};

use crate::OrchestratorContext;

use super::super::{
    ProjectFamilyExportRows, ProjectFamilyExportSelection, ProjectFamilyImportPackage,
    ProjectFamilyMediaFileImporter, project_family_export_error_to_app_error,
    project_family_import_error_to_app_error,
};

impl OrchestratorContext {
    pub async fn project_family_export_selection(
        &self,
        root_project_id: i64,
        include_derived_projects: bool,
    ) -> Result<ProjectFamilyExportSelection, AppError> {
        a4n_db::project::project_family_export_selection(
            self.db.connection(),
            root_project_id,
            include_derived_projects,
        )
        .await
        .map_err(project_family_export_error_to_app_error)
    }

    pub async fn derived_project_export_selection(
        &self,
        derived_project_id: i64,
    ) -> Result<ProjectFamilyExportSelection, AppError> {
        a4n_db::project::derived_project_export_selection(self.db.connection(), derived_project_id)
            .await
            .map_err(project_family_export_error_to_app_error)
    }

    pub async fn project_family_export_rows(
        &self,
        root_project_id: i64,
        derived_project_ids: &[i64],
        derived_source_snapshot_ids: &[i64],
    ) -> Result<ProjectFamilyExportRows, AppError> {
        a4n_db::project::project_family_export_rows(
            self.db.connection(),
            root_project_id,
            derived_project_ids,
            derived_source_snapshot_ids,
        )
        .await
        .map_err(project_family_export_error_to_app_error)
    }

    pub async fn import_project_family_package_rows<Importer>(
        &self,
        user_id: i64,
        package: &ProjectFamilyImportPackage,
        media_importer: &mut Importer,
    ) -> Result<Vec<novel_project::Model>, AppError>
    where
        Importer: ProjectFamilyMediaFileImporter,
    {
        a4n_db::project_family_import::import_project_family_package_rows(
            self.db.connection(),
            user_id,
            package,
            media_importer,
        )
        .await
        .map_err(project_family_import_error_to_app_error)
    }
}
