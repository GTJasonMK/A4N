use std::collections::HashMap;

use a4n_core::error::AppError;
use serde_json::Value;

use crate::galgame::GalgameSceneRegenerationPlan;
use crate::source_sync::contracts::RegenerateGalgameScenePayload;
use crate::source_sync::decisions::payload::source_sync_galgame_scene_write_record;

#[derive(Debug, Clone, Default)]
pub(in crate::source_sync) struct PreparedSourceSyncDecisionContext {
    pub(in crate::source_sync) galgame_scene_regeneration_plans:
        HashMap<i64, GalgameSceneRegenerationPlan>,
}

pub(in crate::source_sync) fn require_galgame_scene_regeneration_plan(
    prepared_context: &PreparedSourceSyncDecisionContext,
    conflict_id: i64,
) -> Result<&GalgameSceneRegenerationPlan, AppError> {
    prepared_context
        .galgame_scene_regeneration_plans
        .get(&conflict_id)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("missing_galgame_scene_regeneration_plan_for_conflict_{conflict_id}"),
            operation: "regenerate_scene".to_string(),
        })
}

pub(in crate::source_sync) fn prepared_galgame_scene_regeneration_decision(
    prepared_context: &PreparedSourceSyncDecisionContext,
    conflict_id: i64,
    payload: RegenerateGalgameScenePayload,
    decision_payload_json: Value,
) -> Result<a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput, AppError> {
    let plan = require_galgame_scene_regeneration_plan(prepared_context, conflict_id)?;
    Ok(
        a4n_db::source_sync::PreparedSourceSyncDecisionRecordInput::RegenerateScene {
            conflict_id,
            galgame_scene_id: payload.galgame_scene_id,
            reason: payload.reason,
            expected_scene_index: plan.scene.scene_index,
            expected_source_snapshot_chapter_id: plan.scene.source_snapshot_chapter_id,
            generated_scene: source_sync_galgame_scene_write_record(plan.scene.clone()),
            stage_outputs_json: plan.stage_outputs_json.clone(),
            decision_payload_json,
        },
    )
}
