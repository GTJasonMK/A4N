use super::super::access::verify_optimize_session_access;
use crate::AppState;

const PATCH_SET_STATUS_PENDING: &str = "pending";
const PATCH_SET_STATUS_APPLIED: &str = "applied";
const PATCH_SET_STATUS_REJECTED: &str = "rejected";
const PATCH_DECISION_ACCEPTED: &str = "accepted";
const PATCH_DECISION_REJECTED: &str = "rejected";
const PATCH_APPLY_STATUS_PENDING: &str = "pending";
const PATCH_APPLY_STATUS_APPLIED: &str = "applied";

pub async fn reject_pending_optimize_patch_sets(
    state: &AppState,
    user_id: i64,
    session_id: i64,
) -> Result<(), String> {
    verify_optimize_session_access(state, user_id, session_id).await?;
    reject_pending_patch_sets(state, session_id).await
}

async fn reject_pending_patch_sets(state: &AppState, session_id: i64) -> Result<(), String> {
    let patch_sets = state
        .orchestrator
        .pending_optimization_patch_sets(session_id, PATCH_SET_STATUS_PENDING)
        .await
        .map_err(|err| err.to_string())?;
    for patch_set in patch_sets {
        state
            .orchestrator
            .update_optimization_patch_set_status(
                patch_set,
                PATCH_SET_STATUS_REJECTED,
                PATCH_DECISION_REJECTED,
                PATCH_APPLY_STATUS_PENDING,
            )
            .await
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}

pub(super) async fn mark_matching_patch_sets_applied(
    state: &AppState,
    session_id: i64,
    base_content_hash: &str,
) -> Result<(), String> {
    let patch_sets = state
        .orchestrator
        .matching_pending_optimization_patch_sets(
            session_id,
            base_content_hash,
            PATCH_SET_STATUS_PENDING,
        )
        .await
        .map_err(|err| err.to_string())?;
    for patch_set in patch_sets {
        state
            .orchestrator
            .update_optimization_patch_set_status(
                patch_set,
                PATCH_SET_STATUS_APPLIED,
                PATCH_DECISION_ACCEPTED,
                PATCH_APPLY_STATUS_APPLIED,
            )
            .await
            .map_err(|err| err.to_string())?;
    }
    Ok(())
}
