use super::{ApiError, command_input, to_value};
use crate::state::ServerState;
use a4n_runtime::host_api::canvas_workflow as canvas_workflow_actions;
use serde_json::Value;

pub(super) fn is_canvas_web_command(cmd: &str) -> bool {
    matches!(
        cmd,
        "get_canvas_graph"
            | "read_canvas_graph"
            | "edit_canvas_graph"
            | "get_canvas_node_contracts"
            | "save_canvas_graph"
            | "save_canvas_text_asset"
            | "preview_canvas_node_request"
            | "run_canvas_graph"
            | "run_canvas_node"
            | "import_canvas_media"
    )
}

pub(super) async fn run_canvas_web_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    cmd: &str,
) -> Result<Value, ApiError> {
    match cmd {
        "read_canvas_graph" => to_value(
            canvas_workflow_actions::read_canvas_graph(
                state.runtime(),
                user_id,
                command_input(args, cmd)?,
            )
            .await
            .map_err(ApiError::bad_request)?,
        ),
        "edit_canvas_graph" => to_value(
            canvas_workflow_actions::edit_canvas_graph(
                state.runtime(),
                user_id,
                command_input(args, cmd)?,
            )
            .await
            .map_err(ApiError::bad_request)?,
        ),
        "get_canvas_graph" => get_canvas_graph(state, user_id, args).await,
        "get_canvas_node_contracts" => get_canvas_node_contracts(state, user_id, args).await,
        "save_canvas_graph" => save_canvas_graph(state, user_id, args).await,
        "save_canvas_text_asset" => save_canvas_text_asset(state, user_id, args).await,
        "preview_canvas_node_request" => preview_canvas_node_request(state, user_id, args).await,
        "run_canvas_graph" => run_canvas_graph(state, user_id, args).await,
        "run_canvas_node" => run_canvas_node(state, user_id, args).await,
        "import_canvas_media" => import_canvas_media(state, user_id, args).await,
        _ => Err(ApiError::bad_request(format!(
            "不支持的 Canvas Web command: {cmd}"
        ))),
    }
}

async fn get_canvas_graph(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        canvas_workflow_actions::get_canvas_graph(
            state.runtime(),
            user_id,
            command_input(args, "get_canvas_graph")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn get_canvas_node_contracts(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        canvas_workflow_actions::get_canvas_node_contracts(
            state.runtime(),
            user_id,
            command_input(args, "get_canvas_node_contracts")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn save_canvas_graph(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        canvas_workflow_actions::save_canvas_graph(
            state.runtime(),
            user_id,
            command_input(args, "save_canvas_graph")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn save_canvas_text_asset(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        canvas_workflow_actions::save_canvas_text_asset(
            state.runtime(),
            user_id,
            command_input(args, "save_canvas_text_asset")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn preview_canvas_node_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        canvas_workflow_actions::preview_canvas_node_request(
            state.runtime(),
            user_id,
            command_input(args, "preview_canvas_node_request")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn run_canvas_graph(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        canvas_workflow_actions::run_canvas_graph(
            state.runtime(),
            user_id,
            command_input(args, "run_canvas_graph")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn run_canvas_node(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        canvas_workflow_actions::run_canvas_node(
            state.runtime(),
            user_id,
            command_input(args, "run_canvas_node")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

async fn import_canvas_media(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        canvas_workflow_actions::import_canvas_media(
            state.runtime(),
            user_id,
            command_input(args, "import_canvas_media")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}
