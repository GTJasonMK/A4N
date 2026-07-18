mod writer;

use a4n_core::models::project::novel_project;
use chrono::Utc;

use crate::{actions::app_error_message, state::AppState};

use super::package_io::read_video_studio_package_bytes;
use super::validation::validate_package_manifest;
use writer::RuntimeVideoStudioImportMediaFileWriter;

pub(in super::super) async fn import_video_studio_project_package_bytes(
    state: &AppState,
    user_id: i64,
    bytes: Vec<u8>,
) -> Result<novel_project::Model, String> {
    if bytes.is_empty() {
        return Err("视频画布项目包不能为空".to_string());
    }

    let (package, media_payload) = read_video_studio_package_bytes(state, bytes).await?;
    validate_package_manifest(&package, &media_payload)?;

    let mut written_files = Vec::new();
    let rows = package.into_import_rows();
    let mut media_file_writer =
        RuntimeVideoStudioImportMediaFileWriter::new(state, &media_payload, &mut written_files);
    let import_result = state
        .orchestrator
        .import_video_studio_project_record(
            user_id,
            rows,
            Utc::now().naive_utc(),
            &mut media_file_writer,
        )
        .await;
    let project = match import_result {
        Ok(project) => project,
        Err(err) => {
            return Err(crate::file_utils::error_with_rollback_file_cleanup(
                app_error_message(err),
                &written_files,
                "已写入的视频工作室项目包媒体文件",
            ));
        }
    };

    Ok(project)
}
