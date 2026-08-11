use serde_json::{Value, json};

use super::super::{
    ApiError, ServerState, TaskKind, command_input, embedding_model_actions, finish_web_task,
    i64_arg, optional_bool_field, optional_string_field, rag_search_actions, register_web_task,
    sanitize_optional_task_id, task_records_actions, web_fire_and_forget_progress_reporter,
};

pub(in crate::routes::commands) async fn rag_enhanced_search(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let input: rag_search_actions::EnhancedSearchInput =
        command_input(args, "rag_enhanced_search")?;
    Ok(json!(
        rag_search_actions::rag_enhanced_search(state.runtime(), user_id, project_id, input)
            .await?
    ))
}

pub(in crate::routes::commands) async fn rag_prepare_enhanced_search(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let input: rag_search_actions::EnhancedSearchInput =
        command_input(args, "rag_prepare_enhanced_search")?;
    Ok(json!(
        rag_search_actions::rag_prepare_enhanced_search(
            state.runtime(),
            user_id,
            project_id,
            input,
        )
        .await?
    ))
}

pub(in crate::routes::commands) async fn rag_enhanced_search_with_vectors(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let input: rag_search_actions::RagEnhancedSearchWithVectorsInput =
        command_input(args, "rag_enhanced_search_with_vectors")?;
    Ok(json!(
        rag_search_actions::rag_enhanced_search_with_vectors(
            state.runtime(),
            user_id,
            project_id,
            input,
        )
        .await?
    ))
}

pub(in crate::routes::commands) async fn rag_ingest_all(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
    let rebuild = optional_bool_field(args, "rebuild")?;
    let scope =
        a4n_runtime::rag_ingest::rag_ingest_task_scope(state.runtime(), user_id, project_id)
            .await
            .map_err(ApiError::bad_request)?;
    let task = register_web_task(
        state,
        user_id,
        task_id.clone(),
        TaskKind::RagIngestion,
        scope.project_id,
        scope.total,
        "准备同步 RAG 数据",
    );
    let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
    let report_progress = web_fire_and_forget_progress_reporter(task.clone());
    let reporter: a4n_runtime::rag_ingest::RagProgressReporter<'_> = task_id
        .as_ref()
        .map(|_| &report_progress as &(dyn Fn(u32, u32, &str) + Send + Sync));
    let mut progress = a4n_runtime::rag_ingest::RagIngestProgress::new(
        scope.total,
        reporter,
        cancel_token.as_ref(),
    );
    let result = a4n_runtime::rag_ingest::rag_ingest_all(
        state.runtime(),
        user_id,
        project_id,
        task_id.as_deref(),
        rebuild,
        Some(&mut progress),
    )
    .await
    .map_err(|err| {
        if task.as_ref().is_some_and(|handle| handle.is_cancelled()) {
            a4n_core::error::AppError::Cancelled
        } else {
            a4n_core::error::AppError::Internal(anyhow::anyhow!(err))
        }
    });
    let value = finish_web_task(state, task_id, result)?;
    Ok(value)
}

pub(in crate::routes::commands) async fn rag_prepare_ingest_all(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let rebuild = optional_bool_field(args, "rebuild")?;
    Ok(json!(
        a4n_runtime::rag_ingest::rag_prepare_ingest_all(
            state.runtime(),
            user_id,
            project_id,
            rebuild,
        )
        .await
        .map_err(ApiError::bad_request)?
    ))
}

pub(in crate::routes::commands) async fn rag_upload_ingest_embeddings(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let input: a4n_runtime::rag_ingest::ClientRagIngestUploadInput =
        command_input(args, "rag_upload_ingest_embeddings")?;
    Ok(json!(
        a4n_runtime::rag_ingest::rag_upload_ingest_embeddings(
            state.runtime(),
            user_id,
            project_id,
            input,
        )
        .await
        .map_err(ApiError::bad_request)?
    ))
}

pub(in crate::routes::commands) async fn list_local_embedding_models(
    state: &ServerState,
    _user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    Ok(json!(
        embedding_model_actions::list_local_embedding_models(state.runtime()).await?
    ))
}

pub(in crate::routes::commands) async fn get_embedding_model_download_info(
    state: &ServerState,
    _user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: embedding_model_actions::EmbeddingModelInput =
        command_input(args, "get_embedding_model_download_info")?;
    Ok(json!(
        embedding_model_actions::get_embedding_model_download_info(state.runtime(), input).await?
    ))
}

pub(in crate::routes::commands) async fn download_embedding_model(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: embedding_model_actions::DownloadEmbeddingModelInput =
        command_input(args, "download_embedding_model")?;
    let task_id =
        embedding_model_actions::normalize_optional_embedding_download_task_id(input.task_id)?;
    let task = register_web_task(
        state,
        user_id,
        task_id.clone(),
        TaskKind::EmbeddingDownload,
        None,
        embedding_model_actions::EMBEDDING_MODEL_DOWNLOAD_TASK_TOTAL,
        embedding_model_actions::EMBEDDING_MODEL_DOWNLOAD_INITIAL_MESSAGE,
    );
    let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
    let (progress_tx, progress_worker) = match task.as_ref() {
        Some(task) => {
            let (tx, mut rx) =
                tokio::sync::mpsc::unbounded_channel::<embedding_model_actions::DownloadProgress>();
            let task = task.clone();
            let worker = tokio::spawn(async move {
                while let Some(progress) = rx.recv().await {
                    task.update_progress_bytes(
                        progress.downloaded_bytes,
                        progress.total_bytes,
                        embedding_model_actions::EMBEDDING_MODEL_DOWNLOAD_TASK_TOTAL,
                        &progress.message,
                    );
                }
            });
            (Some(tx), Some(worker))
        }
        None => (None, None),
    };
    let mut result = embedding_model_actions::download_embedding_model(
        state.runtime(),
        embedding_model_actions::EmbeddingModelInput { model: input.model },
        progress_tx,
        cancel_token,
    )
    .await;
    if let Some(worker) = progress_worker
        && let Err(join_error) = worker.await
    {
        let bridge_error = format!("模型下载进度事件任务异常退出: {join_error}");
        result = Err(a4n_core::error::AppError::Internal(anyhow::anyhow!(
            "{}",
            match result {
                Ok(_) => bridge_error,
                Err(error) => format!("{error}；{bridge_error}"),
            }
        )));
    }
    let summary = finish_web_task(state, task_id, result)?;
    Ok(json!(summary))
}

pub(in crate::routes::commands) async fn cancel_embedding_model_download(
    state: &ServerState,
    _user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: embedding_model_actions::CancelEmbeddingModelDownloadInput =
        command_input(args, "cancel_embedding_model_download")?;
    let task_id = embedding_model_actions::required_task_id(input)?;
    if !task_records_actions::cancel_registered_task(state.runtime(), &task_id) {
        return Err(ApiError::bad_request(format!(
            "模型下载任务不存在或已结束：{task_id}"
        )));
    }
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn delete_local_embedding_model(
    state: &ServerState,
    _user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: embedding_model_actions::EmbeddingModelInput =
        command_input(args, "delete_local_embedding_model")?;
    embedding_model_actions::delete_local_embedding_model(state.runtime(), input).await?;
    Ok(Value::Null)
}
