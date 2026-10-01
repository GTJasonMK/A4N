mod artifacts;
mod graph_record;
mod references;
mod transfer;

pub(crate) use transfer::{
    DeferredVideoStudioImportGenerationSpecTarget, VideoStudioProjectImportIdMaps,
    import_video_studio_canvas_graphs, import_video_studio_canvas_text_assets,
    import_video_studio_generation_specs, update_imported_canvas_node_refs,
    update_imported_generation_spec_metadata, update_imported_generation_spec_targets,
    update_imported_media_metadata,
};

pub use artifacts::{
    CanvasAssemblyContractSyncSummary, CanvasImageMediaAssetEnsureInput,
    CanvasMediaAssetProbeRecord, CanvasNodeArtifactUpdateInput, CanvasNodeObservationUpdateInput,
    CanvasVideoMediaAssetEnsureInput, canvas_media_asset_by_id,
    ensure_canvas_image_media_asset_record, ensure_canvas_video_media_asset_record,
    mark_canvas_generation_spec_status_record, sync_canvas_assembly_contract_records,
    update_canvas_node_artifact_record, update_canvas_node_observation_record,
    upsert_canvas_prompt_artifact_record, upsert_freeform_canvas_generation_spec_record,
};
pub(crate) use graph_record::records::{
    canvas_edge_record, canvas_graph_record, canvas_node_record, canvas_text_asset_record,
};
pub use graph_record::{
    CanvasEdgeRecordInput, CanvasGraphRecordDocument, CanvasGraphRecordInput,
    CanvasGraphRecordSaveResult, CanvasNodeRecordInput, get_or_create_canvas_graph_record,
    save_canvas_graph_record, upsert_canvas_text_asset_record,
    validate_canvas_record_document_edge_contract, validate_canvas_record_document_is_dag,
    validate_canvas_record_viewport_json,
};
pub use transfer::{
    VideoStudioProjectExportRows, VideoStudioProjectImportError,
    VideoStudioProjectImportMediaError, VideoStudioProjectImportMediaFileWriter,
    VideoStudioProjectImportRows, import_video_studio_project_record,
    video_studio_project_export_rows,
};

pub(crate) use references::{
    generated_video_ref_project_ids, mark_canvas_generated_image_refs_missing,
    mark_canvas_generated_video_refs_missing_for_projects,
    mark_canvas_generated_video_refs_missing_for_projects_and_videos,
    mark_freeform_canvas_generated_refs_missing,
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
    #[error(
        "画布 version 冲突: graph_id={graph_id}, expected_version={expected_version}, current_version={current_version}"
    )]
    VersionConflict {
        graph_id: i64,
        expected_version: i32,
        current_version: i32,
    },
    #[error(transparent)]
    Database(#[from] DbErr),
}
