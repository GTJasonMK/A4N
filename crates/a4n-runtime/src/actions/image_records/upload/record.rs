use super::file::create_imported_image_output;
use crate::{
    AppState, cleanup_outbox::enqueue_file_artifact_cleanup_by_project,
    file_utils::persist_created_media_file,
};
use a4n_core::models::ai::generated_image;
use a4n_orchestrator::media::PreparedImportedImageUpload;
use std::io::Write;

pub(super) async fn persist_imported_image(
    state: &AppState,
    user_id: i64,
    prepared: PreparedImportedImageUpload,
) -> Result<generated_image::Model, String> {
    let project_id = prepared.project_id;
    let chapter_id = prepared.chapter_id;
    let extension = prepared.decoded.extension;
    let (output_path, mut output_file) =
        create_imported_image_output(state.data_dir(), project_id, chapter_id, extension)?;
    let bytes = &prepared.decoded.bytes;
    let (relative_path, file_size) = persist_created_media_file(
        state.data_dir(),
        &output_path,
        |_| {
            output_file
                .write_all(bytes)
                .map_err(|err| format!("写入导入图片失败：{err}"))
        },
        "导入图片文件",
        "导入图片",
        "导入图片文件",
    )?;
    let record = match state
        .orchestrator()
        .create_imported_image_record_for_user_validated(
            user_id,
            prepared,
            relative_path.clone(),
            file_size,
        )
        .await
    {
        Ok(record) => record,
        Err(err) => {
            let cleanup_error = enqueue_file_artifact_cleanup_by_project(
                state,
                &format!("image-upload:{project_id}:{relative_path}"),
                std::slice::from_ref(&relative_path),
            )
            .await
            .err();
            let message = format!("导入图片记录创建失败：{err}");
            return Err(match cleanup_error {
                Some(cleanup) if !cleanup.trim().is_empty() => {
                    format!("{message}；后续清理也失败：{cleanup}")
                }
                _ => message,
            });
        }
    };
    Ok(record)
}
