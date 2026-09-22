use a4n_domain::galgame::GalgamePipelineStartStage;

use super::should_restore_galgame_stage_checkpoint;

#[test]
fn clears_downstream_checkpoints_when_restarting_from_staging() {
    let keys = GalgamePipelineStartStage::CharacterStaging.checkpoint_keys_to_clear();

    assert!(!keys.contains(&"scenes"));
    assert!(!keys.contains(&"lines_draft"));
    assert!(keys.contains(&"lines_staged"));
    assert!(keys.contains(&"visual_plan"));
    assert!(keys.contains(&"asset_generation"));
}

#[test]
fn restores_only_upstream_checkpoints_when_start_stage_is_set() {
    assert!(should_restore_galgame_stage_checkpoint(
        false,
        Some(GalgamePipelineStartStage::VisualPlanning),
        GalgamePipelineStartStage::CharacterStaging.order(),
    ));
    assert!(!should_restore_galgame_stage_checkpoint(
        false,
        Some(GalgamePipelineStartStage::VisualPlanning),
        GalgamePipelineStartStage::VisualPlanning.order(),
    ));
    assert!(should_restore_galgame_stage_checkpoint(true, None, 1));
}
