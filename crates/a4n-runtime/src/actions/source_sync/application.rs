use super::{ApplySourceSyncInput, SourceSyncPlanSummary};
use crate::AppState;
use a4n_error::AppError;

pub async fn apply_source_sync_decisions(
    state: &AppState,
    user_id: i64,
    input: ApplySourceSyncInput,
) -> Result<SourceSyncPlanSummary, AppError> {
    state
        .source_sync_mutation_application()
        .apply(user_id, input, state)
        .await
}
