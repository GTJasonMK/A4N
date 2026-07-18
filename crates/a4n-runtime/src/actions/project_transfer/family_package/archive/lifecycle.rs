use a4n_core::models::project::novel_project;

use crate::state::AppState;

use super::super::import_rows::{
    RuntimeProjectFamilyMediaFileImporter, cleanup_imported_media_files,
    project_family_import_package,
};
use super::super::validation::{validate_media_manifest, validate_package_manifest};
use super::super::{ImportMediaPayload, ProjectFamilyPackage};

pub(super) async fn import_project_family_package_parts(
    state: &AppState,
    user_id: i64,
    package: ProjectFamilyPackage,
    media_payload: ImportMediaPayload,
) -> Result<Vec<novel_project::Model>, String> {
    validate_package_manifest(&package)?;
    validate_media_manifest(&package, &media_payload)?;

    let mut written_files = Vec::new();
    let import_package = project_family_import_package(&package);
    let mut media_importer = RuntimeProjectFamilyMediaFileImporter::new(
        &state.config.data_dir,
        &media_payload,
        &mut written_files,
    );
    let import_result = state
        .orchestrator
        .import_project_family_package_rows(user_id, &import_package, &mut media_importer)
        .await;
    let imported_projects = match import_result {
        Ok(projects) => projects,
        Err(err) => {
            return Err(cleanup_imported_media_files(
                crate::actions::app_error_message(err),
                &written_files,
            ));
        }
    };
    Ok(imported_projects)
}
