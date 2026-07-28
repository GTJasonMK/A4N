mod cursor;
pub mod destructive_lifecycle;

use a4n_api_contract::generation_tasks::{
    CancelGenerationTaskInput, GenerationTaskHandle, GenerationTaskHistoryPage,
    GenerationTaskHistoryQuery, GenerationTaskSummary,
};
use a4n_db::generation::PersistedGenerationTask;
pub use a4n_db::generation::{DEFAULT_GENERATION_HISTORY_LIMIT, MAX_GENERATION_HISTORY_LIMIT};
pub use a4n_orchestrator::GenerationTaskAccessError;
pub use a4n_orchestrator::generation::events::{GenerationReplay, GenerationSubscription};
pub use a4n_orchestrator::generation::terminal::GenerationTerminalError;
use a4n_orchestrator::generation_task_summary as summarize_generation_task;
pub use cursor::{
    GenerationCursorError, GenerationHistoryCursor, decode_generation_history_cursor,
    encode_generation_history_cursor,
};

use crate::AppState;

pub async fn cancel_generation_task(
    state: &AppState,
    user_id: i64,
    input: CancelGenerationTaskInput,
) -> Result<GenerationTaskHandle, String> {
    state
        .generation_tasks()
        .request_cancel(user_id, &input.task_id, input.attempt)
        .await
        .map_err(|error| error.to_string())
}

pub async fn cancel_generation_task_typed(
    state: &AppState,
    user_id: i64,
    input: CancelGenerationTaskInput,
) -> Result<GenerationTaskHandle, GenerationTerminalError> {
    state
        .generation_tasks()
        .request_cancel(user_id, &input.task_id, input.attempt)
        .await
}

pub async fn authorize_generation_task(
    state: &AppState,
    user_id: i64,
    task_id: &str,
    attempt: u32,
) -> Result<PersistedGenerationTask, GenerationTaskAccessError> {
    state
        .generation_tasks()
        .authorize_task(user_id, task_id, attempt)
        .await
}

pub fn generation_task_summary_for_host(
    task: &PersistedGenerationTask,
) -> Result<a4n_api_contract::generation_tasks::GenerationTaskSummary, GenerationTaskAccessError> {
    summarize_generation_task(task)
}

pub async fn generation_task_summary(
    state: &AppState,
    user_id: i64,
    task_id: &str,
    attempt: u32,
) -> Result<GenerationTaskSummary, GenerationTaskAccessError> {
    state
        .generation_tasks()
        .task_summary(user_id, task_id, attempt)
        .await
}

pub async fn list_active_generation_tasks(
    state: &AppState,
    user_id: i64,
    project_id: Option<i64>,
) -> Result<Vec<GenerationTaskSummary>, GenerationTaskAccessError> {
    state
        .generation_tasks()
        .list_active_summaries(user_id, project_id)
        .await
}

pub async fn list_generation_task_history(
    state: &AppState,
    user_id: i64,
    input: GenerationTaskHistoryQuery,
    cursor: Option<GenerationHistoryCursor>,
) -> Result<GenerationTaskHistoryPage, GenerationTaskAccessError> {
    let result = state
        .generation_tasks()
        .list_history_summaries(user_id, input.project_id, input.limit, cursor)
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
        .generation_tasks()
        .subscribe_task(task, after_sequence)
}

pub fn replay_generation_task(
    state: &AppState,
    task_id: &str,
    attempt: u32,
    after_sequence: Option<u64>,
) -> Result<GenerationReplay, GenerationTaskAccessError> {
    state
        .generation_tasks()
        .replay_task(task_id, attempt, after_sequence)
}
