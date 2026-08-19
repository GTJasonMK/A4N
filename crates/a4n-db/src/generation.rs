use a4n_core::models::{
    generation::{
        generation_run, generation_run_target, generation_spec, generation_spec_entity_ref,
        generation_spec_media_ref, generation_spec_text_anchor, llm_call_trace, prompt_artifact,
    },
    media::media_provenance,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait, sea_query::Expr,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const RUN_STATUS_RUNNING: &str = "running";
const RUN_STATUS_COMPLETED: &str = "completed";
const RUN_STATUS_FAILED: &str = "failed";
const RUN_STATUS_CANCELLED: &str = "cancelled";
const RUN_STATUS_INTERRUPTED: &str = "interrupted";

mod validation;
pub(in crate::generation) use validation::verify_canonical_json_sha256;
pub use validation::{canonical_json_bytes, canonical_json_sha256, canonical_json_text};
use validation::{
    normalize_one_of, normalize_optional_owned_text, required_owned_text, required_text,
    validate_positive_id,
};

mod types;
pub(crate) use types::append_generation_artifact_target;
use types::{AttachGenerationRunTargetInput, CreateGenerationRunInput, INTERNAL_PROMPT_TOKENS};
pub use types::{
    AttachGenerationRunsToWorkflowInput, GENERATION_ENTITY_KINDS, GENERATION_ENTITY_ROLES,
    GENERATION_MEDIA_REF_ROLES, GENERATION_PROMPT_KIND_CANVAS_PROMPT,
    GENERATION_PROMPT_KIND_CANVAS_SHOT_SCRIPT, GENERATION_PROMPT_KIND_VIDEO_PROMPT,
    GENERATION_PROMPT_KINDS, GENERATION_SPEC_STATUSES,
    GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE, GENERATION_SPEC_TARGET_ENTITY_KINDS,
    GENERATION_SPEC_TARGET_KIND_CANVAS_ASSEMBLY, GENERATION_SPEC_TARGET_KIND_CANVAS_IMAGE,
    GENERATION_SPEC_TARGET_KIND_CANVAS_PROMPT, GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO,
    GENERATION_SPEC_TARGET_KINDS, GENERATION_TEXT_ANCHOR_ROLES, GenerationDbError,
    GenerationRunCheckpointContext, GenerationRunTraceTarget, LlmCallUsageTraceInput,
    PromptArtifactInsertInput, RecordCompletedGenerationRunInput,
    RecordCompletedGenerationRunWithCheckpointInput, RecordLlmCallTraceInput,
    RecordStartedGenerationRunInput, SPEC_STATUS_DRAFT, generation_checkpoint_fingerprint_sha256,
    is_valid_generation_entity_kind, is_valid_generation_entity_role,
    is_valid_generation_media_ref_role, is_valid_generation_spec_status,
    is_valid_generation_spec_target_entity_kind, is_valid_generation_spec_target_kind,
    is_valid_generation_text_anchor_role,
};
pub(in crate::generation) use types::{
    GENERATION_PROMPT_KIND_IMAGE_PROMPT, VIDEO_PROMPT_REQUIRED_SECTIONS,
};

mod runs;
#[cfg(test)]
pub(in crate::generation) use runs::usage_trace_fields;
pub use runs::{
    attach_runs_to_workflow_for_target, cancel_runs_for_target, complete_runs_for_target,
    delete_generation_run_targets_for, fail_runs_for_target,
    generation_run_exists_for_target_output, record_completed_generation_run,
    record_completed_generation_run_in_transaction,
    record_completed_run_with_checkpoint_transaction, record_llm_call_trace,
};
pub(crate) use runs::{
    interrupt_runs_for_target, record_completed_generation_run_with_checkpoint_in,
    record_started_generation_run,
};

mod refs;
pub(crate) use refs::{
    insert_generation_entity_ref, insert_generation_media_ref, insert_generation_text_anchor_ref,
    validate_entity_ref, validate_required_prompt_refs,
};
#[cfg(test)]
pub(in crate::generation) use refs::{normalize_entity_kind, normalize_media_ref_role};

mod prompts;
#[cfg(test)]
pub(in crate::generation) use prompts::validate_prompt_artifact_text;
pub(crate) use prompts::{
    insert_prompt_artifact, normalize_video_prompt_kind, prompt_artifact_by_id,
};
pub use prompts::{is_valid_generation_prompt_kind, render_prompt_text};

mod tasks;
pub use tasks::{
    GENERATION_SEQUENCE_LEASE_SIZE, GenerationPublicSnapshot, GenerationSequenceLease,
    GenerationTaskCheckpointPage, GenerationTaskListCursor, GenerationTaskPersistenceError,
    GenerationTaskReservation, PersistGenerationCheckpointInput, PersistedGenerationCheckpoint,
    PersistedGenerationTask, ReserveGenerationTaskInput, extend_generation_sequence_lease,
    generation_public_snapshot_for_summary, list_active_generation_task_checkpoints,
    list_generation_task_history_checkpoints, load_generation_task,
    load_generation_task_checkpoint, persist_generation_checkpoint,
    persisted_generation_task_from_checkpoint, reserve_generation_task,
    started_event_from_checkpoint,
};

mod task_lifecycle;
pub use task_lifecycle::{
    GenerationCancelTarget, GenerationCorruptRecoveryState, GenerationTaskLifecycleError,
    LockedGenerationCompletion, PersistGenerationLifecycleInput, expire_generation_snapshot_bodies,
    generation_cancel_targets, generation_interruption_batch_summary,
    load_generation_corrupt_recovery_state, lock_generation_completion,
    persist_generation_cancel_requested, persist_generation_cancel_requested_batch,
    persist_generation_corrupt_terminal, persist_generation_status_changed,
    persist_generation_terminal, persist_locked_generation_completion,
    purge_expired_generation_task_trees,
};

mod lifecycle_gate;
pub use lifecycle_gate::{
    GenerationLifecycleGateError, GenerationLifecycleTask, LifecycleGateClaimState,
    PROJECT_GATE_ACTIVE, PendingLifecycleOperation, ProjectLifecycleGateClaim,
    ProjectLifecycleOperation, UserLifecycleGateClaim, active_generation_tasks,
    active_generation_tasks_for_project, active_generation_tasks_for_user,
    begin_project_lifecycle_gate, begin_user_disable_gate, complete_project_reset_gate,
    complete_user_disable_gate, pending_lifecycle_operations, restore_project_lifecycle_gate,
    restore_user_lifecycle_gate,
};
pub(crate) use lifecycle_gate::{
    PROJECT_GATE_DELETING, PROJECT_GATE_RESETTING, require_project_lifecycle_gate,
};

#[cfg(test)]
mod tests;

pub(crate) async fn delete_generation_spec_records_by_ids<C>(
    db: &C,
    generation_spec_ids: Vec<i64>,
    prompt_artifact_ids: Vec<i64>,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    if generation_spec_ids.is_empty() {
        return Ok(());
    }
    let mut provenance_update = media_provenance::Entity::update_many()
        .col_expr(
            media_provenance::Column::GenerationSpecId,
            Expr::value(Option::<i64>::None),
        )
        .filter(media_provenance::Column::GenerationSpecId.is_in(generation_spec_ids.clone()));
    if !prompt_artifact_ids.is_empty() {
        provenance_update = provenance_update.col_expr(
            media_provenance::Column::PromptArtifactId,
            Expr::value(Option::<i64>::None),
        );
    }
    provenance_update.exec(db).await?;
    if !prompt_artifact_ids.is_empty() {
        media_provenance::Entity::update_many()
            .col_expr(
                media_provenance::Column::PromptArtifactId,
                Expr::value(Option::<i64>::None),
            )
            .filter(media_provenance::Column::PromptArtifactId.is_in(prompt_artifact_ids))
            .exec(db)
            .await?;
    }
    generation_spec_media_ref::Entity::delete_many()
        .filter(
            generation_spec_media_ref::Column::GenerationSpecId.is_in(generation_spec_ids.clone()),
        )
        .exec(db)
        .await?;
    generation_spec_text_anchor::Entity::delete_many()
        .filter(
            generation_spec_text_anchor::Column::GenerationSpecId
                .is_in(generation_spec_ids.clone()),
        )
        .exec(db)
        .await?;
    generation_spec_entity_ref::Entity::delete_many()
        .filter(
            generation_spec_entity_ref::Column::GenerationSpecId.is_in(generation_spec_ids.clone()),
        )
        .exec(db)
        .await?;
    prompt_artifact::Entity::delete_many()
        .filter(prompt_artifact::Column::GenerationSpecId.is_in(generation_spec_ids.clone()))
        .exec(db)
        .await?;
    generation_spec::Entity::delete_many()
        .filter(generation_spec::Column::Id.is_in(generation_spec_ids))
        .exec(db)
        .await?;
    Ok(())
}
