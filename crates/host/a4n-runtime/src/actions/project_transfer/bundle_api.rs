use super::{
    ExportProjectBundleDataInput, ExportProjectPackageData, ImportProjectPackageUploadInput,
    PROJECT_BUNDLE_PACKAGE_SUFFIX,
    host_port::RuntimeProjectPackageTransferHostPort,
    package_files::{export_package_data, project_package_import_input},
};
use crate::state::AppState;
use a4n_api_contract::project_transfer::{
    ExportProjectBundleApplicationInput, ProjectBundleImportReport,
};

pub async fn export_project_bundle_data(
    state: &AppState,
    user_id: i64,
    input: ExportProjectBundleDataInput,
) -> Result<ExportProjectPackageData, String> {
    let host_port = RuntimeProjectPackageTransferHostPort::new(state);
    let exported = state
        .project_bundle_transfer_application()
        .export_bundle(
            ExportProjectBundleApplicationInput {
                user_id,
                project_id: input.project_id,
                include_derived_projects: input.include_derived_projects,
            },
            &host_port,
        )
        .await
        .map_err(|error| error.to_string())?;
    export_package_data(
        &exported.project_title,
        exported.package_bytes,
        PROJECT_BUNDLE_PACKAGE_SUFFIX,
    )
}

pub async fn inspect_project_bundle_upload(
    state: &AppState,
    user_id: i64,
    input: ImportProjectPackageUploadInput,
) -> Result<ProjectBundleImportReport, String> {
    let input =
        project_package_import_input(user_id, input, PROJECT_BUNDLE_PACKAGE_SUFFIX, "统一项目包")?;
    let mut host_port = RuntimeProjectPackageTransferHostPort::new(state);
    state
        .project_bundle_transfer_application()
        .inspect_bundle(input, &mut host_port)
        .await
        .map_err(|error| error.to_string())
}

pub async fn import_project_bundle_upload(
    state: &AppState,
    user_id: i64,
    input: ImportProjectPackageUploadInput,
) -> Result<ProjectBundleImportReport, String> {
    let input =
        project_package_import_input(user_id, input, PROJECT_BUNDLE_PACKAGE_SUFFIX, "统一项目包")?;
    let mut host_port = RuntimeProjectPackageTransferHostPort::new(state);
    state
        .project_bundle_transfer_application()
        .import_bundle(input, &mut host_port)
        .await
        .map_err(|error| error.to_string())
}
