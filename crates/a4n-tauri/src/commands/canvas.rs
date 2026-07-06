use a4n_runtime::AppState;
use a4n_runtime::actions::canvas_workflow as canvas_actions;
use a4n_runtime::actions::canvas_workflow::{
    CanvasNodeContract, CanvasNodeRequestPreview, CanvasRunSummary, GetCanvasGraphInput,
    GetCanvasNodeContractsInput, ImportCanvasMediaInput, ImportedCanvasMedia,
    PreviewCanvasNodeRequestInput, RunCanvasGraphInput, RunCanvasNodeInput, SaveCanvasGraphInput,
    SaveCanvasTextAssetInput,
};
use a4n_services::canvas::CanvasGraphDocument;
use tauri::State;

#[tauri::command]
pub async fn get_canvas_graph(
    state: State<'_, AppState>,
    user_id: i64,
    input: GetCanvasGraphInput,
) -> Result<CanvasGraphDocument, String> {
    canvas_actions::get_canvas_graph(&state, user_id, input).await
}

#[tauri::command]
pub async fn get_canvas_node_contracts(
    state: State<'_, AppState>,
    user_id: i64,
    input: GetCanvasNodeContractsInput,
) -> Result<Vec<CanvasNodeContract>, String> {
    canvas_actions::get_canvas_node_contracts(&state, user_id, input).await
}

#[tauri::command]
pub async fn save_canvas_graph(
    state: State<'_, AppState>,
    user_id: i64,
    input: SaveCanvasGraphInput,
) -> Result<CanvasGraphDocument, String> {
    canvas_actions::save_canvas_graph(&state, user_id, input).await
}

#[tauri::command]
pub async fn save_canvas_text_asset(
    state: State<'_, AppState>,
    user_id: i64,
    input: SaveCanvasTextAssetInput,
) -> Result<a4n_core::models::canvas::canvas_text_asset::Model, String> {
    canvas_actions::save_canvas_text_asset(&state, user_id, input).await
}

#[tauri::command]
pub async fn preview_canvas_node_request(
    state: State<'_, AppState>,
    user_id: i64,
    input: PreviewCanvasNodeRequestInput,
) -> Result<CanvasNodeRequestPreview, String> {
    canvas_actions::preview_canvas_node_request(&state, user_id, input).await
}

#[tauri::command]
pub async fn run_canvas_graph(
    state: State<'_, AppState>,
    user_id: i64,
    input: RunCanvasGraphInput,
) -> Result<CanvasRunSummary, String> {
    canvas_actions::run_canvas_graph(&state, user_id, input).await
}

#[tauri::command]
pub async fn run_canvas_node(
    state: State<'_, AppState>,
    user_id: i64,
    input: RunCanvasNodeInput,
) -> Result<CanvasRunSummary, String> {
    canvas_actions::run_canvas_node(&state, user_id, input).await
}

#[tauri::command]
pub async fn import_canvas_media(
    state: State<'_, AppState>,
    user_id: i64,
    input: ImportCanvasMediaInput,
) -> Result<ImportedCanvasMedia, String> {
    canvas_actions::import_canvas_media(&state, user_id, input).await
}
