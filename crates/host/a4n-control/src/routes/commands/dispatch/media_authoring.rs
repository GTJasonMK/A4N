use super::super::{
    ApiError, ServerState, TaskKind, finish_web_task, register_web_task, strict_command_input,
    to_value, web_stream_delta_reporter,
};
use a4n_runtime::host_api::media_authoring as actions;
use serde_json::Value;

pub(super) async fn try_dispatch(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    cmd: &str,
) -> Result<Option<Value>, ApiError> {
    let result = match cmd {
        "get_media_authoring_workspace" => to_value(
            actions::get_media_authoring_workspace(
                state.runtime(),
                user_id,
                strict_command_input(args, cmd)?,
            )
            .await?,
        )?,
        "save_media_authoring_document" => to_value(
            actions::save_media_authoring_document(
                state.runtime(),
                user_id,
                strict_command_input(args, cmd)?,
            )
            .await?,
        )?,
        "adopt_media_authoring_document" => to_value(
            actions::adopt_media_authoring_document(
                state.runtime(),
                user_id,
                strict_command_input(args, cmd)?,
            )
            .await?,
        )?,
        "delete_media_authoring_document" => to_value(
            actions::delete_media_authoring_document(
                state.runtime(),
                user_id,
                strict_command_input(args, cmd)?,
            )
            .await?,
        )?,
        "list_video_adaptation_references" => to_value(
            actions::list_video_adaptation_references(
                state.runtime(),
                user_id,
                strict_command_input(args, cmd)?,
            )
            .await?,
        )?,
        "generate_media_adaptation_plan" => {
            let input: a4n_api_contract::media_authoring::GenerateAdaptationPlanInput =
                strict_command_input(args, cmd)?;
            let task = register_web_task(
                state,
                user_id,
                Some(input.task_id.clone()),
                TaskKind::LlmCall,
                input.project_id,
                1,
                "正在生成计划草稿…",
            )?;
            let stream_reporter = web_stream_delta_reporter(task.clone());
            let result = actions::generate_media_adaptation_plan(
                state.runtime(),
                user_id,
                input,
                Some(&stream_reporter),
            )
            .await;
            to_value(finish_web_task(state, task.as_ref(), result)?)?
        }
        "preview_media_adaptation_plan_request" => to_value(
            actions::preview_media_adaptation_plan_request(
                state.runtime(),
                user_id,
                strict_command_input(args, cmd)?,
            )
            .await?,
        )?,
        _ => return Ok(None),
    };
    Ok(Some(result))
}
