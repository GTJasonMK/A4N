use super::contracts::SourceSyncDecisionInput;
use a4n_core::error::AppError;

pub const SYNC_RUN_MODE_DRY_RUN: &str = "dry_run";
pub const SYNC_RUN_MODE_APPLY: &str = "apply";
pub const SYNC_RUN_STATUS_PLANNED: &str = "planned";
pub const SYNC_RUN_STATUS_APPLYING: &str = "applying";
pub const SYNC_RUN_STATUS_COMPLETED: &str = "completed";
pub const CONFLICT_STATUS_PENDING: &str = "pending";
pub const CONFLICT_STATUS_RESOLVED: &str = "resolved";
pub const CONFLICT_STATUS_IGNORED: &str = "ignored";

const SYNC_RUN_MODES: &[&str] = &[SYNC_RUN_MODE_DRY_RUN, SYNC_RUN_MODE_APPLY];
pub const SYNC_RUN_STATUSES: &[&str] = &[
    SYNC_RUN_STATUS_PLANNED,
    SYNC_RUN_STATUS_APPLYING,
    SYNC_RUN_STATUS_COMPLETED,
];
pub const SOURCE_CHANGE_KINDS: &[&str] =
    &["added", "updated", "reordered", "deleted", "moved", "split"];
pub const LOCAL_STATE_KINDS: &[&str] = &[
    "user_edited",
    "generated",
    "imported",
    "assembled",
    "exported",
    "clean",
    "draft",
];
pub const CONFLICT_SEVERITIES: &[&str] = &["blocking", "warning", "info"];
pub const CONFLICT_STATUSES: &[&str] = &[
    CONFLICT_STATUS_PENDING,
    CONFLICT_STATUS_RESOLVED,
    CONFLICT_STATUS_IGNORED,
];
pub const RESOLUTION_STRATEGIES: &[&str] = &[
    "keep_local",
    "adopt_source",
    "rebuild_prompt",
    "rebind_anchor",
    "regenerate_scene",
    "create_replacement",
    "manual",
    "detach",
    "delete_local",
];
pub const DECISION_ACTORS: &[&str] = &["user", "system"];

pub fn is_valid_sync_run_mode(value: &str) -> bool {
    SYNC_RUN_MODES.contains(&value)
}

pub fn is_valid_sync_run_status(value: &str) -> bool {
    SYNC_RUN_STATUSES.contains(&value)
}

pub fn is_valid_source_change_kind(value: &str) -> bool {
    SOURCE_CHANGE_KINDS.contains(&value)
}

pub fn is_valid_local_state_kind(value: &str) -> bool {
    LOCAL_STATE_KINDS.contains(&value)
}

pub fn is_valid_conflict_severity(value: &str) -> bool {
    CONFLICT_SEVERITIES.contains(&value)
}

pub fn is_valid_conflict_status(value: &str) -> bool {
    CONFLICT_STATUSES.contains(&value)
}

pub fn is_valid_resolution_strategy(value: &str) -> bool {
    RESOLUTION_STRATEGIES.contains(&value)
}

pub fn is_valid_decision_actor(value: &str) -> bool {
    DECISION_ACTORS.contains(&value)
}

pub(super) fn source_sync_decisions_require_galgame_preparation(
    decisions: &[SourceSyncDecisionInput],
) -> Result<bool, AppError> {
    for decision in decisions {
        if normalize_strategy(&decision.strategy)? == "regenerate_scene" {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn normalize_strategy(value: &str) -> Result<&str, AppError> {
    let value = value.trim();
    if is_valid_resolution_strategy(value) {
        Ok(value)
    } else {
        Err(AppError::InvalidArgument(format!(
            "同步冲突解决策略无效: {value}"
        )))
    }
}
