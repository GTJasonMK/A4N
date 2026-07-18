use super::{
    contracts::{
        ExportProjectPackageData, ExportVideoStudioProjectDataInput,
        ImportVideoStudioProjectUploadInput,
    },
    package_files::{
        decode_uploaded_project_file, export_package_data, project_record_by_id,
        validate_uploaded_project_file_name,
    },
    video_studio_package::{
        export_video_studio_project_package_bytes, import_video_studio_project_package_bytes,
    },
};
use crate::{actions::access, state::AppState};
use a4n_core::models::project::novel_project;

pub async fn export_video_studio_project_data(
    state: &AppState,
    user_id: i64,
    input: ExportVideoStudioProjectDataInput,
) -> Result<ExportProjectPackageData, String> {
    access::verify_project_owner_string(state, user_id, input.project_id).await?;
    let project = project_record_by_id(state, input.project_id).await?;
    let bytes = export_video_studio_project_package_bytes(state, input.project_id).await?;
    Ok(export_package_data(&project, bytes))
}

pub async fn import_video_studio_project_upload(
    state: &AppState,
    user_id: i64,
    input: ImportVideoStudioProjectUploadInput,
) -> Result<novel_project::Model, String> {
    validate_uploaded_project_file_name(&input.file_name, "zip", "视频画布项目包")?;
    let bytes = decode_uploaded_project_file(&input.data_base64, "视频画布项目包")?;
    import_video_studio_project_package_bytes(state, user_id, bytes).await
}
