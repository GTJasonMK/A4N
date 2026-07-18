use crate::AppState;
use crate::actions::app_error_message;
use crate::actions::video_generation::GeneratedVideoSummary;
use a4n_orchestrator::canvas::{CanvasAssemblyContractSyncInput, CanvasAssemblySourceVideoInput};

pub(in crate::actions::canvas_workflow) async fn sync_canvas_assembly_contract(
    state: &AppState,
    project_id: i64,
    graph_id: i64,
    node_id: i64,
    assembly_video: &GeneratedVideoSummary,
    source_video_ids: &[i64],
    generation_spec_id: Option<i64>,
) -> Result<i64, String> {
    let source_videos = source_video_ids
        .iter()
        .map(|video_id| CanvasAssemblySourceVideoInput {
            video_id: *video_id,
        })
        .collect::<Vec<_>>();
    let contract = state
        .orchestrator
        .sync_canvas_assembly_contract_records(CanvasAssemblyContractSyncInput {
            project_id,
            graph_id,
            node_id,
            assembly_video_id: assembly_video.id,
            source_videos,
            generation_spec_id,
        })
        .await
        .map_err(app_error_message)?;

    Ok(contract.preview_media_asset_id)
}
