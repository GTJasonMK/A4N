use crate::AppState;
use crate::actions::app_error_message;
use a4n_orchestrator::canvas::{CanvasNodeArtifactUpdateInput, CanvasNodeObservationUpdateInput};
use serde_json::Value;

pub(in crate::actions::canvas_workflow) async fn update_canvas_node_artifact(
    state: &AppState,
    node_id: i64,
    ref_type: &str,
    ref_id: i64,
    config_patch: Value,
) -> Result<(), String> {
    state
        .orchestrator
        .update_canvas_node_artifact_record(CanvasNodeArtifactUpdateInput {
            node_id,
            ref_type: ref_type.to_string(),
            ref_id,
            config_patch,
        })
        .await
        .map_err(app_error_message)
}

pub(in crate::actions::canvas_workflow) async fn update_canvas_node_observation(
    state: &AppState,
    node_id: i64,
    config_patch: Value,
) -> Result<(), String> {
    state
        .orchestrator
        .update_canvas_node_observation_record(CanvasNodeObservationUpdateInput {
            node_id,
            config_patch,
        })
        .await
        .map_err(app_error_message)
}
