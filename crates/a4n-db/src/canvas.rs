mod artifacts;
mod graph_record;
mod references;
mod transfer;

pub use artifacts::{
    CanvasAssemblyContractSyncInput, CanvasAssemblyContractSyncSummary,
    CanvasAssemblySourceVideoInput, CanvasImageMediaAssetEnsureInput,
    CanvasNodeArtifactUpdateInput, CanvasNodeObservationUpdateInput,
    CanvasPromptArtifactUpsertInput, CanvasVideoMediaAssetEnsureInput,
    FreeformCanvasGenerationSpecUpsertInput, canvas_media_asset_by_id,
    ensure_canvas_image_media_asset_record, ensure_canvas_video_media_asset_record,
    mark_canvas_generation_spec_status_record, sync_canvas_assembly_contract_records,
    update_canvas_node_artifact_record, update_canvas_node_observation_record,
    upsert_canvas_prompt_artifact_record, upsert_freeform_canvas_generation_spec_record,
};
pub use graph_record::{
    CanvasEdgeRecordInput, CanvasGraphRecordDocument, CanvasGraphRecordInput,
    CanvasNodeRecordInput, CanvasTextAssetRecordInput, get_or_create_canvas_graph_record,
    save_canvas_graph_record, upsert_canvas_text_asset_record,
    validate_canvas_record_document_edge_contract, validate_canvas_record_document_is_dag,
    validate_canvas_record_viewport_json,
};
pub use references::{
    MissingCanvasRef, clear_reset_generated_ref_payload, reset_missing_canvas_ref_for_node,
};
pub use transfer::{
    VideoStudioProjectExportRows, VideoStudioProjectImportError,
    VideoStudioProjectImportMediaError, VideoStudioProjectImportMediaFileWriter,
    VideoStudioProjectImportRows, import_video_studio_project_record,
    video_studio_project_export_rows,
};

pub(crate) use references::{
    generated_video_ref_project_ids, mark_canvas_generated_image_refs_missing,
    mark_canvas_generated_video_refs_missing, mark_freeform_canvas_generated_refs_missing,
};

use sea_orm::DbErr;

const PROJECT_KIND_VIDEO: &str = "video";

#[derive(Debug, thiserror::Error)]
pub enum CanvasDbError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Forbidden(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}
