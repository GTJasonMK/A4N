use a4n_error::AppError;

use super::{PersistentTaskQueue, queue_db_error};

#[cfg(test)]
mod tests;

pub(super) async fn recover_pending(queue: &PersistentTaskQueue) -> Result<(), AppError> {
    let active_task_ids = a4n_db::queue::recoverable_active_task_ids(&queue.db)
        .await
        .map_err(queue_db_error)?;
    a4n_db::queue::interrupt_active_tasks(
        &queue.db,
        &active_task_ids,
        "应用重启后中断未绑定执行器的队列任务",
    )
    .await
    .map_err(queue_db_error)?;
    a4n_db::workflow::recover_stale_in_process_workflows(&queue.db)
        .await
        .map_err(workflow_write_app_error)?;
    a4n_db::video::recover_stale_generated_videos_without_provider_job(&queue.db)
        .await
        .map_err(video_db_error)?;
    Ok(())
}

fn workflow_write_app_error(error: a4n_db::workflow::WorkflowWriteError) -> AppError {
    match error {
        a4n_db::workflow::WorkflowWriteError::InvalidArgument(message)
        | a4n_db::workflow::WorkflowWriteError::NotFound(message)
        | a4n_db::workflow::WorkflowWriteError::InvalidData(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::workflow::WorkflowWriteError::StatusConflict { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        a4n_db::workflow::WorkflowWriteError::Database(error) => AppError::database(error),
    }
}

fn video_db_error(error: a4n_db::video::VideoDbError) -> AppError {
    match error {
        a4n_db::video::VideoDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        a4n_db::video::VideoDbError::InvalidState { current, operation } => {
            AppError::InvalidState { current, operation }
        }
        a4n_db::video::VideoDbError::NotFound(message) => AppError::NotFound(message),
        a4n_db::video::VideoDbError::Forbidden(message) => AppError::Forbidden(message),
        a4n_db::video::VideoDbError::Database(error) => AppError::database(error),
    }
}
