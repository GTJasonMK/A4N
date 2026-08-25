mod claim;
mod contracts;
mod intent;
mod retention;
mod transitions;

#[cfg(test)]
mod tests;

pub use claim::{
    claim_file_artifact_cleanup, claim_generation_cleanup, claim_media_workflow_v2_reset,
    claim_project_deletion_cleanup, media_workflow_v2_reset_pending,
    project_ids_with_incomplete_cleanup,
};
pub(crate) use contracts::{CLEANUP_AUDIT_TTL, validated_file_artifact_cleanup_payload};
pub use contracts::{
    CLEANUP_STATUS_COMPLETED, CLEANUP_STATUS_FAILED, CLEANUP_STATUS_PENDING,
    CLEANUP_STATUS_RUNNING, ClaimedFileArtifactCleanup, ClaimedGenerationCleanup,
    ClaimedMediaWorkflowV2Reset, ClaimedProjectCleanup, FILE_ARTIFACT_CLEANUP_KIND,
    FileArtifactCleanupPayload, GENERATION_ARTIFACT_UNBIND_KIND,
    GENERATION_PROVIDER_JOB_CLEANUP_KIND, GENERATION_STAGING_ARTIFACT_DELETE_KIND,
    GenerationArtifactUnbindPayload, GenerationCleanupOutcome, GenerationCleanupPayload,
    GenerationCleanupScope, GenerationProviderJobCleanupOperation,
    GenerationProviderJobCleanupPayload, GenerationStagingArtifactDeletePayload,
    MEDIA_WORKFLOW_CONTRACT_V2_RESET_KIND, MediaWorkflowV2ResetPayload,
    PROJECT_DELETE_CLEANUP_KIND, ProjectDeletionCleanupPayload,
};
pub use intent::{
    insert_file_artifact_cleanup_intent, insert_generation_artifact_unbind_intent,
    insert_generation_provider_job_cleanup_intent,
    insert_generation_staging_artifact_delete_intent, insert_media_workflow_v2_reset_intent,
    insert_media_workflow_v2_reset_intents, insert_project_deletion_cleanup_intent,
};
pub use retention::purge_completed_cleanup_audits;
pub use transitions::{
    GenerationCleanupObservationOutcome, GenerationCleanupRetryOutcome, complete_claimed_cleanup,
    complete_claimed_generation_cleanup, fail_claimed_cleanup,
    observe_claimed_generation_reconcile, renew_claimed_cleanup, retry_claimed_cleanup,
    retry_or_fail_claimed_generation_cleanup, schedule_provider_delete_from_reconcile,
};
