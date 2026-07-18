use a4n_core::error::AppError;

use super::{
    GalgamePipelineStartStage, galgame_pipeline_stage_keys, parse_galgame_start_stage,
    should_restore_galgame_stage_checkpoint,
};

#[test]
fn parses_all_galgame_pipeline_start_stages() {
    for key in galgame_pipeline_stage_keys() {
        assert!(
            GalgamePipelineStartStage::parse(key).is_some(),
            "stage should parse: {key}"
        );
    }
}

#[test]
fn rejects_unknown_galgame_pipeline_start_stage() {
    let err = parse_galgame_start_stage(Some("unknown_stage")).expect_err("unknown stage");

    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("unknown_stage"));
    assert!(err.to_string().contains("shooting_script_projection"));
}

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
