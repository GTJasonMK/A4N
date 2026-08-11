use super::super::prelude::{
    Path, PathBuf, append_secondary_error, data_dir_relative_file_path,
    remove_created_file_on_error_sync,
};
use super::delete_video_record_error;

pub(in crate::actions::video_records::imports) async fn persist_imported_video_file<F>(
    video: &a4n_orchestrator::OrchestratorContext,
    data_dir: &Path,
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
            data_dir,
            source,
            &output,
            copy_failure_prefix,
            relative_path_label,
            file_label,
        )
        .map(|relative_path| (output, relative_path))
    });
    match result {
        Ok(result) => Ok(result),
        Err(error) => Err(append_secondary_error(
            error,
            delete_video_record_error(video, record_id).await,
        )),
    }
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
