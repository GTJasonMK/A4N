mod cursor;
pub(crate) mod destructive_lifecycle;

use a4n_api_contract::generation_tasks::{
    CancelGenerationTaskInput, GenerationTaskHandle, GenerationTaskHistoryPage,
    GenerationTaskHistoryQuery, GenerationTaskSummary,
};
use a4n_domain::generation::GenerationTaskListCursor;
pub use a4n_generation_host_contract::{
    AuthorizedGenerationTask, GenerationReplay, GenerationSubscription, GenerationTaskAccessError,
    GenerationTerminalError,
};
pub(crate) use cursor::encode_generation_history_cursor;
pub use cursor::{GenerationCursorError, decode_generation_history_cursor};

use crate::AppState;

pub async fn cancel_generation_task(
    state: &AppState,
    user_id: i64,
    input: CancelGenerationTaskInput,
) -> Result<GenerationTaskHandle, GenerationTerminalError> {
    state
        .generation_task_mutation_application()
        .request_generation_cancel(user_id, &input.task_id, input.attempt)
        .await
}

pub async fn authorize_generation_task(
    state: &AppState,
    user_id: i64,
    task_id: &str,
    attempt: u32,
) -> Result<AuthorizedGenerationTask, GenerationTaskAccessError> {
    state
        .generation_task_read_application()
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
        .generation_task_read_application()
        .generation_task_summary_for_user(user_id, task_id, attempt)
        .await
}

pub async fn list_active_generation_tasks(
    state: &AppState,
    user_id: i64,
    project_id: Option<i64>,
) -> Result<Vec<GenerationTaskSummary>, GenerationTaskAccessError> {
    state
        .generation_task_read_application()
        .list_active_generation_task_summaries(user_id, project_id)
        .await
}

pub async fn active_generation_task_count(state: &AppState) -> Result<usize, String> {
    state
        .generation_lifecycle_application()
        .active_generation_tasks()
        .await
        .map(|tasks| tasks.len())
        .map_err(|error| error.to_string())
}

pub async fn list_generation_task_history(
    state: &AppState,
    user_id: i64,
    input: GenerationTaskHistoryQuery,
    cursor: Option<GenerationTaskListCursor>,
) -> Result<GenerationTaskHistoryPage, GenerationTaskAccessError> {
    let result = state
        .generation_task_read_application()
        .list_generation_task_history_summaries(user_id, input.project_id, input.limit, cursor)
        .await?;
    let next_cursor = result
        .next_cursor
        .as_ref()
        .map(encode_generation_history_cursor)
        .transpose()
        .map_err(|error| {
            GenerationTaskAccessError::new(
                "generation_request_invalid",
                format!("生成任务历史 cursor 编码失败: {error}"),
                false,
            )
        })?;
    Ok(GenerationTaskHistoryPage {
        items: result.items,
        next_cursor,
    })
}

pub fn subscribe_generation_task(
    state: &AppState,
    task: &AuthorizedGenerationTask,
    after_sequence: Option<u64>,
) -> Result<GenerationSubscription, GenerationTaskAccessError> {
    state
        .generation_task_read_application()
        .subscribe_generation_task(task, after_sequence)
}
