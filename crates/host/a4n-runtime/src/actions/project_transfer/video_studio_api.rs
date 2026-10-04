use super::{
    CANVAS_PROJECT_PACKAGE_SUFFIX, ExportCanvasProjectDataInput, ExportProjectPackageData,
    ImportProjectPackageUploadInput,
    host_port::RuntimeProjectPackageTransferHostPort,
    package_files::{export_package_data, project_package_import_input},
};
use crate::state::AppState;
use a4n_api_contract::project::ProjectSummary;
use a4n_api_contract::project_transfer::ExportCanvasProjectApplicationInput;

pub async fn export_canvas_project_data(
    state: &AppState,
    user_id: i64,
    input: ExportCanvasProjectDataInput,
) -> Result<ExportProjectPackageData, String> {
    let host_port = RuntimeProjectPackageTransferHostPort::new(state);
    let exported = state
        .canvas_project_transfer_application()
        .export_project(
            ExportCanvasProjectApplicationInput {
                user_id,
                project_id: input.project_id,
            },
            &host_port,
        )
        .await
        .map_err(|error| error.to_string())?;
    export_package_data(
        &exported.project_title,
        exported.package_bytes,
        CANVAS_PROJECT_PACKAGE_SUFFIX,
    )
}

pub async fn import_canvas_project_upload(
    state: &AppState,
    user_id: i64,
    input: ImportProjectPackageUploadInput,
) -> Result<ProjectSummary, String> {
    let input = project_package_import_input(
        user_id,
        input,
        CANVAS_PROJECT_PACKAGE_SUFFIX,
        "Canvas 项目包",
    )?;
    let mut host_port = RuntimeProjectPackageTransferHostPort::new(state);
    state
        .canvas_project_transfer_application()
        .import_project(input, &mut host_port)
        .await
        .map_err(|error| error.to_string())
}

pub async fn import_canvas_project_upload_summary(
    state: &AppState,
    user_id: i64,
    input: ImportProjectPackageUploadInput,
) -> Result<ProjectSummary, String> {
    import_canvas_project_upload(state, user_id, input).await
}
