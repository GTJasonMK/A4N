use std::collections::{HashMap, HashSet};

use a4n_db_models::models::{
    generation::{
        generation_run, generation_start_request, generation_task_checkpoint, workflow_run,
    },
    task::{task_dependency, task_queue},
};
use a4n_service_protocol::{
    BatchCommitPolicy, BatchSummary, GenerationArtifactRef, GenerationEvent,
    GenerationTaskSnapshot, GenerationTaskStatus, JS_SAFE_INTEGER_I64, StartedEvent,
};
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DbBackend, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, Statement,
    sea_query::{CaseStatement, Expr},
};
use sha2::Digest;

use crate::cleanup_outbox::{
    CLEANUP_AUDIT_TTL, GenerationArtifactUnbindPayload, GenerationCleanupScope,
    GenerationProviderJobCleanupOperation, GenerationProviderJobCleanupPayload,
    GenerationStagingArtifactDeletePayload, insert_generation_artifact_unbind_intent,
    insert_generation_provider_job_cleanup_intent,
    insert_generation_staging_artifact_delete_intent,
};

use super::canonical_json_sha256;
use super::lifecycle_gate::require_generation_owner_gates_active;
use super::tasks::{
    GenerationTaskPersistenceError, PersistedGenerationCheckpoint, canonical_uuid_v4,
    enum_from_storage, enum_storage, load_generation_task,
    persisted_generation_task_from_checkpoint, started_event_from_checkpoint, validate_safe_id,
    validate_safe_sequence,
};

mod cancellation;
mod cleanup;
mod gates;

use cancellation::corrupt_recovery_batch_summary;
pub use cancellation::{
    generation_cancel_targets, generation_interruption_batch_summary,
    persist_generation_cancel_requested, persist_generation_cancel_requested_batch,
};
pub use cleanup::{expire_generation_snapshot_bodies, purge_expired_generation_task_trees};
use cleanup::{
    insert_terminal_cleanup_intents, is_terminal_status, terminal_message, terminal_result_ref,
};
pub use gates::{
    LockedGenerationCompletion, load_generation_corrupt_recovery_state, lock_generation_completion,
    persist_generation_corrupt_terminal, persist_generation_status_changed,
    persist_generation_terminal, persist_locked_generation_completion,
};
use gates::{
    PreparedGenerationTransition, extended_sequence_high_water, persist_transition,
    prepare_transition_from_checkpoint, validate_transition_input,
};

const COMPLETED_SNAPSHOT_TTL: Duration = Duration::hours(24);
const RETAINED_DRAFT_TTL: Duration = Duration::days(7);
const TERMINAL_STATUS_SQL: &str = "('completed', 'failed', 'cancelled', 'interrupted')";
const EXPIRED_GENERATION_ROOTS_SQL: &str = r#"
SELECT root.task_id
FROM generation_task_checkpoint AS root
WHERE root.root_task_id = root.task_id
  AND root.status IN ('completed', 'failed', 'cancelled', 'interrupted')
  AND root.expires_at IS NOT NULL
  AND root.expires_at <= NOW()
  AND EXISTS (
      SELECT 1
      FROM "user" AS owner
      WHERE owner.id = root.requested_by_user_id
        AND owner.lifecycle_gate = 'active'
  )
  AND NOT EXISTS (
      SELECT 1
      FROM novel_project AS project
      WHERE project.id IN (root.project_id, root.root_project_id)
        AND project.lifecycle_gate <> 'active'
  )
