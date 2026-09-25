use super::{
    ApiError, ServerState, command_input, finish_web_task, i64_arg, optional_string_field,
    register_web_task, sanitize_optional_task_id, to_value, web_fire_and_forget_progress_reporter,
    web_stream_reporter,
};
use a4n_api_contract::fact_feedback::RunProjectFactFeedbackInput;
use a4n_domain::task_queue::TaskKind;
use a4n_runtime::host_api::fact_feedback as actions;
use serde_json::Value;

pub(super) async fn get_project_fact_feedback(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        actions::get_project_fact_feedback(state.runtime(), user_id, i64_arg(args, "projectId")?)
            .await?,
    )
}

pub(super) async fn run_project_fact_feedback(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: RunProjectFactFeedbackInput = command_input(args, "run_project_fact_feedback")?;
    actions::preflight_project_fact_feedback(state.runtime(), user_id, input.project_id).await?;
    let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
    let task = register_web_task(
        state,
        user_id,
        task_id.clone(),
        TaskKind::LlmCall,
        input.project_id,
        100,
        "正在反馈正文事实",
    )?;
    let cancel = task.as_ref().map(|task| task.cancel_token.clone());
    let reporter = web_fire_and_forget_progress_reporter(task.clone());
    let stream = web_stream_reporter(task.clone());
    let reporter = |current, total, message: &str| {
        reporter(current, total, message);
        Ok(())
    };
    let result = actions::run_project_fact_feedback(
        state.runtime(),
        user_id,
        input,
        actions::ChapterAnalysisApplicationProgress {
            reporter: Some(&reporter),
            stream_reporter: Some(&stream),
            cancel_token: cancel.as_ref(),
            active_task_id: task_id.as_deref(),
        },
    )
    .await;
    to_value(finish_web_task(state, task.as_ref(), result)?)
}

pub(super) async fn preview_project_fact_feedback_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = command_input(args, "preview_project_fact_feedback_request")?;
    to_value(actions::preview_project_fact_feedback_request(state.runtime(), user_id, input).await?)
}
