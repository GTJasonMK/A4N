use a4n_core::models::task::cleanup_outbox;
use a4n_service_protocol::GenerationArtifactRef;
use chrono::Duration;
use sea_orm::DbErr;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::Component;

pub const PROJECT_DELETE_CLEANUP_KIND: &str = "project_delete";
pub const MEDIA_WORKFLOW_CONTRACT_V2_RESET_KIND: &str = "media_workflow_contract_v2_reset";
pub const FILE_ARTIFACT_CLEANUP_KIND: &str = "file_artifact_cleanup";
pub const GENERATION_PROVIDER_JOB_CLEANUP_KIND: &str = "generation_provider_job_cleanup";
pub const GENERATION_STAGING_ARTIFACT_DELETE_KIND: &str = "generation_staging_artifact_delete";
pub const GENERATION_ARTIFACT_UNBIND_KIND: &str = "generation_artifact_unbind";
pub const CLEANUP_STATUS_PENDING: &str = "pending";
pub const CLEANUP_STATUS_RUNNING: &str = "running";
pub const CLEANUP_STATUS_COMPLETED: &str = "completed";
pub const CLEANUP_STATUS_FAILED: &str = "failed";
pub(super) const CLEANUP_LEASE_SECONDS: i64 = 300;
pub(crate) const CLEANUP_AUDIT_TTL: Duration = Duration::days(30);
const CLEANUP_PAYLOAD_VERSION: u32 = 1;

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
    pub(super) const fn as_str(self) -> &'static str {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationCleanupScope {
    pub requested_by_user_id: i64,
    pub project_id: Option<i64>,
    pub task_id: String,
    pub attempt: u32,
}

