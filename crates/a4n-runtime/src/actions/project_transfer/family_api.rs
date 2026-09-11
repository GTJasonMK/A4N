use super::{
    ExportDerivedProjectDataInput, ExportProjectFamilyDataInput, ExportProjectPackageData,
    ImportProjectPackageUploadInput, PROJECT_FAMILY_PACKAGE_SUFFIX,
    host_port::RuntimeProjectPackageTransferHostPort,
    package_files::{export_package_data, project_package_import_input},
};
use crate::state::AppState;
use a4n_api_contract::project::ProjectSummary;
use a4n_api_contract::project_transfer::{
    ExportDerivedProjectPackageApplicationInput, ExportProjectFamilyPackageApplicationInput,
};

pub async fn export_project_family_data(
    state: &AppState,
    user_id: i64,
    input: ExportProjectFamilyDataInput,
) -> Result<ExportProjectPackageData, String> {
    let host_port = RuntimeProjectPackageTransferHostPort::new(state);
    let exported = state
        .project_family_transfer_application()
        .export_project_family(
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
        &exported.project_title,
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
        .project_family_transfer_application()
        .export_derived_project(
            ExportDerivedProjectPackageApplicationInput {
                user_id,
                derived_project_id: input.derived_project_id,
            },
            &host_port,
        )
        .await
        .map_err(|error| error.to_string())?;
    export_package_data(
        &exported.project_title,
        exported.package_bytes,
        PROJECT_FAMILY_PACKAGE_SUFFIX,
    )
}

pub async fn import_project_family_upload(
    state: &AppState,
    user_id: i64,
    input: ImportProjectPackageUploadInput,
) -> Result<Vec<ProjectSummary>, String> {
    let input =
        project_package_import_input(user_id, input, PROJECT_FAMILY_PACKAGE_SUFFIX, "项目族包")?;
    let mut host_port = RuntimeProjectPackageTransferHostPort::new(state);
    state
        .project_family_transfer_application()
        .import_project_family(input, &mut host_port)
        .await
        .map_err(|error| error.to_string())
}

pub async fn import_project_family_upload_summaries(
    state: &AppState,
    user_id: i64,
    input: ImportProjectPackageUploadInput,
) -> Result<Vec<ProjectSummary>, String> {
    import_project_family_upload(state, user_id, input).await
}
