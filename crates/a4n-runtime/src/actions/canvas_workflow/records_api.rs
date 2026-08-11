use super::{
    CanvasGraphDocument, CanvasNodeContract, GetCanvasGraphInput, GetCanvasNodeContractsInput,
    SaveCanvasGraphInput, SaveCanvasTextAssetInput, contracts::canvas_node_contracts,
};
use crate::{
    AppState,
    actions::{access, app_error_message},
};
use a4n_core::models::canvas::canvas_text_asset;
use chrono::Utc;

pub async fn get_canvas_graph(
    state: &AppState,
    user_id: i64,
    input: GetCanvasGraphInput,
) -> Result<CanvasGraphDocument, String> {
    access::verify_project_access_string(state, user_id, input.project_id).await?;
    state
        .orchestrator()
        .get_or_create_canvas_graph_record(input.project_id, input.graph_id, Utc::now().naive_utc())
        .await
        .map_err(app_error_message)
}

pub async fn get_canvas_node_contracts(
    _state: &AppState,
    _user_id: i64,
    _input: GetCanvasNodeContractsInput,
) -> Result<Vec<CanvasNodeContract>, String> {
    Ok(canvas_node_contracts())
}

pub async fn save_canvas_graph(
    state: &AppState,
    user_id: i64,
    input: SaveCanvasGraphInput,
) -> Result<CanvasGraphDocument, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    state
        .orchestrator()
        .save_canvas_graph_record(input.project_id, input.graph, Utc::now().naive_utc())
        .await
        .map_err(app_error_message)
}

pub async fn save_canvas_text_asset(
    state: &AppState,
    user_id: i64,
    input: SaveCanvasTextAssetInput,
) -> Result<canvas_text_asset::Model, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    state
        .orchestrator()
        .upsert_canvas_text_asset_record(input.project_id, input.text_asset, Utc::now().naive_utc())
        .await
        .map_err(app_error_message)
}
