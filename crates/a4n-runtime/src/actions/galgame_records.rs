pub use a4n_orchestrator::galgame::GalgameMediaBindingResult;
pub use a4n_orchestrator::galgame::{
    GALGAME_PIPELINE_TOTAL_STEPS, GalgameAssetPlan, GalgamePipelineProgressUpdate,
    GalgameScriptDetail, InsertGalgameLineInput, ReorderGalgameLinesInput,
    UpdateGalgameChoicesInput, UpdateGalgameLineInput, UpdateGalgameSceneInput,
};

mod asset_generation;
pub use asset_generation::{
    GalgameAssetGenerationItem, GalgameAssetGenerationSummary, RunGalgameAssetGenerationInput,
    plan_galgame_assets, run_galgame_asset_generation,
};
mod media;
pub use media::{
    BindGalgameMediaInput, ImportGalgameAudioInput, UnbindGalgameMediaInput, bind_galgame_media,
    import_galgame_audio, unbind_galgame_media,
};
mod pipeline;
pub use pipeline::{
    GalgamePipelineStatusSummary, GalgameWorkflowCheckpointSummary, GalgameWorkflowRunSummary,
    StartGalgamePipelineInput, cancel_galgame_pipeline, get_galgame_pipeline_status,
    preview_galgame_pipeline_request, start_galgame_pipeline, start_galgame_pipeline_with_progress,
};
mod webgal_export;
pub use webgal_export::export_galgame_webgal;

mod mutations;
mod queries;

pub use mutations::{
    DeleteGalgameLineInput, delete_galgame_line, insert_galgame_line, reorder_galgame_lines,
    update_galgame_choices, update_galgame_line, update_galgame_scene,
};
pub use queries::{GalgameProjectInput, get_galgame_script};

pub(super) use queries::{load_galgame_script_detail, project_record_by_id};
