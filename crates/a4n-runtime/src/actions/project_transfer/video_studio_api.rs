use super::{
    ExportProjectPackageData, ExportVideoStudioProjectDataInput, ImportProjectPackageUploadInput,
    VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX,
    host_port::RuntimeProjectPackageTransferHostPort,
    package_files::{export_package_data, project_package_import_input},
};
use crate::state::AppState;
use a4n_api_contract::project::ProjectSummary;
use a4n_api_contract::project_transfer::ExportVideoStudioProjectApplicationInput;

pub async fn export_video_studio_project_data(
    state: &AppState,
    user_id: i64,
    input: ExportVideoStudioProjectDataInput,
) -> Result<ExportProjectPackageData, String> {
    let host_port = RuntimeProjectPackageTransferHostPort::new(state);
    let exported = state
        .video_studio_transfer_application()
        .export_project(
            ExportVideoStudioProjectApplicationInput {
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
        VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX,
    )
}

pub async fn import_video_studio_project_upload(
    state: &AppState,
    user_id: i64,
    input: ImportProjectPackageUploadInput,
) -> Result<ProjectSummary, String> {
    let input = project_package_import_input(
        user_id,
        input,
        VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX,
        "视频画布项目包",
    )?;
    let mut host_port = RuntimeProjectPackageTransferHostPort::new(state);
    state
        .video_studio_transfer_application()
        .import_project(input, &mut host_port)
        .await
        .map_err(|error| error.to_string())
}

pub async fn import_video_studio_project_upload_summary(
    state: &AppState,
    user_id: i64,
    input: ImportProjectPackageUploadInput,
) -> Result<ProjectSummary, String> {
    import_video_studio_project_upload(state, user_id, input).await
}