impl GenerationCleanupScope {
    pub fn validate(&self) -> Result<(), DbErr> {
        if self.requested_by_user_id <= 0 {
            return Err(DbErr::Custom(
                "generation cleanup requested_by_user_id 必须为正整数".to_string(),
            ));
        }
        if self.project_id.is_some_and(|project_id| project_id <= 0) {
            return Err(DbErr::Custom(
                "generation cleanup project_id 必须为正整数".to_string(),
            ));
        }
        if self.task_id.trim().is_empty() || self.attempt == 0 {
            return Err(DbErr::Custom(
                "generation cleanup task_id 不能为空且 attempt 必须大于 0".to_string(),
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
    ) -> Result<Self, DbErr> {
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
    pub fn from_artifact(artifact: &GenerationArtifactRef) -> Result<Self, DbErr> {
        if !artifact.temporary {
            return Err(DbErr::Custom(
                "staging artifact cleanup 只接受 temporary artifact".to_string(),
            ));
        }
        validate_sha256(&artifact.sha256, "artifact.sha256")?;
        Ok(Self {
            version: CLEANUP_PAYLOAD_VERSION,
            artifact_id: required_text(artifact.artifact_id.clone(), "artifact_id", 256)?,
            sha256: artifact.sha256.clone(),
            revision: required_text(artifact.revision.clone(), "artifact.revision", 256)?,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenerationCleanupPayload {
    ProviderJob(GenerationProviderJobCleanupPayload),
    StagingArtifactDelete(GenerationStagingArtifactDeletePayload),
    ArtifactUnbind(GenerationArtifactUnbindPayload),
}

#[derive(Debug, Clone)]
pub struct ClaimedGenerationCleanup {
    pub record: cleanup_outbox::Model,
    pub payload: GenerationCleanupPayload,
}

impl GenerationArtifactUnbindPayload {
    pub fn from_artifact(artifact: &GenerationArtifactRef) -> Result<Self, DbErr> {
        Ok(Self {
            version: CLEANUP_PAYLOAD_VERSION,
            artifact_id: required_text(artifact.artifact_id.clone(), "artifact_id", 256)?,
            revision: required_text(artifact.revision.clone(), "artifact.revision", 256)?,
        })
    }
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
        let media_paths = normalized_paths(media_paths);
        Self {
            version: CLEANUP_PAYLOAD_VERSION,
            project_id,
            media_paths,
        }
    }
}

pub(crate) fn validated_project_deletion_cleanup_payload(
    project_id: i64,
    media_paths: Vec<String>,
) -> Result<ProjectDeletionCleanupPayload, DbErr> {
    let payload = ProjectDeletionCleanupPayload::new(project_id, media_paths);
    validate_project_scoped_paths(payload.project_id, &payload.media_paths, "项目删除 cleanup")?;
    Ok(payload)
}

#[derive(Debug, Clone)]
pub struct ClaimedProjectCleanup {
    pub record: cleanup_outbox::Model,
    pub payload: ProjectDeletionCleanupPayload,
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

#[derive(Debug, Clone)]
pub struct ClaimedMediaWorkflowV2Reset {
    pub record: cleanup_outbox::Model,
    pub payload: MediaWorkflowV2ResetPayload,
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
        let paths = normalized_paths(paths);
        Self {
            version: CLEANUP_PAYLOAD_VERSION,
            project_id,
            paths,
        }
    }
}

pub(crate) fn validated_file_artifact_cleanup_payload(
    project_id: i64,
    paths: Vec<String>,
) -> Result<FileArtifactCleanupPayload, DbErr> {
    let payload = FileArtifactCleanupPayload::new(project_id, paths);
    validate_project_scoped_paths(payload.project_id, &payload.paths, "文件 artifact cleanup")?;
    Ok(payload)
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
) -> Result<(), DbErr> {
    if project_id <= 0 {
        return Err(DbErr::Custom(format!(
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
        return Err(DbErr::Custom(format!(
            "{payload_label} 路径不属于项目 {project_id}: {path}"
        )));
    }
    Ok(())
}

#[derive(Debug, Clone)]
pub struct ClaimedFileArtifactCleanup {
    pub record: cleanup_outbox::Model,
    pub payload: FileArtifactCleanupPayload,
}

pub(super) fn parse_project_deletion_payload(
    record: &cleanup_outbox::Model,
) -> Result<ProjectDeletionCleanupPayload, DbErr> {
    validate_record_payload_hash(record)?;
    let payload: ProjectDeletionCleanupPayload =
        serde_json::from_value(record.payload_json.clone()).map_err(|error| {
            DbErr::Custom(format!("解析项目删除 cleanup payload 失败: {error}"))
        })?;
    if payload.version != CLEANUP_PAYLOAD_VERSION
        || Some(payload.project_id) != record.project_id
        || record.payload_version != CLEANUP_PAYLOAD_VERSION as i32
    {
        return Err(DbErr::Custom(format!(
            "项目删除 cleanup payload 合同不一致: id={}",
            record.id
        )));
    }
    validated_project_deletion_cleanup_payload(payload.project_id, payload.media_paths).map_err(
        |error| {
            DbErr::Custom(format!(
                "项目删除 cleanup payload 路径无效: id={}：{error}",
                record.id
            ))
        },
    )
}

pub(super) fn parse_media_workflow_v2_reset_payload(
    record: &cleanup_outbox::Model,
) -> Result<MediaWorkflowV2ResetPayload, DbErr> {
    validate_record_payload_hash(record)?;
    let payload: MediaWorkflowV2ResetPayload = serde_json::from_value(record.payload_json.clone())
        .map_err(|error| DbErr::Custom(format!("解析媒体工作流 v2 reset payload 失败: {error}")))?;
    if payload.version != CLEANUP_PAYLOAD_VERSION
        || Some(payload.project_id) != record.project_id
        || record.payload_version != CLEANUP_PAYLOAD_VERSION as i32
    {
        return Err(DbErr::Custom(format!(
            "媒体工作流 v2 reset payload 合同不一致: id={}",
            record.id
        )));
    }
    Ok(payload)
}

pub(super) fn parse_file_artifact_cleanup_payload(
    record: &cleanup_outbox::Model,
) -> Result<FileArtifactCleanupPayload, DbErr> {
    validate_record_payload_hash(record)?;
    let payload: FileArtifactCleanupPayload = serde_json::from_value(record.payload_json.clone())
        .map_err(|error| {
        DbErr::Custom(format!("解析文件 artifact cleanup payload 失败: {error}"))
    })?;
    if payload.version != CLEANUP_PAYLOAD_VERSION
        || Some(payload.project_id) != record.project_id
        || record.payload_version != CLEANUP_PAYLOAD_VERSION as i32
    {
        return Err(DbErr::Custom(format!(
            "文件 artifact cleanup payload 合同不一致: id={}",
            record.id
        )));
    }
    validated_file_artifact_cleanup_payload(payload.project_id, payload.paths).map_err(|error| {
        DbErr::Custom(format!(
            "文件 artifact cleanup payload 路径无效: id={}：{error}",
            record.id
        ))
    })
}

pub(super) fn parse_generation_cleanup_payload(
    record: &cleanup_outbox::Model,
) -> Result<GenerationCleanupPayload, DbErr> {
    validate_record_payload_hash(record)?;
    validate_generation_cleanup_scope(record)?;
    if record.payload_version != CLEANUP_PAYLOAD_VERSION as i32 {
        return Err(DbErr::Custom(format!(
            "generation cleanup payload version 不受支持: id={}, version={}",
            record.id, record.payload_version
        )));
    }

    match record.cleanup_kind.as_str() {
        GENERATION_PROVIDER_JOB_CLEANUP_KIND => {
            let payload: GenerationProviderJobCleanupPayload =
                serde_json::from_value(record.payload_json.clone()).map_err(|error| {
                    DbErr::Custom(format!(
                        "解析 generation provider job cleanup payload 失败: {error}"
                    ))
                })?;
            let normalized = GenerationProviderJobCleanupPayload::new(
                payload.operation,
                payload.provider_kind.clone(),
                payload.provider_job_id.clone(),
                payload.provider_idempotency_key_sha256.clone(),
            )?;
            if normalized != payload {
                return Err(DbErr::Custom(format!(
                    "generation provider job cleanup payload 合同不一致: id={}",
                    record.id
                )));
            }
            Ok(GenerationCleanupPayload::ProviderJob(payload))
        }
        GENERATION_STAGING_ARTIFACT_DELETE_KIND => {
            let payload: GenerationStagingArtifactDeletePayload =
                serde_json::from_value(record.payload_json.clone()).map_err(|error| {
                    DbErr::Custom(format!(
                        "解析 generation staging artifact cleanup payload 失败: {error}"
                    ))
                })?;
            validate_sha256(&payload.sha256, "artifact.sha256")?;
            if payload.version != CLEANUP_PAYLOAD_VERSION
                || required_text(payload.artifact_id.clone(), "artifact_id", 256)?
                    != payload.artifact_id
                || required_text(payload.revision.clone(), "artifact.revision", 256)?
                    != payload.revision
            {
                return Err(DbErr::Custom(format!(
                    "generation staging artifact cleanup payload 合同不一致: id={}",
                    record.id
                )));
            }
            Ok(GenerationCleanupPayload::StagingArtifactDelete(payload))
        }
        GENERATION_ARTIFACT_UNBIND_KIND => {
            let payload: GenerationArtifactUnbindPayload =
                serde_json::from_value(record.payload_json.clone()).map_err(|error| {
                    DbErr::Custom(format!(
                        "解析 generation artifact unbind payload 失败: {error}"
                    ))
                })?;
            if payload.version != CLEANUP_PAYLOAD_VERSION
                || required_text(payload.artifact_id.clone(), "artifact_id", 256)?
                    != payload.artifact_id
                || required_text(payload.revision.clone(), "artifact.revision", 256)?
                    != payload.revision
            {
                return Err(DbErr::Custom(format!(
                    "generation artifact unbind payload 合同不一致: id={}",
                    record.id
                )));
            }
            Ok(GenerationCleanupPayload::ArtifactUnbind(payload))
        }
        kind => Err(DbErr::Custom(format!(
            "未知 generation cleanup kind: id={}, kind={kind}",
            record.id
        ))),
    }
}

fn validate_generation_cleanup_scope(record: &cleanup_outbox::Model) -> Result<(), DbErr> {
    let attempt = record
        .attempt
        .and_then(|attempt| u32::try_from(attempt).ok())
        .ok_or_else(|| {
            DbErr::Custom(format!("generation cleanup attempt 无效: id={}", record.id))
        })?;
    GenerationCleanupScope {
        requested_by_user_id: record.requested_by_user_id.ok_or_else(|| {
            DbErr::Custom(format!(
                "generation cleanup 缺少 requested_by_user_id: id={}",
                record.id
            ))
        })?,
        project_id: record.project_id,
        task_id: record.task_id.clone().ok_or_else(|| {
            DbErr::Custom(format!("generation cleanup 缺少 task_id: id={}", record.id))
        })?,
        attempt,
    }
    .validate()
}

fn validate_record_payload_hash(record: &cleanup_outbox::Model) -> Result<(), DbErr> {
    let actual = crate::generation::canonical_json_sha256(&record.payload_json)
        .map_err(|error| DbErr::Custom(format!("cleanup payload canonical hash 失败: {error}")))?;
    if actual == record.payload_sha256 {
        Ok(())
    } else {
        Err(DbErr::Custom(format!(
            "cleanup payload SHA-256 不匹配: id={}",
            record.id
        )))
    }
}

fn required_text(value: String, label: &str, max_bytes: usize) -> Result<String, DbErr> {
    let value = value.trim().to_string();
    if value.is_empty() || value.len() > max_bytes {
        Err(DbErr::Custom(format!(
            "generation cleanup {label} 必须在 1..={max_bytes} 字节内"
        )))
    } else {
        Ok(value)
    }
}

fn validate_sha256(value: &str, label: &str) -> Result<(), DbErr> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        Ok(())
    } else {
        Err(DbErr::Custom(format!(
            "generation cleanup {label} 必须是 64 位小写 SHA-256 hex"
        )))
    }
}
