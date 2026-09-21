use super::contracts::{
    CLEANUP_LEASE_SECONDS, CLEANUP_STATUS_FAILED, CLEANUP_STATUS_PENDING, CLEANUP_STATUS_RUNNING,
    ClaimedFileArtifactCleanup, ClaimedGenerationCleanup, ClaimedMediaWorkflowV2Reset,
    ClaimedProjectCleanup, FILE_ARTIFACT_CLEANUP_KIND, GENERATION_ARTIFACT_UNBIND_KIND,
    GENERATION_PROVIDER_JOB_CLEANUP_KIND, GENERATION_STAGING_ARTIFACT_DELETE_KIND,
    GenerationCleanupPayload, MEDIA_WORKFLOW_CONTRACT_V2_RESET_KIND, PROJECT_DELETE_CLEANUP_KIND,
    PendingGenerationProviderJobCleanup, parse_file_artifact_cleanup_payload,
    parse_generation_cleanup_payload, parse_media_workflow_v2_reset_payload,
    parse_project_deletion_payload,
};
use super::transitions::fail_claimed_cleanup;
use a4n_db_models::models::task::cleanup_outbox;
use chrono::{Duration, Utc};
use sea_orm::sea_query::{LockBehavior, LockType};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DbErr, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
};

pub async fn claim_project_deletion_cleanup(
    db: &(impl ConnectionTrait + TransactionTrait),
    project_id: Option<i64>,
) -> Result<Option<ClaimedProjectCleanup>, DbErr> {
    loop {
        let Some(record) =
            claim_cleanup_by_kind(db, &[PROJECT_DELETE_CLEANUP_KIND], project_id, true).await?
        else {
            return Ok(None);
        };
        match parse_project_deletion_payload(&record) {
            Ok(payload) => return Ok(Some(ClaimedProjectCleanup { record, payload })),
            Err(error) => {
                fail_claimed_cleanup(
                    db,
                    record.id,
                    required_lease_token(&record)?,
                    error.to_string(),
                )
                .await?;
            }
        }
    }
}

pub async fn claim_media_workflow_v2_reset(
    db: &(impl ConnectionTrait + TransactionTrait),
    project_id: Option<i64>,
) -> Result<Option<ClaimedMediaWorkflowV2Reset>, DbErr> {
    loop {
        let Some(record) = claim_cleanup_by_kind(
            db,
            &[MEDIA_WORKFLOW_CONTRACT_V2_RESET_KIND],
            project_id,
            true,
        )
        .await?
        else {
            return Ok(None);
        };
        match parse_media_workflow_v2_reset_payload(&record) {
            Ok(payload) => return Ok(Some(ClaimedMediaWorkflowV2Reset { record, payload })),
            Err(error) => {
                fail_claimed_cleanup(
                    db,
                    record.id,
                    required_lease_token(&record)?,
                    error.to_string(),
                )
                .await?;
            }
        }
    }
}

pub async fn claim_file_artifact_cleanup(
    db: &(impl ConnectionTrait + TransactionTrait),
    project_id: Option<i64>,
) -> Result<Option<ClaimedFileArtifactCleanup>, DbErr> {
    loop {
        let Some(record) =
            claim_cleanup_by_kind(db, &[FILE_ARTIFACT_CLEANUP_KIND], project_id, true).await?
        else {
            return Ok(None);
        };
        match parse_file_artifact_cleanup_payload(&record) {
            Ok(payload) => return Ok(Some(ClaimedFileArtifactCleanup { record, payload })),
            Err(error) => {
                fail_claimed_cleanup(
                    db,
                    record.id,
                    required_lease_token(&record)?,
                    error.to_string(),
                )
                .await?;
            }
        }
    }
}

pub async fn claim_generation_cleanup(
    db: &(impl ConnectionTrait + TransactionTrait),
    project_id: Option<i64>,
) -> Result<Option<ClaimedGenerationCleanup>, DbErr> {
    loop {
        let Some(record) = claim_cleanup_by_kind(
            db,
            &[
                GENERATION_PROVIDER_JOB_CLEANUP_KIND,
                GENERATION_STAGING_ARTIFACT_DELETE_KIND,
                GENERATION_ARTIFACT_UNBIND_KIND,
            ],
            project_id,
            false,
        )
        .await?
        else {
            return Ok(None);
        };
        match parse_generation_cleanup_payload(&record) {
            Ok(payload) => return Ok(Some(ClaimedGenerationCleanup { record, payload })),
            Err(error) => {
                fail_claimed_cleanup(
                    db,
                    record.id,
                    required_lease_token(&record)?,
                    error.to_string(),
                )
                .await?;
            }
        }
    }
}

