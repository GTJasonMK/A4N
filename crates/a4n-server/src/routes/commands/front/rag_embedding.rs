use serde_json::{Value, json};

use super::super::{
    ApiError, ServerState, TaskType, command_input, embedding_model_actions, emit_workflow_error,
    finish_web_task, i64_arg, optional_string_field, rag_search_actions, register_web_task,
    sanitize_optional_task_id, spawn_embedding_download_progress_bridge, task_records_actions,
    web_fire_and_forget_progress_reporter,
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
    let scope =
        a4n_runtime::rag_ingest::rag_ingest_task_scope(state.runtime(), user_id, project_id)
            .await
            .map_err(ApiError::bad_request)?;
    let task = register_web_task(
        state,
        task_id.clone(),
        TaskType::RagIngestion,
        scope.project_id,
        scope.total,
        "准备同步 RAG 数据",
    )
    .await;
    let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
    let report_progress =
        web_fire_and_forget_progress_reporter(state, task.clone(), task_id.clone());
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
    let value = finish_web_task(state, task_id, result, "RAG 同步完成").await?;
    Ok(value)
}

pub(in crate::routes::commands) async fn rag_prepare_ingest_all(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    Ok(json!(
        a4n_runtime::rag_ingest::rag_prepare_ingest_all(state.runtime(), user_id, project_id)
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
    _user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let mut input: embedding_model_actions::EmbeddingModelInput =
        command_input(args, "download_embedding_model")?;
    let task_id = sanitize_optional_task_id(input.task_id.clone())?;
    input.task_id = task_id.clone();
    let scope = embedding_model_actions::embedding_model_download_task_scope();
    let task = register_web_task(
        state,
        task_id.clone(),
        TaskType::EmbeddingDownload,
        None,
        scope.total,
        scope.initial_message,
    )
    .await;
    let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
    let progress_tx = task_id
        .as_ref()
        .map(|task_id| spawn_embedding_download_progress_bridge(state, task_id, task.clone()));
    let result = embedding_model_actions::download_embedding_model(
        state.runtime(),
        input,
        progress_tx,
        cancel_token,
    )
    .await;
    let summary = finish_web_task(state, task_id, result, "模型已下载并完成初始化").await?;
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
    emit_workflow_error(state, &task_id, "cancelled", "模型下载已停止", false).await;
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
