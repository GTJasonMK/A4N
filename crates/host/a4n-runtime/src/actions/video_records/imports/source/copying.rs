use super::super::contracts::ImportedVideoSource;
use crate::AppState;
use crate::actions::video_records::append_video_record_cleanup_error;
use a4n_storage::{
    create_unique_timestamped_file, data_dir_relative_file_path, remove_created_file_on_error_sync,
};
use std::path::{Path, PathBuf};

pub(in crate::actions::video_records) fn stage_imported_video_source(
    temp_dir: &Path,
    mut source: ImportedVideoSource,
) -> Result<ImportedVideoSource, String> {
    let upload_dir = temp_dir.join("video_uploads");
    std::fs::create_dir_all(&upload_dir)
        .map_err(|error| format!("创建导入视频临时目录失败：{error}"))?;
    let (temp_path, temp_file) =
        create_unique_timestamped_file(&upload_dir, "video_upload", source.extension)
            .map_err(|error| format!("创建导入视频临时文件失败：{error}"))?;
    drop(temp_file);
    let file_label = "导入视频临时文件";
    remove_created_file_on_error_sync(&temp_path, file_label, || {
        std::fs::copy(&source.path, &temp_path)
            .map_err(|error| format!("复制导入视频到临时目录失败：{error}"))?;
        source.path = temp_path.clone();
        source.temporary_path = Some(temp_path.clone());
        Ok(source)
    })
}

pub(in crate::actions::video_records::imports) async fn persist_imported_video_file<F>(
    state: &AppState,
    record_id: i64,
    source: &Path,
    build_output: F,
    copy_failure_prefix: &str,
    relative_path_label: &str,
    file_label: &str,
) -> Result<(PathBuf, String), String>
where
    F: FnOnce() -> Result<PathBuf, String>,
{
    let result = build_output().and_then(|output| {
        copy_imported_video_file(
            state.data_dir(),
            source,
            &output,
            copy_failure_prefix,
            relative_path_label,
            file_label,
        )
        .map(|relative_path| (output, relative_path))
    });
    append_video_record_cleanup_error(
        state.video_record_lifecycle_application(),
        record_id,
        result,
    )
    .await
}

pub(in crate::actions::video_records) fn copy_imported_video_file(
    data_dir: &Path,
    source: &Path,
    output: &Path,
    copy_failure_prefix: &str,
    relative_path_label: &str,
    file_label: &str,
) -> Result<String, String> {
    remove_created_file_on_error_sync(output, file_label, || {
        std::fs::copy(source, output).map_err(|error| format!("{copy_failure_prefix}：{error}"))?;
        data_dir_relative_file_path(data_dir, output, relative_path_label)
    })
}
