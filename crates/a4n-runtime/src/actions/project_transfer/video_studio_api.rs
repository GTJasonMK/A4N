use super::{
    ExportProjectPackageData, ExportVideoStudioProjectDataInput, ImportProjectPackageUploadInput,
    VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX,
    package_files::{
        decode_uploaded_project_file, export_package_data, validate_uploaded_project_file_name,
    },
    video_studio_package::RuntimeVideoStudioProjectTransferHostPort,
};
use crate::state::AppState;
use a4n_core::models::project::novel_project;
use a4n_orchestrator::project::{
    ExportVideoStudioProjectApplicationInput, ImportVideoStudioProjectApplicationInput,
};

pub async fn export_video_studio_project_data(
    state: &AppState,
    user_id: i64,
    input: ExportVideoStudioProjectDataInput,
) -> Result<ExportProjectPackageData, String> {
    let host_port = RuntimeVideoStudioProjectTransferHostPort::new(state);
    let exported = state
        .orchestrator()
        .export_video_studio_project_validated(
            ExportVideoStudioProjectApplicationInput {
                user_id,
                project_id: input.project_id,
            },
            &host_port,
        )
        .await
        .map_err(|error| error.to_string())?;
    Ok(export_package_data(
        &exported.project,
        exported.package_bytes,
        VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX,
    ))
}

pub async fn import_video_studio_project_upload(
    state: &AppState,
    user_id: i64,
    input: ImportProjectPackageUploadInput,
) -> Result<novel_project::Model, String> {
    validate_uploaded_project_file_name(
        &input.file_name,
        VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX,
        "视频画布项目包",
    )?;
    let bytes = decode_uploaded_project_file(&input.data_base64, "视频画布项目包")?;
    let mut host_port = RuntimeVideoStudioProjectTransferHostPort::new(state);
    state
        .orchestrator()
        .import_video_studio_project_validated(
            ImportVideoStudioProjectApplicationInput {
                user_id,
                package_bytes: bytes,
            },
            &mut host_port,
        )
        .await
        .map_err(|error| error.to_string())
}
