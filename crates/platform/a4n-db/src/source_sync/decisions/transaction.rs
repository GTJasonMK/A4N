mod application;
mod generation;
mod regeneration;

use a4n_domain::source_sync::SourceSyncResolutionStrategy;
use sea_orm::TransactionTrait;

use self::application::apply_prepared_source_sync_decision_record;
use super::types::PreparedSourceSyncDecisionRecordInput;
use crate::source_sync::{
    FinalizeSourceSyncApplyRunInput, SourceSyncDbError, SourceSyncPlanRecord,
    finalize_source_sync_apply_run_record, load_source_sync_apply_run_record,
    source_sync_plan_record,
};

pub async fn apply_prepared_source_sync_decisions_records_transaction<T>(
    db: &T,
    sync_run_id: i64,
    decisions: Vec<PreparedSourceSyncDecisionRecordInput>,
) -> Result<SourceSyncPlanRecord, SourceSyncDbError>
where
    T: TransactionTrait,
{
    if decisions.is_empty() {
        return Err(SourceSyncDbError::InvalidArgument(
            "source sync prepared decisions 不能为空".to_string(),
        ));
    }
    let detach_requested = decisions
        .iter()
        .any(|decision| decision.strategy() == SourceSyncResolutionStrategy::Detach);
    let tx = db.begin().await?;
    let run = load_source_sync_apply_run_record(&tx, sync_run_id).await?;
    for decision in decisions {
        apply_prepared_source_sync_decision_record(&tx, &run, decision).await?;
    }
    let updated = finalize_source_sync_apply_run_record(
        &tx,
        FinalizeSourceSyncApplyRunInput {
            run,
            detach_requested,
        },
    )
    .await?;
    let plan = source_sync_plan_record(&tx, updated).await?;
    tx.commit().await?;
    Ok(plan)
}
