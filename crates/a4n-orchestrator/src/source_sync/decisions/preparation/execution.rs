use a4n_core::error::AppError;
use a4n_core::models::source_sync::source_sync_run;
use a4n_domain::source_sync::SourceSyncResolutionStrategy;

use super::context::PreparedSourceSyncDecisionContext;
use super::plan::build_prepared_galgame_scene_regeneration_plan;
use crate::OrchestratorContext;
use crate::source_sync::contracts::{
    RegenerateGalgameScenePayload, SourceSyncDecisionInput, SourceSyncExecutionContext,
};
use crate::source_sync::decisions::payload::parse_decision_payload;
use crate::source_sync::source_sync_db_error;

pub(in crate::source_sync) async fn prepare_decision_execution_context(
    service: &OrchestratorContext,
    run: &source_sync_run::Model,
    decisions: &[SourceSyncDecisionInput],
    execution_context: SourceSyncExecutionContext<'_>,
) -> Result<PreparedSourceSyncDecisionContext, AppError> {
    let mut prepared = PreparedSourceSyncDecisionContext::default();
    let galgame_execution = execution_context
        .galgame
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_galgame_execution_context".to_string(),
            operation: SourceSyncResolutionStrategy::RegenerateScene
                .as_str()
                .to_string(),
        })?;
    let regenerate_conflict_ids = decisions
        .iter()
        .filter(|decision| decision.strategy == SourceSyncResolutionStrategy::RegenerateScene)
        .map(|decision| decision.conflict_id)
        .collect::<Vec<_>>();
    let conflicts = a4n_db::source_sync::source_sync_conflicts_for_decisions(
        service.database.connection(),
        run.id,
        &regenerate_conflict_ids,
        "prepare_source_sync_decision",
    )
    .await
    .map_err(source_sync_db_error)?;
    let speaker_profiles = service
        .speaker_profiles_for_root_project(run.root_project_id)
        .await?;
    for decision in decisions {
        if decision.strategy != SourceSyncResolutionStrategy::RegenerateScene {
            continue;
        }
        let conflict =
            conflicts
                .get(&decision.conflict_id)
                .ok_or_else(|| AppError::InvalidState {
                    current: format!(
                        "missing_source_sync_conflict_for_decision_{}",
                        decision.conflict_id
                    ),
                    operation: "prepare_source_sync_decision".to_string(),
                })?;
        let payload: RegenerateGalgameScenePayload =
            parse_decision_payload(&decision.decision_payload_json)?;
        let plan = build_prepared_galgame_scene_regeneration_plan(
            service,
            run,
            &conflict,
            &payload,
            galgame_execution,
            &speaker_profiles,
        )
        .await?;
        prepared
            .galgame_scene_regeneration_plans
            .insert(conflict.id, plan);
    }
    Ok(prepared)
}
