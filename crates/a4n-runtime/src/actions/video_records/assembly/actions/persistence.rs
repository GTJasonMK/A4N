use crate::AppState;
use crate::actions::video_records::append_video_record_cleanup_error;
use a4n_storage::{data_file_record_facts, remove_created_file_on_error};
use std::future::Future;
use std::path::PathBuf;

pub(super) async fn persist_assembly_record_output<B, R, F>(
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
    R: FnOnce(PathBuf) -> F,
    F: Future<Output = Result<(), String>> + Send,
{
    let result = match build_output() {
        Err(error) => Err(error),
        Ok(output) => {
            let render_output = output.clone();
            let result = remove_created_file_on_error(&output, file_label, async {
                render(render_output).await?;
                data_file_record_facts(
                    state.data_dir(),
                    &output,
                    relative_path_label,
                    metadata_label,
                )
            })
            .await;
            result.map(|(relative_path, file_size)| (output, relative_path, file_size))
        }
    };
    append_video_record_cleanup_error(
        state.video_record_lifecycle_application(),
        record_id,
        result,
    )
    .await
}
