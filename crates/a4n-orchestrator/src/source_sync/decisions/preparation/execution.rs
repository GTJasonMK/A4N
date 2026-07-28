use a4n_core::error::AppError;
use a4n_core::models::source_sync::source_sync_run;

use super::context::PreparedSourceSyncDecisionContext;
use super::plan::build_prepared_galgame_scene_regeneration_plan;
use crate::OrchestratorContext;
use crate::source_sync::contracts::{
    RegenerateGalgameScenePayload, SourceSyncDecisionInput, SourceSyncExecutionContext,
};
use crate::source_sync::decisions::payload::parse_decision_payload;
use crate::source_sync::rules::normalize_strategy;
use crate::source_sync::source_sync_db_error;

pub(in crate::source_sync) async fn prepare_decision_execution_context(
    service: &OrchestratorContext,
    run: &source_sync_run::Model,
    decisions: &[SourceSyncDecisionInput],
    execution_context: SourceSyncExecutionContext<'_>,
) -> Result<PreparedSourceSyncDecisionContext, AppError> {
    let mut prepared = PreparedSourceSyncDecisionContext::default();
    let Some(galgame_execution) = execution_context.galgame else {
        return Ok(prepared);
    };
    for decision in decisions {
        if normalize_strategy(&decision.strategy)? != "regenerate_scene" {
            continue;
        }
        let conflict = a4n_db::source_sync::source_sync_conflict_for_decision(
            service.database.connection(),
            run.id,
            decision.conflict_id,
            "prepare_source_sync_decision",
        )
        .await
        .map_err(source_sync_db_error)?;
        let payload: RegenerateGalgameScenePayload =
            parse_decision_payload(&decision.decision_payload_json)?;
        let plan = build_prepared_galgame_scene_regeneration_plan(
            service,
            run,
            &conflict,
            &payload,
            galgame_execution,
        )
        .await?;
        prepared
            .galgame_scene_regeneration_plans
            .insert(conflict.id, plan);
    }
    Ok(prepared)
}
