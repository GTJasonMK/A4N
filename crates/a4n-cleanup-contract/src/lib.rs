//! Narrow contract for the cleanup outbox application boundary.
//!
//! The database owns ORM rows and transitions. Runtime receives only these
//! pure claims and asks the owner to perform lease transitions.

use a4n_error::AppError;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Component};
use thiserror::Error;

pub const CLEANUP_PAYLOAD_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{0}")]
pub struct CleanupContractError(String);

impl CleanupContractError {
    fn invalid(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationCleanupOutcome {
    Cancelled,
    Deleted,
    Unbound,
    NotFound,
    Unsupported,
    RemoteFailed,
    DeleteScheduled,
}

impl GenerationCleanupOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cancelled => "cancelled",
            Self::Deleted => "deleted",
            Self::Unbound => "unbound",
            Self::NotFound => "not_found",
            Self::Unsupported => "unsupported",
            Self::RemoteFailed => "remote_failed",
            Self::DeleteScheduled => "delete_scheduled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationCleanupRetryOutcome {
    Scheduled,
    Exhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationCleanupObservationOutcome {
    Rescheduled,
    DeadlineExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationCleanupScope {
    pub requested_by_user_id: i64,
    pub project_id: Option<i64>,
    pub task_id: String,
    pub attempt: u32,
}

impl GenerationCleanupScope {
    pub fn validate(&self) -> Result<(), CleanupContractError> {
        if self.requested_by_user_id <= 0 {
            return Err(CleanupContractError::invalid(
                "generation cleanup requested_by_user_id 必须为正整数",
            ));
        }
        if self.project_id.is_some_and(|project_id| project_id <= 0) {
            return Err(CleanupContractError::invalid(
                "generation cleanup project_id 必须为正整数",
            ));
        }
        if self.task_id.trim().is_empty() || self.attempt == 0 {
            return Err(CleanupContractError::invalid(
                "generation cleanup task_id 不能为空且 attempt 必须大于 0",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GenerationProviderJobCleanupOperation {
    Cancel,
    Delete,
    Reconcile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationProviderJobCleanupPayload {
    pub version: u32,
    pub operation: GenerationProviderJobCleanupOperation,
    pub provider_kind: String,
    pub provider_job_id: String,
    pub provider_idempotency_key_sha256: String,
}

impl GenerationProviderJobCleanupPayload {
    pub fn new(
        operation: GenerationProviderJobCleanupOperation,
        provider_kind: String,
        provider_job_id: String,
        provider_idempotency_key_sha256: String,
    ) -> Result<Self, CleanupContractError> {
        let payload = Self {
            version: CLEANUP_PAYLOAD_VERSION,
            operation,
            provider_kind: required_text(provider_kind, "provider_kind", 128)?,
            provider_job_id: required_text(provider_job_id, "provider_job_id", 256)?,
            provider_idempotency_key_sha256,
        };
        validate_sha256(
            &payload.provider_idempotency_key_sha256,
            "provider_idempotency_key_sha256",
        )?;
        Ok(payload)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationStagingArtifactDeletePayload {
    pub version: u32,
    pub artifact_id: String,
    pub sha256: String,
    pub revision: String,
}

impl GenerationStagingArtifactDeletePayload {
    pub fn from_artifact_parts(
        temporary: bool,
        artifact_id: String,
        sha256: String,
        revision: String,
    ) -> Result<Self, CleanupContractError> {
        if !temporary {
            return Err(CleanupContractError::invalid(
                "staging artifact cleanup 只接受 temporary artifact",
            ));
        }
        validate_sha256(&sha256, "artifact.sha256")?;
        Ok(Self {
            version: CLEANUP_PAYLOAD_VERSION,
            artifact_id: required_text(artifact_id, "artifact_id", 256)?,
            sha256,
            revision: required_text(revision, "artifact.revision", 256)?,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationArtifactUnbindPayload {
    pub version: u32,
    pub artifact_id: String,
    pub revision: String,
}

impl GenerationArtifactUnbindPayload {
    pub fn from_artifact_parts(
        artifact_id: String,
        revision: String,
    ) -> Result<Self, CleanupContractError> {
        Ok(Self {
            version: CLEANUP_PAYLOAD_VERSION,
            artifact_id: required_text(artifact_id, "artifact_id", 256)?,
            revision: required_text(revision, "artifact.revision", 256)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerationCleanupPayload {
    ProviderJob(GenerationProviderJobCleanupPayload),
    StagingArtifactDelete(GenerationStagingArtifactDeletePayload),
    ArtifactUnbind(GenerationArtifactUnbindPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectDeletionCleanupPayload {
    pub version: u32,
    pub project_id: i64,
    pub media_paths: Vec<String>,
}

impl ProjectDeletionCleanupPayload {
    pub fn new(project_id: i64, media_paths: Vec<String>) -> Self {
        Self {
            version: CLEANUP_PAYLOAD_VERSION,
            project_id,
            media_paths: normalized_paths(media_paths),
        }
    }

    pub fn validate(&self) -> Result<(), CleanupContractError> {
        validate_project_scoped_paths(self.project_id, &self.media_paths, "项目删除 cleanup")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MediaWorkflowV2ResetPayload {
    pub version: u32,
    pub project_id: i64,
}

impl MediaWorkflowV2ResetPayload {
    pub fn new(project_id: i64) -> Self {
        Self {
            version: CLEANUP_PAYLOAD_VERSION,
            project_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FileArtifactCleanupPayload {
    pub version: u32,
    pub project_id: i64,
    pub paths: Vec<String>,
}

impl FileArtifactCleanupPayload {
    pub fn new(project_id: i64, paths: Vec<String>) -> Self {
        Self {
            version: CLEANUP_PAYLOAD_VERSION,
            project_id,
            paths: normalized_paths(paths),
        }
    }

    pub fn validate(&self) -> Result<(), CleanupContractError> {
        validate_project_scoped_paths(self.project_id, &self.paths, "文件 artifact cleanup")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupClaimRecord {
    pub id: i64,
    pub project_id: Option<i64>,
    pub attempt_count: i32,
    pub lease_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectDeletionCleanupClaim {
    pub record: CleanupClaimRecord,
    pub payload: ProjectDeletionCleanupPayload,
}

#[derive(Debug, Clone)]
pub struct MediaWorkflowV2ResetClaim {
    pub record: CleanupClaimRecord,
    pub payload: MediaWorkflowV2ResetPayload,
}

#[derive(Debug, Clone)]
pub struct FileArtifactCleanupClaim {
    pub record: CleanupClaimRecord,
    pub payload: FileArtifactCleanupPayload,
}

#[derive(Debug, Clone)]
pub struct GenerationCleanupClaim {
    pub record: CleanupClaimRecord,
    pub payload: GenerationCleanupPayload,
}

#[async_trait]
pub trait CleanupOutboxApplicationPort: Send + Sync {
    async fn insert_file_artifact_cleanup_intent(
        &self,
        project_id: i64,
        dedupe_key: String,
        paths: Vec<String>,
    ) -> Result<(), AppError>;

    async fn claim_project_deletion_cleanup(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<ProjectDeletionCleanupClaim>, AppError>;

    async fn claim_media_workflow_v2_reset(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<MediaWorkflowV2ResetClaim>, AppError>;

    async fn claim_file_artifact_cleanup(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<FileArtifactCleanupClaim>, AppError>;

    async fn claim_generation_cleanup(
        &self,
        project_id: Option<i64>,
    ) -> Result<Option<GenerationCleanupClaim>, AppError>;

    async fn project_ids_with_incomplete_cleanup(&self) -> Result<Vec<i64>, AppError>;

    async fn purge_completed_cleanup_audits(&self, limit: usize) -> Result<u64, AppError>;

    async fn reset_media_workflows_to_v2(
        &self,
        project_id: i64,
        cleanup_intent_id: i64,
    ) -> Result<(), AppError>;

    async fn complete_claimed_cleanup(&self, id: i64, lease_token: &str) -> Result<(), AppError>;

    async fn complete_claimed_generation_cleanup(
        &self,
        id: i64,
        lease_token: &str,
        outcome: GenerationCleanupOutcome,
    ) -> Result<(), AppError>;

    async fn observe_claimed_generation_reconcile(
        &self,
        id: i64,
        lease_token: &str,
        retry_after_seconds: i64,
    ) -> Result<GenerationCleanupObservationOutcome, AppError>;

    async fn schedule_provider_delete_from_reconcile(
        &self,
        id: i64,
        lease_token: &str,
    ) -> Result<(), AppError>;

    async fn renew_claimed_cleanup(&self, id: i64, lease_token: &str) -> Result<(), AppError>;

    async fn retry_claimed_cleanup(
        &self,
        id: i64,
        lease_token: &str,
        error: String,
        retry_after_seconds: i64,
    ) -> Result<(), AppError>;

    async fn retry_or_fail_claimed_generation_cleanup(
        &self,
        id: i64,
        lease_token: &str,
        error: String,
    ) -> Result<GenerationCleanupRetryOutcome, AppError>;
}

fn normalized_paths(paths: Vec<String>) -> Vec<String> {
    paths
        .into_iter()
        .map(|path| path.trim().to_string())
        .filter(|path| !path.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn validate_project_scoped_paths(
    project_id: i64,
    paths: &[String],
    payload_label: &str,
) -> Result<(), CleanupContractError> {
    if project_id <= 0 {
        return Err(CleanupContractError::invalid(format!(
            "{payload_label} project_id 必须为正整数"
        )));
    }
    let prefix = format!("projects/{project_id}/");
    if let Some(path) = paths.iter().find(|path| {
        !path.starts_with(&prefix)
            || path.len() == prefix.len()
            || path.contains('\\')
            || std::path::Path::new(path)
                .components()
                .any(|component| !matches!(component, Component::Normal(_)))
    }) {
        return Err(CleanupContractError::invalid(format!(
            "{payload_label} 路径不属于项目 {project_id}: {path}"
        )));
    }
    Ok(())
}

fn required_text(
    value: String,
    label: &str,
    max_bytes: usize,
) -> Result<String, CleanupContractError> {
    let value = value.trim().to_string();
    if value.is_empty() || value.len() > max_bytes {
        Err(CleanupContractError::invalid(format!(
            "generation cleanup {label} 必须在 1..={max_bytes} 字节内"
        )))
    } else {
        Ok(value)
    }
}

fn validate_sha256(value: &str, label: &str) -> Result<(), CleanupContractError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        Ok(())
    } else {
        Err(CleanupContractError::invalid(format!(
            "generation cleanup {label} 必须是 64 位小写 SHA-256 hex"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payloads_normalize_and_validate_project_paths() {
        let payload = FileArtifactCleanupPayload::new(
            42,
            vec![
                " projects/42/media/b.png ".to_string(),
                "projects/42/media/a.png".to_string(),
                "projects/42/media/a.png".to_string(),
            ],
        );
        assert_eq!(payload.paths.len(), 2);
        assert!(payload.validate().is_ok());
        assert!(
            FileArtifactCleanupPayload::new(42, vec!["projects/43/media/a.png".to_string()],)
                .validate()
                .is_err()
        );
    }

    #[test]
    fn generation_payload_constructors_reject_invalid_identity() {
        assert!(
            GenerationProviderJobCleanupPayload::new(
                GenerationProviderJobCleanupOperation::Cancel,
                "video".to_string(),
                "job".to_string(),
                "bad".to_string(),
            )
            .is_err()
        );
        assert!(
            GenerationStagingArtifactDeletePayload::from_artifact_parts(
                false,
                "id".to_string(),
                "a".repeat(64),
                "revision".to_string(),
            )
            .is_err()
        );
    }
}
