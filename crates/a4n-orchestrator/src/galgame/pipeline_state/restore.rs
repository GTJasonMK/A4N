use a4n_domain::galgame::GalgamePipelineStartStage;

pub fn should_restore_galgame_stage_checkpoint(
    resume_from_checkpoint: bool,
    start_from_stage: Option<GalgamePipelineStartStage>,
    stage_order: i32,
) -> bool {
    if let Some(start_stage) = start_from_stage {
        return stage_order < start_stage.order();
    }
    resume_from_checkpoint
}
