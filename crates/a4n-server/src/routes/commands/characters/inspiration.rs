use serde_json::{Value, json};

use super::super::{
    ApiError, ServerState, command_input, emit_workflow_chunk, emit_workflow_complete,
    emit_workflow_error, i64_arg, inspiration_records_actions, to_value, u64_arg,
};

pub(in crate::routes::commands) async fn get_inspiration_history(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let limit = u64_arg(args, "limit")?;
    to_value(
        inspiration_records_actions::get_inspiration_history(
            state.runtime(),
            user_id,
            project_id,
            limit,
        )
        .await?,
    )
}

pub(in crate::routes::commands) async fn converse_inspiration(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: inspiration_records_actions::ConverseInput =
        command_input(args, "converse_inspiration")?;
    Ok(json!(
        inspiration_records_actions::converse_inspiration(state.runtime(), user_id, input).await?
    ))
}

pub(in crate::routes::commands) async fn converse_inspiration_stream(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: inspiration_records_actions::ConverseStreamInput =
        command_input(args, "converse_inspiration_stream")?;
    let task_id = inspiration_records_actions::inspiration_stream_task_id(
        input.project_id,
        input.task_id.clone(),
    )?;
    let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::unbounded_channel::<(String, u32)>();
    let chunk_task_id = task_id.clone();
    let chunk_state = (*state).clone();
    let chunk_worker = tokio::spawn(async move {
        while let Some((content, index)) = chunk_rx.recv().await {
            emit_workflow_chunk(&chunk_state, &chunk_task_id, &content, index).await;
        }
    });

    let result = inspiration_records_actions::converse_inspiration_stream(
        state.runtime(),
        user_id,
        input,
        move |delta, index| {
            let _ = chunk_tx.send((delta.to_string(), index));
        },
    )
    .await;
    let result = match chunk_worker.await {
        Ok(()) => result,
        Err(join_error) => match result {
            Ok(_) => Err(a4n_core::error::AppError::Internal(anyhow::anyhow!(
                "灵感对话流式事件任务异常退出: {join_error}"
            ))),
            Err(error) => Err(a4n_core::error::AppError::Internal(anyhow::anyhow!(
                "{error}；灵感对话流式事件任务异常退出: {join_error}"
            ))),
        },
    };

    match result {
        Ok(full) => {
            emit_workflow_complete(state, &task_id, Some(full.clone())).await;
            Ok(json!(full))
        }
        Err(err) => {
            let message = err.to_string();
            emit_workflow_error(state, &task_id, "stream_error", &message, false).await;
            Err(err.into())
        }
    }
}

pub(in crate::routes::commands) async fn preview_inspiration_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: inspiration_records_actions::PreviewInspirationRequestInput =
        command_input(args, "preview_inspiration_request")?;
    to_value(
        inspiration_records_actions::preview_inspiration_request(state.runtime(), user_id, input)
            .await?,
    )
}
