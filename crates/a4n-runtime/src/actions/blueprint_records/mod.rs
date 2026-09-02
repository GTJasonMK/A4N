mod generation;
mod manual;
mod refinement;
mod versions;
mod world_setting;

pub use a4n_api_contract::blueprint::{
    BlueprintDiffSummary, BlueprintMutationResult, DiffBlueprintVersionsInput,
    ListBlueprintVersionsInput, PreviewBlueprintGenerationInput, PreviewBlueprintRefinementInput,
    RestoreBlueprintVersionInput, SaveBlueprintInput, UpdateWorldSettingInput,
};
pub use generation::{
    get_blueprint, preview_generate_blueprint_request, resume_blueprint_generation_workflow,
    start_blueprint_generation_typed,
};
pub(crate) use manual::blueprint_material_sync_warnings;
pub use manual::{save_blueprint, sync_blueprint_materials};
pub use refinement::{
    preview_refine_blueprint_request, resume_blueprint_refinement_workflow,
    start_blueprint_refinement_typed,
};
pub use versions::{diff_blueprint_versions, list_blueprint_versions, restore_blueprint_version};
pub use world_setting::update_world_setting;

#[cfg(test)]
mod tests;
