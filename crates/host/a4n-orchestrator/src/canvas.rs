mod audio_import;
mod audio_rules;
mod editing_nodes;
mod errors;
mod graph_execution {
    mod execution;
    mod planning;
    mod preparation;

    #[cfg(test)]
    mod tests;
}
mod image_generation;
mod input_rules;
mod media_import;
mod node_execution;
mod output_nodes;
mod probe_nodes;
mod source_checkpoint;
mod source_text;
mod text_generation;
mod video_generation;
mod workflow_rules;

pub(crate) use a4n_canvas_host_contract::CanvasImageNodeApplicationPort;
pub(crate) use a4n_canvas_host_contract::{
    CanvasLlmTextNodeApplicationPort, CanvasTextGenerationLlmPort,
};
pub(crate) use a4n_canvas_host_contract::{CanvasNodeRequestPreview, CanvasPreviewApplicationPort};
pub(crate) use a4n_canvas_host_contract::{
    CanvasVideoGenerationHostPort, CanvasVideoNodeApplicationPort,
};
pub(crate) use a4n_db::canvas::{
    CanvasGraphRecordDocument, CanvasImageMediaAssetEnsureInput, CanvasVideoMediaAssetEnsureInput,
    validate_canvas_record_document_edge_contract, validate_canvas_record_document_is_dag,
    validate_canvas_record_viewport_json,
};
pub(crate) use editing_nodes::CanvasEditingNodeExecutionInput;
pub(crate) use errors::{canvas_db_error, video_studio_project_import_error};
pub(crate) use image_generation::{
    CanvasImageNodeExecutionInput, build_canvas_image_node_input_for_preview,
};
pub(crate) use input_rules::source_node;
pub(crate) use input_rules::{
    canvas_config_f32, canvas_config_i64, canvas_config_string, canvas_edge_order_index,
    canvas_node_has_persisted_runtime_result, canvas_node_is_operation,
    ensure_canvas_provider_slot_ready, i64_to_i32, incoming_edges, validate_canvas_clip_order,
};
pub(crate) use node_execution::CanvasNodeLifecycle;
pub(crate) use node_execution::CanvasNodeRunHandle;
pub(crate) use output_nodes::{
    CanvasOutputDataType, CanvasOutputNodeExecutionInput, CanvasOutputValue,
};
pub(crate) use probe_nodes::CanvasProbeNodeExecutionInput;
pub(crate) use source_checkpoint::source_node_output_id;
pub(crate) use source_text::{
    first_canvas_text_asset_id, source_canvas_text_asset_id, source_text_value,
};
pub(crate) use text_generation::{
    CanvasTextNodeExecutionInput, build_canvas_llm_text_request_for_preview,
};
pub(crate) use video_generation::{
    CanvasVideoNodeExecutionInput, build_canvas_video_node_input_for_preview,
};
pub(crate) use workflow_rules::{
    WORKFLOW_KIND_VIDEO_CANVAS_GRAPH, canvas_graph_workflow_run_input, canvas_node_step_key,
    canvas_node_step_workflow_run_input, canvas_single_node_workflow_run_input,
};
