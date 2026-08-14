use super::{
    ExportDerivedProjectDataInput, ExportProjectFamilyDataInput, ExportProjectPackageData,
    ImportProjectPackageUploadInput, PROJECT_FAMILY_PACKAGE_SUFFIX,
    host_port::RuntimeProjectPackageTransferHostPort,
    package_files::{export_package_data, project_package_import_input},
};
use crate::state::AppState;
use a4n_core::models::project::novel_project;
use a4n_orchestrator::project::{
    ExportDerivedProjectPackageApplicationInput, ExportProjectFamilyPackageApplicationInput,
};

pub async fn export_project_family_data(
    state: &AppState,
    user_id: i64,
    input: ExportProjectFamilyDataInput,
) -> Result<ExportProjectPackageData, String> {
    let host_port = RuntimeProjectPackageTransferHostPort::new(state);
    let exported = state
        .orchestrator()
        .export_project_family_package_validated(
            ExportProjectFamilyPackageApplicationInput {
                user_id,
                root_project_id: input.root_project_id,
                include_derived_projects: input.include_derived_projects,
            },
            &host_port,
        )
        .await
        .map_err(|error| error.to_string())?;
    export_package_data(
        &exported.project,
        exported.package_bytes,
        PROJECT_FAMILY_PACKAGE_SUFFIX,
    )
}

pub async fn export_derived_project_data(
    state: &AppState,
    user_id: i64,
    input: ExportDerivedProjectDataInput,
) -> Result<ExportProjectPackageData, String> {
    let host_port = RuntimeProjectPackageTransferHostPort::new(state);
    let exported = state
        .orchestrator()
        .export_derived_project_package_validated(
            ExportDerivedProjectPackageApplicationInput {
                user_id,
                derived_project_id: input.derived_project_id,
            },
            &host_port,
        )
        .await
        .map_err(|error| error.to_string())?;
    export_package_data(
        &exported.project,
        exported.package_bytes,
        PROJECT_FAMILY_PACKAGE_SUFFIX,
    )
}

pub async fn import_project_family_upload(
    state: &AppState,
    user_id: i64,
    input: ImportProjectPackageUploadInput,
) -> Result<Vec<novel_project::Model>, String> {
    let input =
        project_package_import_input(user_id, input, PROJECT_FAMILY_PACKAGE_SUFFIX, "项目族包")?;
    let mut host_port = RuntimeProjectPackageTransferHostPort::new(state);
    state
        .orchestrator()
        .import_project_family_package_validated(input, &mut host_port)
        .await
        .map_err(|error| error.to_string())
}
