pub use a4n_api_contract::galgame::{
    BindGalgameMediaInput, DeleteGalgameLineInput, GalgameAssetGenerationSummary, GalgameAssetPlan,
    GalgameChoice, GalgameLine, GalgameMediaBindingSummary, GalgamePipelineStatusSummary,
    GalgameProjectInput, GalgameScene, GalgameScriptDetail, ImportGalgameAudioInput,
    InsertGalgameLineInput, PreviewGalgamePipelineInput, ReorderGalgameLinesInput,
    RunGalgameAssetGenerationInput, StartGalgamePipelineInput, UnbindGalgameMediaInput,
    UpdateGalgameChoicesInput, UpdateGalgameLineInput, UpdateGalgameSceneInput,
};
pub use a4n_domain::galgame::GALGAME_PIPELINE_TOTAL_STEPS;
mod asset_generation;
pub use asset_generation::{plan_galgame_assets, run_galgame_asset_generation};
mod media;
pub use media::{bind_galgame_media, import_galgame_audio, unbind_galgame_media};
mod pipeline;
pub use pipeline::{
    cancel_galgame_pipeline, get_galgame_pipeline_status, preview_galgame_pipeline_request,
    start_galgame_pipeline, start_galgame_pipeline_with_progress,
};
mod webgal_export;
pub use webgal_export::export_galgame_webgal;

mod mutations;
mod queries;

pub use mutations::{
    delete_galgame_line, insert_galgame_line, reorder_galgame_lines, update_galgame_choices,
    update_galgame_line, update_galgame_scene,
};
pub use queries::get_galgame_script;
