use serde_json::{Value, json};

use super::super::super::{
    ApiError, ServerState, emit_workflow_complete, emit_workflow_error, emit_workflow_progress,
    i64_arg, manga_pipeline_actions, manga_records_actions, optional_string_field,
    required_json_i32, to_value,
};

pub(in crate::routes::commands) async fn get_manga_pipeline_status(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let status = manga_records_actions::get_manga_pipeline_status(
        state.runtime(),
        user_id,
        project_id,
        chapter_id,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(status)
}

pub(in crate::routes::commands) async fn cancel_manga_pipeline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let cancelled = manga_records_actions::cancel_manga_pipeline(
        state.runtime(),
        user_id,
        project_id,
        chapter_id,
    )
    .await
    .map_err(ApiError::bad_request)?;
    Ok(json!(cancelled))
}

pub(in crate::routes::commands) async fn start_manga_pipeline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = required_json_i32(args, "chapterNumber", "start_manga_pipeline")?;
    let start_from_stage = manga_pipeline_actions::parse_manga_pipeline_start_stage(
        optional_string_field(args, "startFromStage")?,
    )?;
    start_or_resume_manga_pipeline(
        state,
        user_id,
        project_id,
        chapter_number,
        false,
        start_from_stage,
    )
    .await
}

pub(in crate::routes::commands) async fn resume_manga_pipeline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = required_json_i32(args, "chapterNumber", "resume_manga_pipeline")?;
    start_or_resume_manga_pipeline(state, user_id, project_id, chapter_number, true, None).await
}

pub(in crate::routes::commands) async fn start_or_resume_manga_pipeline(
    state: &ServerState,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    resume_from_checkpoint: bool,
    start_from_stage: Option<manga_pipeline_actions::MangaPipelineStartStage>,
) -> Result<Value, ApiError> {
    manga_pipeline_actions::preflight_manga_pipeline(
        state.runtime(),
        user_id,
        project_id,
        chapter_number,
    )
    .await?;

    let task_id = format!(
        "manga-pipeline-{project_id}-{chapter_number}-{}",
        chrono::Utc::now().timestamp_millis()
    );
    let start_message = if resume_from_checkpoint {
        "续传漫画 8 步流水线"
    } else {
        "启动漫画 8 步流水线"
    };
    emit_workflow_progress(state, &task_id, 0, 8, start_message).await;

    let state_clone = state.clone();
    let task_id_clone = task_id.clone();
    let runtime_config_snapshot = a4n_runtime::runtime_config::current_snapshot();
    tokio::spawn(async move {
        a4n_runtime::runtime_config::request_scope(user_id, runtime_config_snapshot, async move {
            emit_workflow_progress(&state_clone, &task_id_clone, 1, 8, "正在运行 LLM 流水线").await;
            let result = manga_pipeline_actions::run_manga_pipeline(
                state_clone.runtime(),
                user_id,
                project_id,
                chapter_number,
                &task_id_clone,
                resume_from_checkpoint,
                start_from_stage,
            )
            .await;
            match result {
                Ok(panels) => {
                    emit_workflow_complete(
                        &state_clone,
                        &task_id_clone,
                        Some(format!(
                            "漫画流水线完成，生成 {} 页整页提示词",
                            panels.len()
                        )),
                    )
                    .await;
                }
                Err(err) => {
                    emit_workflow_error(
                        &state_clone,
                        &task_id_clone,
                        "pipeline_failed",
                        &err.to_string(),
                        false,
                    )
                    .await;
                }
            }
        })
        .await;
    });

    Ok(json!(task_id))
}
