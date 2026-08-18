mod cursor;
pub mod destructive_lifecycle;

use a4n_api_contract::generation_tasks::{
    CancelGenerationTaskInput, GenerationTaskHandle, GenerationTaskHistoryPage,
    GenerationTaskHistoryQuery, GenerationTaskSummary,
};
use a4n_db::generation::{GenerationTaskListCursor, PersistedGenerationTask};
pub use a4n_orchestrator::GenerationTaskAccessError;
pub use a4n_orchestrator::generation::{
    GenerationReplay, GenerationSubscription, GenerationTerminalError,
};
pub use cursor::{
    GenerationCursorError, decode_generation_history_cursor, encode_generation_history_cursor,
};

use crate::AppState;

pub async fn cancel_generation_task(
    state: &AppState,
    user_id: i64,
    input: CancelGenerationTaskInput,
) -> Result<GenerationTaskHandle, GenerationTerminalError> {
    state
        .orchestrator()
        .request_generation_cancel(user_id, &input.task_id, input.attempt)
        .await
}

pub async fn authorize_generation_task(
    state: &AppState,
    user_id: i64,
    task_id: &str,
    attempt: u32,
) -> Result<PersistedGenerationTask, GenerationTaskAccessError> {
    state
        .orchestrator()
        .authorize_generation_task(user_id, task_id, attempt)
        .await
}

pub async fn generation_task_summary(
    state: &AppState,
    user_id: i64,
    task_id: &str,
    attempt: u32,
) -> Result<GenerationTaskSummary, GenerationTaskAccessError> {
    state
        .orchestrator()
        .generation_task_summary_for_user(user_id, task_id, attempt)
        .await
}

pub async fn list_active_generation_tasks(
    state: &AppState,
    user_id: i64,
    project_id: Option<i64>,
) -> Result<Vec<GenerationTaskSummary>, GenerationTaskAccessError> {
    state
        .orchestrator()
        .list_active_generation_task_summaries(user_id, project_id)
        .await
}

pub async fn list_generation_task_history(
    state: &AppState,
    user_id: i64,
    input: GenerationTaskHistoryQuery,
    cursor: Option<GenerationTaskListCursor>,
) -> Result<GenerationTaskHistoryPage, GenerationTaskAccessError> {
    let result = state
        .orchestrator()
        .list_generation_task_history_summaries(user_id, input.project_id, input.limit, cursor)
        .await?;
    let next_cursor = result
        .next_cursor
        .as_ref()
        .map(encode_generation_history_cursor)
        .transpose()
        .map_err(|error| {
            GenerationTaskAccessError::Persistence(
                a4n_db::generation::GenerationTaskPersistenceError::Corrupt(error.to_string()),
            )
        })?;
    Ok(GenerationTaskHistoryPage {
        items: result.items,
        next_cursor,
    })
}

pub fn subscribe_generation_task(
    state: &AppState,
    task: &PersistedGenerationTask,
    after_sequence: Option<u64>,
) -> Result<GenerationSubscription, GenerationTaskAccessError> {
    state
        .orchestrator()
        .subscribe_generation_task(task, after_sequence)
}
