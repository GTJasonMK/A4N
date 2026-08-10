use serde_json::{Value, json};

use super::super::super::{
    ApiError, ServerState, command_input, manga_pipeline_actions, manga_records_actions, to_value,
};

pub(in crate::routes::commands) async fn get_manga_pipeline_status(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_records_actions::MangaChapterInput =
        command_input(args, "get_manga_pipeline_status")?;
    let status = manga_records_actions::get_manga_pipeline_status(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(status)
}

pub(in crate::routes::commands) async fn cancel_manga_pipeline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_records_actions::MangaChapterInput =
        command_input(args, "cancel_manga_pipeline")?;
    let cancelled = manga_records_actions::cancel_manga_pipeline(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(json!(cancelled))
}

pub(in crate::routes::commands) async fn start_manga_pipeline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_pipeline_actions::MangaPipelineStartInput =
        command_input(args, "start_manga_pipeline")?;
    start_or_resume_manga_pipeline(
        state,
        user_id,
        input.project_id,
        input.chapter_number,
        false,
        input.start_from_stage,
    )
    .await
}

pub(in crate::routes::commands) async fn resume_manga_pipeline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_pipeline_actions::MangaPipelineResumeInput =
        command_input(args, "resume_manga_pipeline")?;
    start_or_resume_manga_pipeline(
        state,
        user_id,
        input.project_id,
        input.chapter_number,
        true,
        None,
    )
    .await
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
    let state_clone = state.clone();
    let task_id_clone = task_id.clone();
    let runtime_config_snapshot = a4n_runtime::runtime_config::current_snapshot();
    tokio::spawn(async move {
        a4n_runtime::runtime_config::request_scope(user_id, runtime_config_snapshot, async move {
            if let Err(error) = manga_pipeline_actions::run_manga_pipeline(
                state_clone.runtime(),
                user_id,
                project_id,
                chapter_number,
                &task_id_clone,
                resume_from_checkpoint,
                start_from_stage,
            )
            .await
            {
                tracing::error!(
                    task_id = task_id_clone,
                    project_id,
                    chapter_number,
                    %error,
                    "漫画流水线执行失败"
                );
            }
        })
        .await;
    });

    Ok(json!(task_id))
}
