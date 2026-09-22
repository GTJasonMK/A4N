pub use a4n_cleanup_contract::{
    CLEANUP_PAYLOAD_VERSION, CleanupClaimRecord, CleanupContractError, FileArtifactCleanupClaim,
    FileArtifactCleanupPayload, GenerationArtifactUnbindPayload, GenerationCleanupClaim,
    GenerationCleanupObservationOutcome, GenerationCleanupOutcome, GenerationCleanupPayload,
    GenerationCleanupRetryOutcome, GenerationCleanupScope, GenerationProviderJobCleanupOperation,
    GenerationProviderJobCleanupPayload, GenerationStagingArtifactDeletePayload,
    ProjectDeletionCleanupClaim, ProjectDeletionCleanupPayload,
};
use a4n_db_models::models::task::cleanup_outbox;
use chrono::Duration;
use sea_orm::DbErr;

pub const PROJECT_DELETE_CLEANUP_KIND: &str = "project_delete";
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingGenerationProviderJobCleanup {
    pub task_id: String,
    pub provider_job_id: String,
}

pub(crate) fn validated_project_deletion_cleanup_payload(
    project_id: i64,
    media_paths: Vec<String>,
) -> Result<ProjectDeletionCleanupPayload, DbErr> {
    let payload = ProjectDeletionCleanupPayload::new(project_id, media_paths);
    payload.validate().map_err(contract_error_to_db_error)?;
    Ok(payload)
}

#[derive(Debug, Clone)]
pub struct ClaimedProjectCleanup {
    pub record: cleanup_outbox::Model,
    pub payload: ProjectDeletionCleanupPayload,
}

#[derive(Debug, Clone)]
pub struct ClaimedGenerationCleanup {
    pub record: cleanup_outbox::Model,
    pub payload: GenerationCleanupPayload,
}

pub(crate) fn validated_file_artifact_cleanup_payload(
    project_id: i64,
    paths: Vec<String>,
) -> Result<FileArtifactCleanupPayload, DbErr> {
    let payload = FileArtifactCleanupPayload::new(project_id, paths);
    payload.validate().map_err(contract_error_to_db_error)?;
    Ok(payload)
}

#[derive(Debug, Clone)]
pub struct ClaimedFileArtifactCleanup {
    pub record: cleanup_outbox::Model,
    pub payload: FileArtifactCleanupPayload,
}

impl From<ClaimedProjectCleanup> for ProjectDeletionCleanupClaim {
    fn from(claim: ClaimedProjectCleanup) -> Self {
        Self {
            record: cleanup_claim_record(&claim.record),
            payload: claim.payload,
        }
    }
}

impl From<ClaimedFileArtifactCleanup> for FileArtifactCleanupClaim {
    fn from(claim: ClaimedFileArtifactCleanup) -> Self {
        Self {
            record: cleanup_claim_record(&claim.record),
            payload: claim.payload,
        }
    }
}

impl From<ClaimedGenerationCleanup> for GenerationCleanupClaim {
    fn from(claim: ClaimedGenerationCleanup) -> Self {
        Self {
            record: cleanup_claim_record(&claim.record),
            payload: claim.payload,
        }
    }
}

fn cleanup_claim_record(record: &cleanup_outbox::Model) -> CleanupClaimRecord {
    CleanupClaimRecord {
        id: record.id,
        project_id: record.project_id,
        attempt_count: record.attempt_count,
        lease_token: record.lease_token.clone(),
    }
}

fn contract_error_to_db_error(error: CleanupContractError) -> DbErr {
    DbErr::Custom(error.to_string())
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
            )
            .map_err(contract_error_to_db_error)?;
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
            let normalized = GenerationStagingArtifactDeletePayload::from_artifact_parts(
                true,
                payload.artifact_id.clone(),
                payload.sha256.clone(),
                payload.revision.clone(),
            )
            .map_err(contract_error_to_db_error)?;
            if normalized != payload {
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
            let normalized = GenerationArtifactUnbindPayload::from_artifact_parts(
                payload.artifact_id.clone(),
                payload.revision.clone(),
            )
            .map_err(contract_error_to_db_error)?;
            if normalized != payload {
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
    .map_err(contract_error_to_db_error)
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
