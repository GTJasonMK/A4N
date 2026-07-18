mod assembly;
mod generation;
mod media;
mod node;
mod types;

pub use assembly::{
    canvas_assembly_result_video, canvas_assembly_source_video,
    sync_canvas_assembly_contract_records,
};
pub use generation::{
    mark_canvas_generation_spec_status_record, upsert_canvas_prompt_artifact_record,
    upsert_freeform_canvas_generation_spec_record,
};
pub use media::{
    canvas_media_asset_by_id, canvas_media_source_context, ensure_canvas_image_media_asset_record,
    ensure_canvas_video_media_asset_record,
};
pub use node::{update_canvas_node_artifact_record, update_canvas_node_observation_record};
pub use types::{
    CanvasAssemblyContractSyncInput, CanvasAssemblyContractSyncSummary,
    CanvasAssemblySourceVideoInput, CanvasImageMediaAssetEnsureInput, CanvasMediaSourceContext,
    CanvasNodeArtifactUpdateInput, CanvasNodeObservationUpdateInput,
    CanvasPromptArtifactUpsertInput, CanvasVideoMediaAssetEnsureInput,
    FreeformCanvasGenerationSpecUpsertInput,
};

pub(super) const WORKFLOW_STATUS_COMPLETED: &str = "completed";

#[cfg(test)]
use node::{merge_canvas_artifact_config_patch, merge_canvas_observation_config_patch};

#[cfg(test)]
mod tests;
