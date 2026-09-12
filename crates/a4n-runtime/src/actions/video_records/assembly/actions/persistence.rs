use crate::AppState;
use crate::actions::video_records::append_video_record_cleanup_error;
use a4n_storage::persist_created_media_file;
use std::path::{Path, PathBuf};

pub(super) async fn persist_assembly_record_output<B, R>(
    state: &AppState,
    record_id: i64,
    build_output: B,
    render: R,
    file_label: &str,
    relative_path_label: &str,
    metadata_label: &str,
) -> Result<(PathBuf, String, i64), String>
where
    B: FnOnce() -> Result<PathBuf, String>,
    R: FnOnce(&Path) -> Result<(), String>,
{
    let result = build_output().and_then(|output| {
        persist_created_media_file(
            state.data_dir(),
            &output,
            render,
            file_label,
            relative_path_label,
            metadata_label,
        )
        .map(|(relative_path, file_size)| (output, relative_path, file_size))
    });
    append_video_record_cleanup_error(
        state.video_record_lifecycle_application(),
        record_id,
        result,
    )
    .await
}
