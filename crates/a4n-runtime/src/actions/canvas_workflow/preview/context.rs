use crate::AppState;
use crate::actions::{access, app_error_message};
use a4n_orchestrator::canvas::{
    CanvasGraphRecordDocument as CanvasGraphDocument,
    validate_canvas_record_document_edge_contract as validate_canvas_document_edge_contract,
};
use chrono::Utc;

pub(super) async fn load_canvas_preview_document(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    graph_id: i64,
) -> Result<CanvasGraphDocument, String> {
    access::verify_project_access_string(state, user_id, project_id).await?;
    let document = state
        .orchestrator
        .get_or_create_canvas_graph_record(project_id, Some(graph_id), Utc::now().naive_utc())
        .await
        .map_err(app_error_message)?;
    validate_canvas_document_edge_contract(&document).map_err(|err| err.to_string())?;
    Ok(document)
}
