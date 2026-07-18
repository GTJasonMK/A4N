use crate::routes::commands::{
    ApiError, chapter_generation_actions, command_input, emit_workflow_chunk, finish_web_task,
    i64_arg, optional_i32_field, optional_string_field, register_web_task, required_json_i32,
    string_action_result, to_value, web_progress_reporter,
};
use crate::state::ServerState;
use a4n_core::event::channel::TaskType;
use serde_json::{Value, json};

pub(in crate::routes::commands) async fn generate_chapter_stream(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = required_json_i32(args, "chapterNumber", "generate_chapter_stream")?;
    let input = chapter_generation_actions::GenerateChapterStreamInput {
        project_id,
        chapter_number,
        task_id: optional_string_field(args, "taskId")?,
        writing_notes: optional_string_field(args, "writingNotes")?,
        min_word_count: optional_i32_field(args, "minWordCount")?,
        max_word_count: optional_i32_field(args, "maxWordCount")?,
        client_rag_context: optional_string_field(args, "clientRagContext")?,
    };
    let scope = chapter_generation_actions::generate_chapter_stream_task_scope(
        state.runtime(),
        user_id,
        &input,
    )
    .await
    .map_err(ApiError::bad_request)?;

    let task_id = chapter_generation_actions::chapter_stream_task_id(
        project_id,
        chapter_number,
        input.task_id.clone(),
    )
    .map_err(ApiError::bad_request)?;
    let task = register_web_task(
        state,
        Some(task_id.clone()),
        TaskType::LlmCall,
        scope.project_id,
        scope.total,
        "正在启动章节流式生成",
    )
    .await;
    let cancel_token = task
        .as_ref()
        .map(|handle| handle.cancel_token.clone())
        .unwrap_or_else(tokio_util::sync::CancellationToken::new);
    let mut report_progress = web_progress_reporter(state, task.clone(), Some(task_id.clone()));
    let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::unbounded_channel::<(String, u32)>();
    let chunk_state = (*state).clone();
    let chunk_task_id = task_id.clone();
    let chunk_worker = tokio::spawn(async move {
        while let Some((content, index)) = chunk_rx.recv().await {
            emit_workflow_chunk(&chunk_state, &chunk_task_id, &content, index).await;
        }
    });
    let chunk_task = task.clone();
    let mut report_stream = move |delta: &str, index: u32| {
        if chunk_task
            .as_ref()
            .is_some_and(|handle| handle.is_cancelled())
        {
            return Err(a4n_core::error::AppError::Cancelled);
        }
        chunk_tx.send((delta.to_string(), index)).map_err(|_| {
            a4n_core::error::AppError::InvalidArgument("章节流式事件发送通道已关闭".to_string())
        })?;
        Ok(())
    };
    let result = chapter_generation_actions::generate_chapter_stream(
        state.runtime(),
        user_id,
        chapter_generation_actions::GenerateChapterStreamInput {
            task_id: Some(task_id.clone()),
            ..input
        },
        Some(&mut report_progress),
        Some(&mut report_stream),
        cancel_token,
    )
    .await;
    drop(report_stream);
    let result = match chunk_worker.await {
        Ok(()) => result,
        Err(join_error) => match result {
            Ok(_) => Err(format!("章节流式事件任务异常退出: {join_error}")),
            Err(error) => Err(format!("{error}；章节流式事件任务异常退出: {join_error}")),
        },
    };
    let done_task_id = finish_web_task(
        state,
        Some(task_id),
        string_action_result(
            result,
            task.as_ref().is_some_and(|handle| handle.is_cancelled()),
        ),
        "章节正文生成完成",
    )
    .await?;
    Ok(json!(done_task_id))
}

pub(in crate::routes::commands) async fn prepare_chapter_generation_rag_embedding(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = required_json_i32(
        args,
        "chapterNumber",
        "prepare_chapter_generation_rag_embedding",
    )?;
    let input = chapter_generation_actions::PreviewChapterPromptInput {
        project_id,
        chapter_number,
        writing_notes: optional_string_field(args, "writingNotes")?,
        min_word_count: optional_i32_field(args, "minWordCount")?,
        max_word_count: optional_i32_field(args, "maxWordCount")?,
    };
    Ok(json!(
        chapter_generation_actions::prepare_chapter_generation_rag_embedding(
            state.runtime(),
            user_id,
            input
        )
        .await
        .map_err(ApiError::bad_request)?
    ))
}

pub(in crate::routes::commands) async fn finish_chapter_generation_rag_context(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_generation_actions::FinishChapterGenerationRagContextInput =
        command_input(args, "finish_chapter_generation_rag_context")?;
    Ok(json!(
        chapter_generation_actions::finish_chapter_generation_rag_context(
            state.runtime(),
            user_id,
            input
        )
        .await
        .map_err(ApiError::bad_request)?
    ))
}

pub(in crate::routes::commands) async fn preview_chapter_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_generation_actions::PreviewChapterPromptInput =
        command_input(args, "preview_chapter_prompt")?;
    let preview =
        chapter_generation_actions::preview_chapter_prompt(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(preview)
}
