use super::{CheckImportDedupInput, DedupResult};
use crate::AppState;
use a4n_error::AppError;

pub async fn check_import_dedup(
    state: &AppState,
    user_id: i64,
    input: CheckImportDedupInput,
) -> Result<DedupResult, AppError> {
    state
        .import_review_read_application()
        .dedup(user_id, input)
        .await
}
