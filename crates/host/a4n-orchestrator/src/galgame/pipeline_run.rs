mod checkpoints;
mod contracts;
mod llm;
mod pipeline;
mod progress;
mod regeneration;
mod validation;

pub use contracts::{
    GalgamePipelineRunOptions, GalgameSceneRegenerationInput, GalgameSceneRegenerationPlan,
};
pub use pipeline::galgame_shooting_script_projection;
pub(crate) use validation::validate_source_sync_scene_regeneration_stage_outputs;

#[cfg(test)]
mod tests;
