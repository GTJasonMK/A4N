use a4n_error::AppError;
use serde_json::json;

use crate::galgame::{
    GalgameSceneRegenerationInput, GalgameSceneRegenerationPlan, build_galgame_script_persist_plan,
};

use super::contracts::{GalgameSceneRegenerationStageOutputs, PreparedGalgameSceneRegeneration};

pub(super) fn finalize_galgame_scene_regeneration(
    input: &GalgameSceneRegenerationInput,
    prepared: &PreparedGalgameSceneRegeneration,
    outputs: GalgameSceneRegenerationStageOutputs,
) -> Result<GalgameSceneRegenerationPlan, AppError> {
    let persist_plan = build_galgame_script_persist_plan(
        &prepared.shooting_script_projection,
        &outputs.staging,
        &outputs.visual_plan,
        &outputs.branch_plan,
        &prepared.persist_source_spans,
    )?;
    let mut scene =
        persist_plan
            .scenes
            .into_iter()
            .next()
            .ok_or_else(|| AppError::InvalidState {
                current: "galgame_source_sync_regeneration_empty_plan".to_string(),
                operation: "build_source_sync_scene_regeneration".to_string(),
            })?;
    scene.scene_index = input.scene_index;

    Ok(GalgameSceneRegenerationPlan {
        scene,
        stage_outputs_json: json!({
            "shooting_script_projection": prepared.scenes_json,
            "script_adaptation": outputs.lines_draft_json,
            "character_staging": outputs.lines_staged_json,
            "visual_planning": outputs.visual_plan_json,
            "branch_design": outputs.branch_plan_json,
        }),
    })
}
