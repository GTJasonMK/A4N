use crate::AppState;
use crate::actions::image_generation::generate_canvas_image;
use crate::actions::video_records::assemble_canvas_videos;
use a4n_core::models::canvas::canvas_node;
pub use a4n_orchestrator::canvas::CanvasGraphRecordDocument as CanvasGraphDocument;
use a4n_orchestrator::canvas::validate_canvas_record_document_edge_contract as validate_canvas_document_edge_contract;
use a4n_orchestrator::workflow::{
    WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING,
};
use serde_json::{Value, json};

mod contracts;
mod execution;
mod host_contracts;
mod inputs;
mod media_import;
mod media_nodes;
mod node_lifecycle;
mod output_nodes;
mod persistence;
mod preview;
mod probe_node;
mod records_api;
mod text_nodes;

pub use contracts::{CanvasNodeContract, CanvasPortContract, GetCanvasNodeContractsInput};
pub use execution::{resume_canvas_graph_run, run_canvas_graph, run_canvas_node};
pub use host_contracts::{
    CanvasRunSummary, GetCanvasGraphInput, ImportCanvasMediaInput, ImportedCanvasMedia,
    RunCanvasGraphInput, RunCanvasNodeInput, SaveCanvasGraphInput, SaveCanvasTextAssetInput,
};
pub use media_import::import_canvas_media;
pub use preview::{
    CanvasNodeRequestPreview, PreviewCanvasNodeRequestInput, preview_canvas_node_request,
};
pub use records_api::{
    get_canvas_graph, get_canvas_node_contracts, save_canvas_graph, save_canvas_text_asset,
};

use execution::{finish_canvas_run_failed, record_canvas_node_checkpoint};
use inputs::{
    build_canvas_assemble_input_for_run, build_canvas_image_input_for_run,
    first_canvas_text_asset_id,
};
use node_lifecycle::*;
use output_nodes::*;
use persistence::{
    FreeformCanvasSpecInput, ensure_canvas_image_media_asset, ensure_canvas_video_media_asset,
    mark_canvas_generation_spec_status, sync_canvas_assembly_contract,
    upsert_freeform_canvas_generation_spec,
};

const WORKFLOW_KIND_VIDEO_CANVAS_GRAPH: &str = "video_canvas_graph";
