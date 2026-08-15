use sea_orm::TransactionTrait;

use super::contracts::FinalizeSourceSyncApplyRunInput;
use super::finalization::finalize_source_sync_apply_run_record;
use super::loading::load_source_sync_apply_run_record;
use crate::source_sync::conflicts::apply_unconflicted_source_sync_records;
use crate::source_sync::{SourceSyncDbError, SourceSyncPlanRecord, source_sync_plan_record};

pub async fn apply_unconflicted_source_sync_records_transaction<T>(
    db: &T,
    sync_run_id: i64,
) -> Result<SourceSyncPlanRecord, SourceSyncDbError>
where
    T: TransactionTrait,
{
    let tx = db.begin().await?;
    let run = load_source_sync_apply_run_record(&tx, sync_run_id).await?;
    apply_unconflicted_source_sync_records(&tx, &run).await?;
    let updated = finalize_source_sync_apply_run_record(
        &tx,
        FinalizeSourceSyncApplyRunInput {
            run,
            detach_requested: false,
        },
    )
    .await?;
    let plan = source_sync_plan_record(&tx, updated).await?;
    tx.commit().await?;
    Ok(plan)
}
