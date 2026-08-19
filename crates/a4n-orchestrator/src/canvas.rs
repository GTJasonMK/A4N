mod audio_import;
mod audio_rules;
mod editing_nodes;
pub mod errors;
mod graph_execution;
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

pub use a4n_db::canvas::{
    CanvasAssemblyContractSyncInput, CanvasAssemblyContractSyncSummary,
    CanvasAssemblySourceVideoInput, CanvasGraphRecordDocument, CanvasGraphRecordInput,
    CanvasImageMediaAssetEnsureInput, CanvasPromptArtifactUpsertInput, CanvasTextAssetRecordInput,
    CanvasVideoMediaAssetEnsureInput, FreeformCanvasGenerationSpecUpsertInput, MissingCanvasRef,
    clear_reset_generated_ref_payload, reset_missing_canvas_ref_for_node,
    validate_canvas_record_document_edge_contract, validate_canvas_record_document_is_dag,
    validate_canvas_record_viewport_json,
};
pub use audio_import::CanvasAudioImportHostInput;
pub use editing_nodes::{
    CanvasAssembleVideoNodeInput, CanvasAssembleVideosHostInput, CanvasEditingNodeExecutionInput,
    CanvasEditingNodeHostPort, CanvasEditingNodeInput, CanvasExtractFrameHostInput,
    CanvasExtractFrameNodeInput, CanvasGeneratedImageArtifact, CanvasGeneratedVideoArtifact,
    CanvasTrimVideoHostInput, CanvasTrimVideoNodeInput,
    build_canvas_editing_node_input_for_preview,
};
pub use graph_execution::{
    CanvasGraphExecutionHostPort, CanvasGraphNodeExecutionInput, CanvasGraphNodeExecutionKind,
    CanvasGraphResumePreparationInput, CanvasGraphRunPreparationInput,
    CanvasSingleNodeRunPreparationInput, PreparedCanvasGraphRun,
};
pub use image_generation::{
    CanvasImageNodeExecutionInput, CanvasImageNodeInput, build_canvas_image_node_input_for_preview,
};
pub(crate) use input_rules::source_node;
pub use input_rules::{
    canvas_config_f32, canvas_config_i64, canvas_config_string, canvas_edge_order_index,
    canvas_node_is_operation, i64_to_i32, incoming_edges, validate_canvas_clip_order,
};
pub use media_import::{
    CanvasImageImportApplicationResult, CanvasImageImportHostInput, CanvasImageImportHostPort,
    CanvasVideoImportApplicationResult, CanvasVideoImportHostInput, CanvasVideoImportHostPort,
};
pub(crate) use node_execution::CanvasNodeLifecycle;
pub use node_execution::{CanvasNodeRunHandle, CanvasNodeRunSummary};
pub use output_nodes::{CanvasOutputDataType, CanvasOutputNodeExecutionInput, CanvasOutputValue};
pub use probe_nodes::CanvasProbeNodeExecutionInput;
pub(crate) use source_checkpoint::source_node_output_id;
pub(crate) use source_text::{
    first_canvas_text_asset_id, source_canvas_text_asset_id, source_text_value,
};
pub use text_generation::{
    CanvasTextGenerationLlmCall, CanvasTextGenerationLlmPort, CanvasTextNodeExecutionInput,
    CanvasTextNodeRunHandle, CanvasTextNodeRunSummary, build_canvas_llm_text_request_for_preview,
};
pub use video_generation::{
    CanvasVideoFileInput, CanvasVideoGenerationHostPort, CanvasVideoImageInput,
    CanvasVideoNodeExecutionInput, CanvasVideoNodeInput, CanvasVideoPersistedFile,
    CanvasVideoProviderInput, CanvasVideoRequestPreview, CanvasVideoResolvedImages,
    GenerateCanvasVideoInput, build_canvas_video_node_input_for_preview,
};
pub use workflow_rules::{
    WORKFLOW_KIND_VIDEO_CANVAS_GRAPH, canvas_graph_workflow_business_key,
    canvas_graph_workflow_run_input, canvas_node_step_key, canvas_node_step_workflow_business_key,
    canvas_node_step_workflow_run_input, canvas_single_node_workflow_business_key,
    canvas_single_node_workflow_run_input,
};
