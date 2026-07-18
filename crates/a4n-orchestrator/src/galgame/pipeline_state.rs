mod restore;
mod stage;

pub use restore::{parse_galgame_start_stage, should_restore_galgame_stage_checkpoint};
pub use stage::{
    GALGAME_PIPELINE_TOTAL_STEPS, GalgamePipelineStartStage, galgame_pipeline_stage_keys,
};

#[cfg(test)]
mod tests;