ORDER BY root.expires_at, root.task_id
FOR UPDATE SKIP LOCKED
LIMIT $1
"#;
const LOCK_GENERATION_TREE_SQL: &str = r#"
SELECT task_id
FROM generation_task_checkpoint
WHERE root_task_id = $1
ORDER BY task_id
FOR UPDATE
"#;
const LOAD_GENERATION_TREE_SQL: &str = r#"
WITH RECURSIVE task_tree(task_id, parent_task_id, status, expires_at, depth, path) AS (
    SELECT task_id,
           parent_task_id,
           status,
           expires_at,
           0,
           ARRAY[task_id]::text[]
    FROM generation_task_checkpoint
    WHERE task_id = $1
      AND root_task_id = task_id
    UNION ALL
    SELECT child.task_id,
           child.parent_task_id,
           child.status,
           child.expires_at,
           task_tree.depth + 1,
           task_tree.path || child.task_id
    FROM generation_task_checkpoint AS child
    JOIN task_tree ON child.parent_task_id = task_tree.task_id
    WHERE child.root_task_id = $1
      AND NOT child.task_id = ANY(task_tree.path)
)
SELECT task_id, parent_task_id, status, expires_at, depth
FROM task_tree
ORDER BY depth DESC, task_id ASC
"#;
const RECENT_GENERATION_AUDIT_SQL: &str = r#"
SELECT EXISTS (
    SELECT 1
    FROM project_cleanup_outbox AS cleanup
    WHERE cleanup.task_id IN (
        SELECT task_id
        FROM generation_task_checkpoint
        WHERE root_task_id = $1
    )
      AND (
          cleanup.completed_at IS NULL
          OR cleanup.completed_at > $2
      )
) AS has_recent_audit
"#;

#[derive(Debug, Clone)]
pub struct PersistGenerationLifecycleInput {
    pub task_id: String,
    pub attempt: u32,
    pub expected_checkpoint_revision: i64,
    pub expected_status: GenerationTaskStatus,
    pub public_snapshot: GenerationTaskSnapshot,
}

#[derive(Debug, Clone)]
pub struct GenerationCorruptRecoveryState {
    pub task_id: String,
    pub attempt: u32,
    pub expected_checkpoint_revision: i64,
    pub expected_status: GenerationTaskStatus,
    pub reserved_through_sequence: u64,
    pub started: StartedEvent,
    pub batch_summary: Option<BatchSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationCancelTarget {
    pub task_id: String,
    pub attempt: u32,
    pub requested_by_user_id: i64,
    pub status: GenerationTaskStatus,
    pub checkpoint_revision: i64,
    pub root_task_id: String,
}

#[derive(Debug, thiserror::Error)]
pub enum GenerationTaskLifecycleError {
    #[error(transparent)]
    Persistence(#[from] GenerationTaskPersistenceError),
    #[error("generation lifecycle CAS 冲突: status={current_status}, revision={current_revision}")]
    Conflict {
        current_status: String,
        current_revision: i64,
    },
    #[error("generation task 无权访问: {0}")]
    Forbidden(String),
    #[error("generation task attempt 不匹配: expected={expected}, actual={actual}")]
    AttemptMismatch { expected: u32, actual: u32 },
    #[error("generation lifecycle 数据损坏: {0}")]
    Corrupt(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

impl GenerationTaskLifecycleError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Persistence(error) => error.code(),
            Self::Conflict { current_status, .. }
                if matches!(current_status.as_str(), "finalizing" | "completed") =>
            {
                "generation_cancel_too_late"
            }
            Self::Conflict { current_status, .. }
                if matches!(
                    current_status.as_str(),
                    "failed" | "cancelled" | "interrupted"
                ) =>
            {
                "generation_terminal_already_set"
            }
            Self::Conflict { .. } | Self::Database(_) => "generation_persist_failed",
            Self::Forbidden(_) => "generation_task_forbidden",
            Self::AttemptMismatch { .. } => "generation_attempt_mismatch",
            Self::Corrupt(_) => "generation_checkpoint_corrupt",
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_transitions_and_ttls_are_fixed() {
        assert!(gates::valid_status_changed_transition(
            GenerationTaskStatus::Reserved,
            GenerationTaskStatus::Running
        ));
        assert!(gates::valid_status_changed_transition(
            GenerationTaskStatus::Running,
            GenerationTaskStatus::Finalizing
        ));
        assert!(!gates::valid_status_changed_transition(
            GenerationTaskStatus::Running,
            GenerationTaskStatus::Completed
        ));
        assert!(
            gates::validate_terminal_transition(
                GenerationTaskStatus::Finalizing,
                GenerationTaskStatus::Completed
            )
            .is_ok()
        );
        assert!(
            gates::validate_terminal_transition(
                GenerationTaskStatus::Running,
                GenerationTaskStatus::Cancelled
            )
            .is_err()
        );
        assert_eq!(COMPLETED_SNAPSHOT_TTL, Duration::hours(24));
        assert_eq!(RETAINED_DRAFT_TTL, Duration::days(7));
        assert_eq!(gates::extended_sequence_high_water(257, 258).unwrap(), 513);
    }
}
