use super::{
    CanvasNodeContract, GetCanvasGraphInput, GetCanvasNodeContractsInput, SaveCanvasGraphInput,
    SaveCanvasTextAssetInput,
};
use crate::{AppState, actions::app_error_message};
use a4n_api_contract::canvas::{CanvasGraphDocument, CanvasTextAssetSummary};

pub async fn get_canvas_graph(
    state: &AppState,
    user_id: i64,
    input: GetCanvasGraphInput,
) -> Result<CanvasGraphDocument, String> {
    state
        .canvas_read_application()
        .get_graph(user_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn get_canvas_node_contracts(
    _state: &AppState,
    _user_id: i64,
    _input: GetCanvasNodeContractsInput,
) -> Result<Vec<CanvasNodeContract>, String> {
    Ok(a4n_api_contract::canvas::canvas_node_contracts())
}

pub async fn save_canvas_graph(
    state: &AppState,
    user_id: i64,
    input: SaveCanvasGraphInput,
) -> Result<CanvasGraphDocument, String> {
    state
        .canvas_mutation_application()
        .save_graph(user_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn read_canvas_graph(
    state: &AppState,
    user_id: i64,
    input: a4n_api_contract::canvas::ReadCanvasGraphInput,
) -> Result<a4n_api_contract::canvas::CanvasGraphReadResult, String> {
    state
        .canvas_read_application()
        .read_graph(user_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn edit_canvas_graph(
    state: &AppState,
    user_id: i64,
    input: a4n_api_contract::canvas::EditCanvasGraphInput,
) -> Result<a4n_api_contract::canvas::CanvasGraphEditResult, String> {
    state
        .canvas_mutation_application()
        .edit_graph(user_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn save_canvas_text_asset(
    state: &AppState,
    user_id: i64,
    input: SaveCanvasTextAssetInput,
) -> Result<CanvasTextAssetSummary, String> {
    state
        .canvas_mutation_application()
        .save_text_asset(user_id, input)
        .await
        .map_err(app_error_message)
}