/// 返回仍可能占用远端 provider 资源的 generation cleanup intent。
///
/// 终态 generation 仍可能有未完成的 cancel/reconcile/delete；生命周期停用
/// 必须先看到这些 intent 收敛，不能只扫描活动 checkpoint。
pub async fn pending_generation_provider_job_cleanups<C>(
    db: &C,
) -> Result<Vec<PendingGenerationProviderJobCleanup>, DbErr>
where
    C: ConnectionTrait,
{
    let records = cleanup_outbox::Entity::find()
        .filter(cleanup_outbox::Column::CleanupKind.eq(GENERATION_PROVIDER_JOB_CLEANUP_KIND))
        .filter(cleanup_outbox::Column::Status.is_in([
            CLEANUP_STATUS_PENDING,
            CLEANUP_STATUS_RUNNING,
            CLEANUP_STATUS_FAILED,
        ]))
        .order_by_asc(cleanup_outbox::Column::Id)
        .all(db)
        .await?;

    records
        .into_iter()
        .map(|record| {
            let task_id = record.task_id.clone().ok_or_else(|| {
                DbErr::Custom(format!(
                    "provider job cleanup 缺少 task_id: id={}",
                    record.id
                ))
            })?;
            let GenerationCleanupPayload::ProviderJob(payload) =
                parse_generation_cleanup_payload(&record)?
            else {
                return Err(DbErr::Custom(format!(
                    "provider job cleanup payload 类型错误: id={}",
                    record.id
                )));
            };
            Ok(PendingGenerationProviderJobCleanup {
                task_id,
                provider_job_id: payload.provider_job_id,
            })
        })
        .collect()
}

async fn claim_cleanup_by_kind(
    db: &(impl ConnectionTrait + TransactionTrait),
    cleanup_kinds: &[&str],
    project_id: Option<i64>,
    count_attempt: bool,
) -> Result<Option<cleanup_outbox::Model>, DbErr> {
    let tx = db.begin().await?;
    let now = Utc::now().naive_utc();
    let due = Condition::any()
        .add(
            Condition::all()
                .add(cleanup_outbox::Column::Status.eq(CLEANUP_STATUS_PENDING))
                .add(cleanup_outbox::Column::NextAttemptAt.lte(now)),
        )
        .add(
            Condition::all()
                .add(cleanup_outbox::Column::Status.eq(CLEANUP_STATUS_RUNNING))
                .add(cleanup_outbox::Column::LeaseExpiresAt.lte(now)),
        );
    let mut query = cleanup_outbox::Entity::find()
        .filter(cleanup_outbox::Column::CleanupKind.is_in(cleanup_kinds.iter().copied()))
        .filter(due)
        .order_by_asc(cleanup_outbox::Column::NextAttemptAt)
        .order_by_asc(cleanup_outbox::Column::Id)
        .lock_with_behavior(LockType::Update, LockBehavior::SkipLocked);
    if let Some(project_id) = project_id {
        query = query.filter(cleanup_outbox::Column::ProjectId.eq(project_id));
    }
    let Some(record) = query.one(&tx).await? else {
        tx.commit().await?;
        return Ok(None);
    };
    let lease_token = uuid::Uuid::new_v4().to_string();
    let attempt_count = if count_attempt {
        record
            .attempt_count
            .checked_add(1)
            .ok_or_else(|| DbErr::Custom("清理任务 attempt_count 超出 i32 范围".to_string()))?
    } else {
        record.attempt_count
    };
    let mut active: cleanup_outbox::ActiveModel = record.into();
    active.status = Set(CLEANUP_STATUS_RUNNING.to_string());
    active.attempt_count = Set(attempt_count);
    active.lease_token = Set(Some(lease_token));
    active.lease_expires_at = Set(Some(now + Duration::seconds(CLEANUP_LEASE_SECONDS)));
    active.updated_at = Set(now);
    let record = active.update(&tx).await?;
    tx.commit().await?;
    Ok(Some(record))
}

fn required_lease_token(record: &cleanup_outbox::Model) -> Result<&str, DbErr> {
    record.lease_token.as_deref().ok_or_else(|| {
        DbErr::Custom(format!(
            "已 claim 的 cleanup outbox 缺少 lease_token: id={}",
            record.id
        ))
    })
}

pub async fn media_workflow_v2_reset_pending<C>(db: &C, project_id: i64) -> Result<bool, DbErr>
where
    C: ConnectionTrait,
{
    Ok(cleanup_outbox::Entity::find()
        .select_only()
        .column(cleanup_outbox::Column::Id)
        .filter(cleanup_outbox::Column::ProjectId.eq(project_id))
        .filter(cleanup_outbox::Column::CleanupKind.eq(MEDIA_WORKFLOW_CONTRACT_V2_RESET_KIND))
        .filter(cleanup_outbox::Column::Status.is_in([
            CLEANUP_STATUS_PENDING,
            CLEANUP_STATUS_RUNNING,
            CLEANUP_STATUS_FAILED,
        ]))
        .into_tuple::<i64>()
        .one(db)
        .await?
        .is_some())
}

pub async fn project_ids_with_incomplete_cleanup<C>(db: &C) -> Result<Vec<i64>, DbErr>
where
    C: ConnectionTrait,
{
    cleanup_outbox::Entity::find()
        .select_only()
        .column(cleanup_outbox::Column::ProjectId)
        .filter(cleanup_outbox::Column::ProjectId.is_not_null())
        .filter(cleanup_outbox::Column::Status.is_in([
            CLEANUP_STATUS_PENDING,
            CLEANUP_STATUS_RUNNING,
            CLEANUP_STATUS_FAILED,
        ]))
        .distinct()
        .order_by_asc(cleanup_outbox::Column::ProjectId)
        .into_tuple::<Option<i64>>()
        .all(db)
        .await
        .map(|rows| rows.into_iter().flatten().collect())
}
