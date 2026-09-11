use std::time::{SystemTime, UNIX_EPOCH};

mod execution;
mod preview;

use a4n_error::AppError;

pub use a4n_api_contract::manga::{MangaPipelineResumeInput, MangaPipelineStartInput};
pub use execution::{preflight_manga_pipeline, run_manga_pipeline};
pub use preview::preview_manga_pipeline_requests;

pub fn new_manga_pipeline_task_id(
    project_id: i64,
    chapter_number: i32,
) -> Result<String, AppError> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| AppError::InvalidState {
            current: format!("系统时间早于 UNIX epoch: {error}"),
            operation: "生成漫画流水线任务 ID".to_string(),
        })?
        .as_nanos();
    Ok(format!(
        "manga-pipeline-{project_id}-{chapter_number}-{timestamp}"
    ))
}
